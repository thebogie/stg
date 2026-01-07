use shared::models::game::Game;
use shared::dto::game::GameDto;
use arangors::Database;
use arangors::client::reqwest::ReqwestClient;
use arangors::document::options::{InsertOptions, UpdateOptions, RemoveOptions};
use crate::third_party::BGGService;
use serde::{Deserialize, Serialize};

// Database-only game model (without source field)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameDb {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev")]
    pub rev: String,
    pub name: String,
    #[serde(rename = "year_published")]
    pub year_published: Option<i32>,
    #[serde(rename = "bgg_id")]
    pub bgg_id: Option<i32>,
    pub description: Option<String>,
}

impl From<GameDb> for Game {
    fn from(db_game: GameDb) -> Self {
        Game {
            id: db_game.id,
            rev: db_game.rev,
            name: db_game.name,
            year_published: db_game.year_published,
            bgg_id: db_game.bgg_id,
            description: db_game.description,
            source: shared::models::game::GameSource::Database,
        }
    }
}

#[derive(Clone)]
pub struct GameRepositoryImpl {
    pub db: Database<ReqwestClient>,
    pub bgg_service: Option<BGGService>,
}

#[async_trait::async_trait]
pub trait GameRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Option<Game>;
    async fn find_all(&self) -> Vec<Game>;
    async fn search(&self, query: &str) -> Vec<Game>;
    async fn search_dto(&self, query: &str) -> Vec<GameDto>;
    async fn search_db_only(&self, query: &str) -> Vec<Game>;
    async fn search_db_only_dto(&self, query: &str) -> Vec<GameDto>;
    async fn get_game_recommendations(&self, player_id: &str, limit: i32) -> Result<Vec<serde_json::Value>, String>;
    async fn get_similar_games(&self, game_id: &str, limit: i32) -> Result<Vec<serde_json::Value>, String>;
    async fn get_popular_games(&self, limit: i32) -> Result<Vec<serde_json::Value>, String>;
    async fn create(&self, game: Game) -> Result<Game, String>;
    async fn update(&self, game: Game) -> Result<Game, String>;
    async fn delete(&self, id: &str) -> Result<(), String>;
}

impl GameRepositoryImpl {
    pub fn new(db: Database<ReqwestClient>) -> Self {
        Self { 
            db,
            bgg_service: None,
        }
    }

    pub fn new_with_bgg(db: Database<ReqwestClient>, bgg_service: BGGService) -> Self {
        Self { 
            db,
            bgg_service: Some(bgg_service),
        }
    }
}

