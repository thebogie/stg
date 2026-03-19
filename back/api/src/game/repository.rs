use crate::cache::{CacheKeys, CacheTTL, RedisCache};
use crate::db::Db;
use crate::surreal_helpers::{record_id_from_row, select_one_by_record_id_scoped};
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
    /// When set (e.g. in integration tests), ensure NS/DB scope is set on the connection that executes each query.
    pub ns: Option<String>,
    pub db_name: Option<String>,
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
        Self {
            db,
            bgg_service: None,
            cache: None,
            ns: None,
            db_name: None,
        }
    }

    pub fn new_with_bgg(db: Db, bgg_service: BGGService) -> Self {
        Self {
            db,
            bgg_service: Some(bgg_service),
            cache: None,
            ns: None,
            db_name: None,
        }
    }

    pub fn new_with_cache(db: Db, cache: Arc<RedisCache>) -> Self {
        Self {
            db,
            bgg_service: None,
            cache: Some(cache),
            ns: None,
            db_name: None,
        }
    }

    pub fn new_with_bgg_and_cache(db: Db, bgg_service: BGGService, cache: Arc<RedisCache>) -> Self {
        Self {
            db,
            bgg_service: Some(bgg_service),
            cache: Some(cache),
            ns: None,
            db_name: None,
        }
    }

    /// For integration tests: ensure each query runs with the given NS/DB (scope isn't reliably persisted across WS connections).
    pub fn new_with_scope(db: Db, ns: String, db_name: String) -> Self {
        Self {
            db,
            bgg_service: None,
            cache: None,
            ns: Some(ns),
            db_name: Some(db_name),
        }
    }

    /// For production: BGG + cache + per-query NS/DB scope so reads/writes hit the expected database.
    pub fn new_with_bgg_and_cache_and_scope(
        db: Db,
        bgg_service: BGGService,
        cache: Arc<RedisCache>,
        ns: String,
        db_name: String,
    ) -> Self {
        Self {
            db,
            bgg_service: Some(bgg_service),
            cache: Some(cache),
            ns: Some(ns),
            db_name: Some(db_name),
        }
    }

    async fn ensure_scope(&self) {
        if let (Some(ref ns), Some(ref db_name)) = (&self.ns, &self.db_name) {
            let _ = self.db.use_ns(ns).use_db(db_name).await;
        }
    }

    fn query_with_scope(&self, core: &str) -> String {
        if let (Some(ref ns), Some(ref db_name)) = (&self.ns, &self.db_name) {
            let ns_ok = ns.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            let db_ok = db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if ns_ok && db_ok {
                return format!("USE NS {}; USE DB {}; {}", ns, db_name, core);
            }
        }
        core.to_string()
    }

    fn scope_result_index(&self) -> usize {
        if self.ns.is_some() && self.db_name.is_some() {
            2
        } else {
            0
        }
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

    async fn update_game_doc_by_key(&self, key: &str, doc: &serde_json::Value) -> Result<(), String> {
        self.ensure_scope().await;
        // Migration compatibility: depending on how the DB was imported, record id keys may be UUID-typed
        // (type::uuid("...")) or plain string keys. Try UUID first when possible, then string-key fallback.
        if uuid::Uuid::parse_str(key).is_ok() {
            let _ = self
                .db
                .query(self.query_with_scope("UPDATE type::record('game', type::uuid($key)) MERGE $doc"))
                .bind(("key", key.to_string()))
                .bind(("doc", doc.clone()))
                .await;
        }
        let res = self
            .db
            .query(self.query_with_scope("UPDATE type::record('game', $key) MERGE $doc"))
            .bind(("key", key.to_string()))
            .bind(("doc", doc.clone()))
            .await;
        res.map(|_| ()).map_err(|e| format!("Failed to update game: {}", e))
    }

    async fn delete_game_by_key(&self, key: &str) -> Result<(), String> {
        self.ensure_scope().await;
        if uuid::Uuid::parse_str(key).is_ok() {
            let _ = self
                .db
                .query(self.query_with_scope("DELETE type::record('game', type::uuid($key))"))
                .bind(("key", key.to_string()))
                .await;
        }
        let res = self
            .db
            .query(self.query_with_scope("DELETE type::record('game', $key)"))
            .bind(("key", key.to_string()))
            .await;
        res.map(|_| ()).map_err(|e| format!("Failed to delete game: {}", e))
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

        let game = select_one_by_record_id_scoped(
            &self.db,
            "game",
            id,
            self.ns.as_deref(),
            self.db_name.as_deref(),
        )
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

        self.ensure_scope().await;
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

        self.ensure_scope().await;
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
        self.ensure_scope().await;
        if let Some(bgg_id) = game.bgg_id {
            if let Ok(mut res) = self
                .db
                .query(self.query_with_scope("SELECT * FROM game WHERE bgg_id = $bgg_id LIMIT 1"))
                .bind(("bgg_id", bgg_id))
                .await
            {
                let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
                if let Some(v) = rows.into_iter().next() {
                    if let Some(existing) = value_to_game(&v) {
                        return Ok(existing);
                    }
                }
            }
        }

        // Use UUID-typed record ids when possible; backend is compatible with string-key migrations too.
        let key = uuid::Uuid::new_v4().to_string();
        // For option<T> fields, omit when None so Surreal gets NONE (not NULL) under SCHEMAFULL.
        let mut doc = serde_json::Map::new();
        doc.insert("name".to_string(), serde_json::Value::String(game.name));
        if let Some(v) = game.year_published {
            doc.insert("year_published".to_string(), serde_json::Value::from(v));
        }
        if let Some(v) = game.bgg_id {
            doc.insert("bgg_id".to_string(), serde_json::Value::from(v));
        }
        if let Some(v) = game.description {
            doc.insert("description".to_string(), serde_json::Value::String(v));
        }
        let doc = serde_json::Value::Object(doc);
        self.ensure_scope().await;
        let mut res = self
            .db
            .query(self.query_with_scope("CREATE type::record('game', $key) CONTENT $doc RETURN AFTER"))
            .bind(("key", key.clone()))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to create game: {}", e))?;
        let rows: Vec<serde_json::Value> = res
            .take(self.scope_result_index())
            .map_err(|e| format!("Failed to parse created game: {}", e))?;
        let created_game = rows
            .into_iter()
            .next()
            .and_then(|v| value_to_game(&v))
            .ok_or_else(|| "Game CREATE returned no record".to_string())?;
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::game(&created_game.id)).await;
            let _ = cache.delete(&CacheKeys::game_list()).await;
            let _ = cache.invalidate_pattern("games:search:").await;
        }
        Ok(created_game)
    }

    async fn update(&self, game: Game) -> Result<Game, String> {
        let game = game;
        let key = game.id.trim_start_matches("game/").trim_start_matches("game:").to_string();
        self.ensure_scope().await;
        // Omit option fields when None (avoid NULL under SCHEMAFULL option<T>).
        let mut doc = serde_json::Map::new();
        doc.insert("name".to_string(), serde_json::Value::String(game.name.clone()));
        if let Some(v) = game.year_published {
            doc.insert("year_published".to_string(), serde_json::Value::from(v));
        }
        if let Some(v) = game.bgg_id {
            doc.insert("bgg_id".to_string(), serde_json::Value::from(v));
        }
        if let Some(ref v) = game.description {
            doc.insert("description".to_string(), serde_json::Value::String(v.clone()));
        }
        let doc = serde_json::Value::Object(doc);
        self.update_game_doc_by_key(&key, &doc).await?;
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::game(&game.id)).await;
            let _ = cache.delete(&CacheKeys::game_list()).await;
            let _ = cache.invalidate_pattern("games:search:").await;
        }
        Ok(game)
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        let key = id.trim_start_matches("game/").trim_start_matches("game:").to_string();
        self.ensure_scope().await;
        self.delete_game_by_key(&key).await?;
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::game(id)).await;
            let _ = cache.delete(&CacheKeys::game_list()).await;
            let _ = cache.invalidate_pattern("games:search:").await;
        }
        Ok(())
    }
}
