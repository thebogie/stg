//! Retroactively rename contests to `{Game} — {Weekday Mon D}`.
//!
//! Prefer (loads `config/.env.dev` like backend-watch):
//!   `./scripts/backfill-contest-names.sh`
//!
//! Dry-run (default): prints planned renames.
//! Apply: `./scripts/backfill-contest-names.sh --apply`
//!
//! Production (on server, against prod SurrealDB):
//!   ENV_FILE_PATH=/path/to/.env.prod ./scripts/backfill-contest-names.sh --apply
//! Or exec into backend container with prod env and run the same cargo bin.

use backend::config::Config;
use backend::contest::backfill_names::{apply_contest_name_backfill, plan_contest_name_backfill};
use backend::db::connect_surreal;
use std::path::{Path, PathBuf};

fn ensure_env_file_path_for_local() {
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

fn print_usage() {
    eprintln!(
        "Usage: backfill_contest_names [--apply] [--limit N]\n\
         \n\
         Default is dry-run: lists contests that would be renamed.\n\
         --apply  write new names to SurrealDB\n\
         --limit N  process only the first N contests (ordered by start)"
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ensure_env_file_path_for_local();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let mut apply = false;
    let mut limit: Option<usize> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--apply" => apply = true,
            "--limit" => {
                i += 1;
                limit = Some(
                    args.get(i)
                        .ok_or_else(|| anyhow::anyhow!("--limit requires a number"))?
                        .parse()
                        .map_err(|_| anyhow::anyhow!("--limit must be a positive integer"))?,
                );
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
        i += 1;
    }

    let config = Config::load().map_err(|e| anyhow::anyhow!("{}", e))?;
    let db = connect_surreal(&config.database).await?;

    let summary = plan_contest_name_backfill(&db, limit)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    println!(
        "Contests scanned: {} | unchanged: {} | to update: {}",
        summary.total, summary.unchanged, summary.to_update
    );

    for plan in &summary.plans {
        println!("{}: {:?} -> {:?}", plan.contest_id, plan.old_name, plan.new_name);
    }

    if !apply {
        if summary.to_update > 0 {
            println!("\nDry run only. Re-run with --apply to write changes.");
        }
        return Ok(());
    }

    if summary.plans.is_empty() {
        println!("Nothing to apply.");
        return Ok(());
    }

    let updated = apply_contest_name_backfill(&db, &summary.plans)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    println!("Updated {} contest name(s).", updated);
    Ok(())
}
