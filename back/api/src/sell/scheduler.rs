//! Background cleanup for expired sell-listing photos.

use chrono::Utc;
use log::{error, info};
use shared::Result;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

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
        info!("Starting sell-listing image cleanup scheduler...");

        let repo = self.repo.clone();
        let last_run = self.last_run.clone();
        tokio::spawn(async move {
            loop {
                match repo.purge_expired_listings().await {
                    Ok(n) if n > 0 => info!("Purged {} expired sell listings", n),
                    Ok(_) => {}
                    Err(e) => error!("Sell listing cleanup failed: {}", e),
                }
                *last_run.lock().unwrap() = Some(Utc::now());
                sleep(Duration::from_secs(3600)).await;
            }
        });
        Ok(())
    }
}
