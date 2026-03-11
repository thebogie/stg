# Client-Side Analytics Architecture for stg_rd

## Overview

This document outlines the architecture for implementing client-side analytics in the stg_rd project. The goal is to move analytics processing from the server to the client, providing instant responses and better user experience.

## Architecture Benefits

### 🚀 **Performance Improvements**
- **Instant UI updates** - No loading states for analytics
- **Offline capability** - Analytics work without internet
- **Reduced server load** - Fewer database queries
- **Better mobile experience** - Reduced network calls

### 🎯 **User Experience**
- **Real-time filtering** - Instant date range, game, venue filtering
- **Interactive charts** - Hover effects, drill-downs without server calls
- **Custom queries** - Users can slice data any way they want
- **Responsive interface** - Snappy, desktop-like analytics experience

### 📊 **Scalability**
- **Server offloading** - Database queries distributed across clients
- **Reduced bandwidth** - One-time data transfer vs repeated API calls
- **Better caching** - Browser handles data persistence

## Data Flow

### 1. **Login Flow**
```
User Login → Fetch Core Stats → Load Raw Contest Data → Build Local Cache → Ready for Analytics
```

### 2. **Data Sync Strategy**
```
Initial Load: Full contest data + related entities
Subsequent: Delta updates (new contests only)
Fallback: Server-side analytics if client data stale
```

### 3. **Analytics Processing**
```
User Query → Client Filter → Local Computation → Instant Results
```

## Implementation Phases

### Phase 1: Core Infrastructure ✅
- [x] Client data models (`ClientContest`, `ClientGame`, etc.)
- [x] Storage layer (`ClientStorage` trait, `LocalStorageClient`)
- [x] Analytics manager (`AnalyticsDataManager`)
- [x] DTOs for client sync (`ClientSyncRequest`, `ClientSyncResponse`)

### Phase 2: Backend API Endpoints
- [ ] `/api/client/sync` - Full/delta data sync
- [ ] `/api/client/analytics` - Real-time analytics queries
- [ ] `/api/client/validate` - Data integrity verification

### Phase 3: Frontend Integration
- [ ] Analytics data manager integration
- [ ] Login-time data loading
- [ ] Real-time analytics computation
- [ ] Offline capability

### Phase 4: Advanced Features
- [ ] Data compression
- [ ] Smart delta sync
- [ ] Background sync
- [ ] Data versioning

## Data Models

### Core Client Models

#### `ClientContest`
```rust
pub struct ClientContest {
    pub id: String,
    pub name: String,
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    pub game: ClientGame,
    pub venue: ClientVenue,
    pub participants: Vec<ClientParticipant>,
    pub my_result: ClientResult,
}
```

#### `ClientAnalyticsCache`
```rust
pub struct ClientAnalyticsCache {
    pub last_updated: DateTime<FixedOffset>,
    pub player_id: String,
    pub core_stats: CoreStats,
    pub contests: Vec<ClientContest>,
    pub game_lookup: HashMap<String, ClientGame>,
    pub venue_lookup: HashMap<String, ClientVenue>,
    pub opponent_lookup: HashMap<String, ClientOpponent>,
}
```

### Storage Strategy

#### Memory Cache (Primary)
- **Fastest access** - All analytics in RAM
- **Auto-expiry** - Configurable cache age (default: 24 hours)
- **Smart refresh** - Only reload when needed

#### Persistent Storage (Secondary)
- **IndexedDB** - Primary storage (modern browsers)
- **localStorage** - Fallback (older browsers)
- **Size limits** - Configurable (default: 50MB)
- **Compression** - For large datasets

## API Endpoints

### 1. **Client Data Sync**
```http
POST /api/client/sync
Content-Type: application/json

{
  "player_id": "player/123",
  "last_contest_id": "contest/456",
  "last_sync": "2024-01-15T10:00:00Z",
  "full_sync": false,
  "limit": 100,
  "include_related": true
}
```

**Response:**
```json
{
  "player_id": "player/123",
  "sync_timestamp": "2024-01-15T10:30:00Z",
  "data_version": "1.0.0",
  "contests": [...],
  "games": [...],
  "venues": [...],
  "players": [...],
  "sync_metadata": {...}
}
```

### 2. **Real-time Analytics**
```http
POST /api/client/analytics
Content-Type: application/json

{
  "player_id": "player/123",
  "query": {
    "date_range": {
      "start": "2024-01-01T00:00:00Z",
      "end": "2024-01-31T23:59:59Z"
    },
    "games": ["game/123", "game/456"],
    "min_players": 2,
    "max_players": 6
  },
  "use_cache": true
}
```

## Frontend Integration

### 1. **Analytics Manager Setup**
```rust
use shared::models::client_storage::{AnalyticsDataManager, LocalStorageClient, StorageConfig};

let config = StorageConfig::default();
let storage = Box::new(LocalStorageClient::new(config));
let mut analytics_manager = AnalyticsDataManager::new(storage, config);
```

### 2. **Login-time Data Loading**
```rust
// On successful login
let cache = analytics_manager.initialize_player_analytics(&player_id).await?;

if cache.needs_refresh(24) {
    // Sync fresh data from server
    let sync_response = sync_client_data(&player_id).await?;
    let updated_cache = analytics_manager.sync_analytics_data(
        &player_id,
        sync_response.contests,
        sync_response.games,
        sync_response.venues,
        sync_response.players,
    ).await?;
}
```

