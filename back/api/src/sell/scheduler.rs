//! Background cleanup for expired sell-listing photos.

use chrono::Utc;
use shared::Result;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::observability::events::log_scheduler_event;

use super::repository::SellListingRepositoryImpl;

#[derive(Clone)]
pub struct SellImageCleanupScheduler {
    repo: Arc<SellListingRepositoryImpl>,
    is_running: bool,
    last_run: Arc<Mutex<Option<chrono::DateTime<Utc>>>>,
}

impl SellImageCleanupScheduler {
    pub fn new(repo: SellListingRepositoryImpl) -> Self {
        Self {
            repo: Arc::new(repo),
            is_running: false,
            last_run: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Ok(());
        }
        self.is_running = true;
        log_scheduler_event("sell_cleanup", "scheduler.start", None);

        let repo = self.repo.clone();
        let last_run = self.last_run.clone();
        tokio::spawn(async move {
            log_scheduler_event("sell_cleanup", "scheduler.loop.start", None);
            loop {
                match repo.purge_expired_listings().await {
                    Ok(n) if n > 0 => {
                        log_scheduler_event(
                            "sell_cleanup",
                            "scheduler.tick.success",
                            Some(&format!("purged {} listings", n)),
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        log_scheduler_event(
                            "sell_cleanup",
                            "scheduler.tick.error",
                            Some(&e.to_string()),
                        );
                    }
                }
                *last_run.lock().unwrap() = Some(Utc::now());
                sleep(Duration::from_secs(3600)).await;
            }
        });
        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running = false;
        log_scheduler_event("sell_cleanup", "scheduler.stop", None);
    }
}
