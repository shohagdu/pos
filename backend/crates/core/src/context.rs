use std::net::IpAddr;

use uuid::Uuid;

/// Per-request metadata recorded in audit logs.
#[derive(Debug, Clone, Default)]
pub struct RequestMeta {
    pub request_id: String,
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
}

impl RequestMeta {
    /// The client IP in MySQL `INET6_ATON()` layout: 4 bytes for IPv4, 16 for IPv6.
    pub fn ip_bytes(&self) -> Option<Vec<u8>> {
        self.ip.map(|ip| match ip {
            IpAddr::V4(v4) => v4.octets().to_vec(),
            IpAddr::V6(v6) => v6.octets().to_vec(),
        })
    }

    /// The user agent cut to the 500 characters the audit and auth tables keep.
    pub fn user_agent_truncated(&self) -> Option<&str> {
        self.user_agent.as_deref().map(|ua| truncate_chars(ua, 500))
    }
}

/// Cuts `s` to at most `max_chars` characters without splitting a code point.
pub fn truncate_chars(s: &str, max_chars: usize) -> &str {
    s.char_indices().nth(max_chars).map_or(s, |(idx, _)| &s[..idx])
}

/// Identity of a request acting inside one tenant. The tenant id must come
/// from the verified access token, never from the request body or URL.
#[derive(Debug, Clone)]
pub struct TenantCtx {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub meta: RequestMeta,
}

/// Identity of a platform-level operation (Super Admin, scheduled jobs).
/// Platform transactions bypass the tenant guard triggers, so every use is audited.
#[derive(Debug, Clone)]
pub struct PlatformCtx {
    pub actor_id: Option<Uuid>,
    pub reason: String,
    pub meta: RequestMeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_bytes_match_inet6_aton_layout() {
        let v4 = RequestMeta { ip: Some(IpAddr::from([203, 0, 113, 7])), ..RequestMeta::default() };
        assert_eq!(v4.ip_bytes(), Some(vec![203, 0, 113, 7]));
        let v6 = RequestMeta { ip: Some(IpAddr::from([0u16, 0, 0, 0, 0, 0, 0, 1])), ..RequestMeta::default() };
        assert_eq!(v6.ip_bytes().map(|b| b.len()), Some(16));
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        assert_eq!(truncate_chars("চালচাল", 3), "চাল");
        assert_eq!(truncate_chars("abc", 10), "abc");
    }
}
