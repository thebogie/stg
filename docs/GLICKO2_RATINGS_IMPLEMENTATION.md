# Glicko2 Ratings Implementation

## Overview

The stg_rd project now includes a complete Glicko2 rating system that automatically calculates and updates player skill ratings based on contest performance. This system provides fair, mathematically sound skill assessment that improves with more games played.

## What is Glicko2?

Glicko2 is an improvement over the original Glicko rating system, designed by Mark Glickman. It's more sophisticated than ELO and provides:

- **Rating**: Your skill level (higher = better, starting at 1500)
- **Rating Deviation (RD)**: Uncertainty in your rating (lower = more certain)
- **Volatility**: How much your rating can change (higher = more volatile)

## System Architecture

### Backend Components

#### 1. Glicko2 Algorithm (`backend/src/ratings/glicko.rs`)
- Complete mathematical implementation of Glicko2 formulas
- Handles rating updates, RD inflation, and volatility calculations
- Default parameters: μ=1500, φ=350, τ=0.06

#### 2. Ratings Repository (`backend/src/ratings/repository.rs`)
- Database operations for ratings storage and retrieval
- Collections: `rating_latest` and `rating_history`
- Efficient queries for leaderboards and player ratings

#### 3. Ratings Use Case (`backend/src/ratings/usecase.rs`)
- Business logic for monthly recalculation
- Processes contest results and updates player ratings
- Handles both global and game-specific ratings

#### 4. Ratings Controller (`backend/src/ratings/controller.rs`)
- REST API endpoints for ratings management
- `/api/ratings/recompute` - Monthly recalculation
- `/api/ratings/leaderboard` - Global leaderboard
- `/api/ratings/player/{player_id}` - Individual player ratings

### Database Schema

#### `rating_latest` Collection
```json
{
  "player_id": "player_123",
  "scope_type": "global",
  "scope_id": null,
  "rating": 1650.0,
  "rd": 120.0,
  "volatility": 0.06,
  "games_played": 25,
  "last_period_end": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T02:00:00Z"
}
```

#### `rating_history` Collection
```json
{
  "player_id": "player_123",
  "scope_type": "global",
  "scope_id": null,
  "period_end": "2024-01-01T00:00:00Z",
  "rating": 1650.0,
  "rd": 120.0,
  "volatility": 0.06,
  "period_games": 5,
  "wins": 3,
  "losses": 2,
  "draws": 0,
  "created_at": "2024-01-01T02:00:00Z"
}
```

### Frontend Components

#### 1. Profile Page (`frontend/src/pages/profile.rs`)
- **Overview Tab**: Displays current Glicko2 ratings
- **Ratings Tab**: Detailed rating information and explanations
- Real-time rating updates and confidence indicators

#### 2. Analytics Dashboard (`frontend/src/pages/analytics_dashboard.rs`)
- **Global Leaderboard**: Shows top players by Glicko2 rating
- **Rating Statistics**: Visual representation of rating distribution
- **Player Comparisons**: Head-to-head rating analysis

## Monthly Recalculation Process

### Automatic Schedule
- **Frequency**: Monthly on the 1st at 2:00 AM UTC
- **Trigger**: **Backend scheduler** automatically runs monthly recalculation
- **Scope**: Processes all contests from the previous month

### Manual Trigger
```bash
# Trigger recalculation for a specific period
curl -X POST "http://localhost:8080/api/ratings/scheduler/trigger?period=2024-01"

# Trigger recalculation for the previous month (default)
curl -X POST "http://localhost:8080/api/ratings/scheduler/trigger"

# Check scheduler status
curl "http://localhost:8080/api/ratings/scheduler/status"
```

### Backend Scheduler
The monthly recalculation now runs **automatically within the backend** as a background task:

- **No External Dependencies**: Runs directly in the Rust backend
- **Automatic Startup**: Starts when the backend application starts
- **Intelligent Timing**: Checks every hour if monthly recalculation is due
- **Comprehensive Monitoring**: Real-time status and health checks
- **Manual Control**: Can be triggered manually for any period

**Note**: The old cron job approach has been replaced with this more reliable backend scheduler. See `docs/BACKEND_SCHEDULER_IMPLEMENTATION.md` for complete details.

### Admin Authorization
The scheduler monitor and manual controls are protected by admin authorization. See `docs/ADMIN_AUTHORIZATION_SYSTEM.md` for setup and usage details.

## API Endpoints

### 1. Monthly Recalculation
```http
POST /api/ratings/recompute?period=YYYY-MM
```
**Response**: `{"status": "started"}`

