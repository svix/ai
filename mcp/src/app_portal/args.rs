use serde::Deserialize;

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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct UpdateTransformationArgs {
    /// The endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
    /// The transformation code (a JavaScript `handler` function). Omit to leave
    /// the code unchanged.
    pub code: Option<String>,
    /// Whether the transformation is enabled. Omit to leave unchanged.
    pub enabled: Option<bool>,
}
