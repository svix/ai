//! Entry point. Transport is selected by `MCP_TRANSPORT` (`stdio` default, or
//! `http`). stdio reads `SVIX_TOKEN` / `SVIX_APP_ID` from the environment; http
//! reads the Svix MCP token per-request from the `Authorization: Bearer <token>`
//! header (the token also encodes the application id).
//! `SVIX_SERVER_URL` optionally overrides the API base URL.

mod server;

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
use svix::api::{Svix, SvixOptions};

use crate::server::SvixDebugServer;

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

async fn run_stdio() -> anyhow::Result<()> {
    let token = std::env::var("SVIX_TOKEN")
        .context("SVIX_TOKEN environment variable must be set to a Svix API token")?;
    let app_id = std::env::var("SVIX_APP_ID").context(
        "SVIX_APP_ID environment variable must be set to the application this token debugs",
    )?;
    let svix = Svix::new(token, Some(svix_options()));

    tracing::info!("starting svix-app-portal-mcp over stdio");

    let service = SvixDebugServer::new(svix, app_id)
        .serve(stdio())
        .await
        .context("failed to start MCP server")?;
    service.waiting().await.context("MCP server error")?;
    Ok(())
}

async fn run_http() -> anyhow::Result<()> {
    let addr = std::env::var("MCP_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    // Placeholder token is replaced per request via `Svix::with_token`.
    let template = Svix::new("placeholder".to_string(), Some(svix_options()));

    let service = StreamableHttpService::new(
        move || Ok(SvixDebugServer::with_bearer_header_auth(template.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // The slug segment is a cosmetic alias so a user can connect MCP clients for
    // several Svix customers, environments, and regions without the URLs
    // colliding; it is ignored by the server, which authenticates entirely from
    // the bearer token.
    let app = axum::Router::new()
        .nest_service("/mcp/{slug}", service)
        .layer(from_fn(require_authorization));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;
    tracing::info!("starting svix-app-portal-mcp over HTTP on http://{addr}/mcp/{{slug}}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .context("HTTP server error")?;
    Ok(())
}

/// Require a token (401 if absent). The token is the base64 MCP token issued by
/// the Svix app portal, passed as `Authorization: Bearer <token>`; it is decoded
/// per request (see `server`) to obtain the API token and application id.
async fn require_authorization(req: axum::extract::Request, next: Next) -> Response {
    if !req.headers().contains_key(header::AUTHORIZATION) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, r#"Bearer realm="svix-app-portal-mcp""#)],
            "missing Svix token: pass it as `Authorization: Bearer <token>`",
        )
            .into_response();
    }

    next.run(req).await
}

fn svix_options() -> SvixOptions {
    SvixOptions {
        server_url: std::env::var("SVIX_SERVER_URL").ok(),
        ..Default::default()
    }
}
