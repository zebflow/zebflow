//! Shared pipeline security guards.

use std::net::{IpAddr, ToSocketAddrs};

use crate::pipeline::PipelineError;

/// Validates that a user-controlled outbound HTTP URL does not target local or
/// private infrastructure.
pub fn validate_outbound_http_url(url: &str, node_kind: &str) -> Result<(), PipelineError> {
    validate_outbound_http_url_with_policy(url, node_kind, &OutboundHttpPolicy::default())
}

/// The hosts one node bundle declared, and the package that declared them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredHosts {
    /// `spec.package` of the bundle this declaration came from.
    pub package: String,
    /// `spec.hosts`, already validated by the `NodeBundle` contract.
    pub hosts: Vec<String>,
}

/// Egress allowlist governing everything a bundle-provided node runs.
///
/// A bundle declares `spec.hosts`; this is the runtime half of that
/// declaration. It is attached to the engine that runs a bundle's composite
/// function, so it governs the whole inner subtree rather than only the node
/// the graph names.
///
/// Layers stack outermost first. When a bundle composes another bundle's node,
/// the inner subtree answers to both declarations, because the outer bundle is
/// still the reason the connection is being made at all.
///
/// Scope is deliberately bundle-provided nodes only. A project's own pipeline
/// calling `n.http.request` carries no policy and is not restricted: the threat
/// model is third-party code the user installed, not the user's own work.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BundleEgress {
    layers: Vec<DeclaredHosts>,
}

impl BundleEgress {
    /// Adds one bundle's declaration to whatever policy already governs the
    /// caller, returning `None` when the result would restrict nothing.
    ///
    /// An absent and an empty `spec.hosts` are the same bytes on the wire —
    /// `#[serde(default)]` makes them indistinguishable — so an empty list
    /// cannot mean "deny all" without retroactively breaking every bundle
    /// published before enforcement existed. It therefore adds no layer, which
    /// means unrestricted. That is a deliberate, temporary limit and not an
    /// oversight; closing it needs a way to tell the two apart.
    pub fn extend(parent: Option<&BundleEgress>, package: &str, hosts: &[String]) -> Option<Self> {
        let mut layers = parent
            .map(|policy| policy.layers.clone())
            .unwrap_or_default();
        if !hosts.is_empty() {
            layers.push(DeclaredHosts {
                package: package.to_string(),
                hosts: hosts.to_vec(),
            });
        }
        (!layers.is_empty()).then_some(Self { layers })
    }

    /// Whether any declaration governs this policy.
    pub fn is_active(&self) -> bool {
        !self.layers.is_empty()
    }

    /// The innermost bundle governing the caller — the one whose function
    /// pipeline is currently running.
    pub fn innermost(&self) -> Option<&DeclaredHosts> {
        self.layers.last()
    }

    /// Refuses an outbound URL whose host no governing bundle declared.
    ///
    /// A URL that will not parse is refused rather than allowed through: the
    /// host is the thing being checked, so failing to read one is a failure to
    /// check.
    pub fn check_url(&self, url: &str, node_kind: &str) -> Result<(), PipelineError> {
        if !self.is_active() {
            return Ok(());
        }
        let parsed = reqwest::Url::parse(url).map_err(|err| {
            PipelineError::new(
                "FW_EGRESS_URL_INVALID",
                format!("{node_kind} outbound URL is invalid: {err}"),
            )
        })?;
        let Some(host) = parsed.host_str() else {
            return Err(PipelineError::new(
                "FW_EGRESS_URL_INVALID",
                format!("{node_kind} outbound URL must include a host"),
            ));
        };
        self.check_host(host, node_kind)
    }

    /// Refuses a host no governing bundle declared, naming the host and the
    /// bundle whose list refused it.
    pub fn check_host(&self, host: &str, node_kind: &str) -> Result<(), PipelineError> {
        let host = normalize_host(host);
        for layer in &self.layers {
            if layer
                .hosts
                .iter()
                .any(|pattern| declared_host_matches(pattern, &host))
            {
                continue;
            }
            return Err(PipelineError::new(
                "FW_EGRESS_UNDECLARED_HOST",
                format!(
                    "{node_kind} outbound host '{host}' is not declared by node bundle '{}' \
                     (spec.hosts: {})",
                    layer.package,
                    layer.hosts.join(", ")
                ),
            ));
        }
        Ok(())
    }

    /// Refuses a node whose destination cannot be read at the egress guard.
    ///
    /// A node that can reach the network but whose target Zebflow never sees as
    /// a URL cannot be checked against a declaration. Inside a governed subtree
    /// it is refused rather than allowed, so "enforced" means enforced for every
    /// egress path in that subtree and not only the ones we can read.
    pub fn refuse_uncheckable(&self, node_kind: &str) -> PipelineError {
        let package = self
            .innermost()
            .map(|layer| layer.package.as_str())
            .unwrap_or("unknown");
        PipelineError::new(
            "FW_EGRESS_UNCHECKED_NODE",
            format!(
                "node '{node_kind}' cannot run inside node bundle '{package}': it reaches the \
                 network through a destination Zebflow cannot check against the bundle's \
                 declared hosts"
            ),
        )
    }
}