### 2. Global Leaderboard
```http
GET /api/ratings/leaderboard?scope=global&min_games=10&limit=50
```
**Response**: Array of player ratings sorted by rating

### 3. Player Ratings
```http
GET /api/ratings/player/{player_id}
```
**Response**: Array of ratings for different scopes

### 4. Game-Specific Ratings
```http
GET /api/ratings/leaderboard?scope=game/{game_id}&min_games=5&limit=25
```

## Rating Calculation Process

### 1. Contest Processing
- Fetch all contests in the specified month
- Extract player results and placements
- Calculate opponent samples for each player

### 2. Rating Updates
- Apply Glicko2 formulas to update ratings
- Consider opponent strength and contest results
- Update RD and volatility based on performance

### 3. Data Persistence
- Store updated ratings in `rating_latest`
- Record rating history in `rating_history`
- Maintain audit trail of all changes

## Configuration

### Default Parameters
```rust
Glicko2Params {
    default_rating: 1500.0,    // Starting rating
    default_rd: 350.0,         // Starting RD
    default_vol: 0.06,         // Starting volatility
    tau: 0.5,                  // Volatility constraint
}
```

### Environment Variables
```bash
# Backend configuration
RUST_LOG=info
BACKEND_URL=http://localhost:8080

# Database configuration
ARANGO_URL=http://localhost:8529
ARANGO_DB=stg_rd
ARANGO_ROOT_PASSWORD=your_password
```

## Testing

### Manual Testing
```bash
# Test ratings recalculation
curl -X POST "http://localhost:8080/api/ratings/recompute"

# Check leaderboard
curl "http://localhost:8080/api/ratings/leaderboard"

# View player ratings
curl "http://localhost:8080/api/ratings/player/player_123"
```

### Integration Tests
```bash
# Run backend tests
cd backend && cargo test

# Run frontend tests
cd frontend && cargo test

# Run full test suite
cargo test --workspace
```

## Monitoring and Maintenance

### Log Files
- **Cron Logs**: `/var/log/monthly_ratings_recompute.log`
- **Application Logs**: Backend console output
- **Database Logs**: ArangoDB logs

### Health Checks
```bash
# Check backend health
curl "http://localhost:8080/health"

# Check ratings endpoint
curl "http://localhost:8080/api/ratings/leaderboard?limit=1"
```

### Performance Considerations
- **Monthly Recalculation**: Runs during low-traffic hours (2:00 AM)
- **Database Indexes**: Optimized for leaderboard queries
- **Caching**: Frontend caches rating data for better UX

## Troubleshooting

### Common Issues

#### 1. Ratings Not Updating
- Check cron job status: `crontab -l`
- Verify backend accessibility
- Check application logs for errors

#### 2. Empty Leaderboard
- Ensure players have participated in contests
- Check minimum games threshold (default: 5)
- Verify contest data exists in database

#### 3. API Errors
- Check database connectivity
- Verify ArangoDB collections exist
- Check authentication middleware

### Debug Commands
```bash
# Check cron job status
sudo crontab -l

# View recent cron logs
tail -f /var/log/cron

# Check ratings log
tail -f /var/log/monthly_ratings_recompute.log

# Test database connection
arangosh --server.database stg_rd
```

## Future Enhancements

### Planned Features
1. **Real-time Rating Updates**: Live rating changes during contests
2. **Game-Specific Ratings**: Separate ratings for different game types
3. **Rating Confidence Intervals**: Statistical confidence in ratings
4. **Rating History Charts**: Visual rating progression over time
5. **Seasonal Rating Resets**: Optional rating resets for new seasons

### Performance Optimizations
1. **Batch Processing**: Process multiple months simultaneously
2. **Parallel Computation**: Multi-threaded rating calculations
3. **Incremental Updates**: Only recalculate changed ratings
4. **Redis Caching**: Cache frequently accessed rating data

## Contributing

### Development Setup
1. Clone the repository
2. Install dependencies: `cargo build`
3. Set up environment variables
4. Run migrations: `./migrations/run-migrations.sh`
5. Start services: `docker-compose up`

### Code Standards
- Follow Rust coding conventions
- Add tests for new rating logic
- Update documentation for API changes
- Use meaningful commit messages

## Support

For questions or issues related to the Glicko2 ratings system:

1. Check this documentation first
2. Review application logs
3. Check GitHub issues
4. Contact the development team

---

**Last Updated**: January 2025
**Version**: 1.0.0
**Status**: Production Ready ✅
