use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value};
use svix::api::IngestSourceIn;

/// An app portal tool's arguments, plus the ingest source they apply to, merged
/// so the tool takes `{source_id, ...}` flat.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct WithSource<T> {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    #[serde(flatten)]
    pub inner: T,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ListSourcesArgs {
    /// Max number of sources to return. Defaults to 20.
    pub limit: Option<i32>,
    /// Pagination iterator returned by a previous call.
    pub iterator: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct SourceArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CreateSourceArgs {
    /// Human-readable name for the source (e.g. "Stripe production").
    pub name: String,
    /// The provider sending the webhooks, which decides how Svix verifies them.
    #[serde(rename = "type")]
    pub source_type: String,
    /// Provider-specific configuration, most often the signing secret the
    /// provider gave you (e.g. `{"secret": "whsec_..."}` for Stripe). Required
    /// for most providers; omit for `generic-webhook`. If you get a validation
    /// error, the message names the fields this provider needs.
    pub config: Option<Value>,
    /// Optional unique identifier for the source, to address it by your own id.
    pub uid: Option<String>,
    /// Arbitrary key/value metadata to store on the source.
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct UpdateSourceArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// Human-readable name for the source.
    pub name: String,
    /// The provider sending the webhooks. See `create_source` for the full list.
    #[serde(rename = "type")]
    pub source_type: String,
    /// Provider-specific configuration (e.g. the provider's signing secret).
    pub config: Option<Value>,
    /// Optional unique identifier for the source.
    pub uid: Option<String>,
    /// Arbitrary key/value metadata to store on the source.
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct SourcePortalArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// How long the link stays valid, in seconds. Between 3600 (1 hour) and
    /// 604800 (7 days). Defaults to 7 days.
    pub expiry: Option<i32>,
    /// Whether the portal opens in read-only mode.
    pub read_only: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ListIngestEndpointsArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// Max number of endpoints to return. Defaults to 20.
    pub limit: Option<i32>,
    /// Pagination iterator returned by a previous call.
    pub iterator: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct IngestEndpointArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// The ingest endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CreateIngestEndpointArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// The URL Svix forwards the received webhooks to — your own handler.
    pub url: String,
    /// Human-readable description of the endpoint.
    pub description: Option<String>,
    /// Optional unique identifier for the endpoint.
    pub uid: Option<String>,
    /// Max deliveries per second to this endpoint.
    pub rate_limit: Option<u16>,
    /// Create the endpoint disabled (no deliveries until enabled).
    pub disabled: Option<bool>,
    /// Arbitrary key/value metadata to store on the endpoint.
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct UpdateIngestEndpointArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// The ingest endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
    /// The URL Svix forwards the received webhooks to. Required: an update
    /// replaces the endpoint, so passing the current URL keeps it unchanged.
    pub url: String,
    /// Human-readable description of the endpoint.
    pub description: Option<String>,
    /// Optional unique identifier for the endpoint.
    pub uid: Option<String>,
    /// Max deliveries per second to this endpoint.
    pub rate_limit: Option<u16>,
    /// Whether the endpoint is disabled.
    pub disabled: Option<bool>,
    /// Arbitrary key/value metadata to store on the endpoint.
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct RotateIngestSecretArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// The ingest endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
    /// The new signing secret (`whsec_` followed by base64). Omit to have Svix
    /// generate one, which is recommended.
    pub key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct UpdateIngestHeadersArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// The ingest endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
    /// The custom headers to send with every delivery. Replaces the existing
    /// set.
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct UpdateIngestTransformationArgs {
    /// The ingest source ID or UID (e.g. `src_...`).
    pub source_id: String,
    /// The ingest endpoint ID or UID (e.g. `ep_...`).
    pub endpoint_id: String,
    /// The transformation code (a JavaScript `handler` function). Omit to leave
    /// the code unchanged.
    pub code: Option<String>,
    /// Whether the transformation is enabled. Omit to leave unchanged.
    pub enabled: Option<bool>,
}

/// Build an [`IngestSourceIn`] from the provider `type` and its free-form
/// `config`.
///
/// The provider config is an enum of ~40 variants in the SDK; going through JSON
/// keeps every provider (and any the SDK gains later) reachable without
/// enumerating them here.
pub(crate) fn source_in(
    name: String,
    source_type: &str,
    config: Option<Value>,
    uid: Option<String>,
    metadata: Option<HashMap<String, String>>,
) -> Result<IngestSourceIn, String> {
    let mut obj = Map::new();
    obj.insert("name".to_owned(), Value::String(name));
    obj.insert("type".to_owned(), Value::String(source_type.to_owned()));
    if let Some(config) = config {
        obj.insert("config".to_owned(), config);
    }
    if let Some(uid) = uid {
        obj.insert("uid".to_owned(), Value::String(uid));
    }
    if let Some(metadata) = metadata {
        obj.insert(
            "metadata".to_owned(),
            serde_json::to_value(metadata).map_err(|e| e.to_string())?,
        );
    }

    serde_json::from_value(Value::Object(obj)).map_err(|e| {
        format!(
            "invalid source: {e}. Check that `type` is a supported provider and that `config` has \
             the fields that provider requires (usually its signing secret)."
        )
    })
}
