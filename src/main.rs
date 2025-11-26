#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env early; ignore if missing.
    dotenvy::dotenv().ok();

    match coding_agent_search::run().await {
        Ok(()) => Ok(()),
        Err(err) => {
            let payload = serde_json::json!({
                "error": {
                    "code": err.code,
                    "kind": err.kind,
                    "message": err.message,
                    "hint": err.hint,
                    "retryable": err.retryable,
                }
            });
            eprintln!("{}", payload);
            std::process::exit(err.code);
        }
    }
}
