mod backend;
mod domain;
mod local_ledger;
mod quickbooks;
mod server;
mod xero;

use std::sync::Arc;

use adk_mcp_sdk::{HealthCheck, ServerManifest};
use rmcp::{ServiceExt, transport::stdio};

use crate::local_ledger::LocalLedger;
use crate::quickbooks::QuickBooksBackend;
use crate::server::FinanceServer;
use crate::xero::XeroBackend;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let manifest = ServerManifest::from_file(std::path::Path::new("mcp-server.toml"))?;
    let errors = manifest.validate();
    if !errors.is_empty() {
        for e in &errors { tracing::error!("manifest: {e}"); }
        anyhow::bail!("invalid mcp-server.toml ({} error(s))", errors.len());
    }

    // Detect backend from env vars
    let backend: Arc<dyn crate::backend::FinanceBackend> =
        if let (Ok(token), Ok(realm)) = (std::env::var("QUICKBOOKS_ACCESS_TOKEN"), std::env::var("QUICKBOOKS_REALM_ID")) {
            tracing::info!("Using QuickBooks Online backend");
            Arc::new(QuickBooksBackend::new(token, realm))
        } else if let (Ok(token), Ok(tenant)) = (std::env::var("XERO_ACCESS_TOKEN"), std::env::var("XERO_TENANT_ID")) {
            tracing::info!("Using Xero backend");
            Arc::new(XeroBackend::new(token, tenant))
        } else {
            tracing::info!("No API credentials found — using local in-memory ledger");
            Arc::new(LocalLedger::new())
        };

    let server = FinanceServer { backend };

    let health = server.check_health().await;
    if !health.healthy {
        tracing::error!(message = ?health.message, "Health check failed");
        std::process::exit(1);
    }

    tracing::info!("{} v{} starting on stdio", manifest.display_name, manifest.version);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
