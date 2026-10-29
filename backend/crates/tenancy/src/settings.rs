//! Business settings: profile, regional and invoice/tax options (wizard steps
//! 2, 3 and 5). Updates use optimistic concurrency on `tenant_settings.version`.

use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};

use crate::provisioning::{clean, validate_business_type, validate_contact, validate_name};

/// Currencies and time zones the product supports today (D-15). Money is
/// kept with 2 decimals, so only currencies with 2 minor digits are offered.
pub const CURRENCIES: &[&str] =
    &["BDT", "INR", "PKR", "NPR", "LKR", "USD", "EUR", "GBP", "AED", "SAR", "QAR", "MYR", "SGD", "AUD", "CAD"];
pub const TIMEZONES: &[&str] = &["Asia/Dhaka", "Asia/Kolkata", "Asia/Kathmandu", "Europe/Rome", "UTC"];
pub const RECEIPT_PAPERS: &[&str] = &["58mm", "80mm", "a4"];
/// Countries whose registration details the profile knows: Bangladesh (BIN,
/// trade licence) and Italy (Partita IVA, codice fiscale, R.E.A.).
pub const COUNTRIES: &[&str] = &["BD", "IT"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct BusinessSettings {
    // profile
    pub name: String,
    pub slug: String,
    pub legal_name: Option<String>,
    pub business_type: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub district: Option<String>,
    /// `BD` or `IT`; decides which registration fields apply.
    pub country_code: String,
    /// The VAT registration number: BIN in Bangladesh, Partita IVA in Italy.
    pub bin_number: Option<String>,
    pub trade_license_no: Option<String>,
    /// Italy: codice fiscale.
    pub tax_code: Option<String>,
    /// Italy: R.E.A. number at the Registro Imprese.
    pub rea_number: Option<String>,
    /// Italy: the Registro Imprese office, e.g. `Brescia`.
    pub registry_office: Option<String>,
    // regional
    pub currency_code: String,
    pub timezone: String,
    pub locale: String,
    pub fiscal_year_start_month: u8,
    // invoice & tax
    pub invoice_prefix: String,
    pub receipt_paper: String,
    pub receipt_footer: Option<String>,
    pub auto_print_receipt: bool,
    pub is_vat_registered: bool,
    pub prices_include_tax: bool,
    /// Send back in `If-Match` when updating.
    pub version: i32,
}

/// Fields to change; absent fields stay as they are, an empty string clears
/// an optional text field.
#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    pub name: Option<String>,
    pub legal_name: Option<String>,
    pub business_type: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub district: Option<String>,
    pub country_code: Option<String>,
    pub bin_number: Option<String>,
    pub trade_license_no: Option<String>,
    pub tax_code: Option<String>,
    pub rea_number: Option<String>,
    pub registry_office: Option<String>,
    pub currency_code: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
    pub fiscal_year_start_month: Option<u8>,
    pub invoice_prefix: Option<String>,
    pub receipt_paper: Option<String>,
    pub receipt_footer: Option<String>,
    pub auto_print_receipt: Option<bool>,
    pub is_vat_registered: Option<bool>,
    pub prices_include_tax: Option<bool>,
}

#[derive(sqlx::FromRow)]
struct Row {
    name: String,
    slug: String,
    legal_name: Option<String>,
    business_type: String,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    district: Option<String>,
    country_code: String,
    bin_number: Option<String>,
    trade_license_no: Option<String>,
    tax_code: Option<String>,
    rea_number: Option<String>,
    registry_office: Option<String>,
    currency_code: String,
    timezone: String,
    locale: String,
    fiscal_year_start_month: i8,
    receipt_paper: String,
    receipt_footer: Option<String>,
    auto_print_receipt: bool,
    is_vat_registered: bool,
    prices_include_tax: bool,
    version: i32,
}

