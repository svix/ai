//! MCP server for debugging webhook delivery problems for a single Svix
//! application. The app is fixed for the session, so no tool takes an `app_id`.

use rmcp::{
    ErrorData as McpError, RoleServer,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::Deserialize;
use svix::api::{
    EndpointGetStatsOptions, EndpointListOptions, MessageAttemptListByEndpointOptions,
    MessageAttemptListByMsgOptions, MessageListOptions, MessageStatus, Svix,
};

const DEFAULT_LIMIT: i32 = 20;
const APP_ID_HEADER: &str = "x-svix-app-id";

/// How the Svix client and application id are obtained for a request.
#[derive(Clone)]
enum ClientSource {
    /// stdio mode: static client and app id from `SVIX_TOKEN` / `SVIX_APP_ID`.
    Static { svix: Svix, app_id: String },
    /// HTTP mode: token and app id read per-request from the connection.
    BearerHeader(Svix),
}

#[derive(Clone)]
pub(crate) struct SvixDebugServer {
    source: ClientSource,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ListEndpointsArgs {
    /// Max number of endpoints to return. Defaults to 20.
    pub limit: Option<i32>,
    /// Pagination iterator returned by a previous call.
    pub iterator: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct EndpointArgs {
    /// The endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct EndpointStatsArgs {
    /// The endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
    /// Start of the window, an RFC3339 date string. Defaults to 7 days ago.
    pub since: Option<String>,
    /// End of the window, an RFC3339 date string. Defaults to now.
    pub until: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ListMessagesArgs {
    /// Only include messages of these event types.
    pub event_types: Option<Vec<String>>,
    /// Only include messages sent on this channel.
    pub channel: Option<String>,
    /// Only include messages created after this RFC3339 date string.
    pub after: Option<String>,
    /// Only include messages created before this RFC3339 date string.
    pub before: Option<String>,
    /// Max number of messages to return. Defaults to 20.
    pub limit: Option<i32>,
    /// Pagination iterator returned by a previous call.
    pub iterator: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct AttemptsByEndpointArgs {
    /// The endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
    /// Filter by delivery status. One of `success`, `pending`, `fail`,
    /// `sending`, `canceled`. Omit to return all statuses.
    pub status: Option<String>,
    /// Only include attempts created after this RFC3339 date string.
    pub after: Option<String>,
    /// Only include attempts created before this RFC3339 date string.
    pub before: Option<String>,
    /// Max number of attempts to return. Defaults to 20.
    pub limit: Option<i32>,
    /// Pagination iterator returned by a previous call.
    pub iterator: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct AttemptsByMessageArgs {
    /// The message ID (e.g. `msg_...`).
    pub msg_id: String,
    /// Filter by delivery status. One of `success`, `pending`, `fail`,
    /// `sending`, `canceled`. Omit to return all statuses.
    pub status: Option<String>,
    /// Max number of attempts to return. Defaults to 20.
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct GetMessageArgs {
    /// The message ID (e.g. `msg_...`).
    pub msg_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct GetAttemptArgs {
    /// The message ID (e.g. `msg_...`).
    pub msg_id: String,
    /// The message attempt ID (e.g. `atmpt_...`).
    pub attempt_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ResendArgs {
    /// The message ID (e.g. `msg_...`).
    pub msg_id: String,
    /// The endpoint ID or UID to resend the message to (e.g. `ep_...`).
    pub endpoint_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct RecoverArgs {
    /// The endpoint ID or UID to recover (e.g. `ep_...`).
    pub endpoint_id: String,
    /// Replay all failed messages since this RFC3339 date string.
    pub since: String,
}

#[tool_router]
impl SvixDebugServer {
    /// stdio mode: static client and app id from the environment.
    pub(crate) fn new(svix: Svix, app_id: String) -> Self {
        Self {
            source: ClientSource::Static { svix, app_id },
            tool_router: Self::tool_router(),
        }
    }

    /// HTTP mode: token and app id come per-request from the connection.
    pub(crate) fn with_bearer_header_auth(template: Svix) -> Self {
        Self {
            source: ClientSource::BearerHeader(template),
            tool_router: Self::tool_router(),
        }
    }

    fn client(&self, ctx: &RequestContext<RoleServer>) -> Result<Svix, McpError> {
        match &self.source {
            ClientSource::Static { svix, .. } => Ok(svix.clone()),
            ClientSource::BearerHeader(template) => Ok(template.with_token(bearer_token(ctx)?)),
        }
    }

    fn app_id(&self, ctx: &RequestContext<RoleServer>) -> Result<String, McpError> {
        match &self.source {
            ClientSource::Static { app_id, .. } => Ok(app_id.clone()),
            ClientSource::BearerHeader(_) => app_id_from_request(ctx),
        }
    }

    #[tool(
        description = "Get the application this session is scoped to: its name, UID, and metadata. Start here to confirm which application you are debugging."
    )]
    async fn get_application(
        &self,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let res = svix.application().get(self.app_id(&ctx)?).await;
        to_result(res)
    }

    #[tool(
        description = "List the endpoints configured for this application, including their URL, enabled/disabled state, filtered event types, and rate limits. Use this to see where the customer's webhooks are supposed to be delivered."
    )]
    async fn list_endpoints(
        &self,
        Parameters(args): Parameters<ListEndpointsArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let options = EndpointListOptions {
            limit: Some(args.limit.unwrap_or(DEFAULT_LIMIT)),
            iterator: args.iterator,
            ..Default::default()
        };
        let res = svix
            .endpoint()
            .list(self.app_id(&ctx)?, Some(options))
            .await;
        to_result(res)
    }

    #[tool(
        description = "Get the full configuration of a single endpoint: URL, description, enabled state, channels, filtered event types, headers metadata, and disabled-since timestamp. Use this to confirm an endpoint is configured the way the customer expects."
    )]
    async fn get_endpoint(
        &self,
        Parameters(args): Parameters<EndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let res = svix
            .endpoint()
            .get(self.app_id(&ctx)?, args.endpoint_id)
            .await;
        to_result(res)
    }

    #[tool(
        description = "Get delivery statistics for an endpoint over a time window: counts of success, pending, fail, and sending attempts. A high fail count is the clearest signal that an endpoint is broken."
    )]
    async fn get_endpoint_stats(
        &self,
        Parameters(args): Parameters<EndpointStatsArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let options = EndpointGetStatsOptions {
            since: args.since,
            until: args.until,
        };
        let res = svix
            .endpoint()
            .get_stats(self.app_id(&ctx)?, args.endpoint_id, Some(options))
            .await;
        to_result(res)
    }

    #[tool(
        description = "List messages sent to this application, newest first. Filter by event type, channel, and time window. Use this to find the message a customer is asking about before drilling into its delivery attempts."
    )]
    async fn list_messages(
        &self,
        Parameters(args): Parameters<ListMessagesArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let options = MessageListOptions {
            limit: Some(args.limit.unwrap_or(DEFAULT_LIMIT)),
            iterator: args.iterator,
            channel: args.channel,
            before: args.before,
            after: args.after,
            event_types: args.event_types,
            with_content: Some(true),
            ..Default::default()
        };
        let res = svix.message().list(self.app_id(&ctx)?, Some(options)).await;
        to_result(res)
    }

    #[tool(
        description = "List delivery attempts for an endpoint, newest first. Filter by `status` (use `fail` to see only failed deliveries) and by time window. Each attempt includes the HTTP response code and, when available, the response body returned by the customer's server. This is the primary tool for diagnosing why deliveries are failing."
    )]
    async fn list_attempts_by_endpoint(
        &self,
        Parameters(args): Parameters<AttemptsByEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let status = match parse_status(args.status.as_deref()) {
            Ok(s) => s,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(e)])),
        };
        let options = MessageAttemptListByEndpointOptions {
            limit: Some(args.limit.unwrap_or(DEFAULT_LIMIT)),
            iterator: args.iterator,
            status,
            after: args.after,
            before: args.before,
            with_content: Some(true),
            with_msg: Some(true),
            ..Default::default()
        };
        let res = svix
            .message_attempt()
            .list_by_endpoint(self.app_id(&ctx)?, args.endpoint_id, Some(options))
            .await;
        to_result(res)
    }

    #[tool(
        description = "List the delivery attempts made for a single message across all endpoints, newest first. Use this when a customer reports that one specific event never arrived, to see which endpoints it was attempted against and how each responded."
    )]
    async fn list_attempts_by_message(
        &self,
        Parameters(args): Parameters<AttemptsByMessageArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let status = match parse_status(args.status.as_deref()) {
            Ok(s) => s,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(e)])),
        };
        let options = MessageAttemptListByMsgOptions {
            limit: Some(args.limit.unwrap_or(DEFAULT_LIMIT)),
            status,
            with_content: Some(true),
            ..Default::default()
        };
        let res = svix
            .message_attempt()
            .list_by_msg(self.app_id(&ctx)?, args.msg_id, Some(options))
            .await;
        to_result(res)
    }

    #[tool(
        description = "Get a single message including its event type, channels, and the JSON payload that Svix tried to deliver. Use this to confirm the producer actually sent the data the customer expected."
    )]
    async fn get_message(
        &self,
        Parameters(args): Parameters<GetMessageArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let res = svix
            .message()
            .get(self.app_id(&ctx)?, args.msg_id, None)
            .await;
        to_result(res)
    }

    #[tool(
        description = "Get a single delivery attempt in full detail: the HTTP response status code, the response body the customer's server returned, the timestamp, and the trigger type. Use this to read the exact error a failing endpoint produced."
    )]
    async fn get_attempt(
        &self,
        Parameters(args): Parameters<GetAttemptArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let res = svix
            .message_attempt()
            .get(self.app_id(&ctx)?, args.msg_id, args.attempt_id, None)
            .await;
        to_result(res)
    }

    #[tool(
        description = "Resend a single message to a specific endpoint, creating a fresh delivery attempt. Use this to verify a fix for one message. This performs a real delivery. Only call it when the user has asked to resend."
    )]
    async fn resend_message(
        &self,
        Parameters(args): Parameters<ResendArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let res = svix
            .message_attempt()
            .resend(self.app_id(&ctx)?, args.msg_id, args.endpoint_id, None)
            .await;
        to_result(res)
    }

    #[tool(
        description = "Recover (replay) all failed messages for an endpoint since a given date. Use this after a customer's previously-broken endpoint is fixed to redeliver everything it missed. This enqueues many real deliveries. Only call it when the user has explicitly asked to recover an endpoint."
    )]
    async fn recover_endpoint(
        &self,
        Parameters(args): Parameters<RecoverArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.client(&ctx)?;
        let res = svix
            .endpoint()
            .recover(
                self.app_id(&ctx)?,
                args.endpoint_id,
                svix::api::RecoverIn {
                    since: args.since,
                    until: None,
                },
                None,
            )
            .await;
        to_result(res)
    }
}