#[async_trait::async_trait]
impl GameRepository for GameRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Option<Game> {
        let query = arangors::AqlQuery::builder()
            .query("FOR g IN game FILTER g._id == @id LIMIT 1 RETURN g")
            .bind_var("id", id)
            .build();
        match self.db.aql_query::<GameDb>(query).await {
            Ok(mut cursor) => cursor.pop().map(|db_game| Game::from(db_game)),
            Err(_) => None,
        }
    }

    async fn find_all(&self) -> Vec<Game> {
        let query = arangors::AqlQuery::builder()
            .query("FOR g IN game RETURN g")
            .build();
        match self.db.aql_query::<GameDb>(query).await {
            Ok(cursor) => {
                let db_games: Vec<GameDb> = cursor.into_iter().collect();
                // Convert database games to full Game models with source field
                db_games.into_iter().map(|db_game| Game::from(db_game)).collect()
            },
            Err(_) => Vec::new(),
        }
    }

    async fn search(&self, query: &str) -> Vec<Game> {
        let max_results = 20;
        let mut results = Vec::new();

        log::info!("=== Starting game search for query: '{}' ===", query);
        
        // Debug: Check database connection and collection
        log::info!("Database name: {}", self.db.name());
        log::info!("Database URL: {:?}", self.db.url());
        
        // Check if games collection exists
        match self.db.collection("game").await {
            Ok(collection) => {
                log::info!("Games collection found: {}", collection.name());
            },
            Err(e) => {
                log::warn!("Games collection not found or error: {}", e);
            }
        }

        // Debug: Check total games in database
        let count_query = arangors::AqlQuery::builder()
            .query("RETURN LENGTH(FOR g IN game RETURN g)")
            .build();
        
        match self.db.aql_query::<i32>(count_query).await {
            Ok(mut cursor) => {
                if let Some(count) = cursor.pop() {
                    log::info!("Total games in database: {}", count);
                    if count == 0 {
                        log::warn!("Database is empty! No games found in database.");
                    }
                }
            },
            Err(e) => {
                log::warn!("Failed to count games in database: {}", e);
            }
        }

        // Search by name
        let name_query = arangors::AqlQuery::builder()
            .query("FOR g IN game FILTER CONTAINS(LOWER(g.name), LOWER(@query)) LIMIT @limit RETURN g")
            .bind_var("query", query)
            .bind_var("limit", max_results)
            .build();
        
        match self.db.aql_query::<GameDb>(name_query).await {
            Ok(cursor) => {
                let db_games: Vec<GameDb> = cursor.into_iter().collect();
                log::info!("Name search returned {} games", db_games.len());
                
                // Convert database games to full Game models with source field
                let games: Vec<Game> = db_games.into_iter().map(|db_game| {
                    log::info!("  Game: ID={}, Name='{}', Year={:?}, BGG={:?}", 
                              db_game.id, db_game.name, db_game.year_published, db_game.bgg_id);
                    Game::from(db_game)
                }).collect();
                
                results.extend(games);
            },
            Err(e) => {
                log::error!("Failed to search games by name: {}", e);
            }
        }

        // Search by description if we haven't reached the limit
        if results.len() < max_results {
            let remaining_limit = max_results - results.len();
            let desc_query = arangors::AqlQuery::builder()
                .query("FOR g IN game FILTER g.description != null AND CONTAINS(LOWER(g.description), LOWER(@query)) LIMIT @limit RETURN g")
                .bind_var("query", query)
                .bind_var("limit", remaining_limit)
                .build();
            
            match self.db.aql_query::<GameDb>(desc_query).await {
                Ok(cursor) => {
                    let db_games: Vec<GameDb> = cursor.into_iter().collect();
                    log::info!("Description search returned {} games", db_games.len());
                    
                    // Convert database games to full Game models with source field
                    let games: Vec<Game> = db_games.into_iter().map(|db_game| {
                        log::info!("  Game from description: ID={}, Name='{}', Year={:?}, BGG={:?}", 
                                  db_game.id, db_game.name, db_game.year_published, db_game.bgg_id);
                        Game::from(db_game)
                    }).collect();
                    
                    results.extend(games);
                },
                Err(e) => {
                    log::error!("Failed to search games by description: {}", e);
                }
            }
        }

        // Always try to fill remaining slots with BGG API results (if available)
        if results.len() < max_results && self.bgg_service.is_some() {
            log::info!("BGG API is available, attempting to fill {} remaining slots", max_results - results.len());
            if let Some(ref bgg_service) = self.bgg_service {
                match bgg_service.search_games(query).await {
                    Ok(bgg_results) => {
                        let remaining_limit = max_results - results.len();
                        log::info!("BGG API returned {} results, adding {} to fill remaining slots", 
                                  bgg_results.len(), std::cmp::min(remaining_limit, bgg_results.len()));
                        
                        for game in bgg_results.into_iter().take(remaining_limit) {
                            // Convert BGG game to Game model with BGG source
                            results.push(Game {
                                id: game.id.clone(),
                                rev: game.rev.clone(),
                                name: game.name.clone(),
                                year_published: game.year_published,
                                bgg_id: game.bgg_id,
                                description: game.description.clone(),
                                source: shared::models::game::GameSource::BGG,
                            });
                        }
                    }
                    Err(e) => {
                        log::warn!("BGG API search failed: {}", e);
                    }
                }
            }
        } else if results.len() < max_results {
            log::info!("BGG API not available, cannot fill remaining {} slots", max_results - results.len());
        }

        log::info!("Total search results: {} games", results.len());
        for game in &results {
            log::info!("Final game result: ID={}, Name={}, Source={:?}", 
                      game.id, game.name, game.source);
        }

        results
    }

    async fn get_game_recommendations(&self, player_id: &str, limit: i32) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Getting game recommendations for player: {}", player_id);
        
        let query = arangors::AqlQuery::builder()
            .query(r#"
                // Graph traversal: player -> games -> similar players -> new games
                FOR player IN player
                  FILTER player._id == @player_id
                  
                  // Get all games this player has played
                  LET played_games = (
                    FOR result IN resulted_in
                      FILTER result._to == player._id
                      LET contest = DOCUMENT(result._from)
                      FOR game_edge IN played_with
                        FILTER game_edge._from == contest._id
                        LET game = DOCUMENT(game_edge._to)
                        COLLECT game_id = game._id INTO unique_games
                        RETURN DOCUMENT(game_id[0])
                  )
                  
                  // Find players who like similar games
                  LET similar_players = (
                    FOR game IN played_games
                      FOR contest_edge IN played_with
                        FILTER contest_edge._to == game._id
                        LET contest = DOCUMENT(contest_edge._from)
                        FOR result IN resulted_in
                          FILTER result._from == contest._id
                          LET other_player = DOCUMENT(result._to)
                          FILTER other_player._id != @player_id
                          COLLECT other_player_id = other_player._id, other_player_data = other_player INTO player_games
                          
                          // Calculate similarity score based on common games
                          LET common_games = LENGTH(player_games)
                          LET total_games = LENGTH(
                            FOR result IN resulted_in
                            FILTER result._to == other_player_id
                            LET contest = DOCUMENT(result._from)
                            FOR game_edge IN played_with
                              FILTER game_edge._from == contest._id
                              RETURN 1
                          )
                          LET similarity_score = total_games > 0 ? (common_games * 100.0) / total_games : 0.0
                          
                          RETURN {
                            player_id: other_player_id,
                            player_handle: other_player_data.handle,
                            similarity_score: similarity_score,
                            common_games: common_games
                          }
                  )
                  
                  // Get games that similar players like but this player hasn't played
                  LET recommended_games = (
                    FOR similar_player IN similar_players
                      FILTER similar_player.similarity_score > 30.0  // Only consider players with >30% similarity
                      FOR result IN resulted_in
                        FILTER result._to == similar_player.player_id
                        LET contest = DOCUMENT(result._from)
                        FOR game_edge IN played_with
                          FILTER game_edge._from == contest._id
                          LET game = DOCUMENT(game_edge._to)
                          
                          // Check if player hasn't played this game
                          FILTER game._id NOT IN played_games[*]._id
                          
                          // Calculate recommendation score
                          LET recommendation_score = similar_player.similarity_score * similar_player.common_games
                          
                          COLLECT game_id = game._id, game_data = game INTO game_recommendations
                          LET total_score = SUM(
                            FOR gr IN game_recommendations
                            RETURN gr.recommendation_score
                          )
                          LET total_plays = LENGTH(game_recommendations)
                          
                          RETURN {
                            game_id: game_id,
                            game_name: game_data.name,
                            game_year: game_data.year_published,
                            recommendation_score: total_score,
                            total_plays: total_plays,
                            similar_players: LENGTH(game_recommendations)
                          }
                  )
                  
                  // Return top recommendations
                  FOR game IN recommended_games
                  SORT game.recommendation_score DESC, game.total_plays DESC
                  LIMIT @limit
                  RETURN game
            "#)
            .bind_var("player_id", player_id)
            .bind_var("limit", limit)
            .build();
        
        match self.db.aql_query::<serde_json::Value>(query).await {
            Ok(cursor) => {
                let results: Vec<serde_json::Value> = cursor.into_iter().collect();
                log::info!("✅ Game recommendations retrieved for player: {} ({} games)", player_id, results.len());
                Ok(results)
            }
            Err(e) => {
                log::error!("❌ Failed to get game recommendations: {}", e);
                Err(format!("Database query failed: {}", e))
            }
        }
    }

    async fn get_similar_games(&self, game_id: &str, limit: i32) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Getting similar games for game: {}", game_id);
        
        let query = arangors::AqlQuery::builder()
            .query(r#"
                // Graph traversal: game -> players -> other games
                FOR game IN game
                  FILTER game._id == @game_id
                  
                  // Get all players who have played this game
                  LET game_players = (
                    FOR contest_edge IN played_with
                      FILTER contest_edge._to == game._id
                      LET contest = DOCUMENT(contest_edge._from)
                      FOR result IN resulted_in
                        FILTER result._from == contest._id
                        LET player = DOCUMENT(result._to)
                        COLLECT player_id = player._id INTO unique_players
                        RETURN player_id
                  )
                  
                  // Find other games these players enjoy
                  LET similar_games = (
                    FOR player_id IN game_players
                      FOR result IN resulted_in
                        FILTER result._to == player_id
                        LET contest = DOCUMENT(result._from)
                        FOR game_edge IN played_with
                          FILTER game_edge._from == contest._id
                          LET other_game = DOCUMENT(game_edge._to)
                          FILTER other_game._id != @game_id
                          
                          // Calculate similarity based on player overlap
                          COLLECT other_game_id = other_game._id, other_game_data = other_game INTO game_players
                          LET common_players = LENGTH(game_players)
                          LET total_players = LENGTH(game_players)
                          
                          LET similarity_score = total_players > 0 ? (common_players * 100.0) / total_players : 0.0
                          
                          RETURN {
                            game_id: other_game_id,
                            game_name: other_game_data.name,
                            game_year: other_game_data.year_published,
                            similarity_score: similarity_score,
                            common_players: common_players
                          }
                  )
                  
                  // Return top similar games
                  FOR game IN similar_games
                  SORT game.similarity_score DESC, game.common_players DESC
                  LIMIT @limit
                  RETURN game
            "#)
            .bind_var("game_id", game_id)
            .bind_var("limit", limit)
            .build();
        
        match self.db.aql_query::<serde_json::Value>(query).await {
            Ok(cursor) => {
                let results: Vec<serde_json::Value> = cursor.into_iter().collect();
                log::info!("✅ Similar games retrieved for game: {} ({} games)", game_id, results.len());
                Ok(results)
            }
            Err(e) => {
                log::error!("❌ Failed to get similar games: {}", e);
                Err(format!("Database query failed: {}", e))
            }
        }
    }

    async fn get_popular_games(&self, limit: i32) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Getting popular games (limit: {})", limit);
        
        let query = arangors::AqlQuery::builder()
            .query(r#"
                // Graph traversal: games -> contests -> players
                FOR game IN game
                  
                  // Count total plays for this game
                  LET total_plays = LENGTH(
                    FOR contest_edge IN played_with
                      FILTER contest_edge._to == game._id
                      RETURN contest_edge
                  )
                  
                  // Get unique players who have played this game
                  LET unique_players = LENGTH(
                    FOR contest_edge IN played_with
                      FILTER contest_edge._to == game._id
                      LET contest = DOCUMENT(contest_edge._from)
                      FOR result IN resulted_in
                        FILTER result._from == contest._id
                        COLLECT player_id = result._to INTO unique_player_ids
                        RETURN 1
                  )
                  
                  // Calculate popularity score (plays + unique players)
                  LET popularity_score = total_plays + unique_players
                  
                  // Only return games that have been played
                  FILTER total_plays > 0
                  
                  // Return game with popularity metrics
                  RETURN {
                    game_id: game._id,
                    game_name: game.name,
                    game_year: game.year_published,
                    total_plays: total_plays,
                    unique_players: unique_players,
                    popularity_score: popularity_score
                  }
            "#)
            .bind_var("limit", limit)
            .build();
        
        match self.db.aql_query::<serde_json::Value>(query).await {
            Ok(cursor) => {
                let mut results: Vec<serde_json::Value> = cursor.into_iter().collect();
                
                // Sort by popularity score and limit results
                results.sort_by(|a, b| {
                    let score_a = a["popularity_score"].as_f64().unwrap_or(0.0);
                    let score_b = b["popularity_score"].as_f64().unwrap_or(0.0);
                    score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                });
                
                results.truncate(limit as usize);
                log::info!("✅ Popular games retrieved ({} games)", results.len());
                Ok(results)
            }
            Err(e) => {
                log::error!("❌ Failed to get popular games: {}", e);
                Err(format!("Database query failed: {}", e))
            }
        }
    }

    async fn search_dto(&self, query: &str) -> Vec<GameDto> {
        let games = self.search(query).await;
        games.into_iter().map(|game| GameDto::from(&game)).collect()
    }

    async fn search_db_only(&self, query: &str) -> Vec<Game> {
        let max_results = 20;
        let mut results = Vec::new();

        // Search by name in DB
        let name_query = arangors::AqlQuery::builder()
            .query("FOR g IN game FILTER CONTAINS(LOWER(g.name), LOWER(@query)) LIMIT @limit RETURN g")
            .bind_var("query", query)
            .bind_var("limit", max_results)
            .build();
        if let Ok(cursor) = self.db.aql_query::<GameDb>(name_query).await {
            let db_games: Vec<GameDb> = cursor.into_iter().collect();
            results.extend(db_games.into_iter().map(|db_game| Game::from(db_game)));
        }

        // Search by description if capacity remains
        if results.len() < max_results {
            let remaining_limit = max_results - results.len();
            let desc_query = arangors::AqlQuery::builder()
                .query("FOR g IN game FILTER g.description != null AND CONTAINS(LOWER(g.description), LOWER(@query)) LIMIT @limit RETURN g")
                .bind_var("query", query)
                .bind_var("limit", remaining_limit)
                .build();
            if let Ok(cursor) = self.db.aql_query::<GameDb>(desc_query).await {
                let db_games: Vec<GameDb> = cursor.into_iter().collect();
                results.extend(db_games.into_iter().map(|db_game| Game::from(db_game)));
            }
        }

        results
    }

    async fn search_db_only_dto(&self, query: &str) -> Vec<GameDto> {
        let games = self.search_db_only(query).await;
        games.into_iter().map(|game| GameDto::from(&game)).collect()
    }

    async fn create(&self, game: Game) -> Result<Game, String> {
        let collection = self.db.collection("game").await
            .map_err(|e| format!("Failed to get collection: {}", e))?;

        // If bgg_id is present, check for existing game with same bgg_id
        if let Some(bgg_id) = game.bgg_id {
            let query = arangors::AqlQuery::builder()
                .query("FOR g IN game FILTER g.bgg_id == @bgg_id LIMIT 1 RETURN g")
                .bind_var("bgg_id", bgg_id)
                .build();
            if let Ok(mut cursor) = self.db.aql_query::<GameDb>(query).await {
                if let Some(db_game) = cursor.pop() {
                    // Return the actual DB game, not the input game
                    return Ok(Game::from(db_game));
                }
            }
        }

        match collection.create_document(game.clone(), InsertOptions::default()).await {
            Ok(created_doc) => {
                let header = created_doc.header().unwrap();
                Ok(Game {
                    id: header._id.clone(),
                    rev: header._rev.clone(),
                    name: game.name,
                    year_published: game.year_published,
                    bgg_id: game.bgg_id,
                    description: game.description,
                    source: game.source,
                })
            },
            Err(e) => Err(format!("Failed to create game: {}", e)),
        }
    }

    async fn update(&self, game: Game) -> Result<Game, String> {
        let collection = self.db.collection("game").await
            .map_err(|e| format!("Failed to get collection: {}", e))?;
        
        // Arango expects document key, not full _id
        let key = game.id.split_once('/').map(|(_, k)| k).unwrap_or(&game.id);
        match collection.update_document(key, game.clone(), UpdateOptions::default()).await {
            Ok(_updated_doc) => {
                // Fetch the latest doc to return consistent data
                let aql = arangors::AqlQuery::builder()
                    .query("RETURN DOCUMENT(@id)")
                    .bind_var("id", format!("game/{}", key))
                    .build();
                match self.db.aql_query::<GameDb>(aql).await {
                    Ok(mut cursor) => {
                        if let Some(db_game) = cursor.pop() {
                            Ok(Game::from(db_game))
                        } else {
                            Err("Updated game not found".to_string())
                        }
                    }
                    Err(e) => Err(format!("Failed to fetch updated game: {}", e)),
                }
            },
            Err(e) => Err(format!("Failed to update game: {}", e)),
        }
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        let collection = self.db.collection("game").await
            .map_err(|e| format!("Failed to get collection: {}", e))?;
        
        // Arango expects document key, not full _id
        let key = id.split_once('/').map(|(_, k)| k).unwrap_or(id);
        match collection.remove_document::<serde_json::Value>(key, RemoveOptions::default(), None).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete game: {}", e)),
        }
    }
} 