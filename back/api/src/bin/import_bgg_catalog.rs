//! Load `boardgames_ranks.csv` into Surreal table `bgg_catalog`.
//!
//! Prefer (loads `config/.env.dev` like backend-watch):
//!   `./scripts/import-bgg-catalog.sh [path] [batch_id]`
//!
//! Or from repo root, `ENV_FILE_PATH` is auto-set to `config/.env.dev` when unset and that file exists.
//! Raw: `cargo run -p backend --bin import_bgg_catalog -- data/bgg/boardgames_ranks.csv`
//!
//! Optional second arg: `import_batch` string (default: random UUID). Set `RUST_LOG=info` for progress.
//!
//! `BGG_IMPORT_MAX_ROWS`: positive integer imports only that many games, chosen by **newest `yearpublished` first**
//! (full file is read into memory to sort). Omit or `0` for full file in CSV order (~175k rows).

use std::path::{Path, PathBuf};

use backend::bgg_catalog::import::import_csv_from_path;
use backend::bgg_catalog::import_cli::{looks_like_doc_placeholder, parse_bgg_import_max_rows};
use backend::config::Config;
use backend::db::connect_surreal;
use uuid::Uuid;

/// Match backend-watch: use `config/.env.dev` when `ENV_FILE_PATH` is not set (avoids empty Surreal password).
fn ensure_env_file_path_for_local_import() {
    if std::env::var("ENV_FILE_PATH")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        return;
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let try_paths = [
        PathBuf::from("config/.env.dev"),
        manifest.join("../../config/.env.dev"),
    ];
    for p in &try_paths {
        if p.is_file() {
            if let Ok(abs) = p.canonicalize() {
                std::env::set_var("ENV_FILE_PATH", abs.as_os_str());
                return;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ensure_env_file_path_for_local_import();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let config = Config::load().map_err(|e| anyhow::anyhow!("{}", e))?;

    let path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("BGG_RANKS_CSV_PATH").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/bgg/boardgames_ranks.csv"));

    let batch = std::env::args()
        .nth(2)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if !path.is_file() {
        if looks_like_doc_placeholder(&path) {
            anyhow::bail!(
                "The first argument looks like a documentation placeholder (`{}`), not a file path. \
                 Replace it with your CSV, e.g.:\n\
                   cargo run -p backend --bin import_bgg_catalog -- data/bgg/boardgames_ranks.csv\n\
                 Or omit the argument to use the default path data/bgg/boardgames_ranks.csv.",
                path.display()
            );
        }
        anyhow::bail!(
            "CSV not found: {}.\nUse a real path (e.g. data/bgg/boardgames_ranks.csv).",
            path.display()
        );
    }

    log::info!(
        "import_bgg_catalog: path={} batch={}",
        path.display(),
        batch
    );

    let db = connect_surreal(&config.database).await.map_err(|e| {
        anyhow::anyhow!(
            "{e}\n\
             \n\
             Hint: Surreal must accept the same credentials as `backend-watch`. Options:\n\
               • ./scripts/import-bgg-catalog.sh data/bgg/boardgames_ranks.csv\n\
               • export ENV_FILE_PATH=\"$PWD/config/.env.dev\"   # or .env.prod\n\
               • source scripts/load-env.sh dev && cargo run -p backend --bin import_bgg_catalog -- data/bgg/boardgames_ranks.csv\n"
        )
    })?;
    let max_rows = parse_bgg_import_max_rows();
    if let Some(n) = max_rows {
        log::info!("import_bgg_catalog: BGG_IMPORT_MAX_ROWS={n} (partial import)");
    }
    let stats = import_csv_from_path(&db, path.as_path(), &batch, max_rows).await?;
    log::info!("import_bgg_catalog: complete {:?}", stats);
    Ok(())
}
