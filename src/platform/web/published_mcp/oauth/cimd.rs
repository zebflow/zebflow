//! Client ID metadata documents (the MCP spec's preferred registration):
//! a `client_id` that is an `https` URL names a JSON document, served
//! there, saying the client's name and redirect URIs. ChatGPT and Claude
//! Code use one.
//!
//! The fetch is someone else's URL fetched by this office, so it goes
//! through the outbound policy every node's fetch does (no loopback, private
//! or link-local address — checked on the address actually connected to),
//! follows no redirect, gives up after [`TIMEOUT`] and reads at most
//! [`MAX_BYTES`]. A document is cached as its `Cache-Control: max-age`
//! says, between one minute and a day (an hour when it says nothing).

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use reqwest::Url;
use serde_json::Value;

use crate::platform::services::published_oauth::{CimdClient, PublishedOAuthService, rules};

pub const MAX_BYTES: usize = 32 * 1024;
pub const TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_TTL: Duration = Duration::from_secs(3600);

/// Whether `client_id` names a metadata document rather than a registration.
pub(super) fn is_document_id(client_id: &str) -> bool {
    client_id.starts_with("https://")
}

/// The client a document names, from the cache or fetched.
pub(super) async fn resolve(store: &PublishedOAuthService, client_id: &str) -> Result<CimdClient, String> {
    if let Some(client) = store.cached_cimd(client_id) {
        return Ok(client);
    }
    let (client, ttl) = fetch(client_id).await?;
    store.cache_cimd(client_id, client.clone(), ttl);
    Ok(client)
}

/// The URL as a document id may be one: `https`, a host, a path, no
/// fragment, no credentials.
fn document_url(client_id: &str) -> Result<Url, String> {
    let url = Url::parse(client_id).map_err(|e| format!("client_id is not a URL: {e}"))?;
    if url.scheme() != "https" || url.host_str().is_none() || url.path() == "/" || url.fragment().is_some() || !url.username().is_empty() || url.password().is_some() {
        return Err("a client_id URL is https, with a path, and without a fragment or credentials".to_string());
    }
    Ok(url)
}

/// The one address to connect to, after the outbound policy has passed
/// every address the name resolves to.
fn checked_address(url: &Url) -> Result<SocketAddr, String> {
    let host = url.host_str().unwrap_or_default().trim_start_matches('[').trim_end_matches(']').to_string();
    let port = url.port_or_known_default().unwrap_or(443);
    let addresses: Vec<SocketAddr> = (host.as_str(), port).to_socket_addrs().map_err(|e| format!("the client's host does not resolve: {e}"))?.collect();
    if addresses.is_empty() {
        return Err("the client's host does not resolve".to_string());
    }
    for address in &addresses {
        let literal = match address {
            SocketAddr::V4(v4) => v4.ip().to_string(),
            SocketAddr::V6(v6) => format!("[{}]", v6.ip()),
        };
        crate::pipeline::security::validate_outbound_http_url(&format!("https://{literal}/"), "client metadata fetch")
            .map_err(|e| e.message)?;
    }
    if url.host_str().is_some_and(|h| h.eq_ignore_ascii_case("localhost")) {
        return Err("a client metadata document is never on localhost".to_string());
    }
    Ok(addresses[0])
}

async fn fetch(client_id: &str) -> Result<(CimdClient, Duration), String> {
    let url = document_url(client_id)?;
    let lookup = url.clone();
    let address = tokio::task::spawn_blocking(move || checked_address(&lookup)).await.map_err(|e| e.to_string())??;
    let host = url.host_str().unwrap_or_default().to_string();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT)
        .resolve(&host, address)
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(url.clone())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| format!("the client's metadata document could not be fetched: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("the client's metadata document answered {}", response.status()));
    }
    let ttl = cache_ttl(response.headers().get(reqwest::header::CACHE_CONTROL).and_then(|v| v.to_str().ok()));
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > MAX_BYTES {
            return Err(format!("the client's metadata document is over {MAX_BYTES} bytes"));
        }
    }
    Ok((parse_document(client_id, &bytes)?, ttl))
}

