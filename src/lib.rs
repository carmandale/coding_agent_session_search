pub mod config;
pub mod connectors;
pub mod indexer;
pub mod model;
pub mod search;
pub mod storage;
pub mod ui;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use indexer::IndexOptions;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Command-line interface.
#[derive(Parser, Debug)]
#[command(
    name = "cass",
    version,
    about = "Unified TUI search over coding agent histories"
)]
pub struct Cli {
    /// Path to the SQLite database (defaults to platform data dir)
    #[arg(long)]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch interactive TUI
    Tui {
        /// Render once and exit (headless-friendly)
        #[arg(long, default_value_t = false)]
        once: bool,

        /// Override data dir (matches index --data-dir)
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// Run indexer
    Index {
        /// Perform full rebuild
        #[arg(long)]
        full: bool,

        /// Watch for changes and reindex automatically
        #[arg(long)]
        watch: bool,

        /// Override data dir (index + db). Defaults to platform data dir.
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// Generate shell completions to stdout
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Generate man page to stdout
    Man,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Determine data_dir early for logging
    let data_dir = match &cli.command {
        Some(Commands::Tui { data_dir, .. }) => data_dir.clone(),
        Some(Commands::Index { data_dir, .. }) => data_dir.clone(),
        _ => None,
    }
    .unwrap_or_else(default_data_dir);

    std::fs::create_dir_all(&data_dir).ok();

    // Initialize logging to file
    let file_appender = tracing_appender::rolling::daily(&data_dir, "cass.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    // If TUI is NOT running (e.g. Index command or explicit Tui with once?), we might want stdout?
    // But user wants "nicely indexed... in background".
    // Let's log to file always for consistency, and maybe stdout if not TUI?
    // Actually, for TUI apps, file logging is best.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking).compact().with_target(false).with_ansi(false))
        .init();

    let command = cli.command.unwrap_or(Commands::Tui {
        once: false,
        data_dir: None,
    });

    match command {
        Commands::Tui { once, data_dir } => {
            maybe_prompt_for_update(once).await?;
            
            // Spawn background indexer if not in once mode (unless we want it there too? probably not for CI)
            if !once {
                let bg_data_dir = data_dir.clone().unwrap_or_else(default_data_dir);
                let bg_db = cli.db.clone();
                spawn_background_indexer(bg_data_dir, bg_db);
            }

            // Re-resolve data_dir for TUI (it handles None -> default internally, but we passed None above if not set)
            ui::tui::run_tui(data_dir, once)
        }
        Commands::Index {
            full,
            watch,
            data_dir,
        } => run_index_with_data(cli.db, full, watch, data_dir),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(
                shell,
                &mut cmd,
                "cass",
                &mut std::io::stdout(),
            );
            Ok(())
        }
        Commands::Man => {
            let cmd = Cli::command();
            let man = clap_mangen::Man::new(cmd);
            let mut out = std::io::stdout();
            man.render(&mut out)?;
            Ok(())
        }
    }
}

fn spawn_background_indexer(data_dir: PathBuf, db: Option<PathBuf>) {
    std::thread::spawn(move || {
        let db_path = db.unwrap_or_else(|| data_dir.join("agent_search.db"));
        let opts = IndexOptions {
            full: false,
            watch: true,
            db_path,
            data_dir,
        };
        if let Err(e) = indexer::run_index(opts) {
            warn!("Background indexer failed: {}", e);
        }
    });
}


fn run_index_with_data(
    db_override: Option<PathBuf>,
    full: bool,
    watch: bool,
    data_dir_override: Option<PathBuf>,
) -> Result<()> {
    let data_dir = data_dir_override.unwrap_or_else(default_data_dir);
    let db_path = db_override.unwrap_or_else(|| data_dir.join("agent_search.db"));
    let opts = IndexOptions {
        full,
        watch,
        db_path,
        data_dir,
    };
    indexer::run_index(opts)
}

pub fn default_db_path() -> PathBuf {
    default_data_dir().join("agent_search.db")
}

pub fn default_data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "coding-agent-search", "coding-agent-search")
        .expect("project dirs available")
        .data_dir()
        .to_path_buf()
}

const OWNER: &str = "Dicklesworthstone";
const REPO: &str = "coding_agent_session_search";

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    tag_name: String,
}

async fn maybe_prompt_for_update(once: bool) -> Result<()> {
    if once
        || std::env::var("CI").is_ok()
        || std::env::var("TUI_HEADLESS").is_ok()
        || std::env::var("CODING_AGENT_SEARCH_NO_UPDATE_PROMPT").is_ok()
        || !io::stdin().is_terminal()
    {
        return Ok(());
    }

    let client = Client::builder()
        .user_agent("coding-agent-search (update-check)")
        .timeout(Duration::from_secs(3))
        .build()?;

    let Some((latest_tag, latest_ver)) = latest_release_version(&client).await else {
        return Ok(());
    };

    let current_ver =
        Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| Version::new(0, 1, 0));
    if latest_ver <= current_ver {
        return Ok(());
    }

    println!(
        "A newer version is available: current v{}, latest {}. Update now? (y/N): ",
        current_ver, latest_tag
    );
    print!("> ");
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return Ok(());
    }
    if !matches!(input.trim(), "y" | "Y") {
        return Ok(());
    }

    info!(target: "update", "starting self-update to {}", latest_tag);
    match run_self_update(&latest_tag) {
        Ok(true) => {
            println!("Update complete. Please restart cass.");
            std::process::exit(0);
        }
        Ok(false) => {
            warn!(target: "update", "self-update failed (installer returned error)");
        }
        Err(err) => {
            warn!(target: "update", "self-update failed: {err}");
        }
    }

    Ok(())
}

async fn latest_release_version(client: &Client) -> Option<(String, Version)> {
    let url = format!("https://api.github.com/repos/{OWNER}/{REPO}/releases/latest");
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let info: ReleaseInfo = resp.json().await.ok()?;
    let tag = info.tag_name;
    let version_str = tag.trim_start_matches('v');
    let version = Version::parse(version_str).ok()?;
    Some((tag, version))
}

#[cfg(windows)]
fn run_self_update(tag: &str) -> Result<bool> {
    let ps_cmd = format!(
        "irm https://raw.githubusercontent.com/{OWNER}/{REPO}/{tag}/install.ps1 | iex; install.ps1 -EasyMode -Verify -Version {tag}"
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .status()?;
    if status.success() {
        info!(target: "update", "updated to {tag}");
        Ok(true)
    } else {
        warn!(target: "update", "installer returned non-zero status: {status:?}");
        Ok(false)
    }
}

#[cfg(not(windows))]
fn run_self_update(tag: &str) -> Result<bool> {
    let sh_cmd = format!(
        "curl -fsSL https://raw.githubusercontent.com/{OWNER}/{REPO}/{tag}/install.sh | bash -s -- --easy-mode --verify --version {tag}"
    );
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&sh_cmd)
        .status()?;
    if status.success() {
        info!(target: "update", "updated to {tag}");
        Ok(true)
    } else {
        warn!(target: "update", "installer returned non-zero status: {status:?}");
        Ok(false)
    }
}
