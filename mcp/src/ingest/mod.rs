//! The ingest MCP server: setting up and debugging *incoming* webhooks.
//!
//! The token is scoped to the organization rather than to one application, so
//! every tool names the source it acts on. The app portal tools are inherited
//! wholesale (see [`crate::app_portal::tools`]); the only difference is that
//! here the application is resolved from the source per call, via
//! [`portal::source_app`].

pub(crate) mod args;
mod portal;

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
use svix::api::{
    IngestEndpointHeadersIn, IngestEndpointIn, IngestEndpointListOptions, IngestEndpointSecretIn,
    IngestEndpointTransformationPatch, IngestEndpointUpdate, IngestSourceConsumerPortalAccessIn,
    IngestSourceListOptions, Svix,
};

use self::args::*;
use crate::{
    app_portal::{args as portal_args, tools},
    common::{ClientSource, DEFAULT_LIMIT, to_result, tool_error},
};

#[derive(Clone)]
pub(crate) struct IngestServer {
    source: ClientSource,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl IngestServer {
    /// stdio mode: everything from the environment.
    pub(crate) fn new(svix: Svix, customer_name: Option<String>) -> Self {
        Self {
            source: ClientSource::Static {
                svix,
                app_id: None,
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

    /// The Svix client and application id a source's messages and delivery
    /// attempts live under.
    async fn source_app(
        &self,
        source_id: &str,
        ctx: &RequestContext<RoleServer>,
    ) -> Result<Result<(Svix, String), CallToolResult>, McpError> {
        let svix = self.source.client(ctx)?;
        Ok(match portal::source_app(&svix, source_id).await {
            Ok(app) => Ok(app),
            Err(e) => Err(tool_error(format!("Svix API error: {e}"))),
        })
    }

    // ---- Sources -----------------------------------------------------------

    #[tool(
        description = "List the ingest sources in this organization. A source is one provider you receive webhooks from (Stripe, GitHub, ...); it has an `ingestUrl` you give that provider, and its `type` decides how Svix verifies what arrives. Start here to find the source you are working with."
    )]
    async fn list_sources(
        &self,
        Parameters(args): Parameters<ListSourcesArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let options = IngestSourceListOptions {
            limit: Some(args.limit.unwrap_or(DEFAULT_LIMIT)),
            iterator: args.iterator,
            ..Default::default()
        };
        to_result(svix.ingest().source().list(Some(options)).await)
    }

    #[tool(
        description = "Get one ingest source: its name, type, `ingestUrl`, and provider configuration. The `ingestUrl` is the URL to paste into the provider's webhook settings — everything that provider POSTs there is verified by Svix and forwarded to the source's ingest endpoints."
    )]
    async fn get_source(
        &self,
        Parameters(args): Parameters<SourceArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        to_result(svix.ingest().source().get(args.source_id).await)
    }