/// `max-age` clamped to one minute … one day; an hour when absent.
fn cache_ttl(cache_control: Option<&str>) -> Duration {
    let max_age = cache_control.and_then(|value| {
        value.split(',').find_map(|part| part.trim().strip_prefix("max-age=").and_then(|n| n.trim().parse::<u64>().ok()))
    });
    match max_age {
        Some(seconds) => Duration::from_secs(seconds.clamp(60, 86_400)),
        None => DEFAULT_TTL,
    }
}

/// The document's client: its `client_id` must be the URL it was fetched
/// from; its redirect URIs registrable; a public client.
pub(super) fn parse_document(client_id: &str, bytes: &[u8]) -> Result<CimdClient, String> {
    let document: Value = serde_json::from_slice(bytes).map_err(|e| format!("the client's metadata document is not JSON: {e}"))?;
    if document.get("client_id").and_then(Value::as_str) != Some(client_id) {
        return Err("the client's metadata document names another client_id".to_string());
    }
    if let Some(method) = document.get("token_endpoint_auth_method").and_then(Value::as_str)
        && method != "none"
    {
        return Err(format!("token_endpoint_auth_method '{method}' is not offered; public clients use none"));
    }
    let redirect_uris: Vec<String> = document
        .get("redirect_uris")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    if redirect_uris.is_empty() || redirect_uris.len() > rules::MAX_REDIRECT_URIS {
        return Err(format!("the client's metadata document needs 1 to {} redirect_uris", rules::MAX_REDIRECT_URIS));
    }
    for uri in &redirect_uris {
        rules::acceptable_redirect(uri)?;
    }
    let name = document.get("client_name").and_then(Value::as_str).unwrap_or_default();
    let name: String = name.trim().chars().filter(|c| !c.is_control()).take(rules::MAX_CLIENT_NAME).collect();
    let client_name = if name.is_empty() { Url::parse(client_id).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default() } else { name };
    Ok(CimdClient { client_name, redirect_uris })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ID: &str = "https://client.example/oauth/client.json";

    #[test]
    fn a_document_names_itself_and_its_redirects() {
        let doc = json!({ "client_id": ID, "client_name": "Reader", "redirect_uris": ["https://client.example/cb", "http://127.0.0.1/callback"] });
        let client = parse_document(ID, doc.to_string().as_bytes()).unwrap();
        assert_eq!(client.client_name, "Reader");
        assert_eq!(client.redirect_uris.len(), 2);
        let other = json!({ "client_id": "https://other.example/client.json", "redirect_uris": ["https://client.example/cb"] });
        assert!(parse_document(ID, other.to_string().as_bytes()).is_err(), "a document for another id is refused");
        let secret = json!({ "client_id": ID, "redirect_uris": ["https://client.example/cb"], "token_endpoint_auth_method": "private_key_jwt" });
        assert!(parse_document(ID, secret.to_string().as_bytes()).is_err());
        let bad = json!({ "client_id": ID, "redirect_uris": ["http://client.example/cb"] });
        assert!(parse_document(ID, bad.to_string().as_bytes()).is_err());
    }

    #[test]
    fn only_an_https_document_url_on_a_public_address_is_fetched() {
        assert!(document_url("http://client.example/client.json").is_err());
        assert!(document_url("https://client.example/").is_err());
        assert!(document_url("https://user:pw@client.example/c.json").is_err());
        for private in ["https://127.0.0.1/client.json", "https://10.0.0.8/client.json", "https://[::1]/client.json", "https://localhost/client.json", "https://169.254.169.254/latest"] {
            let url = document_url(private).unwrap();
            assert!(checked_address(&url).is_err(), "{private} must be refused");
        }
    }

    #[test]
    fn the_cache_follows_max_age_within_bounds() {
        assert_eq!(cache_ttl(None), DEFAULT_TTL);
        assert_eq!(cache_ttl(Some("public, max-age=600")), Duration::from_secs(600));
        assert_eq!(cache_ttl(Some("max-age=5")), Duration::from_secs(60));
        assert_eq!(cache_ttl(Some("max-age=999999")), Duration::from_secs(86_400));
    }
}
