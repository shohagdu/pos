use pos_core::AppError;
use sqlx::mysql::MySqlDatabaseError;

// MySQL server error numbers
const ER_DUP_ENTRY: u16 = 1062;
const ER_NO_REFERENCED_ROW: u16 = 1452;
const ER_ROW_IS_REFERENCED: u16 = 1451;
const ER_CHECK_CONSTRAINT_VIOLATED: u16 = 3819;
const ER_SIGNAL_EXCEPTION: u16 = 1644;
const ER_LOCK_WAIT_TIMEOUT: u16 = 1205;
const ER_LOCK_DEADLOCK: u16 = 1213;

/// Maps a driver error to an [`AppError`] (DB design §9.18 error mapping).
pub fn map_db_error(err: sqlx::Error) -> AppError {
    match &err {
        sqlx::Error::RowNotFound => return AppError::NotFound,
        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_) => {
            return AppError::Unavailable(Box::new(err));
        }
        _ => {}
    }

    let Some(db_err) = err.as_database_error().and_then(|e| e.try_downcast_ref::<MySqlDatabaseError>()) else {
        return AppError::internal(err);
    };

    match db_err.number() {
        ER_DUP_ENTRY => AppError::Conflict {
            code: "DUPLICATE",
            message: "a record with the same unique value already exists".into(),
        },
        ER_NO_REFERENCED_ROW => {
            AppError::Unprocessable { code: "INVALID_REFERENCE", message: "a referenced record does not exist".into() }
        }
        ER_ROW_IS_REFERENCED => {
            AppError::Conflict { code: "IN_USE", message: "the record is referenced by other records".into() }
        }
        ER_CHECK_CONSTRAINT_VIOLATED => AppError::Unprocessable {
            code: "CONSTRAINT_VIOLATION",
            message: "the data violates a business rule".into(),
        },
        ER_LOCK_WAIT_TIMEOUT | ER_LOCK_DEADLOCK => AppError::Retryable(Box::new(err)),
        ER_SIGNAL_EXCEPTION => match db_err.message() {
            "PERIOD_LOCKED" => {
                AppError::Locked { code: "PERIOD_LOCKED", message: "the fiscal period is locked".into() }
            }
            guard @ ("TENANT_GUARD" | "APPEND_ONLY") => {
                // A guard firing means application code tried to cross a tenant
                // boundary or rewrite a ledger: a bug, alert on it (SEV1).
                tracing::error!(guard, error = %err, "database guard trigger fired");
                AppError::internal(err)
            }
            _ => AppError::internal(err),
        },
        _ => AppError::internal(err),
    }
}
