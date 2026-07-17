//! Entry point for the two Svix MCP servers.
//!
//! - **app portal** (`/app/{app_id}`): debug webhook delivery for one Svix
//!   application — the webhooks a customer *sends*.
//! - **ingest** (`/ingest`): set up and debug the webhooks an organization
//!   *receives* from third-party providers. It inherits the app portal's tools
//!   and adds the ingest ones.
//!
//! `MCP_TRANSPORT` picks the transport; see README.md for the environment.

mod app_portal;
mod common;
mod ingest;

use anyhow::Context;
use axum::{
    middleware::{Next, from_fn},
    response::{IntoResponse, Response},
};
use http::{StatusCode, header};
use rmcp::{
    ServiceExt,
    transport::{
        stdio,
        streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager},
    },
};
use svix::api::Svix;

use crate::{app_portal::AppPortalServer, common::svix_options, ingest::IngestServer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logs must go to stderr; stdout is the stdio MCP stream.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    match std::env::var("MCP_TRANSPORT").as_deref() {
        Ok("http") => run_http().await,
        Ok("stdio") | Err(_) => run_stdio().await,
        Ok(other) => anyhow::bail!("unknown MCP_TRANSPORT {other:?}; expected `stdio` or `http`"),
    }
}

/// stdio serves a single server, picked by `MCP_SERVER`.
async fn run_stdio() -> anyhow::Result<()> {
    let token = std::env::var("SVIX_TOKEN")
        .context("SVIX_TOKEN environment variable must be set to a Svix API token")?;
    let customer_name = std::env::var("SVIX_CUSTOMER_NAME")
        .ok()
        .filter(|s| !s.is_empty());
    let svix = Svix::new(token, Some(svix_options()));

    match std::env::var("MCP_SERVER").as_deref() {
        Ok("ingest") => {
            tracing::info!("starting svix-ingest-mcp over stdio");
            let service = IngestServer::new(svix, customer_name)
                .serve(stdio())
                .await
                .context("failed to start MCP server")?;
            service.waiting().await.context("MCP server error")?;
        }
        Ok("app-portal") | Err(_) => {
            let app_id = std::env::var("SVIX_APP_ID").context(
                "SVIX_APP_ID environment variable must be set to the application this token debugs",
            )?;
            tracing::info!("starting svix-app-portal-mcp over stdio");
            let service = AppPortalServer::new(svix, app_id, customer_name)
                .serve(stdio())
                .await
                .context("failed to start MCP server")?;
            service.waiting().await.context("MCP server error")?;
        }
        Ok(other) => {
            anyhow::bail!("unknown MCP_SERVER {other:?}; expected `app-portal` or `ingest`")
        }
    }
    Ok(())
}

/// http serves both servers; the path picks one. Each authenticates entirely
/// from the bearer token, so the app portal's `{app_id}` segment is only there
/// to keep one user's MCP clients for several applications from colliding.
async fn run_http() -> anyhow::Result<()> {
    let addr = std::env::var("MCP_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    // Placeholder token is replaced per request via `Svix::with_token`.
    let template = Svix::new("placeholder".to_string(), Some(svix_options()));

    let app_portal = StreamableHttpService::new(
        {
            let template = template.clone();
            move || Ok(AppPortalServer::with_bearer_header_auth(template.clone()))
        },
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let ingest = StreamableHttpService::new(
        move || Ok(IngestServer::with_bearer_header_auth(template.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let app = axum::Router::new()
        .nest_service("/app/{app_id}", app_portal)
        .nest_service("/ingest", ingest)
        .layer(from_fn(require_authorization));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;
    tracing::info!(
        "starting svix mcp over HTTP: app portal on http://{addr}/app/{{app_id}}, ingest on \
         http://{addr}/ingest"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .context("HTTP server error")?;
    Ok(())
}

/// Require a token (401 if absent). Decoding it is `common`'s job.
async fn require_authorization(req: axum::extract::Request, next: Next) -> Response {
    if !req.headers().contains_key(header::AUTHORIZATION) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, r#"Bearer realm="svix-mcp""#)],
            "missing Svix token: pass it as `Authorization: Bearer <token>`",
        )
            .into_response();
    }

    next.run(req).await
}
