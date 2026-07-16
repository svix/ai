//! The app portal MCP server: debugging webhook delivery problems for a single
//! Svix application. The app is fixed for the session, so no tool takes an
//! `app_id`.

pub(crate) mod args;
pub(crate) mod tools;

use rmcp::{
    ErrorData as McpError, RoleServer,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Implementation, InitializeRequestParams, InitializeResult,
        ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use svix::api::Svix;

use self::args::*;
use crate::common::ClientSource;

#[derive(Clone)]
pub(crate) struct AppPortalServer {
    source: ClientSource,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AppPortalServer {
    /// stdio mode: everything from the environment.
    pub(crate) fn new(svix: Svix, app_id: String, customer_name: Option<String>) -> Self {
        Self {
            source: ClientSource::Static {
                svix,
                app_id: Some(app_id),
                customer_name,
            },
            tool_router: Self::tool_router(),
        }
    }

    /// HTTP mode: everything comes per-request from the token.
    pub(crate) fn with_bearer_header_auth(template: Svix) -> Self {
        Self {
            source: ClientSource::BearerHeader(template),
            tool_router: Self::tool_router(),
        }
    }

    /// The Svix client and the application every tool here operates on.
    fn app(&self, ctx: &RequestContext<RoleServer>) -> Result<(Svix, String), McpError> {
        Ok((self.source.client(ctx)?, self.source.app_id(ctx)?))
    }

    #[tool(
        description = "Get the application this session is scoped to: its name, UID, and metadata. Start here to confirm which application you are debugging."
    )]
    async fn get_application(
        &self,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::get_application(&svix, app_id).await
    }

    #[tool(
        description = "List the endpoints configured for this application, including their URL, enabled/disabled state, filtered event types, and rate limits. Use this to see where the customer's webhooks are supposed to be delivered."
    )]
    async fn list_endpoints(
        &self,
        Parameters(args): Parameters<ListEndpointsArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::list_endpoints(&svix, app_id, args).await
    }

    #[tool(
        description = "Get the full configuration of a single endpoint: URL, description, enabled state, channels, filtered event types, headers metadata, and disabled-since timestamp. Use this to confirm an endpoint is configured the way the customer expects."
    )]
    async fn get_endpoint(
        &self,
        Parameters(args): Parameters<EndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::get_endpoint(&svix, app_id, args).await
    }

    #[tool(
        description = "Get delivery statistics for an endpoint over a time window: counts of success, pending, fail, and sending attempts. A high fail count is the clearest signal that an endpoint is broken."
    )]
    async fn get_endpoint_stats(
        &self,
        Parameters(args): Parameters<EndpointStatsArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::get_endpoint_stats(&svix, app_id, args).await
    }

    #[tool(
        description = "Get the transformation configured for an endpoint: its JavaScript code, whether it is enabled, and its variables. Transformations rewrite the payload before delivery, so a broken or disabled transformation is a common cause of malformed or missing webhooks."
    )]
    async fn get_transformation(
        &self,
        Parameters(args): Parameters<EndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::get_transformation(&svix, app_id, args).await
    }

    #[tool(
        description = "Update an endpoint's transformation: set its JavaScript `code` and/or toggle `enabled`. Omitted fields are left unchanged. This modifies the live endpoint configuration, so only call it when the user has asked to change the transformation. Returns the updated transformation."
    )]
    async fn update_transformation(
        &self,
        Parameters(args): Parameters<UpdateTransformationArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::update_transformation(&svix, app_id, args).await
    }

    #[tool(
        description = "List messages sent to this application, newest first. Filter by event type, channel, and time window. Use this to find the message a customer is asking about before drilling into its delivery attempts."
    )]
    async fn list_messages(
        &self,
        Parameters(args): Parameters<ListMessagesArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::list_messages(&svix, app_id, args).await
    }

    #[tool(
        description = "List delivery attempts for an endpoint, newest first. Filter by `status` (use `fail` to see only failed deliveries) and by time window. Each attempt includes the HTTP response code and, when available, the response body returned by the customer's server. This is the primary tool for diagnosing why deliveries are failing."
    )]
    async fn list_attempts_by_endpoint(
        &self,
        Parameters(args): Parameters<AttemptsByEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::list_attempts_by_endpoint(&svix, app_id, args).await
    }

    #[tool(
        description = "List the delivery attempts made for a single message across all endpoints, newest first. Use this when a customer reports that one specific event never arrived, to see which endpoints it was attempted against and how each responded."
    )]
    async fn list_attempts_by_message(
        &self,
        Parameters(args): Parameters<AttemptsByMessageArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::list_attempts_by_message(&svix, app_id, args).await
    }

    #[tool(
        description = "Get a single message including its event type, channels, and the JSON payload that Svix tried to deliver. Use this to confirm the producer actually sent the data the customer expected."
    )]
    async fn get_message(
        &self,
        Parameters(args): Parameters<GetMessageArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::get_message(&svix, app_id, args).await
    }

    #[tool(
        description = "Get a single delivery attempt in full detail: the HTTP response status code, the response body the customer's server returned, the timestamp, and the trigger type. Use this to read the exact error a failing endpoint produced."
    )]
    async fn get_attempt(
        &self,
        Parameters(args): Parameters<GetAttemptArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::get_attempt(&svix, app_id, args).await
    }

    #[tool(
        description = "Resend a single message to a specific endpoint, creating a fresh delivery attempt. Use this to verify a fix for one message. This performs a real delivery. Only call it when the user has asked to resend."
    )]
    async fn resend_message(
        &self,
        Parameters(args): Parameters<ResendArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::resend_message(&svix, app_id, args).await
    }

    #[tool(
        description = "Recover (replay) all failed messages for an endpoint since a given date. Use this after a customer's previously-broken endpoint is fixed to redeliver everything it missed. This enqueues many real deliveries. Only call it when the user has explicitly asked to recover an endpoint."
    )]
    async fn recover_endpoint(
        &self,
        Parameters(args): Parameters<RecoverArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = self.app(&ctx)?;
        tools::recover_endpoint(&svix, app_id, args).await
    }
}