#[tool_handler]
impl rmcp::ServerHandler for SvixDebugServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "svix-app-portal-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Debug Svix webhook delivery problems for an application. The application \
                 is fixed for this session, so no tool takes an app id. Confirm which app you are \
                 debugging with get_application, find the endpoint (list_endpoints / get_endpoint), \
                 check its health (get_endpoint_stats), then drill into failures with \
                 list_attempts_by_endpoint (status=fail) or, starting from a specific event, \
                 list_messages → list_attempts_by_message to read the HTTP status and response body \
                 the customer's server returned. Inspect the delivered data with get_message / \
                 get_attempt. Once the customer's endpoint is fixed, resend_message replays one \
                 message and recover_endpoint replays all failed messages since a date. IDs accept \
                 either Svix IDs (ep_/msg_) or customer-assigned UIDs.",
            )
    }
}

fn bearer_token(ctx: &RequestContext<RoleServer>) -> Result<String, McpError> {
    let parts = ctx
        .extensions
        .get::<http::request::Parts>()
        .ok_or_else(|| {
            McpError::invalid_request(
                "this server requires the Svix token in the Authorization header, but no HTTP \
             request context was found",
                None,
            )
        })?;

    let header = parts
        .headers
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| McpError::invalid_request("missing Authorization header", None))?;

    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .unwrap_or(header)
        .trim();

    if token.is_empty() {
        return Err(McpError::invalid_request(
            "empty bearer token in Authorization header",
            None,
        ));
    }
    Ok(token.to_string())
}

fn app_id_from_request(ctx: &RequestContext<RoleServer>) -> Result<String, McpError> {
    let parts = ctx
        .extensions
        .get::<http::request::Parts>()
        .ok_or_else(|| {
            McpError::invalid_request(
                "this server requires the application id, but no HTTP request context was found",
                None,
            )
        })?;

    let app_id = parts
        .headers
        .get(APP_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            McpError::invalid_request(
                "missing application id: pass it as a `?app_id=<app_id>` query parameter or an \
                 `x-svix-app-id` header",
                None,
            )
        })?;

    Ok(app_id.to_string())
}

fn parse_status(status: Option<&str>) -> Result<Option<MessageStatus>, String> {
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

fn to_result<T: serde::Serialize>(
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
