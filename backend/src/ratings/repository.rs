use arangors::client::ClientExt;
use arangors::{AqlQuery, Database};
use serde_json::Value;
use shared::{Result, SharedError};

#[derive(Clone)]
pub struct RatingsRepository<C: ClientExt> {
    pub db: Database<C>,
}

impl<C: ClientExt> RatingsRepository<C> {
    pub fn new(db: Database<C>) -> Self {
        Self { db }
    }

    pub async fn get_contests_in_period(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR c IN contest
                  FILTER c.start >= @start AND c.start < @end
                  RETURN c
            "#,
            )
            .bind_var("start", start)
            .bind_var("end", end)
            .build();
        let res = self
            .db
            .aql_query::<serde_json::Value>(query)
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contests: {}", e)))?;
        Ok(res)
    }

    pub async fn get_contest_results(
        &self,
        contest_id: &str,
    ) -> Result<Vec<(String, Option<i32>)>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN resulted_in
                  FILTER r._from == @contest_id
                  RETURN { player_id: r._to, place: r.place }
            "#,
            )
            .bind_var("contest_id", contest_id)
            .build();
        let res = self
            .db
            .aql_query::<serde_json::Value>(query)
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch contest results: {}", e))
            })?;
        let mut out = Vec::new();
        for v in res {
            let pid = v
                .get("player_id")
                .and_then(|x| x.as_str())
                .ok_or_else(|| SharedError::Database("missing player_id".into()))?
                .to_string();
            let place = v.get("place").and_then(|x| x.as_i64()).map(|n| n as i32);
            out.push((pid, place));
        }
        Ok(out)
    }

    pub async fn get_contest_players(&self, contest_id: &str) -> Result<Vec<String>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN resulted_in
                  FILTER r._from == @contest_id
                  RETURN r._to
            "#,
            )
            .bind_var("contest_id", contest_id)
            .build();
        let res = self.db.aql_query::<String>(query).await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch contest players: {}", e))
        })?;
        Ok(res)
    }

    pub async fn get_contest_game(&self, contest_id: &str) -> Result<Option<String>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                LET pw = FIRST(FOR pw IN played_with FILTER pw._from == @contest_id RETURN pw)
                RETURN pw != null ? pw._to : null
            "#,
            )
            .bind_var("contest_id", contest_id)
            .build();
        let mut res = self
            .db
            .aql_query::<Option<String>>(query)
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contest game: {}", e)))?;
        Ok(res.pop().unwrap_or(None))
    }

    pub async fn get_latest_rating(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
        player_id: &str,
    ) -> Result<Option<Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN rating_latest
                  FILTER r.scope_type == @scope_type
                    AND r.player_id == @player_id
                    AND ((@scope_id == null AND r.scope_id == null) OR r.scope_id == @scope_id)
                  LIMIT 1
                  RETURN r
            "#,
            )
            .bind_var("scope_type", scope_type)
            .bind_var("scope_id", scope_id)
            .bind_var("player_id", player_id)
            .build();
        let mut res =
            self.db.aql_query::<Value>(query).await.map_err(|e| {
                SharedError::Database(format!("Failed to fetch latest rating: {}", e))
            })?;
        Ok(res.pop())
    }

    /// Fetch all player_ids that have a latest rating for a given scope
    pub async fn get_all_latest_player_ids(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN rating_latest
                  FILTER r.scope_type == @scope_type
                    AND ((@scope_id == null AND r.scope_id == null) OR r.scope_id == @scope_id)
                  RETURN r.player_id
            "#,
            )
            .bind_var("scope_type", scope_type)
            .bind_var("scope_id", scope_id)
            .build();
        let res = self.db.aql_query::<String>(query).await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch latest player ids: {}", e))
        })?;
        Ok(res)
    }

    pub async fn upsert_latest_rating(&self, doc: Value) -> Result<()> {
        let query = AqlQuery::builder()
            .query(r#"
                UPSERT { player_id: @doc.player_id, scope_type: @doc.scope_type, scope_id: @doc.scope_id }
                INSERT @doc
                UPDATE @doc IN rating_latest
            "#)
            .bind_var("doc", doc)
            .build();
        self.db
            .aql_query::<Value>(query)
            .await
            .map_err(|e| SharedError::Database(format!("Failed to upsert latest rating: {}", e)))?;
        Ok(())
    }

    pub async fn insert_rating_history(&self, doc: Value) -> Result<()> {
        let query = AqlQuery::builder()
            .query(
                r#"
                INSERT @doc INTO rating_history
            "#,
            )
            .bind_var("doc", doc)
            .build();
        self.db.aql_query::<Value>(query).await.map_err(|e| {
            SharedError::Database(format!("Failed to insert rating history: {}", e))
        })?;
        Ok(())
    }

    pub async fn get_leaderboard(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
        min_games: i32,
        limit: i32,
    ) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(r#"
                // Optimized graph traversal for leaderboard with all-time stats
                FOR r IN rating_latest
                  FILTER r.scope_type == @scope_type
                    AND ((@scope_id == null AND r.scope_id == null) OR r.scope_id == @scope_id)
                  
                  // Get player data using DOCUMENT() like the working debug query
                  LET player = DOCUMENT(r.player_id)
                  
                  // Get all contest results for this player through resulted_in edges
                  LET contest_history = (
                    FOR result IN resulted_in
                      FILTER result._to == r.player_id
                      LET contest = DOCUMENT(result._from)
                      RETURN {
                        place: result.place,
                        result: result.result,
                        contest_date: contest.start,
                        contest_id: contest._id
                      }
                  )
                  
                  // Calculate all-time statistics
                  LET total_games = LENGTH(contest_history)
                  LET wins = LENGTH(FOR c IN contest_history FILTER c.place == 1 RETURN c)
                  LET win_rate = total_games > 0 ? (wins * 100.0) / total_games : 0.0
                  
                  // Filter by minimum games
                  FILTER total_games >= @min_games
                  
                  // Sort by rating (highest first)
                  SORT r.rating DESC
                  LIMIT @limit
                  
                  RETURN {
                    player_id: r.player_id,
                    handle: player.handle,
                    firstname: player.firstname,
                    rating: r.rating,
                    rd: r.rd,
                    games_played: total_games,
                    wins: wins,
                    win_rate: win_rate,
                    last_active: total_games > 0 ? MAX(FOR c IN contest_history RETURN c.contest_date) : null,
                    contest_id: total_games > 0 ? FIRST(FOR c IN contest_history SORT c.contest_date DESC RETURN c.contest_id) : null
                  }
            "#)
            .bind_var("scope_type", scope_type)
            .bind_var("scope_id", scope_id)
            .bind_var("min_games", min_games)
            .bind_var("limit", limit)
            .build();
        let res =
            self.db.aql_query::<Value>(query).await.map_err(|e| {
                SharedError::Database(format!("Failed to fetch leaderboard: {}", e))
            })?;
        Ok(res)
    }

    /// Simple leaderboard query that just returns rating data without complex joins
    pub async fn get_simple_leaderboard(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
        min_games: i32,
        limit: i32,
    ) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(r#"
                // Optimized graph traversal for simple leaderboard with all-time stats
                FOR player IN player
                  // Get all contest results for this player through resulted_in edges
                  LET contest_history = (
                    FOR result IN resulted_in
                      FILTER result._to == player._id
                      RETURN 1
                  )
                  
                  // Calculate all-time games played
                  LET total_games = LENGTH(contest_history)
                  
                  // Get current rating from rating_latest
                  LET current_rating = FIRST(
                    FOR r IN rating_latest
                      FILTER r.player_id == player._id AND r.scope_type == @scope_type
                        AND ((@scope_id == null AND r.scope_id == null) OR r.scope_id == @scope_id)
                      RETURN r
                  )
                  
                  // Filter by minimum games and ensure player has ratings
                  FILTER total_games >= @min_games AND current_rating != null
                  
                  // Sort by rating (highest first)
                  SORT current_rating.rating DESC
                  LIMIT @limit
                  
                  RETURN {
                    player_id: player._id,
                    rating: current_rating.rating,
                    rd: current_rating.rd,
                    games_played: total_games,
                    last_active: current_rating.last_period_end,
                    player_handle: player.handle != null ? player.handle : CONCAT(player.firstname, " (", player.email, ")"),
                    player_firstname: player.firstname,
                    player_email: player.email
                  }
            "#)
            .bind_var("scope_type", scope_type)
            .bind_var("scope_id", scope_id)
            .bind_var("min_games", min_games)
            .bind_var("limit", limit)
            .build();
        let res = self.db.aql_query::<Value>(query).await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch simple leaderboard: {}", e))
        })?;
        Ok(res)
    }

    /// Diagnostic function to check what's happening with player IDs
    pub async fn debug_player_ids(&self) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN rating_latest
                  LIMIT 5
                  LET player = DOCUMENT(r.player_id)
                  LET player_exists = player != null
                  LET player_collection = FIRST(FOR p IN player LIMIT 1 RETURN p._id)
                  RETURN {
                    rating_player_id: r.player_id,
                    player_exists: player_exists,
                    player_handle: player != null ? player.handle : "NULL",
                    player_collection_sample: player_collection,
                    debug_info: {
                        rating_id_length: LENGTH(r.player_id),
                        rating_id_starts_with_player: STARTS_WITH(r.player_id, "player/"),
                        player_doc_type: player != null ? TYPENAME(player) : "NULL"
                    }
                  }
            "#,
            )
            .build();
        let res = self
            .db
            .aql_query::<Value>(query)
            .await
            .map_err(|e| SharedError::Database(format!("Failed to debug player IDs: {}", e)))?;
        Ok(res)
    }

    /// Check what's in resulted_in edges vs player collection
    pub async fn debug_resulted_in_vs_players(&self) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                // Get a sample of resulted_in edges
                FOR edge IN resulted_in
                  LIMIT 5
                  LET player = DOCUMENT(edge._to)
                  LET player_exists = player != null
                  RETURN {
                    edge_id: edge._id,
                    edge_from: edge._from,
                    edge_to: edge._to,
                    player_exists: player_exists,
                    player_handle: player != null ? player.handle : "NULL",
                    player_firstname: player != null ? player.firstname : "NULL",
                    player_email: player != null ? player.email : "NULL"
                  }
            "#,
            )
            .build();
        let res = self.db.aql_query::<Value>(query).await.map_err(|e| {
            SharedError::Database(format!("Failed to debug resulted_in vs players: {}", e))
        })?;
        Ok(res)
    }

    /// Check what collections exist in the database
    pub async fn debug_collections(&self) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR collection IN COLLECTIONS()
                  LET doc_count = LENGTH(collection)
                  RETURN {
                    collection_name: collection.name,
                    collection_type: collection.type,
                    document_count: doc_count
                  }
            "#,
            )
            .build();
        let res =
            self.db.aql_query::<Value>(query).await.map_err(|e| {
                SharedError::Database(format!("Failed to debug collections: {}", e))
            })?;
        Ok(res)
    }

    /// Debug function to check what fields are in the player collection
    pub async fn debug_player_fields(&self) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR player IN player
                  LIMIT 3
                  RETURN {
                    player_id: player._id,
                    all_fields: player,
                    handle: player.handle,
                    firstname: player.firstname,
                    email: player.email
                  }
            "#,
            )
            .build();
        let res =
            self.db.aql_query::<Value>(query).await.map_err(|e| {
                SharedError::Database(format!("Failed to debug player fields: {}", e))
            })?;
        Ok(res)
    }

    /// Simple test to see what's in a player document using DOCUMENT()
    pub async fn debug_player_document(&self, player_id: &str) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                LET player = DOCUMENT(@player_id)
                RETURN {
                    player_id: @player_id,
                    player_exists: player != null,
                    all_fields: player,
                    handle: player != null ? player.handle : "NULL",
                    firstname: player != null ? player.firstname : "NULL",
                    email: player != null ? player.email : "NULL"
                }
            "#,
            )
            .bind_var("player_id", player_id)
            .build();
        let res = self.db.aql_query::<Value>(query).await.map_err(|e| {
            SharedError::Database(format!("Failed to debug player document: {}", e))
        })?;
        Ok(res)
    }

    pub async fn get_player_latest_ratings(&self, player_id: &str) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN rating_latest
                  FILTER r.player_id == @player_id
                  RETURN r
            "#,
            )
            .bind_var("player_id", player_id)
            .build();
        let res = self.db.aql_query::<Value>(query).await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch player latest ratings: {}", e))
        })?;
        Ok(res)
    }

    pub async fn get_rating_history(
        &self,
        player_id: &str,
        scope_type: &str,
        scope_id: Option<&str>,
        limit: i32,
    ) -> Result<Vec<Value>> {
        // First try singular collection name
        let singular_query = AqlQuery::builder()
            .query(
                r#"
                FOR h IN rating_history
                  FILTER h.player_id == @player_id
                    AND h.scope_type == @scope_type
                    AND ((@scope_id == null AND h.scope_id == null) OR h.scope_id == @scope_id)
                  SORT h.period_end DESC
                  LIMIT @limit
                  RETURN h
            "#,
            )
            .bind_var("player_id", player_id)
            .bind_var("scope_type", scope_type)
            .bind_var("scope_id", scope_id)
            .bind_var("limit", limit)
            .build();

        match self.db.aql_query::<Value>(singular_query).await {
            Ok(res) => Ok(res),
            Err(e) => {
                let err_str = e.to_string();
                // If collection not found or similar, try pluralized collection
                if err_str.contains("not found") || err_str.contains("collection") {
                    let plural_query = AqlQuery::builder()
                        .query(r#"
                            FOR h IN ratings_history
                              FILTER h.player_id == @player_id
                                AND h.scope_type == @scope_type
                                AND ((@scope_id == null AND h.scope_id == null) OR h.scope_id == @scope_id)
                              SORT h.period_end DESC
                              LIMIT @limit
                              RETURN h
                        "#)
                        .bind_var("player_id", player_id)
                        .bind_var("scope_type", scope_type)
                        .bind_var("scope_id", scope_id)
                        .bind_var("limit", limit)
                        .build();
                    let res = self
                        .db
                        .aql_query::<Value>(plural_query)
                        .await
                        .map_err(|e2| {
                            SharedError::Database(format!(
                                "Failed to fetch ratings history (plural): {}",
                                e2
                            ))
                        })?;
                    Ok(res)
                } else {
                    Err(SharedError::Database(format!(
                        "Failed to fetch rating history: {}",
                        err_str
                    )))
                }
            }
        }
    }

    pub async fn clear_all_ratings(&self) -> Result<()> {
        // Clear rating_latest collection
        let clear_latest_query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN rating_latest
                    REMOVE r IN rating_latest
            "#,
            )
            .build();
        self.db
            .aql_query::<Value>(clear_latest_query)
            .await
            .map_err(|e| SharedError::Database(format!("Failed to clear rating_latest: {}", e)))?;

        // Clear rating_history collection
        let clear_history_query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN rating_history
                    REMOVE r IN rating_history
            "#,
            )
            .build();
        self.db
            .aql_query::<Value>(clear_history_query)
            .await
            .map_err(|e| SharedError::Database(format!("Failed to clear rating_history: {}", e)))?;

        Ok(())
    }

    pub async fn get_earliest_contest_date(&self) -> Result<String> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR c IN contest
                    SORT c.start ASC
                    LIMIT 1
                    RETURN c.start
            "#,
            )
            .build();
        let mut res = self.db.aql_query::<String>(query).await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch earliest contest date: {}", e))
        })?;

        // If no contests found, default to 2000-01-01
        let earliest_date = res
            .pop()
            .unwrap_or_else(|| "2000-01-01T00:00:00Z".to_string());
        Ok(earliest_date)
    }

    /// Leaderboard with player info extracted from contest data
    pub async fn get_leaderboard_with_contest_data(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
        min_games: i32,
        limit: i32,
    ) -> Result<Vec<Value>> {
        let query = AqlQuery::builder()
            .query(
                r#"
                FOR r IN rating_latest
                  FILTER r.scope_type == @scope_type
                    AND ((@scope_id == null AND r.scope_id == null) OR r.scope_id == @scope_id)
                    AND r.games_played >= @min_games
                  
                  // Try to get player info from contest data
                  LET player_contest_data = (
                    FOR contest IN contest
                      FOR result IN resulted_in
                        FILTER result._from == contest._id AND result._to == r.player_id
                        LIMIT 1
                        RETURN {
                          contest_name: contest.name,
                          contest_date: contest.start,
                          player_place: result.place
                        }
                  )[0]
                  
                  SORT r.rating DESC
                  LIMIT @limit
                  RETURN {
                    player_id: r.player_id,
                    rating: r.rating,
                    rd: r.rd,
                    games_played: r.games_played,
                    last_active: r.last_period_end,
                    player_handle: CONCAT("Player ", SUBSTRING(r.player_id, 8, 8)),
                    player_firstname: "Unknown",
                    player_email: "Unknown",
                    contest_info: player_contest_data
                  }
            "#,
            )
            .bind_var("scope_type", scope_type)
            .bind_var("scope_id", scope_id)
            .bind_var("min_games", min_games)
            .bind_var("limit", limit)
            .build();
        let res = self.db.aql_query::<Value>(query).await.map_err(|e| {
            SharedError::Database(format!(
                "Failed to fetch leaderboard with contest data: {}",
                e
            ))
        })?;
        Ok(res)
    }
}