#[tool_handler]
impl rmcp::ServerHandler for AppPortalServer {
    fn get_info(&self) -> ServerInfo {
        // The session-agnostic default; `initialize` names the customer.
        server_info(None)
    }

    /// Override the default `initialize` (which just returns `get_info`) to
    /// tailor the description and triggers to this session's customer.
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        context.peer.set_peer_info(request);
        Ok(server_info(self.source.customer_name(&context).as_deref()))
    }
}

/// Build the server info. Naming the customer (e.g. "Acme") makes agents
/// trigger this server whenever the user is working on that customer's
/// webhooks.
fn server_info(customer: Option<&str>) -> ServerInfo {
    const WORKFLOW: &str = "The application is fixed for this session, so no tool takes an app id. \
        Confirm which app you are debugging with get_application, find the endpoint (list_endpoints \
        / get_endpoint), check its health (get_endpoint_stats), then drill into failures with \
        list_attempts_by_endpoint (status=fail) or, starting from a specific event, list_messages → \
        list_attempts_by_message to read the HTTP status and response body the customer's server \
        returned. Inspect the delivered data with get_message / get_attempt. Once the customer's \
        endpoint is fixed, resend_message replays one message and recover_endpoint replays all \
        failed messages since a date. IDs accept either the raw IDs (ep_/msg_) or customer-assigned \
        UIDs.";

    let mut implementation = Implementation::new("svix-app-portal-mcp", env!("CARGO_PKG_VERSION"));

    let instructions = match customer {
        Some(customer) => {
            implementation.title = Some(format!("{customer} Webhooks"));
            implementation.description = Some(format!("Debug {customer} webhook delivery."));
            format!(
                "Debug {customer} webhook delivery. Use this server whenever the user is working \
                 with {customer} webhooks — for example they mention {customer}, or the codebase \
                 integrates {customer} webhooks (an endpoint that receives {customer} events, \
                 {customer} webhook signature verification, or a {customer} webhook handler). \
                 {WORKFLOW}"
            )
        }
        None => format!("Debug Svix webhook delivery problems for an application. {WORKFLOW}"),
    };

    ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
        .with_server_info(implementation)
        .with_instructions(instructions)
}