### 3. **Real-time Analytics Queries**
```rust
let query = AnalyticsQuery {
    date_range: Some(DateRange {
        start: start_date,
        end: end_date,
    }),
    games: selected_games,
    venues: selected_venues,
    opponents: selected_opponents,
    min_players: min_players,
    max_players: max_players,
};

let analytics = analytics_manager.get_analytics(&player_id, query).await?;

// Use analytics.stats, analytics.game_performance, etc.
```

## Performance Optimizations

### 1. **Data Compression**
- **Threshold**: 1MB+ datasets get compressed
- **Algorithm**: LZ4 or similar fast compression
- **Ratio**: Typically 2-4x compression for contest data

### 2. **Smart Caching**
- **Core stats**: Computed once, cached in memory
- **Lookup tables**: Pre-built for fast access
- **Lazy loading**: Related data loaded on demand

### 3. **Delta Updates**
- **Initial sync**: Full dataset
- **Subsequent syncs**: Only new contests
- **Conflict resolution**: Server timestamp wins

### 4. **Memory Management**
- **Size limits**: Configurable per player
- **LRU eviction**: Remove least used data
- **Garbage collection**: Clean up expired caches

## Storage Limits & Quotas

### Browser Storage Limits
- **localStorage**: 5-10MB per domain
- **IndexedDB**: 50MB+ (varies by browser)
- **Memory**: Limited by device RAM

### Recommended Limits
- **Per player**: 50MB max
- **Contest history**: 1000 contests max
- **Cache age**: 24 hours default
- **Compression**: Enable for 1MB+ datasets

## Error Handling & Fallbacks

### 1. **Storage Failures**
```rust
match analytics_manager.get_analytics(&player_id, query).await {
    Ok(analytics) => Ok(analytics),
    Err(StorageError::NotAvailable(_)) => {
        // Fallback to server-side analytics
        fallback_server_analytics(&player_id, query).await
    }
    Err(e) => Err(e),
}
```

### 2. **Data Corruption**
- **Hash verification** - Client sends data hash to server
- **Auto-recovery** - Corrupted data triggers full re-sync
- **Backup validation** - Server validates client data integrity

### 3. **Network Issues**
- **Offline mode** - Analytics work with cached data
- **Background sync** - Retry failed syncs when online
- **Progressive loading** - Load essential data first

## Testing Strategy

### 1. **Unit Tests**
- **Data models** - Serialization/deserialization
- **Analytics computation** - Correct calculations
- **Storage operations** - CRUD operations

### 2. **Integration Tests**
- **End-to-end sync** - Full data flow
- **Performance tests** - Large dataset handling
- **Error scenarios** - Network failures, corruption

### 3. **Browser Tests**
- **Storage APIs** - IndexedDB, localStorage
- **Memory usage** - Large dataset performance
- **Cross-browser** - Chrome, Firefox, Safari

## Migration Strategy

### 1. **Phase 1: Dual Mode**
- Keep existing server-side analytics
- Add client-side analytics alongside
- Feature flag to switch between modes

### 2. **Phase 2: Client Primary**
- Make client-side analytics primary
- Server-side as fallback only
- Gradual rollout to users

### 3. **Phase 3: Server Optimization**
- Remove server-side analytics endpoints
- Optimize server for data sync only
- Reduce server resource usage

## Monitoring & Metrics

### 1. **Performance Metrics**
- **Cache hit rate** - Percentage of requests served from cache
- **Sync duration** - Time to sync data from server
- **Memory usage** - Client-side memory consumption
- **Storage usage** - Persistent storage consumption

### 2. **User Experience Metrics**
- **Analytics response time** - Time to compute results
- **Offline usage** - Percentage of offline analytics
- **Error rates** - Storage failures, sync errors
- **User satisfaction** - Performance ratings

### 3. **System Health**
- **Data integrity** - Corruption detection rates
- **Sync success rate** - Successful data synchronization
- **Storage quota** - Browser storage usage
- **Cache efficiency** - Memory vs storage hit rates

## Security Considerations

### 1. **Data Privacy**
- **Local storage only** - No analytics data sent to third parties
- **User consent** - Opt-in for client-side analytics
- **Data retention** - Clear data lifecycle policies

### 2. **Data Validation**
- **Server verification** - Validate client data integrity
- **Hash verification** - Cryptographic data verification
- **Input sanitization** - Prevent malicious queries

### 3. **Access Control**
- **Player isolation** - Users can only access their own data
- **Session validation** - Verify user authentication
- **Rate limiting** - Prevent abuse of sync endpoints

## Future Enhancements

### 1. **Advanced Analytics**
- **Machine learning** - Predictive performance analysis
- **Pattern recognition** - Identify gaming trends
- **Recommendations** - Suggest games, strategies

### 2. **Social Features**
- **Friend comparisons** - Compare stats with friends
- **Leaderboards** - Real-time competitive rankings
- **Achievement sharing** - Social achievement system

### 3. **Data Export**
- **CSV/JSON export** - Download analytics data
- **Chart generation** - Export charts as images
- **Report generation** - PDF analytics reports

## Conclusion

Client-side analytics will transform the stg_rd user experience by providing:

1. **Instant analytics** - No more loading states
2. **Interactive exploration** - Real-time filtering and drilling
3. **Offline capability** - Analytics work anywhere
4. **Better performance** - Reduced server load and bandwidth
5. **Scalability** - Handle more users with same server resources

The architecture is designed to be:
- **Progressive** - Implement in phases
- **Robust** - Multiple fallback strategies
- **Efficient** - Smart caching and compression
- **User-friendly** - Seamless experience

This approach positions stg_rd as a modern, responsive gaming analytics platform that provides desktop-quality experience on any device.
