//! Reaching an ingest source's messages and delivery attempts.
//!
//! Ingest sources are backed by a regular Svix application (uid `app_<source
//! id>`) on a separate deployment, so an organization's ingest token cannot read
//! its messages directly. Minting a consumer portal token for the source does
//! give an application-scoped token for that backing app, which is what lets the
//! app portal tools run unchanged here.

use svix::api::{IngestSourceConsumerPortalAccessIn, Svix};

/// Resolve an ingest source to a Svix client and application id its messages and
/// attempts can be read with. `source_id` may be an id or uid, and is resolved
/// to the id because the backing application's uid derives from it.
///
/// A fresh portal token is minted per call rather than cached: a cache would
/// have to be keyed by the caller's token as well as the source, and getting
/// that wrong would hand one organization a portal token for another's source.
pub(crate) async fn source_app(
    svix: &Svix,
    source_id: &str,
) -> Result<(Svix, String), svix::error::Error> {
    let source = svix.ingest().source().get(source_id.to_owned()).await?;

    let access = svix
        .ingest()
        .dashboard(
            source.id.clone(),
            IngestSourceConsumerPortalAccessIn {
                expiry: None,
                read_only: None,
            },
            None,
        )
        .await?;

    // `with_token` reuses the connection pool and any `SVIX_SERVER_URL`
    // override; otherwise the base URL comes from the token's region suffix.
    Ok((svix.with_token(access.token), format!("app_{}", source.id)))
}
