use crate::cache::{CacheKeys, CacheTTL, RedisCache};
use crate::db::Db;
use crate::surreal_helpers::{record_id_from_row, select_one_by_record_id};
use crate::third_party::BGGService;
use shared::dto::game::GameDto;
use shared::models::game::Game;
use std::sync::Arc;

fn value_to_game(v: &serde_json::Value) -> Option<Game> {
    let id = record_id_from_row(v, None)?;
    Some(Game {
        id,
        rev: v.get("_rev").or_else(|| v.get("rev")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
        name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        year_published: v.get("year_published").and_then(|x| x.as_i64()).map(|n| n as i32),
        bgg_id: v.get("bgg_id").and_then(|x| x.as_i64()).map(|n| n as i32),
        description: v.get("description").and_then(|x| x.as_str()).map(String::from),
        source: shared::models::game::GameSource::Database,
    })
}

#[derive(Clone)]
pub struct GameRepositoryImpl {
    pub db: Db,
    pub bgg_service: Option<BGGService>,
    pub cache: Option<Arc<RedisCache>>,
}

#[async_trait::async_trait]
pub trait GameRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Option<Game>;
    async fn find_all(&self) -> Vec<Game>;
    async fn search(&self, query: &str) -> Vec<Game>;
    async fn search_dto(&self, query: &str) -> Vec<GameDto>;
    async fn search_db_only(&self, query: &str) -> Vec<Game>;
    async fn search_db_only_dto(&self, query: &str) -> Vec<GameDto>;
    async fn get_game_recommendations(
        &self,
        player_id: &str,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>, String>;
    async fn get_similar_games(
        &self,
        game_id: &str,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>, String>;
    async fn get_popular_games(&self, limit: i32) -> Result<Vec<serde_json::Value>, String>;
    async fn create(&self, game: Game) -> Result<Game, String>;
    async fn update(&self, game: Game) -> Result<Game, String>;
    async fn delete(&self, id: &str) -> Result<(), String>;
}

impl GameRepositoryImpl {
    pub fn new(db: Db) -> Self {
        Self { db, bgg_service: None, cache: None }
    }

    pub fn new_with_bgg(db: Db, bgg_service: BGGService) -> Self {
        Self { db, bgg_service: Some(bgg_service), cache: None }
    }

    pub fn new_with_cache(db: Db, cache: Arc<RedisCache>) -> Self {
        Self { db, bgg_service: None, cache: Some(cache) }
    }

    /// Fill search results with BGG results when DB returns fewer than max_results.
    pub async fn search_fill_bgg(&self, query: &str, mut results: Vec<Game>) -> Vec<Game> {
        let max_results = 20;
        if results.len() < max_results && self.bgg_service.is_some() {
            if let Some(ref bgg_service) = self.bgg_service {
                if let Ok(bgg_results) = bgg_service.search_games(query).await {
                    let remaining = max_results - results.len();
                    for game in bgg_results.into_iter().take(remaining) {
                        results.push(Game {
                            id: game.id,
                            rev: game.rev,
                            name: game.name,
                            year_published: game.year_published,
                            bgg_id: game.bgg_id,
                            description: game.description,
                            source: shared::models::game::GameSource::BGG,
                        });
                    }
                }
            }
        }
        results
    }

    pub fn new_with_bgg_and_cache(db: Db, bgg_service: BGGService, cache: Arc<RedisCache>) -> Self {
        Self { db, bgg_service: Some(bgg_service), cache: Some(cache) }
    }
}

#[async_trait::async_trait]
impl GameRepository for GameRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Option<Game> {
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::game(id);
            if let Ok(Some(cached_game)) = cache.get::<Game>(&cache_key).await {
                log::debug!("Cache hit for game: {}", id);
                return Some(cached_game);
            }
        }

        let game = select_one_by_record_id(&self.db, "game", id)
            .await
            .and_then(|v| value_to_game(&v));
        if let Some(ref g) = game {
            if let Some(ref cache) = self.cache {
                let _ = cache.set_with_ttl(&CacheKeys::game(id), g, CacheTTL::game()).await;
            }
        }
        game
    }

    async fn find_all(&self) -> Vec<Game> {
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::game_list();
            if let Ok(Some(cached_games)) = cache.get::<Vec<Game>>(&cache_key).await {
                log::debug!("Cache hit for game list");
                return cached_games;
            }
        }

        let mut res = match self.db.query("SELECT * FROM game").await {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let games: Vec<Game> = rows.into_iter().filter_map(|v| value_to_game(&v)).collect();
        if let Some(ref cache) = self.cache {
            let _ = cache.set_with_ttl(&CacheKeys::game_list(), &games, CacheTTL::game_list()).await;
        }
        games
    }

    async fn search(&self, query: &str) -> Vec<Game> {
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::game_search(query);
            if let Ok(Some(cached_games)) = cache.get::<Vec<Game>>(&cache_key).await {
                log::debug!("Cache hit for game search: {}", query);
                return cached_games;
            }
        }

        let max_results = 20i64;
        let mut results = Vec::new();
        let q_owned = query.to_string();

        let mut res = match self.db
            .query("SELECT * FROM game WHERE string::contains(string::lowercase(name), string::lowercase($q)) LIMIT $limit")
            .bind(("q", q_owned.clone()))
            .bind(("limit", max_results))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to search games by name: {}", e);
                return self.search_fill_bgg(query, results).await;
            }
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        results.extend(rows.into_iter().filter_map(|v| value_to_game(&v)));

        if results.len() < max_results as usize {
            let remaining = max_results - results.len() as i64;
            let mut res2 = match self.db
                .query("SELECT * FROM game WHERE description != NONE AND string::contains(string::lowercase(description), string::lowercase($q)) LIMIT $limit")
                .bind(("q", q_owned.clone()))
                .bind(("limit", remaining))
                .await
            {
                Ok(r) => r,
                Err(_) => return self.search_fill_bgg(query, results).await,
            };
            let rows2: Vec<serde_json::Value> = res2.take(0).unwrap_or_default();
            for v in rows2 {
                if let Some(g) = value_to_game(&v) {
                    if !results.iter().any(|x| x.id == g.id) {
                        results.push(g);
                    }
                }
            }
        }

        let out = self.search_fill_bgg(query, results).await;
        if let Some(ref cache) = self.cache {
            let _ = cache.set_with_ttl(&CacheKeys::game_search(query), &out, CacheTTL::game_search()).await;
        }
        out
    }

    async fn get_game_recommendations(
        &self,
        player_id: &str,
        _limit: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Getting game recommendations for player: {}", player_id);
        // TODO: implement with SurrealQL graph-style queries
        Ok(Vec::new())
    }

    async fn get_similar_games(
        &self,
        game_id: &str,
        _limit: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Getting similar games for game: {}", game_id);
        // TODO: implement with SurrealQL
        Ok(Vec::new())
    }

    async fn get_popular_games(&self, _limit: i32) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Getting popular games");
        // TODO: implement with SurrealQL (count played_with per game, order, limit)
        Ok(Vec::new())
    }

    async fn search_dto(&self, query: &str) -> Vec<GameDto> {
        self.search(query).await.into_iter().map(|g| GameDto::from(&g)).collect()
    }

    async fn search_db_only(&self, query: &str) -> Vec<Game> {
        let q_owned = query.to_string();
        let mut res = match self.db
            .query("SELECT * FROM game WHERE string::contains(string::lowercase(name), string::lowercase($q)) LIMIT 20")
            .bind(("q", q_owned.clone()))
            .await
        {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut results: Vec<Game> = rows.into_iter().filter_map(|v| value_to_game(&v)).collect();
        if results.len() < 20 {
            let mut res2 = match self.db
                .query("SELECT * FROM game WHERE description != NONE AND string::contains(string::lowercase(description), string::lowercase($q)) LIMIT $limit")
                .bind(("q", q_owned))
                .bind(("limit", 20 - results.len() as i64))
                .await
            {
                Ok(r) => r,
                Err(_) => return results,
            };
            let rows2: Vec<serde_json::Value> = res2.take(0).unwrap_or_default();
            for v in rows2 {
                if let Some(g) = value_to_game(&v) {
                    if !results.iter().any(|x| x.id == g.id) {
                        results.push(g);
                    }
                }
            }
        }
        results
    }

    async fn search_db_only_dto(&self, query: &str) -> Vec<GameDto> {
        self.search_db_only(query).await.into_iter().map(|g| GameDto::from(&g)).collect()
    }

    async fn create(&self, game: Game) -> Result<Game, String> {
        if let Some(bgg_id) = game.bgg_id {
            if let Ok(mut res) = self.db.query("SELECT * FROM game WHERE bgg_id = $bgg_id LIMIT 1").bind(("bgg_id", bgg_id)).await {
                let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
                if let Some(v) = rows.into_iter().next() {
                    if let Some(existing) = value_to_game(&v) {
                        return Ok(existing);
                    }
                }
            }
        }

        let key = uuid::Uuid::new_v4().to_string();
        let doc = serde_json::json!({
            "name": game.name,
            "year_published": game.year_published,
            "bgg_id": game.bgg_id,
            "description": game.description,
        });
        self.db
            .query("CREATE type::record('game', $key) CONTENT $doc")
            .bind(("key", key.clone()))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to create game: {}", e))?;
        let created_game = Game {
            id: format!("game/{}", key),
            rev: String::new(),
            name: game.name,
            year_published: game.year_published,
            bgg_id: game.bgg_id,
            description: game.description,
            source: game.source,
        };
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::game(&created_game.id)).await;
            let _ = cache.delete(&CacheKeys::game_list()).await;
            let _ = cache.invalidate_pattern("games:search:").await;
        }
        Ok(created_game)
    }

    async fn update(&self, game: Game) -> Result<Game, String> {
        let key = game.id.trim_start_matches("game/").trim_start_matches("game:").to_string();
        let doc = serde_json::json!({
            "name": game.name,
            "year_published": game.year_published,
            "bgg_id": game.bgg_id,
            "description": game.description,
        });
        self.db
            .query("UPDATE type::record('game', $key) MERGE $doc")
            .bind(("key", key))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to update game: {}", e))?;
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::game(&game.id)).await;
            let _ = cache.delete(&CacheKeys::game_list()).await;
            let _ = cache.invalidate_pattern("games:search:").await;
        }
        Ok(game)
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        let key = id.trim_start_matches("game/").trim_start_matches("game:").to_string();
        self.db
            .query("DELETE type::record('game', $key)")
            .bind(("key", key))
            .await
            .map_err(|e| format!("Failed to delete game: {}", e))?;
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::game(id)).await;
            let _ = cache.delete(&CacheKeys::game_list()).await;
            let _ = cache.invalidate_pattern("games:search:").await;
        }
        Ok(())
    }
}
