//! Verify contest outcome scores against a live SurrealDB (prod-copy or prod).
//!
//! Usage (from repo root, Surreal up with prod seed):
//!   ./scripts/verify-contest-scores.sh
//!   CONTEST_KEY=9e230f40-18e5-439f-82d2-50dea1860e5d ./scripts/verify-contest-scores.sh

use backend::config::Config;
use backend::contest::repository::ContestRepositoryImpl;
use backend::db::connect_surreal;
use std::path::{Path, PathBuf};

fn ensure_env_file_path() {
    if std::env::var("ENV_FILE_PATH")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        return;
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    for p in [PathBuf::from("config/.env.dev"), manifest.join("../../config/.env.dev")] {
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
    ensure_env_file_path();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .try_init();

    let contest_key = std::env::var("CONTEST_KEY")
        .unwrap_or_else(|_| "9e230f40-18e5-439f-82d2-50dea1860e5d".to_string());
    let contest_id = format!("contest/{}", contest_key.trim().trim_start_matches("contest/"));

    let config = Config::load().map_err(|e| anyhow::anyhow!("{}", e))?;
    let db = connect_surreal(&config.database).await?;
    let repo = ContestRepositoryImpl::new_with_scope(
        db.clone(),
        None,
        config.database.ns.clone(),
        config.database.name.clone(),
    );

    let Some(dto) = repo.find_details_by_id_using(&contest_id, &db).await else {
        anyhow::bail!("contest not found: {contest_id}");
    };

    println!("Contest: {}", dto.name);
    println!("Outcomes: {}", dto.outcomes.len());
    let mut with_score = 0usize;
    for o in &dto.outcomes {
        let score = o.score.trim();
        if !score.is_empty() {
            with_score += 1;
        }
        println!(
            "  place={} result={} score={}",
            o.place,
            o.result,
            if score.is_empty() { "—" } else { score }
        );
    }

    if with_score == 0 {
        anyhow::bail!("no outcome scores returned (expected at least one for Shopmen / prod sample)");
    }

    println!("OK: {with_score}/{} outcomes have scores", dto.outcomes.len());
    Ok(())
}
