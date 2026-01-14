use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn set_session(&self, session_id: &str, email: &str) -> Result<(), String>;
    async fn get_session(&self, session_id: &str) -> Result<Option<String>, String>;
    async fn delete_session(&self, session_id: &str) -> Result<(), String>;
}

#[derive(Clone)]
pub struct RedisSessionStore {
    pub client: redis::Client,
}

#[async_trait]
impl SessionStore for RedisSessionStore {
    async fn set_session(&self, session_id: &str, email: &str) -> Result<(), String> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| e.to_string())?;
        redis::cmd("SETEX")
            .arg(session_id)
            .arg(3600)
            .arg(email)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<String>, String> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| e.to_string())?;
        let result: Result<Option<String>, redis::RedisError> = redis::cmd("GET")
            .arg(session_id)
            .query_async(&mut conn)
            .await;

        match result {
            Ok(Some(email)) => Ok(Some(email)),
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| e.to_string())?;
        redis::cmd("DEL")
            .arg(session_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())
    }
}

#[derive(Clone)]
pub struct MockSessionStore {
    pub sessions: Arc<Mutex<HashMap<String, String>>>,
}

impl MockSessionStore {
    pub fn _new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SessionStore for MockSessionStore {
    async fn set_session(&self, session_id: &str, email: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.to_string(), email.to_string());
        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<String>, String> {
        let sessions = self.sessions.lock().await;
        Ok(sessions.get(session_id).cloned())
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_session_store_creation() {
        let _store = MockSessionStore::_new();
        assert!(true); // Store created successfully
    }

    #[tokio::test]
    async fn test_mock_session_store_set_session() {
        let store = MockSessionStore::_new();
        let result = store.set_session("test_session", "test@example.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_session_store_get_session() {
        let store = MockSessionStore::_new();

        // Set a session first
        store
            .set_session("test_session", "test@example.com")
            .await
            .unwrap();

        // Get the session
        let email = store.get_session("test_session").await.unwrap();
        assert_eq!(email, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_mock_session_store_get_nonexistent_session() {
        let store = MockSessionStore::_new();
        let email = store.get_session("nonexistent_session").await.unwrap();
        assert_eq!(email, None);
    }

    #[tokio::test]
    async fn test_mock_session_store_delete_session() {
        let store = MockSessionStore::_new();

        // Set a session first
        store
            .set_session("test_session", "test@example.com")
            .await
            .unwrap();

        // Verify it exists
        let email = store.get_session("test_session").await.unwrap();
        assert_eq!(email, Some("test@example.com".to_string()));

        // Delete the session
        let result = store.delete_session("test_session").await;
        assert!(result.is_ok());

        // Verify it's gone
        let email = store.get_session("test_session").await.unwrap();
        assert_eq!(email, None);
    }

    #[tokio::test]
    async fn test_mock_session_store_delete_nonexistent_session() {
        let store = MockSessionStore::_new();
        let result = store.delete_session("nonexistent_session").await;
        assert!(result.is_ok()); // Should not error
    }

    #[tokio::test]
    async fn test_mock_session_store_multiple_sessions() {
        let store = MockSessionStore::_new();

        // Set multiple sessions
        store
            .set_session("session1", "user1@example.com")
            .await
            .unwrap();
        store
            .set_session("session2", "user2@example.com")
            .await
            .unwrap();
        store
            .set_session("session3", "user3@example.com")
            .await
            .unwrap();

        // Verify all sessions exist
        assert_eq!(
            store.get_session("session1").await.unwrap(),
            Some("user1@example.com".to_string())
        );
        assert_eq!(
            store.get_session("session2").await.unwrap(),
            Some("user2@example.com".to_string())
        );
        assert_eq!(
            store.get_session("session3").await.unwrap(),
            Some("user3@example.com".to_string())
        );

        // Delete one session
        store.delete_session("session2").await.unwrap();

        // Verify only that one is gone
        assert_eq!(
            store.get_session("session1").await.unwrap(),
            Some("user1@example.com".to_string())
        );
        assert_eq!(store.get_session("session2").await.unwrap(), None);
        assert_eq!(
            store.get_session("session3").await.unwrap(),
            Some("user3@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_session_store_session_overwrite() {
        let store = MockSessionStore::_new();

        // Set initial session
        store
            .set_session("test_session", "user1@example.com")
            .await
            .unwrap();
        assert_eq!(
            store.get_session("test_session").await.unwrap(),
            Some("user1@example.com".to_string())
        );

        // Overwrite with new email
        store
            .set_session("test_session", "user2@example.com")
            .await
            .unwrap();
        assert_eq!(
            store.get_session("test_session").await.unwrap(),
            Some("user2@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_session_store_empty_session_id() {
        let store = MockSessionStore::_new();

        // Test with empty session ID
        let result = store.set_session("", "test@example.com").await;
        assert!(result.is_ok());

        let email = store.get_session("").await.unwrap();
        assert_eq!(email, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_mock_session_store_empty_email() {
        let store = MockSessionStore::_new();

        // Test with empty email
        let result = store.set_session("test_session", "").await;
        assert!(result.is_ok());

        let email = store.get_session("test_session").await.unwrap();
        assert_eq!(email, Some("".to_string()));
    }

    #[tokio::test]
    async fn test_mock_session_store_special_characters() {
        let store = MockSessionStore::_new();

        // Test with special characters in session ID
        let session_id = "session_with_special_chars_!@#$%^&*()";
        let email = "test@example.com";

        store.set_session(session_id, email).await.unwrap();
        assert_eq!(
            store.get_session(session_id).await.unwrap(),
            Some(email.to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_session_store_unicode_characters() {
        let store = MockSessionStore::_new();

        // Test with Unicode characters
        let session_id = "session_with_unicode_🚀🎮";
        let email = "test@example.com";

        store.set_session(session_id, email).await.unwrap();
        assert_eq!(
            store.get_session(session_id).await.unwrap(),
            Some(email.to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_session_store_concurrent_access() {
        let store = MockSessionStore::_new();
        let store_clone = store.clone();
        let store_clone2 = store.clone();

        // Simulate concurrent access
        let handle1 = tokio::spawn(async move {
            store_clone
                .set_session("session1", "user1@example.com")
                .await
                .unwrap();
            store_clone.get_session("session1").await.unwrap()
        });

        let handle2 = tokio::spawn(async move {
            store_clone2
                .set_session("session2", "user2@example.com")
                .await
                .unwrap();
            store_clone2.get_session("session2").await.unwrap()
        });

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert_eq!(result1, Some("user1@example.com".to_string()));
        assert_eq!(result2, Some("user2@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_mock_session_store_session_id_format() {
        let store = MockSessionStore::_new();

        // Test various session ID formats
        let test_cases = vec![
            "simple_session",
            "session_with_underscores",
            "session-with-dashes",
            "session123",
            "SESSION_UPPERCASE",
            "session.with.dots",
        ];

        for session_id in test_cases {
            let email = "test@example.com";
            store.set_session(session_id, email).await.unwrap();
            assert_eq!(
                store.get_session(session_id).await.unwrap(),
                Some(email.to_string())
            );
        }
    }

    #[tokio::test]
    async fn test_mock_session_store_email_validation() {
        let store = MockSessionStore::_new();

        // Test various email formats
        let test_emails = vec![
            "simple@example.com",
            "user.name@example.com",
            "user+tag@example.com",
            "user@subdomain.example.com",
            "user123@example.co.uk",
        ];

        for email in test_emails {
            let session_id = format!("session_{}", email.replace("@", "_at_"));
            store.set_session(&session_id, email).await.unwrap();
            assert_eq!(
                store.get_session(&session_id).await.unwrap(),
                Some(email.to_string())
            );
        }
    }

    #[tokio::test]
    async fn test_mock_session_store_cleanup_after_delete() {
        let store = MockSessionStore::_new();

        // Add multiple sessions
        for i in 1..=10 {
            let session_id = format!("session_{}", i);
            let email = format!("user{}@example.com", i);
            store.set_session(&session_id, &email).await.unwrap();
        }

        // Delete some sessions
        for i in [1, 3, 5, 7, 9].iter() {
            let session_id = format!("session_{}", i);
            store.delete_session(&session_id).await.unwrap();
        }

        // Verify remaining sessions
        for i in 1..=10 {
            let session_id = format!("session_{}", i);
            let expected_email = if [1, 3, 5, 7, 9].contains(&i) {
                None
            } else {
                Some(format!("user{}@example.com", i))
            };
            assert_eq!(
                store.get_session(&session_id).await.unwrap(),
                expected_email
            );
        }
    }
}
