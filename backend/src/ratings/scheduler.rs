use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use log::{error, info, warn};
use shared::Result;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration, Instant};

use super::usecase::RatingsUsecase;

use arangors::client::ClientExt;

/// Background scheduler for monthly Glicko2 ratings recalculation
#[derive(Clone)]
pub struct RatingsScheduler<C: ClientExt + Send + Sync + 'static> {
    usecase: Arc<RatingsUsecase<C>>,
    last_run: Arc<Mutex<Option<DateTime<Utc>>>>,
    is_running: bool,
}

impl<C: ClientExt + Send + Sync + 'static> RatingsScheduler<C> {
    pub fn new(usecase: RatingsUsecase<C>) -> Self {
        Self {
            usecase: Arc::new(usecase),
            last_run: Arc::new(Mutex::new(None)),
            is_running: false,
        }
    }

    /// Start the background scheduler
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            warn!("Ratings scheduler is already running");
            return Ok(());
        }

        self.is_running = true;
        info!("Starting Glicko2 ratings scheduler...");

        // Spawn the background task
        let usecase = self.usecase.clone();
        let last_run = self.last_run.clone();

        tokio::spawn(async move {
            Self::run_scheduler_loop(usecase, last_run).await;
        });

        Ok(())
    }

    /// Stop the background scheduler
    pub fn stop(&mut self) {
        self.is_running = false;
        info!("Stopping Glicko2 ratings scheduler...");
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get the last run time
    pub fn last_run(&self) -> Option<DateTime<Utc>> {
        self.last_run.lock().unwrap().clone()
    }

    /// Main scheduler loop
    async fn run_scheduler_loop(
        usecase: Arc<RatingsUsecase<C>>,
        last_run: Arc<Mutex<Option<DateTime<Utc>>>>,
    ) {
        info!("Glicko2 ratings scheduler loop started");

        loop {
            // Check if it's time to run monthly recalculation
            if Self::should_run_monthly_recalculation(last_run.lock().unwrap().clone()) {
                info!("Starting monthly Glicko2 ratings recalculation...");

                match Self::run_monthly_recalculation(&usecase).await {
                    Ok(()) => {
                        *last_run.lock().unwrap() = Some(Utc::now());
                        info!("Monthly Glicko2 ratings recalculation completed successfully");
                    }
                    Err(e) => {
                        error!("Monthly Glicko2 ratings recalculation failed: {}", e);
                    }
                }
            }

            // Sleep for 1 hour before checking again
            sleep(Duration::from_secs(3600)).await;
        }
    }

    /// Determine if monthly recalculation should run
    fn should_run_monthly_recalculation(last_run: Option<DateTime<Utc>>) -> bool {
        let now = Utc::now();

        // If never run before, check if it's the 1st of the month at 2 AM
        if last_run.is_none() {
            return Self::is_first_of_month_at_2am(now);
        }

        let last = last_run.unwrap();

        // Check if we've moved to a new month since last run
        let last_month = (last.year(), last.month());
        let current_month = (now.year(), now.month());

        if last_month != current_month {
            // Wait until the 1st of the month at 2 AM
            return Self::is_first_of_month_at_2am(now);
        }

        false
    }

    /// Check if current time is 1st of month at 2 AM
    fn is_first_of_month_at_2am(dt: DateTime<Utc>) -> bool {
        dt.day() == 1 && dt.hour() == 2 && dt.minute() < 60
    }

    /// Run the monthly recalculation
    async fn run_monthly_recalculation(usecase: &RatingsUsecase<C>) -> Result<()> {
        let start_time = Instant::now();

        // Determine the period to recalculate (previous month)
        let now = Utc::now();
        let (year, month) = if now.month() == 1 {
            (now.year() - 1, 12)
        } else {
            (now.year(), now.month() - 1)
        };

        let period = format!("{:04}-{:02}", year, month);
        info!("Recalculating ratings for period: {}", period);

        // Run the recalculation
        usecase.recompute_month(Some(period)).await?;

        let duration = start_time.elapsed();
        info!("Monthly recalculation completed in {:?}", duration);

        Ok(())
    }

    /// Manually trigger recalculation for a specific period
    pub async fn trigger_recalculation(&self, period: Option<String>) -> Result<()> {
        info!(
            "Manually triggering ratings recalculation for period: {:?}",
            period
        );

        let start_time = Instant::now();
        self.usecase.recompute_month(period).await?;

        let duration = start_time.elapsed();
        info!("Manual recalculation completed in {:?}", duration);

        Ok(())
    }

    /// Get scheduler status
    pub fn get_status(&self) -> SchedulerStatus {
        SchedulerStatus {
            is_running: self.is_running,
            last_run: self.last_run.lock().unwrap().clone(),
            next_scheduled_run: Self::calculate_next_run_time(),
        }
    }

    /// Calculate when the next scheduled run will occur
    fn calculate_next_run_time() -> DateTime<Utc> {
        let now = Utc::now();

        // If it's already past 2 AM on the 1st, schedule for next month
        if now.day() == 1 && now.hour() >= 2 {
            // Next month, 1st day at 2 AM
            if now.month() == 12 {
                Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 2, 0, 0).unwrap()
            } else {
                Utc.with_ymd_and_hms(now.year(), now.month() + 1, 1, 2, 0, 0)
                    .unwrap()
            }
        } else if now.day() == 1 && now.hour() < 2 {
            // Same day, 2 AM
            Utc.with_ymd_and_hms(now.year(), now.month(), 1, 2, 0, 0)
                .unwrap()
        } else {
            // Next 1st of month at 2 AM
            if now.month() == 12 {
                Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 2, 0, 0).unwrap()
            } else {
                Utc.with_ymd_and_hms(now.year(), now.month() + 1, 1, 2, 0, 0)
                    .unwrap()
            }
        }
    }
}

