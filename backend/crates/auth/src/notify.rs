//! Messages to users: one-time codes and invitation links. The SMS/email
//! gateway integration implements [`Notifier`]; until then codes and links
//! are only logged, and `pos-api` refuses to serve production.

use crate::{identifier::mask, otp::OtpPurpose};

pub trait Notifier: Send + Sync {
    fn send_otp(&self, destination: &str, purpose: OtpPurpose, code: &str);
    fn send_invitation(&self, destination: &str, business_name: &str, link: &str);
}

/// Logs instead of sending. Development and test only.
pub struct LogNotifier;

impl Notifier for LogNotifier {
    fn send_otp(&self, destination: &str, purpose: OtpPurpose, code: &str) {
        tracing::info!(destination = %mask(destination), purpose = purpose.as_str(), code, "OTP issued (no SMS gateway configured)");
    }

    fn send_invitation(&self, destination: &str, business_name: &str, link: &str) {
        tracing::info!(destination = %mask(destination), business = business_name, link, "invitation issued (no SMS/email gateway configured)");
    }
}