    #[tool(
        description = "Create an ingest source for a provider and get back the `ingestUrl` to register with it. `config` carries the provider's own signing secret so Svix can verify what it sends (e.g. `{\"secret\": \"whsec_...\"}` for Stripe); use type `generic-webhook` for a provider Svix has no built-in verification for. Creates real configuration — only call it when the user asked to set up a source. Add an endpoint next with create_ingest_endpoint."
    )]
    async fn create_source(
        &self,
        Parameters(args): Parameters<CreateSourceArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let source_in = match source_in(
            args.name,
            &args.source_type,
            args.config,
            args.uid,
            args.metadata,
        ) {
            Ok(s) => s,
            Err(e) => return Ok(tool_error(e)),
        };
        to_result(svix.ingest().source().create(source_in, None).await)
    }

    #[tool(
        description = "Update an ingest source. This replaces the source's configuration, so pass every field you want to keep (including `type`). Use it to rotate the provider's signing secret in `config` or rename the source. The `ingestUrl` is unaffected."
    )]
    async fn update_source(
        &self,
        Parameters(args): Parameters<UpdateSourceArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let source_in = match source_in(
            args.name,
            &args.source_type,
            args.config,
            args.uid,
            args.metadata,
        ) {
            Ok(s) => s,
            Err(e) => return Ok(tool_error(e)),
        };
        to_result(
            svix.ingest()
                .source()
                .update(args.source_id, source_in)
                .await,
        )
    }

    #[tool(
        description = "Delete an ingest source, its endpoints, and its `ingestUrl`. Webhooks the provider sends afterwards are lost. Destructive and irreversible — only call it when the user has explicitly asked to delete the source."
    )]
    async fn delete_source(
        &self,
        Parameters(args): Parameters<SourceArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        to_result(svix.ingest().source().delete(args.source_id).await)
    }

    #[tool(
        description = "Rotate a source's URL token, producing a new `ingestUrl`. Use this when the old URL leaked. The previous URL keeps working for 48 hours, so update the provider's webhook settings with the returned `ingestUrl` within that window. Only call it when the user has asked to rotate."
    )]
    async fn rotate_source_token(
        &self,
        Parameters(args): Parameters<SourceArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        to_result(
            svix.ingest()
                .source()
                .rotate_token(args.source_id, None)
                .await,
        )
    }

    #[tool(
        description = "Create a magic link into the Ingest consumer portal for a source — the web UI showing its received webhooks, endpoints, and delivery attempts. Hand the returned `url` to the user when they want to click around rather than have you inspect things through these tools."
    )]
    async fn get_source_portal_link(
        &self,
        Parameters(args): Parameters<SourcePortalArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let access = IngestSourceConsumerPortalAccessIn {
            expiry: args.expiry,
            read_only: args.read_only,
        };
        to_result(svix.ingest().dashboard(args.source_id, access, None).await)
    }

    // ---- Ingest endpoints --------------------------------------------------

    #[tool(
        description = "List a source's ingest endpoints: the URLs Svix forwards the received webhooks to (your own handlers), with their enabled state and rate limits. Use this to see where a provider's webhooks end up."
    )]
    async fn list_ingest_endpoints(
        &self,
        Parameters(args): Parameters<ListIngestEndpointsArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let options = IngestEndpointListOptions {
            limit: Some(args.limit.unwrap_or(DEFAULT_LIMIT)),
            iterator: args.iterator,
            ..Default::default()
        };
        to_result(
            svix.ingest()
                .endpoint()
                .list(args.source_id, Some(options))
                .await,
        )
    }

    #[tool(
        description = "Get one ingest endpoint: its URL, description, enabled state, rate limit, and metadata."
    )]
    async fn get_ingest_endpoint(
        &self,
        Parameters(args): Parameters<IngestEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        to_result(
            svix.ingest()
                .endpoint()
                .get(args.source_id, args.endpoint_id)
                .await,
        )
    }

    #[tool(
        description = "Add an ingest endpoint to a source: the URL Svix forwards that provider's webhooks to. Use this to point a source at the handler you are writing (a tunnel URL while developing, the deployed URL in production). Creates real configuration and starts real deliveries, so only call it when the user asked for it. Follow up with get_ingest_endpoint_secret to get the secret the handler verifies signatures with."
    )]
    async fn create_ingest_endpoint(
        &self,
        Parameters(args): Parameters<CreateIngestEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let endpoint = IngestEndpointIn {
            url: args.url,
            description: args.description,
            uid: args.uid,
            rate_limit: args.rate_limit,
            disabled: args.disabled,
            metadata: args.metadata,
            // Let Svix generate it; get_ingest_endpoint_secret reads it back.
            secret: None,
        };
        to_result(
            svix.ingest()
                .endpoint()
                .create(args.source_id, endpoint, None)
                .await,
        )
    }

    #[tool(
        description = "Update an ingest endpoint. This replaces the endpoint's configuration, so pass every field you want to keep — `url` is required. Use it to repoint a source at a new handler URL, or to disable an endpoint. The signing secret is unaffected."
    )]
    async fn update_ingest_endpoint(
        &self,
        Parameters(args): Parameters<UpdateIngestEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let update = IngestEndpointUpdate {
            url: args.url,
            description: args.description,
            uid: args.uid,
            rate_limit: args.rate_limit,
            disabled: args.disabled,
            metadata: args.metadata,
        };
        to_result(
            svix.ingest()
                .endpoint()
                .update(args.source_id, args.endpoint_id, update)
                .await,
        )
    }

    #[tool(
        description = "Delete an ingest endpoint. Webhooks the source receives afterwards are no longer forwarded to it. Destructive — only call it when the user has explicitly asked to delete the endpoint."
    )]
    async fn delete_ingest_endpoint(
        &self,
        Parameters(args): Parameters<IngestEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        to_result(
            svix.ingest()
                .endpoint()
                .delete(args.source_id, args.endpoint_id)
                .await,
        )
    }

    #[tool(
        description = "Get an ingest endpoint's signing secret (`whsec_...`). This is the secret the receiving handler verifies the `webhook-id` / `webhook-timestamp` / `webhook-signature` headers against, using the Svix or Standard Webhooks library. Fetch it when writing or debugging a handler for this endpoint. It is a credential: put it in an environment variable, never in committed code."
    )]
    async fn get_ingest_endpoint_secret(
        &self,
        Parameters(args): Parameters<IngestEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        to_result(
            svix.ingest()
                .endpoint()
                .get_secret(args.source_id, args.endpoint_id)
                .await,
        )
    }

    #[tool(
        description = "Rotate an ingest endpoint's signing secret. The previous secret stays valid for 24 hours, so deploy the new one within that window or deliveries will start failing verification. Omit `key` to let Svix generate the secret. Only call it when the user has asked to rotate."
    )]
    async fn rotate_ingest_endpoint_secret(
        &self,
        Parameters(args): Parameters<RotateIngestSecretArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let secret = IngestEndpointSecretIn { key: args.key };
        to_result(
            svix.ingest()
                .endpoint()
                .rotate_secret(args.source_id, args.endpoint_id, secret, None)
                .await,
        )
    }

    #[tool(
        description = "Get the custom headers Svix sends with every delivery to an ingest endpoint. Sensitive values are censored and listed under `sensitive`."
    )]
    async fn get_ingest_endpoint_headers(
        &self,
        Parameters(args): Parameters<IngestEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        to_result(
            svix.ingest()
                .endpoint()
                .get_headers(args.source_id, args.endpoint_id)
                .await,
        )
    }

    #[tool(
        description = "Set the custom headers Svix sends with every delivery to an ingest endpoint (an `Authorization` header your handler expects, say). This replaces the existing set, so include the headers you want to keep. Modifies live configuration — only call it when the user has asked to change the headers."
    )]
    async fn update_ingest_endpoint_headers(
        &self,
        Parameters(args): Parameters<UpdateIngestHeadersArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let headers = IngestEndpointHeadersIn {
            headers: args.headers,
        };
        to_result(
            svix.ingest()
                .endpoint()
                .update_headers(args.source_id, args.endpoint_id, headers)
                .await,
        )
    }

    #[tool(
        description = "Get an ingest endpoint's transformation: its JavaScript code and whether it is enabled. Transformations rewrite the provider's payload before it reaches your handler, so a broken or disabled one is a common cause of a handler receiving the wrong shape."
    )]
    async fn get_ingest_endpoint_transformation(
        &self,
        Parameters(args): Parameters<IngestEndpointArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        to_result(
            svix.ingest()
                .endpoint()
                .get_transformation(args.source_id, args.endpoint_id)
                .await,
        )
    }

    #[tool(
        description = "Set an ingest endpoint's transformation: its JavaScript `code` (a `handler` function) and/or toggle `enabled`. Omitted fields are left unchanged. Use it to reshape a provider's payload into what your handler expects. Modifies live configuration — only call it when the user has asked to change the transformation."
    )]
    async fn update_ingest_endpoint_transformation(
        &self,
        Parameters(args): Parameters<UpdateIngestTransformationArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let svix = self.source.client(&ctx)?;
        let patch = IngestEndpointTransformationPatch {
            code: args
                .code
                .map_or(js_option::JsOption::Undefined, js_option::JsOption::Some),
            enabled: args.enabled,
        };
        if let Err(e) = svix
            .ingest()
            .endpoint()
            .set_transformation(args.source_id.clone(), args.endpoint_id.clone(), patch)
            .await
        {
            return to_result(Err::<(), _>(e));
        }
        to_result(
            svix.ingest()
                .endpoint()
                .get_transformation(args.source_id, args.endpoint_id)
                .await,
        )
    }

    // ---- Inherited from the app portal server -------------------------------
    //
    // Same implementations, resolved against the application backing the named
    // source. `list_endpoints` / `get_endpoint` and the transformation tools are
    // deliberately not inherited: the ingest versions above address the same
    // endpoints with ingest semantics.

    #[tool(
        description = "Get delivery statistics for an ingest endpoint over a time window: counts of success, pending, fail, and sending attempts. A high fail count means Svix is receiving the provider's webhooks but your handler is rejecting them."
    )]
    async fn get_endpoint_stats(
        &self,
        Parameters(args): Parameters<WithSource<portal_args::EndpointStatsArgs>>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = match self.source_app(&args.source_id, &ctx).await? {
            Ok(app) => app,
            Err(e) => return Ok(e),
        };
        tools::get_endpoint_stats(&svix, app_id, args.inner).await
    }

    #[tool(
        description = "List the webhooks a source has received from its provider, newest first, with the payload of each. Filter by event type, channel, and time window. Use this to confirm the provider actually sent an event, and to read the exact payload your handler has to parse."
    )]
    async fn list_messages(
        &self,
        Parameters(args): Parameters<WithSource<portal_args::ListMessagesArgs>>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = match self.source_app(&args.source_id, &ctx).await? {
            Ok(app) => app,
            Err(e) => return Ok(e),
        };
        tools::list_messages(&svix, app_id, args.inner).await
    }

    #[tool(
        description = "Get one received webhook: its event type, channels, and the JSON payload as it will reach your handler (after any transformation). Use this to write the handler against the real payload rather than a guess."
    )]
    async fn get_message(
        &self,
        Parameters(args): Parameters<WithSource<portal_args::GetMessageArgs>>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = match self.source_app(&args.source_id, &ctx).await? {
            Ok(app) => app,
            Err(e) => return Ok(e),
        };
        tools::get_message(&svix, app_id, args.inner).await
    }

    #[tool(
        description = "List the attempts Svix made to deliver a source's webhooks to one of your ingest endpoints, newest first. Filter by `status` (use `fail` to see only failures) and by time window. Each attempt carries the HTTP status and response body your handler returned, which is the primary tool for diagnosing a handler that rejects webhooks (a 400 usually means signature verification failed)."
    )]
    async fn list_attempts_by_endpoint(
        &self,
        Parameters(args): Parameters<WithSource<portal_args::AttemptsByEndpointArgs>>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = match self.source_app(&args.source_id, &ctx).await? {
            Ok(app) => app,
            Err(e) => return Ok(e),
        };
        tools::list_attempts_by_endpoint(&svix, app_id, args.inner).await
    }

    #[tool(
        description = "List every attempt made to deliver a single received webhook across the source's ingest endpoints. Use this when one specific event never reached your handler, to see which endpoints it was attempted against and how each responded."
    )]
    async fn list_attempts_by_message(
        &self,
        Parameters(args): Parameters<WithSource<portal_args::AttemptsByMessageArgs>>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = match self.source_app(&args.source_id, &ctx).await? {
            Ok(app) => app,
            Err(e) => return Ok(e),
        };
        tools::list_attempts_by_message(&svix, app_id, args.inner).await
    }

    #[tool(
        description = "Get one delivery attempt in full detail: the HTTP status code and response body your handler returned, the timestamp, and the trigger type. Use this to read the exact error a failing handler produced."
    )]
    async fn get_attempt(
        &self,
        Parameters(args): Parameters<WithSource<portal_args::GetAttemptArgs>>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = match self.source_app(&args.source_id, &ctx).await? {
            Ok(app) => app,
            Err(e) => return Ok(e),
        };
        tools::get_attempt(&svix, app_id, args.inner).await
    }

    #[tool(
        description = "Resend one received webhook to an ingest endpoint, creating a fresh delivery attempt. Use this to test a handler fix against a real payload — it also re-signs the request, so it works where replaying a captured payload with curl would fail signature verification. This performs a real delivery. Only call it when the user has asked to resend."
    )]
    async fn resend_message(
        &self,
        Parameters(args): Parameters<WithSource<portal_args::ResendArgs>>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = match self.source_app(&args.source_id, &ctx).await? {
            Ok(app) => app,
            Err(e) => return Ok(e),
        };
        tools::resend_message(&svix, app_id, args.inner).await
    }

    #[tool(
        description = "Replay every webhook that failed to reach an ingest endpoint since a given date. Use this once a broken handler is fixed, to catch up on what it missed. This enqueues many real deliveries. Only call it when the user has explicitly asked to recover."
    )]
    async fn recover_endpoint(
        &self,
        Parameters(args): Parameters<WithSource<portal_args::RecoverArgs>>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (svix, app_id) = match self.source_app(&args.source_id, &ctx).await? {
            Ok(app) => app,
            Err(e) => return Ok(e),
        };
        tools::recover_endpoint(&svix, app_id, args.inner).await
    }
}

