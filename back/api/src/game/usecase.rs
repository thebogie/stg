use crate::game::repository::GameRepository;
use shared::dto::game::GameDto;
use shared::models::game::Game;
use validator::Validate;

#[async_trait::async_trait]
pub trait GameUseCase: Send + Sync {
    async fn get_game(&self, id: &str) -> Result<Game, String>;
    async fn get_all_games(&self) -> Result<Vec<Game>, String>;
    async fn search_games(&self, query: &str) -> Result<Vec<Game>, String>;
    async fn search_games_dto(&self, query: &str) -> Result<Vec<GameDto>, String>;
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
    async fn create_game(&self, game_dto: GameDto) -> Result<Game, String>;
    async fn update_game(&self, id: &str, game_dto: GameDto) -> Result<Game, String>;
    async fn delete_game(&self, id: &str) -> Result<(), String>;
}

pub struct GameUseCaseImpl<R: GameRepository> {
    pub repo: R,
}

#[async_trait::async_trait]
impl<R: GameRepository> GameUseCase for GameUseCaseImpl<R> {
    async fn get_game(&self, id: &str) -> Result<Game, String> {
        self.repo
            .find_by_id(id)
            .await
            .ok_or_else(|| "Game not found".to_string())
    }

    async fn get_all_games(&self) -> Result<Vec<Game>, String> {
        Ok(self.repo.find_all().await)
    }

    async fn search_games(&self, query: &str) -> Result<Vec<Game>, String> {
        Ok(self.repo.search(query).await)
    }

    async fn search_games_dto(&self, query: &str) -> Result<Vec<GameDto>, String> {
        Ok(self.repo.search_dto(query).await)
    }

    async fn create_game(&self, game_dto: GameDto) -> Result<Game, String> {
        // Validate the DTO
        game_dto
            .validate()
            .map_err(|e| format!("Validation error: {}", e))?;

        let game = Game::from(game_dto);
        self.repo.create(game).await
    }

    async fn update_game(&self, id: &str, game_dto: GameDto) -> Result<Game, String> {
        // Validate the DTO
        game_dto
            .validate()
            .map_err(|e| format!("Validation error: {}", e))?;

        // Check if game exists
        let existing_game = self
            .repo
            .find_by_id(id)
            .await
            .ok_or_else(|| "Game not found".to_string())?;

        // Create updated game with existing ID and rev
        let mut updated_game = Game::from(game_dto);
        updated_game.id = existing_game.id;
        updated_game.rev = existing_game.rev;

        self.repo.update(updated_game).await
    }

    async fn delete_game(&self, id: &str) -> Result<(), String> {
        // Check if game exists
        self.repo
            .find_by_id(id)
            .await
            .ok_or_else(|| "Game not found".to_string())?;

        self.repo.delete(id).await
    }

    async fn get_game_recommendations(
        &self,
        player_id: &str,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.repo.get_game_recommendations(player_id, limit).await
    }

    async fn get_similar_games(
        &self,
        game_id: &str,
        limit: i32,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.repo.get_similar_games(game_id, limit).await
    }

    async fn get_popular_games(&self, limit: i32) -> Result<Vec<serde_json::Value>, String> {
        self.repo.get_popular_games(limit).await
    }
}