async fn load(tx: &mut TenantTx, lock: bool) -> AppResult<BusinessSettings> {
    let tenant_id = tx.tenant_id();
    let sql = if lock {
        "SELECT t.name, t.slug, t.legal_name, t.business_type, t.phone, t.email, t.address, t.district, t.country_code, \
         t.bin_number, t.trade_license_no, t.tax_code, t.rea_number, t.registry_office, t.currency_code, t.timezone, t.locale, s.fiscal_year_start_month, s.receipt_paper, \
         s.receipt_footer, s.auto_print_receipt, s.is_vat_registered, s.prices_include_tax, s.version \
         FROM tenant_settings s JOIN tenants t ON t.id = s.tenant_id WHERE s.tenant_id = ? FOR UPDATE"
    } else {
        "SELECT t.name, t.slug, t.legal_name, t.business_type, t.phone, t.email, t.address, t.district, t.country_code, \
         t.bin_number, t.trade_license_no, t.tax_code, t.rea_number, t.registry_office, t.currency_code, t.timezone, t.locale, s.fiscal_year_start_month, s.receipt_paper, \
         s.receipt_footer, s.auto_print_receipt, s.is_vat_registered, s.prices_include_tax, s.version \
         FROM tenant_settings s JOIN tenants t ON t.id = s.tenant_id WHERE s.tenant_id = ?"
    };
    let row: Row = sqlx::query_as(sql).bind(tenant_id).fetch_one(&mut **tx).await.map_err(map_db_error)?;
    let prefix: Option<String> = sqlx::query_scalar(
        "SELECT prefix FROM document_sequences d JOIN branches b ON b.tenant_id = d.tenant_id AND b.id = d.branch_id \
         WHERE d.tenant_id = ? AND d.doc_type = 'sale' AND b.is_default",
    )
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    Ok(BusinessSettings {
        name: row.name,
        slug: row.slug,
        legal_name: row.legal_name,
        business_type: row.business_type,
        phone: row.phone,
        email: row.email,
        address: row.address,
        district: row.district,
        country_code: row.country_code,
        bin_number: row.bin_number,
        trade_license_no: row.trade_license_no,
        tax_code: row.tax_code,
        rea_number: row.rea_number,
        registry_office: row.registry_office,
        currency_code: row.currency_code,
        timezone: row.timezone,
        locale: row.locale,
        fiscal_year_start_month: u8::try_from(row.fiscal_year_start_month).unwrap_or(7),
        invoice_prefix: prefix.unwrap_or_else(|| "INV".to_owned()),
        receipt_paper: row.receipt_paper,
        receipt_footer: row.receipt_footer,
        auto_print_receipt: row.auto_print_receipt,
        is_vat_registered: row.is_vat_registered,
        prices_include_tax: row.prices_include_tax,
        version: row.version,
    })
}

pub async fn get(tx: &mut TenantTx) -> AppResult<BusinessSettings> {
    load(tx, false).await
}

fn one_of(value: String, allowed: &[&str], field: &str, code: &str) -> AppResult<String> {
    if allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(AppError::field(field, code, format!("must be one of: {}", allowed.join(", "))))
    }
}

