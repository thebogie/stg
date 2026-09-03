use crate::player::error::PlayerError;
use crate::player::repository::PlayerRepository;
use argon2::{Argon2, PasswordHasher};
use chrono::Utc;
use shared::dto::player::{AdminCreatePlayerRequest, AdminUpdatePlayerRequest, CreatePlayerRequest};
use validator::Validate;
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
    async fn admin_update_player(
        &self,
        player_id: &str,
        request: AdminUpdatePlayerRequest,
    ) -> Result<Player, PlayerError>;
    async fn admin_reset_password(
        &self,
        player_id: &str,
        new_password: &str,
    ) -> Result<Player, PlayerError>;
    async fn admin_create_player(
        &self,
        request: AdminCreatePlayerRequest,
    ) -> Result<Player, PlayerError>;
    async fn admin_delete_player(&self, player_id: &str) -> Result<(), PlayerError>;
    async fn admin_set_active(&self, player_id: &str, is_active: bool) -> Result<Player, PlayerError>;
}

pub struct PlayerUseCaseImpl<R: PlayerRepository> {
    pub repo: R,
}

#[async_trait::async_trait]
impl<R: PlayerRepository> PlayerUseCase for PlayerUseCaseImpl<R> {
    async fn login(&self, login: PlayerLogin) -> Result<Player, PlayerError> {
        if let Some(player) = self.repo.find_by_email(&login.email).await {
            if !player.is_active {
                return Err(PlayerError::AccountDisabled);
            }
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
        let created = self
            .repo
            .create(player)
            .await
            .map_err(PlayerError::DatabaseError)?;

        // SurrealDB over remote WS can exhibit brief read-after-write delay. Ensure the player is
        // queryable by email before returning 201 so subsequent login/duplicate checks are stable.
        // Keep the delay short so this doesn't slow down normal operation materially.
        for _ in 0..10 {
            if let Some(p) = self.repo.find_by_email(&created.email).await {
                return Ok(p);
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        Ok(created)
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
            .map_err(PlayerError::DatabaseError)
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
        if let Some(existing_player) = self.repo.find_by_handle(new_handle).await {
            // Allow no-op updates (setting handle to its current value).
            // Also allow if the "existing" record is the same player.
            if existing_player.id != player.id {
                return Err(PlayerError::AlreadyExists);
            }
        }

        // Update handle
        player.handle = new_handle.to_string();

        // Save to database
        self.repo
            .update(player)
            .await
            .map_err(PlayerError::DatabaseError)
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
            .map_err(PlayerError::DatabaseError)
    }

    async fn admin_update_player(
        &self,
        player_id: &str,
        request: AdminUpdatePlayerRequest,
    ) -> Result<Player, PlayerError> {
        request
            .validate()
            .map_err(|e| PlayerError::ValidationError(e.to_string()))?;

        if request.firstname.is_none()
            && request.handle.is_none()
            && request.email.is_none()
            && request.is_admin.is_none()
        {
            return Err(PlayerError::ValidationError(
                "At least one field must be provided".to_string(),
            ));
        }

        let mut player = self
            .repo
            .find_by_id(player_id)
            .await
            .ok_or(PlayerError::NotFound)?;

        if let Some(ref firstname) = request.firstname {
            player.firstname = firstname.clone();
        }

        if let Some(ref new_handle) = request.handle {
            if let Some(existing_player) = self.repo.find_by_handle(new_handle).await {
                if existing_player.id != player.id {
                    return Err(PlayerError::AlreadyExists);
                }
            }
            player.handle = new_handle.clone();
        }

        if let Some(ref new_email) = request.email {
            if let Some(existing_player) = self.repo.find_by_email(new_email).await {
                if existing_player.id != player.id {
                    return Err(PlayerError::AlreadyExists);
                }
            }
            player.email = new_email.clone();
        }

        if let Some(is_admin) = request.is_admin {
            player.is_admin = is_admin;
        }

        self.repo
            .update(player)
            .await
            .map_err(PlayerError::DatabaseError)
    }

    async fn admin_reset_password(
        &self,
        player_id: &str,
        new_password: &str,
    ) -> Result<Player, PlayerError> {
        if new_password.len() < 8 {
            return Err(PlayerError::ValidationError(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        let mut player = self
            .repo
            .find_by_id(player_id)
            .await
            .ok_or(PlayerError::NotFound)?;

        let salt_string = argon2::password_hash::SaltString::generate(
            &mut argon2::password_hash::rand_core::OsRng,
        );
        let salt = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt_string)
            .map_err(|e| PlayerError::DatabaseError(format!("Failed to hash password: {}", e)))?;

        player.password = salt.to_string();

        // Note: existing Redis sessions remain valid until expiry.
        self.repo
            .update(player)
            .await
            .map_err(PlayerError::DatabaseError)
    }

    async fn admin_create_player(
        &self,
        request: AdminCreatePlayerRequest,
    ) -> Result<Player, PlayerError> {
        request
            .validate()
            .map_err(|e| PlayerError::ValidationError(e.to_string()))?;

        if self.repo.find_by_email(&request.email).await.is_some() {
            return Err(PlayerError::AlreadyExists);
        }
        if let Some(_existing) = self.repo.find_by_handle(&request.handle).await {
            return Err(PlayerError::AlreadyExists);
        }

        let salt_string = argon2::password_hash::SaltString::generate(
            &mut argon2::password_hash::rand_core::OsRng,
        );
        let salt = Argon2::default()
            .hash_password(request.password.as_bytes(), &salt_string)
            .map_err(|e| PlayerError::DatabaseError(format!("Failed to hash password: {}", e)))?;

        let player = Player::new_for_db(
            request.firstname.clone(),
            request.handle.clone(),
            request.email.clone(),
            salt.to_string(),
            Utc::now().fixed_offset(),
            request.is_admin,
        )
        .map_err(|e| PlayerError::DatabaseError(format!("Failed to create player: {}", e)))?;

        let created = self
            .repo
            .create(player)
            .await
            .map_err(PlayerError::DatabaseError)?;

        for _ in 0..10 {
            if let Some(p) = self.repo.find_by_email(&created.email).await {
                return Ok(p);
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        Ok(created)
    }

    async fn admin_delete_player(&self, player_id: &str) -> Result<(), PlayerError> {
        if self.repo.find_by_id(player_id).await.is_none() {
            return Err(PlayerError::NotFound);
        }

        let contest_count = self
            .repo
            .count_contests_as_creator(player_id)
            .await
            .map_err(PlayerError::DatabaseError)?;
        if contest_count > 0 {
            return Err(PlayerError::ValidationError(format!(
                "Cannot delete player: created {} contest(s). Reassign or delete those contests first.",
                contest_count
            )));
        }

        self.repo
            .delete_player(player_id)
            .await
            .map_err(PlayerError::DatabaseError)
    }

    async fn admin_set_active(
        &self,
        player_id: &str,
        is_active: bool,
    ) -> Result<Player, PlayerError> {
        if self.repo.find_by_id(player_id).await.is_none() {
            return Err(PlayerError::NotFound);
        }
        self.repo
            .set_active_status(player_id, is_active)
            .await
            .map_err(PlayerError::DatabaseError)
    }
}
