//! Integration testing against the same docker stack (SurrealDB + Redis).
//! Start stack: ./deploy/stack.sh start

pub mod app_setup;
pub mod env;

pub use env::TestEnvironment;

use anyhow::Result;
use std::time::Duration;

/// Create test env and wait for ready with timeout.
pub async fn create_test_env_with_timeout() -> Result<TestEnvironment> {
    let env = tokio::time::timeout(Duration::from_secs(30), TestEnvironment::new())
        .await
        .map_err(|_| anyhow::anyhow!("Test environment setup timed out"))??;
    tokio::time::timeout(Duration::from_secs(60), env.wait_for_ready())
        .await
        .map_err(|_| anyhow::anyhow!("wait_for_ready timed out"))??;
    Ok(env)
}

/// Builder for test env (no dump loading; same stack only).
pub struct TestEnvironmentBuilder;

impl TestEnvironmentBuilder {
    pub fn new() -> Self {
        Self
    }
    pub fn with_data_dump(self, _path: &str) -> Self {
        self
    }
    pub fn with_database_name(self, _name: &str) -> Self {
        self
    }
    pub fn skip_data_load_if_missing(self) -> Self {
        self
    }
    pub fn with_default_data_dump(self) -> Self {
        self
    }
    pub async fn build(self) -> Result<TestEnvironment> {
        TestEnvironment::new().await
    }
}

impl Default for TestEnvironmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn test_env_with_prod_data() -> Result<TestEnvironment> {
    TestEnvironment::new().await
}

pub async fn test_env_with_dump(_dump_path: &str) -> Result<TestEnvironment> {
    TestEnvironment::new().await
}

pub async fn test_env_with_prod_data_and_db(_db_name: &str) -> Result<TestEnvironment> {
    TestEnvironment::new().await
}

#[macro_export]
macro_rules! create_authenticated_user {
    ($app:expr, $email:expr, $username:expr) => {{
        let register_req = actix_web::test::TestRequest::post()
            .uri("/api/players/register")
            .set_json(&serde_json::json!({
                "username": $username,
                "email": $email,
                "password": "password123"
            }))
            .to_request();
        let register_resp = actix_web::test::call_service(&$app, register_req).await;
        assert!(register_resp.status().is_success(), "User registration should succeed");
        let mut session_id = String::new();
        for _ in 0..5 {
            let login_req = actix_web::test::TestRequest::post()
                .uri("/api/players/login")
                .set_json(&serde_json::json!({
                    "email": $email,
                    "password": "password123"
                }))
                .to_request();
            let login_resp = actix_web::test::call_service(&$app, login_req).await;
            if login_resp.status().is_success() {
                let login_body: shared::dto::player::LoginResponse =
                    actix_web::test::read_body_json(login_resp).await;
                session_id = login_body.session_id;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(!session_id.is_empty(), "User login should succeed");
        session_id
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_environment_creation() {
        let env = TestEnvironment::new().await.unwrap();
        assert!(!env.surrealdb_url().is_empty());
        assert!(!env.redis_url().is_empty());
    }
}