fn apply(current: &BusinessSettings, patch: SettingsPatch) -> AppResult<BusinessSettings> {
    let mut next = current.clone();
    if let Some(name) = patch.name {
        next.name = validate_name(&name, "name", 200)?;
    }
    if let Some(v) = patch.legal_name {
        next.legal_name = clean(Some(v), "legal_name", 200)?;
    }
    if let Some(v) = patch.business_type {
        validate_business_type(&v)?;
        next.business_type = v;
    }
    if patch.phone.is_some() || patch.email.is_some() {
        let (phone, email) = validate_contact(patch.phone.clone(), patch.email.clone())?;
        if patch.phone.is_some() {
            next.phone = phone;
        }
        if patch.email.is_some() {
            next.email = email;
        }
    }
    if let Some(v) = patch.address {
        next.address = clean(Some(v), "address", 500)?;
    }
    if let Some(v) = patch.district {
        next.district = clean(Some(v), "district", 100)?;
    }
    if let Some(v) = patch.bin_number {
        next.bin_number = clean(Some(v), "bin_number", 30)?;
    }
    if let Some(v) = patch.trade_license_no {
        next.trade_license_no = clean(Some(v), "trade_license_no", 50)?;
    }
    if let Some(v) = patch.country_code {
        next.country_code = one_of(v.trim().to_uppercase(), COUNTRIES, "country_code", "UNSUPPORTED_COUNTRY")?;
    }
    if let Some(v) = patch.tax_code {
        next.tax_code = clean(Some(v), "tax_code", 30)?;
    }
    if let Some(v) = patch.rea_number {
        next.rea_number = clean(Some(v), "rea_number", 20)?;
    }
    if let Some(v) = patch.registry_office {
        next.registry_office = clean(Some(v), "registry_office", 100)?;
    }
    if next.country_code == "IT" {
        // Checked in Italy only; a Bangladeshi BIN has no published check digit.
        next.bin_number = next.bin_number.as_deref().map(partita_iva).transpose()?;
        next.tax_code = next.tax_code.as_deref().map(codice_fiscale).transpose()?;
    }
    if let Some(v) = patch.currency_code {
        next.currency_code = one_of(v, CURRENCIES, "currency_code", "UNSUPPORTED_CURRENCY")?;
    }
    if let Some(v) = patch.timezone {
        next.timezone = one_of(v, TIMEZONES, "timezone", "UNSUPPORTED_TIMEZONE")?;
    }
    if let Some(v) = patch.locale {
        next.locale = one_of(v, &["en", "bn", "it"], "locale", "INVALID_LOCALE")?;
    }
    if let Some(month) = patch.fiscal_year_start_month {
        if !(1..=12).contains(&month) {
            return Err(AppError::field("fiscal_year_start_month", "INVALID_MONTH", "choose a month from 1 to 12"));
        }
        next.fiscal_year_start_month = month;
    }
    if let Some(v) = patch.invoice_prefix {
        let v = v.trim().to_uppercase();
        if v.is_empty() || v.len() > 10 || !v.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
            return Err(AppError::field("invoice_prefix", "INVALID_PREFIX", "use 1–10 letters or digits"));
        }
        next.invoice_prefix = v;
    }
    if let Some(v) = patch.receipt_paper {
        next.receipt_paper = one_of(v, RECEIPT_PAPERS, "receipt_paper", "INVALID_PAPER")?;
    }
    if let Some(v) = patch.receipt_footer {
        next.receipt_footer = clean(Some(v), "receipt_footer", 500)?;
    }
    if let Some(v) = patch.auto_print_receipt {
        next.auto_print_receipt = v;
    }
    if let Some(v) = patch.is_vat_registered {
        next.is_vat_registered = v;
    }
    if let Some(v) = patch.prices_include_tax {
        next.prices_include_tax = v;
    }
    Ok(next)
}

/// A Partita IVA: 11 digits with a Luhn-style check digit; spaces and an
/// `IT` prefix are dropped.
fn partita_iva(raw: &str) -> AppResult<String> {
    let invalid = || AppError::field("bin_number", "INVALID_VAT_NUMBER", "enter the 11-digit Partita IVA");
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    let digits = compact.strip_prefix("IT").unwrap_or(&compact);
    if digits.len() != 11 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let d: Vec<u32> = digits.bytes().map(|b| u32::from(b - b'0')).collect();
    let sum: u32 = d[..10]
        .iter()
        .enumerate()
        .map(|(i, &x)| if i % 2 == 1 { (x * 2) - if x * 2 > 9 { 9 } else { 0 } } else { x })
        .sum();
    if (10 - sum % 10) % 10 != d[10] {
        return Err(invalid());
    }
    Ok(digits.to_owned())
}

