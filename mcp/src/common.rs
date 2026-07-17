//! Shared plumbing for both MCP servers: the Svix MCP token, how a request is
//! turned into a Svix client, and tool result formatting.

use base64::{Engine, engine::general_purpose::STANDARD};
use rmcp::{
    ErrorData as McpError, RoleServer,
    model::{CallToolResult, Content},
    service::RequestContext,
};
use serde::Deserialize;
use svix::api::{MessageStatus, Svix, SvixOptions};

pub(crate) const DEFAULT_LIMIT: i32 = 20;

const MCP_TOKEN_PREFIX: &str = "mcp_v1_";

/// Contents of the base64 Svix MCP token. An app portal token is scoped to one
/// application; an ingest token is scoped to the organization and carries no
/// `aid`.
#[derive(Deserialize)]
pub(crate) struct McpTokenContent {
    #[serde(rename = "aid", default)]
    pub app_id: Option<String>,
    #[serde(rename = "tok")]
    pub token: String,
    /// The customer/brand the token was issued for (e.g. "Acme"). The issuer
    /// omits it when it would be empty.
    #[serde(rename = "cust", default)]
    pub customer_name: String,
}

/// How the Svix client (and, for the app portal, the application id) are
/// obtained for a request.
#[derive(Clone)]
pub(crate) enum ClientSource {
    /// stdio mode: everything from the environment.
    Static {
        svix: Svix,
        app_id: Option<String>,
        customer_name: Option<String>,
    },
    /// HTTP mode: everything decoded per-request from the MCP token.
    BearerHeader(Svix),
}

impl ClientSource {
    pub(crate) fn client(&self, ctx: &RequestContext<RoleServer>) -> Result<Svix, McpError> {
        match self {
            Self::Static { svix, .. } => Ok(svix.clone()),
            Self::BearerHeader(template) => Ok(template.with_token(decode_mcp_token(ctx)?.token)),
        }
    }

    /// The application this session is scoped to. App portal only; an ingest
    /// token carries no application.
    pub(crate) fn app_id(&self, ctx: &RequestContext<RoleServer>) -> Result<String, McpError> {
        let app_id = match self {
            Self::Static { app_id, .. } => app_id.clone(),
            Self::BearerHeader(_) => decode_mcp_token(ctx)?.app_id,
        };
        app_id.ok_or_else(|| {
            McpError::invalid_request(
                "this token is not scoped to an application; it cannot be used with the app portal \
                 MCP server",
                None,
            )
        })
    }

    /// The customer/brand name this session is scoped to. `None` when unset or
    /// when the token predates the field, in which case the servers fall back to
    /// generic instructions.
    pub(crate) fn customer_name(&self, ctx: &RequestContext<RoleServer>) -> Option<String> {
        let name = match self {
            Self::Static { customer_name, .. } => customer_name.clone(),
            Self::BearerHeader(_) => decode_mcp_token(ctx).ok().map(|c| c.customer_name),
        };
        name.filter(|s| !s.is_empty())
    }
}

fn decode_mcp_token(ctx: &RequestContext<RoleServer>) -> Result<McpTokenContent, McpError> {
    let parts = ctx
        .extensions
        .get::<http::request::Parts>()
        .ok_or_else(|| {
            McpError::invalid_request(
                "this server requires the Svix MCP token in the Authorization header, but no HTTP \
                 request context was found",
                None,
            )
        })?;

    let header = parts
        .headers
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| McpError::invalid_request("missing Authorization header", None))?;

    let raw = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .unwrap_or(header)
        .trim();

    let encoded = raw.strip_prefix(MCP_TOKEN_PREFIX).ok_or_else(|| {
        McpError::invalid_request(
            "invalid Svix MCP token: expected a token beginning with `mcp_v1_` (get one from the \
             Svix app portal or dashboard)",
            None,
        )
    })?;

    let json = STANDARD
        .decode(encoded)
        .map_err(|_| McpError::invalid_request("invalid Svix MCP token: not valid base64", None))?;

    serde_json::from_slice::<McpTokenContent>(&json)
        .map_err(|_| McpError::invalid_request("invalid Svix MCP token: malformed contents", None))
}

pub(crate) fn svix_options() -> SvixOptions {
    SvixOptions {
        server_url: std::env::var("SVIX_SERVER_URL").ok(),
        ..Default::default()
    }
}

pub(crate) fn parse_status(status: Option<&str>) -> Result<Option<MessageStatus>, String> {
    match status {
        None => Ok(None),
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "success" => Ok(Some(MessageStatus::Success)),
            "pending" => Ok(Some(MessageStatus::Pending)),
            "fail" | "failed" => Ok(Some(MessageStatus::Fail)),
            "sending" => Ok(Some(MessageStatus::Sending)),
            "canceled" | "cancelled" => Ok(Some(MessageStatus::Canceled)),
            other => Err(format!(
                "invalid status {other:?}; expected one of: success, pending, fail, sending, canceled"
            )),
        },
    }
}

pub(crate) fn to_result<T: serde::Serialize>(
    res: Result<T, svix::error::Error>,
) -> Result<CallToolResult, McpError> {
    match res {
        Ok(value) => {
            let json = serde_json::to_string_pretty(&value)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }
        Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
            "Svix API error: {e}"
        ))])),
    }
}

/// A tool result carrying a plain error message, as opposed to a protocol-level
/// failure.
pub(crate) fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(message.into())])
}
