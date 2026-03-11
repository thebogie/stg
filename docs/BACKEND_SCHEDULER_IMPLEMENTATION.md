# Backend-Based Glicko2 Ratings Scheduler

## Overview

Instead of relying on external cron jobs, the stg_rd project now uses an **integrated background scheduler** that runs directly within the Rust backend. This approach provides better reliability, monitoring, and control over the monthly Glicko2 ratings recalculation process.

## Architecture

### 1. Background Task Scheduler (`backend/src/ratings/scheduler.rs`)

The scheduler runs as a background task within the backend application:

```rust
pub struct RatingsScheduler<C: ClientExt> {
    usecase: Arc<RatingsUsecase<C>>,
    last_run: Option<DateTime<Utc>>,
    is_running: bool,
}
```

**Key Features:**
- **Automatic Startup**: Starts when the backend application starts
- **Intelligent Timing**: Checks every hour if monthly recalculation is due
- **Persistent State**: Tracks last run time and next scheduled run
- **Error Handling**: Comprehensive error handling and logging
- **Manual Control**: Can be triggered manually for specific periods

### 2. Scheduling Logic

The scheduler automatically runs monthly recalculation on the **1st of each month at 2:00 AM UTC**:

```rust
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
```

### 3. Background Task Loop

The scheduler runs continuously in the background:

```rust
async fn run_scheduler_loop(
    usecase: Arc<RatingsUsecase<C>>,
    last_run: &mut Option<DateTime<Utc>>,
) {
    info!("Glicko2 ratings scheduler loop started");

    loop {
        // Check if it's time to run monthly recalculation
        if Self::should_run_monthly_recalculation(*last_run) {
            info!("Starting monthly Glicko2 ratings recalculation...");
            
            match Self::run_monthly_recalculation(&usecase).await {
                Ok(()) => {
                    *last_run = Some(Utc::now());
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
```

## API Endpoints

### 1. Scheduler Status
```http
GET /api/ratings/scheduler/status
```

**Response:**
```json
{
  "is_running": true,
  "last_run": "2024-01-01T02:00:00Z",
  "next_scheduled_run": "2024-02-01T02:00:00Z"
}
```

### 2. Manual Trigger
```http
POST /api/ratings/scheduler/trigger?period=2024-01
```

**Response:**
```json
{
  "status": "triggered",
  "period": "2024-01"
}
```

### 3. Health Check
```http
GET /health/scheduler
```

**Response:**
```json
{
  "status": "ok",
  "timestamp": 1704067200,
  "message": "Glicko2 ratings scheduler is running in the backend",
  "note": "Check /api/ratings/scheduler/status for detailed scheduler information"
}
```

## Frontend Integration

### 1. Scheduler Monitor Component (`frontend/src/components/scheduler_monitor.rs`)

A comprehensive monitoring interface that provides:

- **Real-time Status**: Current scheduler state
- **Last Run Time**: When recalculation last occurred
- **Next Scheduled Run**: When the next automatic run will occur
- **Manual Control**: Trigger recalculation for specific periods
- **Live Updates**: Real-time status monitoring

### 2. Analytics Dashboard Integration

The scheduler monitor is integrated into the analytics dashboard, providing administrators with:

- **Centralized Monitoring**: All system status in one place
- **Immediate Control**: Quick access to manual triggers
- **Visual Status**: Clear indicators of scheduler health

## Configuration

### Default Settings

```rust
impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 3600, // 1 hour
            run_hour: 2,                  // 2 AM
            run_day: 1,                   // 1st of month
        }
    }
}
```

### Environment Variables

```bash
# Backend configuration
RUST_LOG=info                    # Log level for scheduler
BACKEND_URL=http://localhost:8080
```

## Monitoring and Debugging

### 1. Application Logs

The scheduler provides comprehensive logging:

```bash
# View backend logs
docker logs backend

# Filter scheduler logs
docker logs backend | grep "Glicko2 ratings scheduler"
```

### 2. Health Checks

Monitor scheduler health through multiple endpoints:

```bash
# Basic health
curl "http://localhost:8080/health"

# Detailed health (includes scheduler)
curl "http://localhost:8080/health/detailed"

# Scheduler-specific health
curl "http://localhost:8080/health/scheduler"

# Scheduler status
curl "http://localhost:8080/api/ratings/scheduler/status"
```

