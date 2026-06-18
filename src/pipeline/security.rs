//! Shared pipeline security guards.

use std::net::{IpAddr, ToSocketAddrs};

use crate::pipeline::PipelineError;

/// Validates that a user-controlled outbound HTTP URL does not target local or
/// private infrastructure.
pub fn validate_outbound_http_url(url: &str, node_kind: &str) -> Result<(), PipelineError> {
    validate_outbound_http_url_with_policy(url, node_kind, &OutboundHttpPolicy::default())
}

/// Credential-owned egress policy for outbound HTTP.
///
/// The default policy allows public HTTP(S) only. `allow_private` does not grant
/// broad private-network access by itself; the URL host must also be explicitly
/// listed in `allowed_hosts`.
#[derive(Debug, Clone, Default)]
pub struct OutboundHttpPolicy {
    pub allow_private: bool,
    pub allowed_hosts: Vec<String>,
}

impl OutboundHttpPolicy {
    fn allows_host(&self, host: &str) -> bool {
        if !self.allow_private {
            return false;
        }
        let host = normalize_host(host);
        self.allowed_hosts
            .iter()
            .map(|item| normalize_host(item))
            .any(|allowed| !allowed.is_empty() && allowed == host)
    }
}

pub fn validate_outbound_http_url_with_policy(
    url: &str,
    node_kind: &str,
    policy: &OutboundHttpPolicy,
) -> Result<(), PipelineError> {
    let parsed = reqwest::Url::parse(url).map_err(|err| {
        PipelineError::new(
            "FW_EGRESS_URL_INVALID",
            format!("{node_kind} outbound URL is invalid: {err}"),
        )
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(PipelineError::new(
            "FW_EGRESS_URL_INVALID",
            format!("{node_kind} outbound URL must use http or https"),
        ));
    }
    let Some(host) = parsed.host_str() else {
        return Err(PipelineError::new(
            "FW_EGRESS_URL_INVALID",
            format!("{node_kind} outbound URL must include a host"),
        ));
    };
    if host.eq_ignore_ascii_case("localhost") && !policy.allows_host(host) {
        return Err(PipelineError::new(
            "FW_EGRESS_DENIED",
            format!("{node_kind} outbound URL targets localhost"),
        ));
    }
    let host_ip_literal = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host_ip_literal.parse::<IpAddr>() {
        return validate_outbound_ip(ip, node_kind, policy, host);
    }

    let port = parsed.port_or_known_default().ok_or_else(|| {
        PipelineError::new(
            "FW_EGRESS_URL_INVALID",
            format!("{node_kind} outbound URL must include a valid port"),
        )
    })?;
    let addrs = (host, port).to_socket_addrs().map_err(|err| {
        PipelineError::new(
            "FW_EGRESS_DNS",
            format!("{node_kind} outbound host could not be resolved: {err}"),
        )
    })?;
    for addr in addrs {
        validate_outbound_ip(addr.ip(), node_kind, policy, host)?;
    }
    Ok(())
}

fn validate_outbound_ip(
    ip: IpAddr,
    node_kind: &str,
    policy: &OutboundHttpPolicy,
    host: &str,
) -> Result<(), PipelineError> {
    if is_blocked_egress_ip(ip) && !policy.allows_host(host) {
        return Err(PipelineError::new(
            "FW_EGRESS_DENIED",
            format!("{node_kind} outbound URL resolves to blocked network address {ip}"),
        ));
    }
    Ok(())
}

fn normalize_host(host: &str) -> String {
    host.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn is_blocked_egress_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, _, _] = ip.octets();
            ip.is_unspecified()
                || ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_multicast()
                || a == 0
                || (a == 100 && (64..=127).contains(&b))
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            ip.is_unspecified()
                || ip.is_loopback()
                || ip.is_multicast()
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OutboundHttpPolicy, is_blocked_egress_ip, validate_outbound_http_url,
        validate_outbound_http_url_with_policy,
    };

    #[test]
    fn egress_policy_blocks_private_and_local_ip_literals() {
        for url in [
            "http://127.0.0.1:8080/",
            "http://10.0.0.5/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/",
            "http://[fc00::1]/",
            "http://[fe80::1]/",
        ] {
            let err = validate_outbound_http_url(url, "test").expect_err(url);
            assert_eq!(err.code, "FW_EGRESS_DENIED");
        }
    }

    #[test]
    fn egress_policy_allows_public_ip_literals() {
        validate_outbound_http_url("https://93.184.216.34/", "test").expect("public IPv4");
        validate_outbound_http_url("https://[2606:2800:220:1:248:1893:25c8:1946]/", "test")
            .expect("public IPv6");
    }

    #[test]
    fn egress_ip_classifier_blocks_carrier_grade_nat() {
        assert!(is_blocked_egress_ip("100.64.0.1".parse().unwrap()));
        assert!(is_blocked_egress_ip("100.127.255.254".parse().unwrap()));
        assert!(!is_blocked_egress_ip("100.128.0.1".parse().unwrap()));
    }

    #[test]
    fn egress_policy_allows_explicit_private_host_only() {
        let policy = OutboundHttpPolicy {
            allow_private: true,
            allowed_hosts: vec!["10.0.0.5".to_string()],
        };
        validate_outbound_http_url_with_policy("http://10.0.0.5/embed", "test", &policy)
            .expect("explicit private IP host allowed");

        let err = validate_outbound_http_url_with_policy("http://10.0.0.6/embed", "test", &policy)
            .expect_err("different private host stays blocked");
        assert_eq!(err.code, "FW_EGRESS_DENIED");
    }

    #[test]
    fn egress_policy_allow_private_requires_host_allowlist() {
        let policy = OutboundHttpPolicy {
            allow_private: true,
            allowed_hosts: Vec::new(),
        };
        let err = validate_outbound_http_url_with_policy("http://10.0.0.5/embed", "test", &policy)
            .expect_err("private host stays blocked without allowed_hosts");
        assert_eq!(err.code, "FW_EGRESS_DENIED");
    }
}
