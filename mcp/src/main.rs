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
    Json,
    middleware::{Next, from_fn},
    response::{IntoResponse, Response},
    routing::get,
};
use clap::Parser;
use http::{StatusCode, header};
use rmcp::{
    ServiceExt,
    transport::{
        stdio,
        streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager},
    },
};
use serde::Serialize;
use svix::api::Svix;

use crate::{app_portal::AppPortalServer, common::svix_options, ingest::IngestServer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::Display)]
#[strum(serialize_all = "kebab-case")]
enum McpTransport {
    Http,
    Stdio,
}

#[derive(Parser)]
struct Args {
    #[clap(long, env = "MCP_TRANSPORT", default_value_t = McpTransport::Stdio)]
    transport: McpTransport,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Logs must go to stderr; stdout is the stdio MCP stream.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    match args.transport {
        McpTransport::Http => run_http().await,
        McpTransport::Stdio => run_stdio().await,
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

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
}

async fn healthcheck() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn wait_for_shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = sigterm => {},
    }
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

    let authenticated = axum::Router::new()
        .nest_service("/app/{app_id}", app_portal)
        .nest_service("/ingest", ingest)
        .layer(from_fn(require_authorization));

    let unauthenticated = axum::Router::new().route("/health", get(healthcheck));

    let app = authenticated.merge(unauthenticated);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;
    let bound = listener.local_addr()?;
    tracing::info!(
        "starting svix mcp over HTTP: app portal on http://{bound}/app/{{app_id}}, ingest on \
         http://{bound}/ingest"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(wait_for_shutdown())
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