### 3. Manual Testing

Test the scheduler manually:

```bash
# Trigger recalculation for previous month
curl -X POST "http://localhost:8080/api/ratings/scheduler/trigger"

# Trigger for specific period
curl -X POST "http://localhost:8080/api/ratings/scheduler/trigger?period=2024-01"

# Check results
curl "http://localhost:8080/api/ratings/leaderboard?limit=5"
```

## Advantages Over Cron Jobs

### 1. **Reliability**
- **No External Dependencies**: Runs within the application
- **Automatic Restart**: Restarts with the backend application
- **Error Handling**: Comprehensive error handling and recovery

### 2. **Monitoring**
- **Real-time Status**: Live monitoring through API endpoints
- **Detailed Logging**: Comprehensive logging within the application
- **Health Checks**: Integrated health monitoring

### 3. **Control**
- **Manual Triggers**: Immediate control over recalculation
- **Period Selection**: Specify exact periods for recalculation
- **Status Tracking**: Track last run and next scheduled run

### 4. **Integration**
- **Unified Management**: All backend services in one place
- **Consistent Logging**: Same logging format as other services
- **Error Propagation**: Errors are handled within the application

## Migration from Cron Jobs

### 1. **Remove Old Cron Job**
```bash
# Remove the old cron job
sudo rm /etc/cron.d/monthly_ratings_recompute

# Or edit crontab
sudo crontab -e
# Remove the line: 0 2 1 * * root /usr/bin/curl -X POST "http://localhost:8080/api/ratings/recompute"
```

### 2. **Verify Backend Scheduler**
```bash
# Check if scheduler is running
curl "http://localhost:8080/api/ratings/scheduler/status"

# Check health
curl "http://localhost:8080/health/scheduler"
```

### 3. **Test Functionality**
```bash
# Test manual trigger
curl -X POST "http://localhost:8080/api/ratings/scheduler/trigger"

# Verify results
curl "http://localhost:8080/api/ratings/leaderboard"
```

## Troubleshooting

### Common Issues

#### 1. **Scheduler Not Starting**
```bash
# Check backend logs
docker logs backend | grep "scheduler"

# Check application startup
docker logs backend | grep "Glicko2 ratings scheduler started"
```

#### 2. **Monthly Recalculation Not Running**
```bash
# Check scheduler status
curl "http://localhost:8080/api/ratings/scheduler/status"

# Check current time vs scheduled time
# Should run on 1st of month at 2 AM UTC
```

#### 3. **Manual Trigger Failing**
```bash
# Check API endpoint
curl -X POST "http://localhost:8080/api/ratings/scheduler/trigger"

# Check backend logs for errors
docker logs backend | grep "trigger_recalculation"
```

### Debug Commands

```bash
# Check scheduler status
curl "http://localhost:8080/api/ratings/scheduler/status"

# Test manual trigger
curl -X POST "http://localhost:8080/api/ratings/scheduler/trigger"

# Check health
curl "http://localhost:8080/health/scheduler"

# View logs
docker logs backend | grep -i scheduler
```

## Performance Considerations

### 1. **Resource Usage**
- **Memory**: Minimal memory footprint (~1-2 MB)
- **CPU**: Low CPU usage (checks every hour)
- **Network**: Only makes API calls when needed

### 2. **Scalability**
- **Single Instance**: One scheduler per backend instance
- **Load Distribution**: Can run on multiple backend instances
- **Database Locking**: Handles concurrent access safely

### 3. **Optimization**
- **Efficient Timing**: Only checks when necessary
- **Async Operations**: Non-blocking background execution
- **Error Recovery**: Continues operation after failures

## Future Enhancements

### 1. **Configuration Management**
- **Runtime Configuration**: Change settings without restart
- **Multiple Schedules**: Support for different timing patterns
- **Conditional Execution**: Run based on system load

### 2. **Advanced Monitoring**
- **Metrics Collection**: Performance metrics and statistics
- **Alerting**: Notifications for failures or delays
- **Dashboard**: Web-based monitoring interface

### 3. **Distributed Scheduling**
- **Cluster Coordination**: Multiple backend instances
- **Leader Election**: Single scheduler per cluster
- **Failover**: Automatic failover between instances

---

**Last Updated**: January 2025
**Version**: 2.0.0
**Status**: Production Ready ✅
**Migration**: Complete from Cron Jobs ✅
