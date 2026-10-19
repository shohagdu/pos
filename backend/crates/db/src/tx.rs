use std::ops::{Deref, DerefMut};

use pos_core::{AppResult, PlatformCtx, TenantCtx};
use sqlx::{MySql, MySqlConnection, MySqlPool, Transaction};
use uuid::Uuid;

use crate::map_db_error;

/// A transaction bound to one tenant.
///
/// Repositories take `&mut TenantTx`, read [`TenantTx::tenant_id`] and bind it
/// into every statement (`WHERE tenant_id = ?`). The guard triggers reject any
/// write for a different tenant. Dropping without `commit` rolls back.
pub struct TenantTx {
    inner: Transaction<'static, MySql>,
    tenant_id: Uuid,
    user_id: Uuid,
    branch_id: Option<Uuid>,
}

impl TenantTx {
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// The branch the request works in (`X-Branch-Id`, already checked
    /// against the member's branch scope); `None` = the default branch.
    pub fn branch_id(&self) -> Option<Uuid> {
        self.branch_id
    }

    pub fn set_branch(&mut self, branch_id: Option<Uuid>) {
        self.branch_id = branch_id;
    }

    pub async fn commit(self) -> AppResult<()> {
        self.inner.commit().await.map_err(map_db_error)
    }
}

impl Deref for TenantTx {
    type Target = MySqlConnection;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for TenantTx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens a tenant transaction. This must be the only way request handlers and
/// jobs reach tenant data.
pub async fn begin_tenant(pool: &MySqlPool, ctx: &TenantCtx) -> AppResult<TenantTx> {
    let mut inner = pool.begin().await.map_err(map_db_error)?;
    sqlx::query("SET @app_tenant_id = ?, @app_user_id = ?, @app_platform = NULL, @app_maintenance = NULL")
        .bind(ctx.tenant_id)
        .bind(ctx.user_id)
        .execute(&mut *inner)
        .await
        .map_err(map_db_error)?;
    Ok(TenantTx { inner, tenant_id: ctx.tenant_id, user_id: ctx.user_id, branch_id: None })
}

/// A platform-level transaction (Super Admin, scheduled jobs). The guard
/// triggers accept writes for any tenant, so callers must audit what they do.
pub struct PlatformTx {
    inner: Transaction<'static, MySql>,
}

impl PlatformTx {
    pub async fn commit(self) -> AppResult<()> {
        self.inner.commit().await.map_err(map_db_error)
    }
}

impl Deref for PlatformTx {
    type Target = MySqlConnection;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PlatformTx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub async fn begin_platform(pool: &MySqlPool, ctx: &PlatformCtx) -> AppResult<PlatformTx> {
    let mut inner = pool.begin().await.map_err(map_db_error)?;
    sqlx::query("SET @app_tenant_id = NULL, @app_user_id = ?, @app_platform = 1, @app_maintenance = NULL")
        .bind(ctx.actor_id)
        .execute(&mut *inner)
        .await
        .map_err(map_db_error)?;
    tracing::info!(reason = %ctx.reason, actor = ?ctx.actor_id, "platform transaction opened");
    Ok(PlatformTx { inner })
}
