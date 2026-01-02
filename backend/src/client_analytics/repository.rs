use async_trait::async_trait;
use shared::dto::client_sync::*;
use shared::error::SharedError;
use shared::models::{contest::Contest, game::Game, venue::Venue, player::Player};
use arangors::{AqlQuery, Database, client::ClientExt};
use chrono::{DateTime, FixedOffset};
use std::collections::HashMap;
use log;

/// Repository trait for client analytics data access
#[async_trait]
pub trait ClientAnalyticsRepository: Send + Sync {
    /// Gets all contests for a player
    async fn get_all_contests_for_player(&self, player_id: &str) -> Result<Vec<Contest>, SharedError>;
    
    /// Gets contests since a specific timestamp
    async fn get_contests_since(&self, player_id: &str, since: DateTime<FixedOffset>) -> Result<Vec<Contest>, SharedError>;
    
    /// Gets filtered contests based on query parameters
    async fn get_filtered_contests(&self, player_id: &str, query: &ClientAnalyticsQuery) -> Result<Vec<Contest>, SharedError>;
    
    /// Gets the game for a specific contest
    async fn get_game_for_contest(&self, contest_id: &str) -> Result<Game, SharedError>;
    
    /// Gets the venue for a specific contest
    async fn get_venue_for_contest(&self, contest_id: &str) -> Result<Venue, SharedError>;
    
    /// Gets all participants for a contest
    async fn get_contest_participants(&self, contest_id: &str) -> Result<Vec<ContestParticipant>, SharedError>;
    
    /// Gets all games a player has played
    async fn get_games_for_player(&self, player_id: &str) -> Result<Vec<Game>, SharedError>;
    
    /// Gets all venues a player has played at
    async fn get_venues_for_player(&self, player_id: &str) -> Result<Vec<Venue>, SharedError>;
    
    /// Gets all opponents a player has faced
    async fn get_opponents_for_player(&self, player_id: &str) -> Result<Vec<Player>, SharedError>;
    
    /// Gets total contest count for a player
    async fn get_total_contests_for_player(&self, player_id: &str) -> Result<usize, SharedError>;
    
    /// Gets the last contest for a player
    async fn get_last_contest_for_player(&self, player_id: &str) -> Result<Option<Contest>, SharedError>;
    
    /// Gets gaming communities and regular opponents
    async fn get_gaming_communities(&self, player_id: &str, min_contests: i32) -> Result<Vec<serde_json::Value>, SharedError>;
    
    /// Gets player networking insights (who plays with whom)
    async fn get_player_networking(&self, player_id: &str) -> Result<serde_json::Value, SharedError>;
}

/// Contest participant with result data
#[derive(Debug, Clone)]
pub struct ContestParticipant {
    pub player_id: String,
    pub handle: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub place: i32,
    pub result: String,
    pub points: Option<i32>,
}

/// Implementation of client analytics repository
pub struct ClientAnalyticsRepositoryImpl<C: ClientExt> {
    db: Database<C>,
}

