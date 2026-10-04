//! The two discovery documents of an `oauth` route: RFC 9728 protected
//! resource metadata and RFC 8414 authorization server metadata, each at
//! the path-inserted well-known URI of the route's connect URL. No OpenID
//! Connect discovery: the route signs nobody in to itself.

use axum::http::StatusCode;
use axum::response::Response;
use serde_json::json;

use super::{Ctx, json_answer};

/// The scopes a client may ask for: `mcp` (this route's tools) and
/// `offline_access` (a refresh token).
pub(super) const SCOPES: [&str; 2] = ["mcp", "offline_access"];

pub(super) fn protected_resource(ctx: &Ctx<'_>) -> Response {
    json_answer(
        StatusCode::OK,
        json!({
            "resource": ctx.issuer.resource(),
            "authorization_servers": [ctx.issuer.resource()],
            "scopes_supported": ["mcp"],
            "bearer_methods_supported": ["header"],
            "resource_name": format!("MCP server {}", ctx.route),
        }),
    )
}

pub(super) fn authorization_server(ctx: &Ctx<'_>) -> Response {
    json_answer(
        StatusCode::OK,
        json!({
            "issuer": ctx.issuer.resource(),
            "authorization_endpoint": ctx.issuer.endpoint("authorize"),
            "token_endpoint": ctx.issuer.endpoint("token"),
            "registration_endpoint": ctx.issuer.endpoint("register"),
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "refresh_token"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported": ["none"],
            "scopes_supported": SCOPES,
            "client_id_metadata_document_supported": true,
            "authorization_response_iss_parameter_supported": true,
        }),
    )
}
