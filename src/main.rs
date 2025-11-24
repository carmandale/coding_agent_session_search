use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env early; ignore if missing.
    dotenvy::dotenv().ok();

    coding_agent_search::run().await
}
