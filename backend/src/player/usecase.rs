use crate::player::error::PlayerError;
use crate::player::repository::PlayerRepository;
use argon2::{Argon2, PasswordHasher};
use chrono::Utc;
use shared::dto::player::CreatePlayerRequest;
use shared::models::player::Player;
use shared::models::player::PlayerLogin;

#[async_trait::async_trait]
pub trait PlayerUseCase: Send + Sync {
    async fn login(&self, login: PlayerLogin) -> Result<Player, PlayerError>;
    async fn get_player(&self, id: &str) -> Result<Player, String>;
    async fn register(&self, registration: CreatePlayerRequest) -> Result<Player, PlayerError>;
    async fn update_email(
        &self,
        email: &str,
        new_email: &str,
        password: &str,
    ) -> Result<Player, PlayerError>;
    async fn update_handle(
        &self,
        email: &str,
        new_handle: &str,
        password: &str,
    ) -> Result<Player, PlayerError>;
    async fn update_password(
        &self,
        email: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<Player, PlayerError>;
}

pub struct PlayerUseCaseImpl<R: PlayerRepository> {
    pub repo: R,
}

#[async_trait::async_trait]
impl<R: PlayerRepository> PlayerUseCase for PlayerUseCaseImpl<R> {
    async fn login(&self, login: PlayerLogin) -> Result<Player, PlayerError> {
        if let Some(player) = self.repo.find_by_email(&login.email).await {
            if player.verify_password(&login.password) {
                Ok(player)
            } else {
                Err(PlayerError::InvalidPassword)
            }
        } else {
            Err(PlayerError::NotFound)
        }
    }

    async fn get_player(&self, id: &str) -> Result<Player, String> {
        // Try to find by ID first (database ID)
        if let Some(player) = self.repo.find_by_id(id).await {
            return Ok(player);
        }

        // Fallback to finding by email
        self.repo
            .find_by_email(id)
            .await
            .ok_or_else(|| "Player not found".to_string())
    }

    async fn register(&self, registration: CreatePlayerRequest) -> Result<Player, PlayerError> {
        // Check if player already exists
        if let Some(_existing_player) = self.repo.find_by_email(&registration.email).await {
            return Err(PlayerError::AlreadyExists);
        }

        // Hash the password
        let salt_string = argon2::password_hash::SaltString::generate(
            &mut argon2::password_hash::rand_core::OsRng,
        );
        let salt = Argon2::default()
            .hash_password(registration.password.as_bytes(), &salt_string)
            .map_err(|e| PlayerError::DatabaseError(format!("Failed to hash password: {}", e)))?;

        let hashed_password = salt.to_string();

        // Create new player
        let player = Player::new_for_db(
            registration.username.clone(),
            registration.username.clone(), // Use username as handle for now
            registration.email.clone(),
            hashed_password,
            Utc::now().fixed_offset(),
            false,
        )
        .map_err(|e| PlayerError::DatabaseError(format!("Failed to create player: {}", e)))?;

        // Save to database
        self.repo
            .create(player)
            .await
            .map_err(|e| PlayerError::DatabaseError(e))
    }

    async fn update_email(
        &self,
        email: &str,
        new_email: &str,
        password: &str,
    ) -> Result<Player, PlayerError> {
        // Find the player by current email
        let mut player = self
            .repo
            .find_by_email(email)
            .await
            .ok_or(PlayerError::NotFound)?;

        // Verify current password
        if !player.verify_password(password) {
            return Err(PlayerError::InvalidPassword);
        }

        // Check if new email already exists
        if let Some(_existing_player) = self.repo.find_by_email(new_email).await {
            return Err(PlayerError::AlreadyExists);
        }

        // Update email
        player.email = new_email.to_string();

        // Save to database
        self.repo
            .update(player)
            .await
            .map_err(|e| PlayerError::DatabaseError(e))
    }

    async fn update_handle(
        &self,
        email: &str,
        new_handle: &str,
        password: &str,
    ) -> Result<Player, PlayerError> {
        // Find the player by email
        let mut player = self
            .repo
            .find_by_email(email)
            .await
            .ok_or(PlayerError::NotFound)?;

        // Verify current password
        if !player.verify_password(password) {
            return Err(PlayerError::InvalidPassword);
        }

        // Check if new handle already exists
        if let Some(_existing_player) = self.repo.find_by_handle(new_handle).await {
            return Err(PlayerError::AlreadyExists);
        }

        // Update handle
        player.handle = new_handle.to_string();

        // Save to database
        self.repo
            .update(player)
            .await
            .map_err(|e| PlayerError::DatabaseError(e))
    }

    async fn update_password(
        &self,
        email: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<Player, PlayerError> {
        // Find the player by email
        let mut player = self
            .repo
            .find_by_email(email)
            .await
            .ok_or(PlayerError::NotFound)?;

        // Verify current password
        if !player.verify_password(current_password) {
            return Err(PlayerError::InvalidPassword);
        }

        // Hash the new password
        let salt_string = argon2::password_hash::SaltString::generate(
            &mut argon2::password_hash::rand_core::OsRng,
        );
        let salt = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt_string)
            .map_err(|e| PlayerError::DatabaseError(format!("Failed to hash password: {}", e)))?;

        // Update password
        player.password = salt.to_string();

        // Save to database
        self.repo
            .update(player)
            .await
            .map_err(|e| PlayerError::DatabaseError(e))
    }
}
