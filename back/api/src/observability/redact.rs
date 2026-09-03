//! Redact sensitive values before they appear in logs.

use once_cell::sync::Lazy;
use regex::Regex;

static SENSITIVE_KEY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(password|passwd|secret|token|api[_-]?key|authorization|bearer|session[_-]?id|cookie|credential)",
    )
    .expect("valid sensitive key regex")
});

static BEARER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)bearer\s+[A-Za-z0-9._~+/=-]+").expect("valid bearer regex"));

static KV_SENSITIVE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(password|passwd|secret|token|api[_-]?key|authorization|session[_-]?id|cookie)=[^&\s]+")
        .expect("valid kv sensitive regex")
});

/// Redact known sensitive patterns from a free-form string.
pub fn redact_sensitive(input: &str) -> String {
    let out = BEARER_RE
        .replace_all(input, "Bearer [REDACTED]")
        .to_string();
    KV_SENSITIVE_RE
        .replace_all(&out, "$1=[REDACTED]")
        .to_string()
}

/// Return `true` if a header or field name should never be logged.
pub fn is_sensitive_field_name(name: &str) -> bool {
    SENSITIVE_KEY_RE.is_match(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_bearer_tokens() {
        let s = "Authorization: Bearer abc123.secret.token";
        let out = redact_sensitive(s);
        assert!(!out.contains("abc123"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_password_fields() {
        let s = "password=hunter2";
        let out = redact_sensitive(s);
        assert!(!out.contains("hunter2"));
    }

    #[test]
    fn sensitive_field_names() {
        assert!(is_sensitive_field_name("Authorization"));
        assert!(is_sensitive_field_name("session_id"));
        assert!(!is_sensitive_field_name("user_id"));
    }
}