/// Status information for the ratings scheduler
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchedulerStatus {
    pub is_running: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_scheduled_run: DateTime<Utc>,
}

/// Configuration for the ratings scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Check interval in seconds (default: 1 hour)
    pub check_interval_seconds: u64,
    /// Hour of day to run recalculation (default: 2 AM)
    pub run_hour: u32,
    /// Day of month to run recalculation (default: 1st)
    pub run_day: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 3600, // 1 hour
            run_hour: 2,                  // 2 AM
            run_day: 1,                   // 1st of month
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_is_first_of_month_at_2am() {
        // Test 1st of month at 2 AM
        let dt = Utc.with_ymd_and_hms(2024, 1, 1, 2, 0, 0).unwrap();
        assert!(
            RatingsScheduler::<arangors::client::reqwest::ReqwestClient>::is_first_of_month_at_2am(
                dt
            )
        );

        // Test 1st of month at 1 AM (should be false)
        let dt = Utc.with_ymd_and_hms(2024, 1, 1, 1, 0, 0).unwrap();
        assert!(
            !RatingsScheduler::<arangors::client::reqwest::ReqwestClient>::is_first_of_month_at_2am(
                dt
            )
        );

        // Test 2nd of month at 2 AM (should be false)
        let dt = Utc.with_ymd_and_hms(2024, 1, 2, 2, 0, 0).unwrap();
        assert!(
            !RatingsScheduler::<arangors::client::reqwest::ReqwestClient>::is_first_of_month_at_2am(
                dt
            )
        );
    }

    #[test]
    fn test_calculate_next_run_time() {
        // Test that the function returns a valid DateTime
        let next_run =
            RatingsScheduler::<arangors::client::reqwest::ReqwestClient>::calculate_next_run_time();

        // Should be a valid datetime
        assert!(next_run > Utc::now());

        // Should be on the 1st of a month at 2 AM
        assert_eq!(next_run.day(), 1);
        assert_eq!(next_run.hour(), 2);
        assert_eq!(next_run.minute(), 0);
        assert_eq!(next_run.second(), 0);
    }
}