impl<C: ClientExt> ClientAnalyticsRepositoryImpl<C> {
    pub fn new(db: Database<C>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<C: ClientExt + Send + Sync> ClientAnalyticsRepository for ClientAnalyticsRepositoryImpl<C> {
    async fn get_all_contests_for_player(&self, player_id: &str) -> Result<Vec<Contest>, SharedError> {
        let query = r#"
            FOR result IN resulted_in
            FILTER result._to == @player_id
            LET contest = DOCUMENT(result._from)
            SORT contest.start DESC
            RETURN contest
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Contest>(aql).await {
            Ok(contests) => {
                log::info!("Retrieved {} contests for player: {}", contests.len(), player_id);
                Ok(contests)
            }
            Err(e) => {
                log::error!("Failed to get contests for player {}: {}", player_id, e);
                Err(SharedError::Database(format!("Failed to query contests: {}", e)))
            }
        }
    }

    async fn get_contests_since(&self, player_id: &str, since: DateTime<FixedOffset>) -> Result<Vec<Contest>, SharedError> {
        let query = r#"
            FOR result IN resulted_in
            FILTER result._to == @player_id
            LET contest = DOCUMENT(result._from)
            FILTER contest.start >= @since
            SORT contest.start DESC
            RETURN contest
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));
        bind_vars.insert("since", serde_json::Value::String(since.to_rfc3339()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Contest>(aql).await {
            Ok(contests) => {
                log::info!("Retrieved {} contests since {} for player: {}", contests.len(), since, player_id);
                Ok(contests)
            }
            Err(e) => {
                log::error!("Failed to get contests since {} for player {}: {}", since, player_id, e);
                Err(SharedError::Database(format!("Failed to query contests since: {}", e)))
            }
        }
    }

    async fn get_filtered_contests(&self, player_id: &str, query: &ClientAnalyticsQuery) -> Result<Vec<Contest>, SharedError> {
        let mut query_parts = Vec::new();
        let mut bind_vars = HashMap::new();
        
        // Base query
        query_parts.push(r#"
            FOR result IN resulted_in
            FILTER result._to == @player_id
            LET contest = DOCUMENT(result._from)
        "#.to_string());
        
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));

        // Date range filter
        if let Some(date_range) = &query.date_range {
            query_parts.push(r#"
                FILTER contest.start >= @start_date AND contest.start <= @end_date
            "#.to_string());
            bind_vars.insert("start_date", serde_json::Value::String(date_range.start.to_rfc3339()));
            bind_vars.insert("end_date", serde_json::Value::String(date_range.end.to_rfc3339()));
        }

        // Game filter
        if let Some(games) = &query.games {
            if !games.is_empty() {
                let game_ids: Vec<serde_json::Value> = games.iter().map(|g| serde_json::Value::String(g.clone())).collect();
                query_parts.push(r#"
                    LET game_edge = FIRST(
                        FOR edge IN played_with
                        FILTER edge._from == contest._id
                        RETURN edge
                    )
                    FILTER game_edge != null
                    LET game = DOCUMENT(game_edge._to)
                    FILTER game._id IN @game_ids
                "#.to_string());
                bind_vars.insert("game_ids", serde_json::Value::Array(game_ids));
            }
        }

        // Venue filter
        if let Some(venues) = &query.venues {
            if !venues.is_empty() {
                let venue_ids: Vec<serde_json::Value> = venues.iter().map(|v| serde_json::Value::String(v.clone())).collect();
                query_parts.push(r#"
                    LET venue_edge = FIRST(
                        FOR edge IN played_at
                        FILTER edge._from == contest._id
                        RETURN edge
                    )
                    FILTER venue_edge != null
                    LET venue = DOCUMENT(venue_edge._to)
                    FILTER venue._id IN @venue_ids
                "#.to_string());
                bind_vars.insert("venue_ids", serde_json::Value::Array(venue_ids));
            }
        }

        // Opponent filter
        if let Some(opponents) = &query.opponents {
            if !opponents.is_empty() {
                let opponent_ids: Vec<serde_json::Value> = opponents.iter().map(|o| serde_json::Value::String(o.clone())).collect();
                query_parts.push(r#"
                    FILTER LENGTH(
                        FOR other_result IN resulted_in
                        FILTER other_result._from == contest._id
                        FILTER other_result._to IN @opponent_ids
                        RETURN other_result
                    ) > 0
                "#.to_string());
                bind_vars.insert("opponent_ids", serde_json::Value::Array(opponent_ids));
            }
        }

        // Player count filter
        if let Some(min_players) = query.min_players {
            query_parts.push(r#"
                FILTER LENGTH(
                    FOR participant IN resulted_in
                    FILTER participant._from == contest._id
                    RETURN participant
                ) >= @min_players
            "#.to_string());
            bind_vars.insert("min_players", serde_json::Value::Number(min_players.into()));
        }

        if let Some(max_players) = query.max_players {
            query_parts.push(r#"
                FILTER LENGTH(
                    FOR participant IN resulted_in
                    FILTER participant._from == contest._id
                    RETURN participant
                ) <= @max_players
            "#.to_string());
            bind_vars.insert("max_players", serde_json::Value::Number(max_players.into()));
        }

        // Result filter
        if let Some(result_filter) = &query.result_filter {
            if !result_filter.is_empty() {
                let results: Vec<serde_json::Value> = result_filter.iter().map(|r| serde_json::Value::String(r.clone())).collect();
                query_parts.push(r#"
                    FILTER result.result IN @results
                "#.to_string());
                bind_vars.insert("results", serde_json::Value::Array(results));
            }
        }

        // Placement range filter
        if let Some(placement_range) = &query.placement_range {
            query_parts.push(r#"
                FILTER result.place >= @min_placement AND result.place <= @max_placement
            "#.to_string());
            bind_vars.insert("min_placement", serde_json::Value::Number(placement_range.min.into()));
            bind_vars.insert("max_placement", serde_json::Value::Number(placement_range.max.into()));
        }

        // Final parts
        query_parts.push(r#"
            SORT contest.start DESC
            RETURN contest
        "#.to_string());

        let full_query = query_parts.join("\n");

        let aql = AqlQuery::builder()
            .query(&full_query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Contest>(aql).await {
            Ok(contests) => {
                log::info!("Retrieved {} filtered contests for player: {}", contests.len(), player_id);
                Ok(contests)
            }
            Err(e) => {
                log::error!("Failed to get filtered contests for player {}: {}", player_id, e);
                Err(SharedError::Database(format!("Failed to query filtered contests: {}", e)))
            }
        }
    }

    async fn get_game_for_contest(&self, contest_id: &str) -> Result<Game, SharedError> {
        let query = r#"
            FOR edge IN played_with
            FILTER edge._from == @contest_id
            LET game = DOCUMENT(edge._to)
            RETURN game
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("contest_id", serde_json::Value::String(contest_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Game>(aql).await {
            Ok(games) => {
                if let Some(game) = games.into_iter().next() {
                    Ok(game)
                } else {
                    Err(SharedError::NotFound(format!("No game found for contest: {}", contest_id)))
                }
            }
            Err(e) => {
                log::error!("Failed to get game for contest {}: {}", contest_id, e);
                Err(SharedError::Database(format!("Failed to query game: {}", e)))
            }
        }
    }

    async fn get_venue_for_contest(&self, contest_id: &str) -> Result<Venue, SharedError> {
        let query = r#"
            FOR edge IN played_at
            FILTER edge._from == @contest_id
            LET venue = DOCUMENT(edge._to)
            RETURN venue
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("contest_id", serde_json::Value::String(contest_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Venue>(aql).await {
            Ok(venues) => {
                if let Some(venue) = venues.into_iter().next() {
                    Ok(venue)
                } else {
                    Err(SharedError::NotFound(format!("No venue found for contest: {}", contest_id)))
                }
            }
            Err(e) => {
                log::error!("Failed to get venue for contest {}: {}", contest_id, e);
                Err(SharedError::Database(format!("Failed to query venue: {}", e)))
            }
        }
    }

    async fn get_contest_participants(&self, contest_id: &str) -> Result<Vec<ContestParticipant>, SharedError> {
        let query = r#"
            FOR result IN resulted_in
            FILTER result._from == @contest_id
            LET player = DOCUMENT(result._to)
            RETURN {
                player_id: player._id,
                handle: player.handle,
                firstname: player.firstname,
                lastname: player.lastname,
                place: result.place,
                result: result.result,
                points: result.points
            }
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("contest_id", serde_json::Value::String(contest_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<serde_json::Value>(aql).await {
            Ok(participants) => {
                let mut result = Vec::new();
                for participant in participants {
                    let participant = ContestParticipant {
                        player_id: participant["player_id"].as_str().unwrap_or("").to_string(),
                        handle: participant["handle"].as_str().unwrap_or("").to_string(),
                        firstname: participant["firstname"].as_str().map(|s| s.to_string()),
                        lastname: participant["lastname"].as_str().map(|s| s.to_string()),
                        place: participant["place"].as_i64().unwrap_or(0) as i32,
                        result: participant["result"].as_str().unwrap_or("").to_string(),
                        points: participant["points"].as_i64().map(|p| p as i32),
                    };
                    result.push(participant);
                }
                Ok(result)
            }
            Err(e) => {
                log::error!("Failed to get participants for contest {}: {}", contest_id, e);
                Err(SharedError::Database(format!("Failed to query participants: {}", e)))
            }
        }
    }

    async fn get_games_for_player(&self, player_id: &str) -> Result<Vec<Game>, SharedError> {
        let query = r#"
            FOR result IN resulted_in
            FILTER result._to == @player_id
            LET contest = DOCUMENT(result._from)
            FOR game_edge IN played_with
            FILTER game_edge._from == contest._id
            LET game = DOCUMENT(game_edge._to)
            COLLECT game_id = game._id INTO unique_games
            RETURN DOCUMENT(game_id)
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Game>(aql).await {
            Ok(games) => {
                log::info!("Retrieved {} unique games for player: {}", games.len(), player_id);
                Ok(games)
            }
            Err(e) => {
                log::error!("Failed to get games for player {}: {}", player_id, e);
                Err(SharedError::Database(format!("Failed to query games: {}", e)))
            }
        }
    }

    async fn get_venues_for_player(&self, player_id: &str) -> Result<Vec<Venue>, SharedError> {
        let query = r#"
            FOR result IN resulted_in
            FILTER result._to == @player_id
            LET contest = DOCUMENT(result._from)
            FOR venue_edge IN played_at
            FILTER venue_edge._from == contest._id
            LET venue = DOCUMENT(venue_edge._to)
            COLLECT venue_id = venue._id INTO unique_venues
            RETURN DOCUMENT(venue_id)
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Venue>(aql).await {
            Ok(venues) => {
                log::info!("Retrieved {} unique venues for player: {}", venues.len(), player_id);
                Ok(venues)
            }
            Err(e) => {
                log::error!("Failed to get venues for player {}: {}", player_id, e);
                Err(SharedError::Database(format!("Failed to query venues: {}", e)))
            }
        }
    }

    async fn get_opponents_for_player(&self, player_id: &str) -> Result<Vec<Player>, SharedError> {
        let query = r#"
            FOR result IN resulted_in
            FILTER result._to == @player_id
            LET contest = DOCUMENT(result._from)
            FOR other_result IN resulted_in
            FILTER other_result._from == contest._id
            FILTER other_result._to != @player_id
            LET opponent = DOCUMENT(other_result._to)
            COLLECT opponent_id = opponent._id INTO unique_opponents
            RETURN DOCUMENT(opponent_id)
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Player>(aql).await {
            Ok(opponents) => {
                log::info!("Retrieved {} unique opponents for player: {}", opponents.len(), player_id);
                Ok(opponents)
            }
            Err(e) => {
                log::error!("Failed to get opponents for player {}: {}", player_id, e);
                Err(SharedError::Database(format!("Failed to query opponents: {}", e)))
            }
        }
    }

    async fn get_total_contests_for_player(&self, player_id: &str) -> Result<usize, SharedError> {
        let query = r#"
            LENGTH(
                FOR result IN resulted_in
                FILTER result._to == @player_id
                RETURN result
            )
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<usize>(aql).await {
            Ok(counts) => {
                if let Some(count) = counts.into_iter().next() {
                    Ok(count)
                } else {
                    Ok(0)
                }
            }
            Err(e) => {
                log::error!("Failed to get contest count for player {}: {}", player_id, e);
                Err(SharedError::Database(format!("Failed to query contest count: {}", e)))
            }
        }
    }

    async fn get_last_contest_for_player(&self, player_id: &str) -> Result<Option<Contest>, SharedError> {
        let query = r#"
            FOR result IN resulted_in
            FILTER result._to == @player_id
            LET contest = DOCUMENT(result._from)
            SORT contest.start DESC
            LIMIT 1
            RETURN contest
        "#;

        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));

        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();

        match self.db.aql_query::<Contest>(aql).await {
            Ok(contests) => {
                Ok(contests.into_iter().next())
            }
            Err(e) => {
                log::error!("Failed to get last contest for player {}: {}", player_id, e);
                Err(SharedError::Database(format!("Failed to query last contest: {}", e)))
            }
        }
    }

    async fn get_gaming_communities(&self, player_id: &str, min_contests: i32) -> Result<Vec<serde_json::Value>, SharedError> {
        log::info!("🔍 Getting gaming communities for player: {}", player_id);
        
        let query = r#"
            // Graph traversal: player -> contests -> other players -> community analysis
            FOR player IN player
              FILTER player._id == @player_id
              
              // Get all contests this player participated in
              LET player_contests = (
                FOR result IN resulted_in
                  FILTER result._to == player._id
                  LET contest = DOCUMENT(result._from)
                  RETURN contest
              )
              
              // Find other players who regularly play in the same contests
              LET regular_opponents = (
                FOR contest IN player_contests
                  FOR result IN resulted_in
                    FILTER result._from == contest._id
                    LET other_player = DOCUMENT(result._to)
                    FILTER other_player._id != @player_id
                    
                    // Count how many times they've played together
                    COLLECT opponent_id = other_player._id, opponent_data = other_player INTO shared_contests
                    LET contest_count = LENGTH(shared_contests)
                    
                    // Only consider players with minimum contest overlap
                    FILTER contest_count >= @min_contests
                    
                    // Calculate opponent strength and frequency
                    LET opponent_rating = FIRST(
                      FOR r IN rating_latest
                      FILTER r.player_id == opponent_id
                      RETURN r.rating
                    )
                    
                    RETURN {
                      opponent_id: opponent_id,
                      opponent_handle: opponent_data.handle,
                      opponent_firstname: opponent_data.firstname,
                      shared_contests: contest_count,
                      last_played: MAX(
                        FOR sc IN shared_contests
                        RETURN sc.contest.start
                      ),
                      opponent_rating: opponent_rating,
                      relationship_strength: contest_count * 10  // Simple scoring
                    }
              )
              
              // Simple community representation: one community per regular opponent
              LET communities = (
                FOR ro IN regular_opponents
                  RETURN {
                    community_leader: ro,
                    members: [],
                    total_members: 1,
                    community_strength: ro.relationship_strength
                  }
              )
              
              RETURN {
                player_id: player._id,
                player_handle: player.handle,
                regular_opponents: regular_opponents,
                gaming_communities: communities
              }
        "#;
        
        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));
        bind_vars.insert("min_contests", serde_json::Value::Number(min_contests.into()));
        
        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();
        
        match self.db.aql_query::<serde_json::Value>(aql).await {
            Ok(cursor) => {
                let results: Vec<serde_json::Value> = cursor.into_iter().collect();
                log::info!("✅ Gaming communities retrieved for player: {} ({} communities)", player_id, results.len());
                Ok(results)
            }
            Err(e) => {
                log::error!("❌ Failed to get gaming communities: {}", e);
                Err(SharedError::Database(format!("Failed to query gaming communities: {}", e)))
            }
        }
    }

    async fn get_player_networking(&self, player_id: &str) -> Result<serde_json::Value, SharedError> {
        log::info!("🔍 Getting player networking insights for player: {}", player_id);
        
        let query = r#"
            // Graph traversal: player -> contests -> opponents -> network analysis
            FOR player IN player
              FILTER player._id == @player_id
              
              // Get all contests and opponents
              LET contest_opponents = (
                FOR result IN resulted_in
                  FILTER result._to == player._id
                  LET contest = DOCUMENT(result._from)
                  FOR other_result IN resulted_in
                    FILTER other_result._from == contest._id
                    LET opponent = DOCUMENT(other_result._to)
                    FILTER opponent._id != @player_id
                    RETURN {
                      contest: contest,
                      opponent: opponent,
                      my_result: result,
                      opponent_result: other_result
                    }
              )
              
              // Analyze opponent relationships
              LET opponent_analysis = (
                FOR co IN contest_opponents
                  COLLECT opponent_id = co.opponent._id, opponent_data = co.opponent INTO opponent_contests
                  
                  LET total_contests = LENGTH(opponent_contests)
                  LET my_wins = LENGTH(
                    FOR oc IN opponent_contests
                    FILTER oc.my_result.place < oc.opponent_result.place
                    RETURN oc
                  )
                  LET win_rate = total_contests > 0 ? (my_wins * 100.0) / total_contests : 0.0
                  
                  // Find common opponents
                  LET common_opponents = (
                    FOR oc IN opponent_contests
                      FOR other_contest IN resulted_in
                        FILTER other_contest._to == opponent_id
                        FOR other_result IN resulted_in
                          FILTER other_result._from == other_contest._from
                            AND other_result._to != @player_id
                            AND other_result._to != opponent_id
                          LET common_opponent = DOCUMENT(other_result._to)
                          COLLECT common_id = common_opponent._id INTO common_list
                          RETURN {
                            opponent_id: common_id,
                            contests: LENGTH(common_list)
                          }
                  )
                  
                  RETURN {
                    opponent_id: opponent_id,
                    opponent_handle: opponent_data.handle,
                    total_contests: total_contests,
                    my_wins: my_wins,
                    win_rate: win_rate,
                    common_opponents: common_opponents,
                    last_played: MAX(
                      FOR oc IN opponent_contests
                      RETURN oc.contest.start
                    )
                  }
              )
              
              // Calculate network metrics
              LET network_metrics = {
                total_opponents: LENGTH(opponent_analysis),
                average_contest_frequency: AVG(
                  FOR oa IN opponent_analysis
                  RETURN oa.total_contests
                ),
                strongest_opponents: (
                  FOR oa IN opponent_analysis
                  SORT oa.total_contests DESC
                  LIMIT 5
                  RETURN oa
                ),
                most_competitive_rivalries: (
                  FOR oa IN opponent_analysis
                  FILTER oa.total_contests >= 3
                  SORT ABS(oa.win_rate - 50.0) ASC
                  LIMIT 5
                  RETURN oa
                )
              }
              
              RETURN {
                player_id: player._id,
                player_handle: player.handle,
                opponent_analysis: opponent_analysis,
                network_metrics: network_metrics
              }
        "#;
        
        let mut bind_vars = HashMap::new();
        bind_vars.insert("player_id", serde_json::Value::String(player_id.to_string()));
        
        let aql = AqlQuery::builder()
            .query(query)
            .bind_vars(bind_vars)
            .build();
        
        match self.db.aql_query::<serde_json::Value>(aql).await {
            Ok(mut cursor) => {
                if let Some(result) = cursor.pop() {
                    log::info!("✅ Player networking insights retrieved for player: {}", player_id);
                    Ok(result)
                } else {
                    Err(SharedError::NotFound("No networking data found".to_string()))
                }
            }
            Err(e) => {
                log::error!("❌ Failed to get player networking: {}", e);
                Err(SharedError::Database(format!("Failed to query player networking: {}", e)))
            }
        }
    }
}