/// A codice fiscale: 16 letters and digits ending in a check letter (people),
/// or a company's 11-digit number (checked like a Partita IVA).
fn codice_fiscale(raw: &str) -> AppResult<String> {
    let invalid = || AppError::field("tax_code", "INVALID_TAX_CODE", "enter the 16-character codice fiscale");
    let code: String = raw.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    if code.len() == 11 && code.bytes().all(|b| b.is_ascii_digit()) {
        return partita_iva(&code).map_err(|_| invalid());
    }
    if code.len() != 16 || !code.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
        return Err(invalid());
    }
    // Values of each character in odd (1st, 3rd, …) positions, 0–9 then A–Z.
    const ODD: [u32; 26] =
        [1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 2, 4, 18, 20, 11, 3, 6, 8, 12, 14, 16, 10, 22, 25, 24, 23];
    let index = |b: u8| u32::from(if b.is_ascii_digit() { b - b'0' } else { b - b'A' });
    let sum: u32 = code
        .bytes()
        .take(15)
        .enumerate()
        .map(|(i, b)| if i % 2 == 0 { ODD[index(b) as usize] } else { index(b) })
        .sum();
    if code.as_bytes()[15] != b'A' + (sum % 26) as u8 {
        return Err(invalid());
    }
    Ok(code)
}

/// Whether any money has been recorded (book lines, sales or purchases).
async fn has_money(tx: &mut TenantTx) -> AppResult<bool> {
    let tenant_id = tx.tenant_id();
    let found: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM account_transactions WHERE tenant_id = ? \
         UNION ALL SELECT 1 FROM sales WHERE tenant_id = ? \
         UNION ALL SELECT 1 FROM purchases WHERE tenant_id = ? AND status <> 'draft' LIMIT 1",
    )
    .bind(tenant_id)
    .bind(tenant_id)
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(found.is_some())
}

pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    expected_version: i32,
    patch: SettingsPatch,
) -> AppResult<BusinessSettings> {
    let current = load(tx, true).await?;
    if current.version != expected_version {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the settings were changed by someone else; reload and try again".into(),
        });
    }
    let mut next = apply(&current, patch)?;
    if next == current {
        return Ok(current);
    }
    if next.currency_code != current.currency_code && has_money(tx).await? {
        // Amounts are stored without a currency; switching would relabel them, not convert them.
        return Err(AppError::field(
            "currency_code",
            "CURRENCY_LOCKED",
            "the currency cannot change after sales, purchases or payments are recorded",
        ));
    }
    next.version = current.version + 1;

    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE tenants SET name = ?, legal_name = ?, business_type = ?, phone = ?, email = ?, address = ?, \
         district = ?, country_code = ?, bin_number = ?, trade_license_no = ?, tax_code = ?, rea_number = ?, \
         registry_office = ?, currency_code = ?, timezone = ?, locale = ? WHERE id = ?",
    )
    .bind(&next.name)
    .bind(&next.legal_name)
    .bind(&next.business_type)
    .bind(&next.phone)
    .bind(&next.email)
    .bind(&next.address)
    .bind(&next.district)
    .bind(&next.country_code)
    .bind(&next.bin_number)
    .bind(&next.trade_license_no)
    .bind(&next.tax_code)
    .bind(&next.rea_number)
    .bind(&next.registry_office)
    .bind(&next.currency_code)
    .bind(&next.timezone)
    .bind(&next.locale)
    .bind(tenant_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE tenant_settings SET fiscal_year_start_month = ?, receipt_paper = ?, receipt_footer = ?, \
         auto_print_receipt = ?, is_vat_registered = ?, prices_include_tax = ?, version = version + 1, updated_by = ? \
         WHERE tenant_id = ?",
    )
    .bind(next.fiscal_year_start_month)
    .bind(&next.receipt_paper)
    .bind(&next.receipt_footer)
    .bind(next.auto_print_receipt)
    .bind(next.is_vat_registered)
    .bind(next.prices_include_tax)
    .bind(user_id)
    .bind(tenant_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if next.invoice_prefix != current.invoice_prefix {
        sqlx::query("UPDATE document_sequences SET prefix = ? WHERE tenant_id = ? AND doc_type = 'sale'")
            .bind(&next.invoice_prefix)
            .bind(tenant_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
    }

    let to_json = |s: &BusinessSettings| serde_json::to_value(s).map_err(AppError::internal);
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("settings.updated")
            .entity("tenant", tenant_id)
            .change(Some(to_json(&current)?), Some(to_json(&next)?)),
    )
    .await?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BusinessSettings {
        BusinessSettings {
            name: "Rahim Store".into(),
            slug: "rahim-store".into(),
            legal_name: None,
            business_type: "grocery".into(),
            phone: None,
            email: None,
            address: Some("Mirpur 10".into()),
            district: None,
            country_code: "BD".into(),
            bin_number: None,
            trade_license_no: None,
            tax_code: None,
            rea_number: None,
            registry_office: None,
            currency_code: "BDT".into(),
            timezone: "Asia/Dhaka".into(),
            locale: "en".into(),
            fiscal_year_start_month: 7,
            invoice_prefix: "INV".into(),
            receipt_paper: "80mm".into(),
            receipt_footer: None,
            auto_print_receipt: true,
            is_vat_registered: false,
            prices_include_tax: false,
            version: 1,
        }
    }

    #[test]
    fn applies_and_clears_fields() {
        let patch = SettingsPatch {
            address: Some("  ".into()),
            invoice_prefix: Some("mir".into()),
            phone: Some("01712345678".into()),
            ..SettingsPatch::default()
        };
        let next = apply(&sample(), patch).expect("valid patch");
        assert_eq!(next.address, None);
        assert_eq!(next.invoice_prefix, "MIR");
        assert_eq!(next.phone.as_deref(), Some("+8801712345678"));
        assert_eq!(next.name, "Rahim Store");
    }

    #[test]
    fn rejects_invalid_values() {
        let field = |patch: SettingsPatch| {
            apply(&sample(), patch).err().and_then(|e| e.field_errors().first().map(|f| f.field.clone()))
        };
        assert_eq!(
            field(SettingsPatch { timezone: Some("Mars/Base".into()), ..Default::default() }).as_deref(),
            Some("timezone")
        );
        assert_eq!(
            field(SettingsPatch { fiscal_year_start_month: Some(13), ..Default::default() }).as_deref(),
            Some("fiscal_year_start_month")
        );
        assert_eq!(
            field(SettingsPatch { invoice_prefix: Some("IN-V".into()), ..Default::default() }).as_deref(),
            Some("invoice_prefix")
        );
        assert_eq!(
            field(SettingsPatch { currency_code: Some("XYZ".into()), ..Default::default() }).as_deref(),
            Some("currency_code")
        );
        assert_eq!(field(SettingsPatch { name: Some("x".into()), ..Default::default() }).as_deref(), Some("name"));
    }

    #[test]
    fn checks_italian_registration() {
        let italy = |bin: &str, tax: &str| {
            apply(
                &sample(),
                SettingsPatch {
                    country_code: Some("it".into()),
                    bin_number: Some(bin.into()),
                    tax_code: Some(tax.into()),
                    ..Default::default()
                },
            )
        };
        let next = italy("IT 03580690984", "ddnmjs87t15z249i").expect("valid Italian numbers");
        assert_eq!(next.country_code, "IT");
        assert_eq!(next.bin_number.as_deref(), Some("03580690984"));
        assert_eq!(next.tax_code.as_deref(), Some("DDNMJS87T15Z249I"));
        // A company's codice fiscale is its 11-digit number.
        assert!(italy("03580690984", "03580690984").is_ok());
        let code =
            |r: AppResult<BusinessSettings>| r.err().and_then(|e| e.field_errors().first().map(|f| f.code.clone()));
        assert_eq!(code(italy("03580690985", "DDNMJS87T15Z249I")).as_deref(), Some("INVALID_VAT_NUMBER"));
        assert_eq!(code(italy("0358069098", "DDNMJS87T15Z249I")).as_deref(), Some("INVALID_VAT_NUMBER"));
        assert_eq!(code(italy("03580690984", "DDNMJS87T15Z249A")).as_deref(), Some("INVALID_TAX_CODE"));
        assert_eq!(
            code(apply(&sample(), SettingsPatch { country_code: Some("FR".into()), ..Default::default() })).as_deref(),
            Some("UNSUPPORTED_COUNTRY")
        );
        // Bangladesh keeps the BIN as entered.
        let bd = apply(&sample(), SettingsPatch { bin_number: Some("000123456-0101".into()), ..Default::default() });
        assert_eq!(bd.expect("BIN").bin_number.as_deref(), Some("000123456-0101"));
    }
}
