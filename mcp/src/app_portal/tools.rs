//! The app portal tool bodies, as plain functions over a Svix client and an
//! application id, so that both servers can call them: the app portal against
//! the application its token is scoped to, the ingest server against the
//! application backing a source (see [`crate::ingest::portal`]). Only the
//! `#[tool]` wrappers differ.

use js_option::JsOption;
use rmcp::{ErrorData as McpError, model::CallToolResult};
use svix::api::{
    EndpointGetStatsOptions, EndpointListOptions, EndpointTransformationPatch,
    MessageAttemptListByEndpointOptions, MessageAttemptListByMsgOptions, MessageListOptions,
    RecoverIn, Svix,
};

use super::args::*;
use crate::common::{DEFAULT_LIMIT, parse_status, to_result, tool_error};

pub(crate) async fn get_application(
    svix: &Svix,
    app_id: String,
) -> Result<CallToolResult, McpError> {
    to_result(svix.application().get(app_id).await)
}

pub(crate) async fn list_endpoints(
    svix: &Svix,
    app_id: String,
    args: ListEndpointsArgs,
) -> Result<CallToolResult, McpError> {
    let options = EndpointListOptions {
        limit: Some(args.limit.unwrap_or(DEFAULT_LIMIT)),
        iterator: args.iterator,
        ..Default::default()
    };
    to_result(svix.endpoint().list(app_id, Some(options)).await)
}

pub(crate) async fn get_endpoint(
    svix: &Svix,
    app_id: String,
    args: EndpointArgs,
) -> Result<CallToolResult, McpError> {
    to_result(svix.endpoint().get(app_id, args.endpoint_id).await)
}

pub(crate) async fn get_endpoint_stats(
    svix: &Svix,
    app_id: String,
    args: EndpointStatsArgs,
) -> Result<CallToolResult, McpError> {
    let options = EndpointGetStatsOptions {
        since: args.since,
        until: args.until,
    };
    to_result(
        svix.endpoint()
            .get_stats(app_id, args.endpoint_id, Some(options))
            .await,
    )
}

pub(crate) async fn get_transformation(
    svix: &Svix,
    app_id: String,
    args: EndpointArgs,
) -> Result<CallToolResult, McpError> {
    to_result(
        svix.endpoint()
            .transformation_get(app_id, args.endpoint_id)
            .await,
    )
}

pub(crate) async fn update_transformation(
    svix: &Svix,
    app_id: String,
    args: UpdateTransformationArgs,
) -> Result<CallToolResult, McpError> {
    let patch = EndpointTransformationPatch {
        code: args.code.map_or(JsOption::Undefined, JsOption::Some),
        enabled: args.enabled,
        variables: JsOption::Undefined,
    };
    if let Err(e) = svix
        .endpoint()
        .patch_transformation(app_id.clone(), args.endpoint_id.clone(), patch)
        .await
    {
        return to_result(Err::<(), _>(e));
    }
    to_result(
        svix.endpoint()
            .transformation_get(app_id, args.endpoint_id)
            .await,
    )
}

pub(crate) async fn list_messages(
    svix: &Svix,
    app_id: String,
    args: ListMessagesArgs,
) -> Result<CallToolResult, McpError> {
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
    to_result(svix.message().list(app_id, Some(options)).await)
}

pub(crate) async fn list_attempts_by_endpoint(
    svix: &Svix,
    app_id: String,
    args: AttemptsByEndpointArgs,
) -> Result<CallToolResult, McpError> {
    let status = match parse_status(args.status.as_deref()) {
        Ok(s) => s,
        Err(e) => return Ok(tool_error(e)),
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
    to_result(
        svix.message_attempt()
            .list_by_endpoint(app_id, args.endpoint_id, Some(options))
            .await,
    )
}

pub(crate) async fn list_attempts_by_message(
    svix: &Svix,
    app_id: String,
    args: AttemptsByMessageArgs,
) -> Result<CallToolResult, McpError> {
    let status = match parse_status(args.status.as_deref()) {
        Ok(s) => s,
        Err(e) => return Ok(tool_error(e)),
    };
    let options = MessageAttemptListByMsgOptions {
        limit: Some(args.limit.unwrap_or(DEFAULT_LIMIT)),
        status,
        with_content: Some(true),
        ..Default::default()
    };
    to_result(
        svix.message_attempt()
            .list_by_msg(app_id, args.msg_id, Some(options))
            .await,
    )
}

pub(crate) async fn get_message(
    svix: &Svix,
    app_id: String,
    args: GetMessageArgs,
) -> Result<CallToolResult, McpError> {
    to_result(svix.message().get(app_id, args.msg_id, None).await)
}

pub(crate) async fn get_attempt(
    svix: &Svix,
    app_id: String,
    args: GetAttemptArgs,
) -> Result<CallToolResult, McpError> {
    to_result(
        svix.message_attempt()
            .get(app_id, args.msg_id, args.attempt_id, None)
            .await,
    )
}

pub(crate) async fn resend_message(
    svix: &Svix,
    app_id: String,
    args: ResendArgs,
) -> Result<CallToolResult, McpError> {
    to_result(
        svix.message_attempt()
            .resend(app_id, args.msg_id, args.endpoint_id, None)
            .await,
    )
}

pub(crate) async fn recover_endpoint(
    svix: &Svix,
    app_id: String,
    args: RecoverArgs,
) -> Result<CallToolResult, McpError> {
    to_result(
        svix.endpoint()
            .recover(
                app_id,
                args.endpoint_id,
                RecoverIn {
                    since: args.since,
                    until: None,
                },
                None,
            )
            .await,
    )
}
