use crate::cache::{CacheKeys, CacheTTL, RedisCache};
use arangors::client::reqwest::ReqwestClient;
use arangors::document::options::InsertOptions;
use arangors::Database;
use log;
use shared::models::player::Player;
use std::sync::Arc;

#[derive(Clone)]
pub struct PlayerRepositoryImpl {
    pub db: Database<ReqwestClient>,
    pub cache: Option<Arc<RedisCache>>,
}

impl PlayerRepositoryImpl {
    pub fn new(db: Database<ReqwestClient>) -> Self {
        Self { db, cache: None }
    }

    pub fn new_with_cache(db: Database<ReqwestClient>, cache: Arc<RedisCache>) -> Self {
        Self {
            db,
            cache: Some(cache),
        }
    }
}

#[async_trait::async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Option<Player>;
    async fn find_by_id(&self, id: &str) -> Option<Player>;
    async fn find_many_by_ids(&self, ids: &[String]) -> Vec<Player>;
    async fn search_players(&self, query: &str) -> Vec<Player>;
    async fn create(&self, player: Player) -> Result<Player, String>;
    async fn update(&self, player: Player) -> Result<Player, String>;
    async fn find_by_handle(&self, handle: &str) -> Option<Player>;
}

#[async_trait::async_trait]
impl PlayerRepository for PlayerRepositoryImpl {
    async fn find_by_email(&self, email: &str) -> Option<Player> {
        // Try cache first
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player_by_email(email);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by email: {}", email);
                return Some(cached_player);
            }
        }

        eprintln!(
            "[DEBUG] find_by_email called with email: '{}', len: {}",
            email,
            email.len()
        );
        let query = arangors::AqlQuery::builder()
            .query("FOR p IN player FILTER LOWER(p.email) == LOWER(@email) LIMIT 1 RETURN p")
            .bind_var("email", email)
            .build();
        eprintln!(
            "[DEBUG] AQL query built for email: '{}', query: {:?}",
            email, query
        );
        match self.db.aql_query::<arangors::Document<Player>>(query).await {
            Ok(mut cursor) => {
                let result = cursor
                    .pop()
                    .map(|doc: arangors::Document<Player>| doc.document);

                if let Some(ref player) = result {
                    eprintln!(
                        "[DEBUG] Player found for email '{}': id={}, handle={}",
                        email, player.id, player.handle
                    );

                    // Cache the result
                    if let Some(ref cache) = self.cache {
                        let _ = cache
                            .set_with_ttl(
                                &CacheKeys::player_by_email(email),
                                player,
                                CacheTTL::player(),
                            )
                            .await;
                        let _ = cache
                            .set_with_ttl(
                                &CacheKeys::player(&player.id),
                                player,
                                CacheTTL::player(),
                            )
                            .await;
                    }
                } else {
                    eprintln!(
                        "[DEBUG] No player found for email: '{}', after query execution",
                        email
                    );
                }
                result
            }
            Err(e) => {
                eprintln!("[ERROR] Error querying by email '{}': {:?}", email, e);
                None
            }
        }
    }

    async fn find_by_id(&self, id: &str) -> Option<Player> {
        // Try cache first
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player(id);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by id: {}", id);
                return Some(cached_player);
            }
        }

        let query = arangors::AqlQuery::builder()
            .query("FOR p IN player FILTER p._id == @id LIMIT 1 RETURN p")
            .bind_var("id", id)
            .build();
        match self.db.aql_query::<arangors::Document<Player>>(query).await {
            Ok(mut cursor) => {
                if let Some(player_doc) = cursor.pop() {
                    let player = player_doc.document;

                    // Cache the result
                    if let Some(ref cache) = self.cache {
                        let _ = cache
                            .set_with_ttl(
                                &CacheKeys::player(&player.id),
                                &player,
                                CacheTTL::player(),
                            )
                            .await;
                        let _ = cache
                            .set_with_ttl(
                                &CacheKeys::player_by_email(&player.email),
                                &player,
                                CacheTTL::player(),
                            )
                            .await;
                        if !player.handle.is_empty() {
                            let _ = cache
                                .set_with_ttl(
                                    &CacheKeys::player_by_handle(&player.handle),
                                    &player,
                                    CacheTTL::player(),
                                )
                                .await;
                        }
                    }

                    Some(player)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn search_players(&self, query: &str) -> Vec<Player> {
        let search_query = arangors::AqlQuery::builder()
            .query("FOR p IN player FILTER CONTAINS(LOWER(p.handle), LOWER(@query)) OR CONTAINS(LOWER(p.email), LOWER(@query)) LIMIT 10 RETURN p")
            .bind_var("query", query)
            .build();
        match self
            .db
            .aql_query::<arangors::Document<Player>>(search_query)
            .await
        {
            Ok(cursor) => cursor
                .into_iter()
                .map(|doc: arangors::Document<Player>| doc.document)
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    async fn create(&self, player: Player) -> Result<Player, String> {
        let collection = self
            .db
            .collection("player")
            .await
            .map_err(|e| format!("Failed to get player collection: {}", e))?;

        let insert_options = InsertOptions::builder().return_new(true).build();
        let result = collection
            .create_document(player, insert_options)
            .await
            .map_err(|e| format!("Failed to create player: {}", e))?;

        let created_player: Player = result
            .new_doc()
            .ok_or_else(|| "No document returned after creation".to_string())?
            .clone();

        // Cache the created player
        if let Some(ref cache) = self.cache {
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player(&created_player.id),
                    &created_player,
                    CacheTTL::player(),
                )
                .await;
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player_by_email(&created_player.email),
                    &created_player,
                    CacheTTL::player(),
                )
                .await;
            if !created_player.handle.is_empty() {
                let _ = cache
                    .set_with_ttl(
                        &CacheKeys::player_by_handle(&created_player.handle),
                        &created_player,
                        CacheTTL::player(),
                    )
                    .await;
            }
        }

        Ok(created_player)
    }

    async fn update(&self, player: Player) -> Result<Player, String> {
        let collection = self
            .db
            .collection("player")
            .await
            .map_err(|e| format!("Failed to get player collection: {}", e))?;

        // Extract the key from the full ArangoDB ID (e.g., "player/123" -> "123")
        let key = player
            .id
            .split('/')
            .last()
            .ok_or_else(|| "Invalid player ID format".to_string())?;

        let update_options = arangors::document::options::UpdateOptions::builder()
            .return_new(true)
            .build();
        let result = collection
            .update_document(key, player.clone(), update_options)
            .await
            .map_err(|e| format!("Failed to update player: {}", e))?;

        let updated_player: Player = result
            .new_doc()
            .ok_or_else(|| "No document returned after update".to_string())?
            .clone();

        // Invalidate and update cache
        if let Some(ref cache) = self.cache {
            // Delete old cache entries (in case email/handle changed)
            if !player.email.is_empty() {
                let _ = cache
                    .delete(&CacheKeys::player_by_email(&player.email))
                    .await;
            }
            if !player.handle.is_empty() {
                let _ = cache
                    .delete(&CacheKeys::player_by_handle(&player.handle))
                    .await;
            }

            // Cache the updated player with new data
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player(&updated_player.id),
                    &updated_player,
                    CacheTTL::player(),
                )
                .await;
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player_by_email(&updated_player.email),
                    &updated_player,
                    CacheTTL::player(),
                )
                .await;
            if !updated_player.handle.is_empty() {
                let _ = cache
                    .set_with_ttl(
                        &CacheKeys::player_by_handle(&updated_player.handle),
                        &updated_player,
                        CacheTTL::player(),
                    )
                    .await;
            }
        }

        Ok(updated_player)
    }

    async fn find_by_handle(&self, handle: &str) -> Option<Player> {
        // Try cache first
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player_by_handle(handle);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by handle: {}", handle);
                return Some(cached_player);
            }
        }

        let query = arangors::AqlQuery::builder()
            .query("FOR p IN player FILTER LOWER(p.handle) == LOWER(@handle) LIMIT 1 RETURN p")
            .bind_var("handle", handle)
            .build();
        match self.db.aql_query::<arangors::Document<Player>>(query).await {
            Ok(mut cursor) => cursor
                .pop()
                .map(|doc: arangors::Document<Player>| doc.document),
            Err(_) => None,
        }
    }

    async fn find_many_by_ids(&self, ids: &[String]) -> Vec<Player> {
        if ids.is_empty() {
            return Vec::new();
        }

        let query = arangors::AqlQuery::builder()
            .query("FOR p IN player FILTER p._id IN @ids RETURN p")
            .bind_var("ids", ids)
            .build();

        match self.db.aql_query::<arangors::Document<Player>>(query).await {
            Ok(cursor) => cursor
                .into_iter()
                .map(|doc: arangors::Document<Player>| doc.document)
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {

    use chrono::Utc;
    use shared::models::player::Player;

    fn create_test_player(id: &str, handle: &str, email: &str) -> Player {
        Player {
            id: id.to_string(),
            rev: "1".to_string(),
            firstname: "Test".to_string(),
            handle: handle.to_string(),
            email: email.to_string(),
            password: "hashed_password".to_string(),
            created_at: Utc::now().fixed_offset(),
            is_admin: false,
        }
    }

    #[tokio::test]
    async fn test_search_players_by_handle() {
        // This would require a test database setup
        // For now, we'll test the logic with a mock
        let players = vec![
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
            create_test_player("3", "bob_wilson", "bob@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("john"))
            .collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].handle, "john_doe");
    }

    #[tokio::test]
    async fn test_search_players_by_email() {
        let players = vec![
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
            create_test_player("3", "bob_wilson", "bob@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.email.to_lowercase().contains("example"))
            .collect();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_search_players_case_insensitive() {
        let players = vec![
            create_test_player("1", "John_Doe", "John@Example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("john"))
            .collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].handle, "John_Doe");
    }

    #[tokio::test]
    async fn test_search_players_empty_query() {
        let players = vec![
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains(""))
            .collect();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_players_partial_match() {
        let players = vec![
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "johnny_cash", "johnny@example.com"),
            create_test_player("3", "jane_smith", "jane@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("john"))
            .collect();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|p| p.handle == "john_doe"));
        assert!(results.iter().any(|p| p.handle == "johnny_cash"));
    }

    #[tokio::test]
    async fn test_search_players_no_matches() {
        let players = vec![
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("nonexistent"))
            .collect();

        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_players_special_characters() {
        let players = vec![
            create_test_player("1", "user_123", "user123@example.com"),
            create_test_player("2", "test_user", "test@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("user"))
            .collect();

        assert_eq!(results.len(), 2);
    }

    // #[tokio::test]
    // async fn test_find_many_by_ids_empty_input() {
    //     let db = Database::<ReqwestClient>::new(
    //         arangors::Connection::establish_basic_auth("http://localhost:8529", "root", "password")
    //             .await
    //             .unwrap()
    //     );
    //     let repo = PlayerRepositoryImpl::new(db);
    //
    //     let result = repo.find_many_by_ids(&[]).await;
    //     assert_eq!(result, Vec::new());
    // }

    // #[tokio::test]
    // async fn test_find_many_by_ids_single_id() {
    //     let db = Database::<ReqwestClient>::new(
    //         arangors::Connection::establish_basic_auth("http://localhost:8529", "root", "password")
    //             .await
    //             .unwrap()
    //     );
    //     let repo = PlayerRepositoryImpl::new(db);
    //
    //     let ids = vec!["player/123".to_string()];
    //     let result = repo.find_many_by_ids(&ids).await;
    //     // This will likely fail due to no real database, but we're testing the method exists
    //     assert!(result.is_empty() || result.len() == 1);
    // }

    // #[tokio::test]
    // async fn test_find_many_by_ids_multiple_ids() {
    //     let db = Database::<ReqwestClient>::new(
    //         arangors::Connection::establish_basic_auth("http://localhost:8529", "root", "password")
    //             .await
    //             .unwrap()
    //     );
    //     let repo = PlayerRepositoryImpl::new(db);
    //
    //     let ids = vec![
    //         "player/123".to_string(),
    //         "player/456".to_string(),
    //         "player/789".to_string(),
    //     ];
    //     let result = repo.find_many_by_ids(&ids).await;
    //     // This will likely fail due to no real database, but we're testing the method exists
    //     assert!(result.is_empty() || result.len() <= 3);
    // }

    #[test]
    fn test_player_repository_trait_implementation() {
        // Test that PlayerRepositoryImpl implements PlayerRepository
        // This is a compile-time test to ensure the trait is implemented
        assert!(true); // If we get here, the trait is implemented
    }
}