/// Matches one declared entry against a normalized host.
///
/// `*.example.com` matches any host below `example.com` and not the apex
/// itself, which is what a wildcard label means everywhere else a host
/// allowlist is written.
fn declared_host_matches(pattern: &str, host: &str) -> bool {
    let pattern = normalize_host(pattern);
    match pattern.strip_prefix("*.") {
        Some(suffix) => {
            !suffix.is_empty()
                && host.len() > suffix.len()
                && host.ends_with(suffix)
                && host.as_bytes()[host.len() - suffix.len() - 1] == b'.'
        }
        None => !pattern.is_empty() && pattern == host,
    }
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
        BundleEgress, OutboundHttpPolicy, is_blocked_egress_ip, validate_outbound_http_url,
        validate_outbound_http_url_with_policy,
    };

    fn hosts(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn declared_hosts_admit_what_the_bundle_stated() {
        let policy = BundleEgress::extend(None, "ml", &hosts(&["api.openai.com", "*.example.org"]))
            .expect("a declaration restricts");
        policy
            .check_url("https://api.openai.com/v1/embeddings", "n.http.request")
            .expect("exact host");
        policy
            .check_url("https://API.OpenAI.com:443/v1", "n.http.request")
            .expect("case and port are not part of the host");
        policy
            .check_url("https://eu.api.example.org/x", "n.http.request")
            .expect("wildcard covers a subdomain");
    }

    #[test]
    fn an_undeclared_host_is_refused_by_name() {
        let policy = BundleEgress::extend(None, "ml", &hosts(&["api.openai.com", "*.example.org"]))
            .expect("a declaration restricts");
        let err = policy
            .check_url("https://evil.example.com/steal", "n.http.request")
            .expect_err("undeclared host");
        assert_eq!(err.code, "FW_EGRESS_UNDECLARED_HOST");
        assert_eq!(
            err.message,
            "n.http.request outbound host 'evil.example.com' is not declared by node bundle 'ml' \
             (spec.hosts: api.openai.com, *.example.org)"
        );

        // A wildcard label is a label, so the apex it hangs off is not covered.
        let err = policy
            .check_url("https://example.org/x", "n.http.request")
            .expect_err("wildcard does not match its own apex");
        assert_eq!(err.code, "FW_EGRESS_UNDECLARED_HOST");
    }

    #[test]
    fn an_empty_declaration_restricts_nothing() {
        assert!(
            BundleEgress::extend(None, "ml", &[]).is_none(),
            "absent and empty are the same bytes on the wire, so empty cannot deny"
        );
        assert!(!BundleEgress::default().is_active());
        BundleEgress::default()
            .check_url("https://anywhere.example.com/", "n.http.request")
            .expect("an inactive policy checks nothing");
    }

    #[test]
    fn a_nested_bundle_answers_to_both_declarations() {
        let outer = BundleEgress::extend(None, "outer", &hosts(&["api.outer.com"]))
            .expect("outer declares");
        let nested = BundleEgress::extend(Some(&outer), "inner", &hosts(&["api.inner.com"]))
            .expect("inner declares");

        for url in [
            "https://api.inner.com/x",
            "https://api.outer.com/x",
            "https://elsewhere.com/x",
        ] {
            let err = nested
                .check_url(url, "n.http.request")
                .expect_err("no host satisfies both lists");
            assert_eq!(err.code, "FW_EGRESS_UNDECLARED_HOST");
        }

        // A bundle that declares nothing does not escape the one that composed it.
        let silent = BundleEgress::extend(Some(&outer), "silent", &[]).expect("outer still holds");
        silent
            .check_url("https://api.outer.com/x", "n.http.request")
            .expect("the outer declaration still admits its own host");
        assert_eq!(
            silent
                .check_url("https://elsewhere.com/x", "n.http.request")
                .expect_err("and still refuses everything else")
                .code,
            "FW_EGRESS_UNDECLARED_HOST"
        );
    }

    #[test]
    fn an_unreadable_destination_is_refused_rather_than_allowed() {
        let policy =
            BundleEgress::extend(None, "ml", &hosts(&["api.openai.com"])).expect("declares");
        assert_eq!(
            policy
                .check_url("not-a-url", "n.http.request")
                .expect_err("a URL that will not parse has no host to check")
                .code,
            "FW_EGRESS_URL_INVALID"
        );
        let err = policy.refuse_uncheckable("n.ai.agent");
        assert_eq!(err.code, "FW_EGRESS_UNCHECKED_NODE");
        assert!(err.message.contains("n.ai.agent"), "{}", err.message);
        assert!(err.message.contains("'ml'"), "{}", err.message);
    }

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