#[tool_handler]
impl rmcp::ServerHandler for IngestServer {
    fn get_info(&self) -> ServerInfo {
        server_info(None)
    }

    /// Tailor the instructions to this session's customer, as the app portal
    /// server does.
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        context.peer.set_peer_info(request);
        Ok(server_info(self.source.customer_name(&context).as_deref()))
    }
}

/// Build the server info. Naming the organization (e.g. "Acme") makes agents
/// trigger this server whenever the user is working on its incoming webhooks.
fn server_info(customer: Option<&str>) -> ServerInfo {
    const MODEL: &str = "Svix Ingest receives webhooks on your behalf: a source has an `ingestUrl` \
        you register with a provider (Stripe, GitHub, ...), Svix verifies what that provider sends, \
        and forwards it to the source's ingest endpoints — your own handlers — signed with a Svix \
        signature your handler verifies using the endpoint's signing secret.";

    const WORKFLOW: &str = "Every tool names the source it acts on; find it with list_sources / \
        get_source. To set up a new provider: create_source (its `config` holds the provider's own \
        signing secret) gives you the `ingestUrl` to register with the provider, \
        create_ingest_endpoint points it at your handler, and get_ingest_endpoint_secret gives you \
        the secret that handler verifies signatures with. To write a handler: read the real payloads \
        with list_messages / get_message. To debug one: get_endpoint_stats for health, then \
        list_attempts_by_endpoint (status=fail) or list_messages → list_attempts_by_message → \
        get_attempt to read the HTTP status and body your handler returned; a 400 usually means \
        signature verification failed (wrong secret, or the body was parsed before verifying). Once \
        the handler is fixed, resend_message replays one webhook (re-signed, so unlike curl it \
        passes verification) and recover_endpoint replays everything that failed since a date. IDs \
        accept raw ids (src_/ep_/msg_) or your own UIDs.";

    let mut implementation = Implementation::new("svix-ingest-mcp", env!("CARGO_PKG_VERSION"));

    let instructions = match customer {
        Some(customer) => {
            implementation.title = Some(format!("{customer} Incoming Webhooks"));
            implementation.description =
                Some(format!("Set up and debug webhooks {customer} receives."));
            format!(
                "Set up and debug the webhooks {customer} receives from third-party providers via \
                 Svix Ingest. Use this server whenever the user is working on incoming webhooks — \
                 writing or debugging a webhook handler, registering a webhook URL with a provider, \
                 or verifying webhook signatures. {MODEL} {WORKFLOW}"
            )
        }
        None => format!(
            "Set up and debug webhooks received from third-party providers via Svix Ingest. Use \
             this server whenever the user is working on incoming webhooks — writing or debugging a \
             webhook handler, registering a webhook URL with a provider, or verifying webhook \
             signatures. {MODEL} {WORKFLOW}"
        ),
    };

    ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
        .with_server_info(implementation)
        .with_instructions(instructions)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn tool_properties(name: &str) -> serde_json::Map<String, serde_json::Value> {
        let router = IngestServer::tool_router();
        let tool = router.get(name).unwrap_or_else(|| panic!("no tool {name}"));
        tool.input_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .cloned()
            .unwrap_or_else(|| panic!("tool {name} has no object schema"))
    }

    /// `WithSource` flattens, where schemars could just as well have emitted a
    /// nested `$ref` that clients would not know how to fill in.
    #[test]
    fn inherited_tools_take_a_flat_source_id() {
        let props = tool_properties("list_messages");
        assert!(props.contains_key("source_id"), "{props:?}");
        // Straight off ListMessagesArgs.
        assert!(props.contains_key("event_types"), "{props:?}");
        assert!(props.contains_key("limit"), "{props:?}");

        let props = tool_properties("get_attempt");
        assert!(props.contains_key("source_id"), "{props:?}");
        assert!(props.contains_key("msg_id"), "{props:?}");
        assert!(props.contains_key("attempt_id"), "{props:?}");
    }

    #[test]
    fn source_in_builds_a_provider_config() {
        let source = source_in(
            "Stripe production".to_owned(),
            "stripe",
            Some(json!({ "secret": "whsec_c2VjcmV0Cg==" })),
            None,
            None,
        )
        .expect("stripe config");
        assert_eq!(
            serde_json::to_value(&source).unwrap(),
            json!({
                "name": "Stripe production",
                "type": "stripe",
                "config": { "secret": "whsec_c2VjcmV0Cg==" },
            })
        );

        // A provider that takes no config.
        let source = source_in("Local".to_owned(), "generic-webhook", None, None, None)
            .expect("generic-webhook config");
        assert_eq!(
            serde_json::to_value(&source).unwrap(),
            json!({ "name": "Local", "type": "generic-webhook" })
        );

        // A config that doesn't match the provider is reported, not silently
        // dropped.
        let err = source_in("Bad".to_owned(), "stripe", Some(json!({})), None, None)
            .expect_err("stripe requires a secret");
        assert!(err.contains("invalid source"), "{err}");
    }
}
