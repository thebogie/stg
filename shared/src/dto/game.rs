use crate::{models::game::GameSource, Game};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Data Transfer Object for Game
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
pub struct GameDto {
    /// Game's ID (optional for creation, will be set by ArangoDB if empty)
    #[serde(rename = "_id", default)]
    pub id: String,

    /// Game's name
    #[validate(length(
        min = 1,
        max = 200,
        message = "Name is required and must be at most 200 characters"
    ))]
    pub name: String,

    /// Year the game was published
    #[serde(rename = "year_published")]
    pub year_published: Option<i32>,

    /// BoardGameGeek ID
    #[serde(rename = "bgg_id")]
    pub bgg_id: Option<i32>,

    /// Game's description
    #[validate(custom(function = "validate_description_len"))]
    pub description: Option<String>,

    /// Source of the game data
    pub source: GameSource,
}

impl From<&Game> for GameDto {
    fn from(game: &Game) -> Self {
        Self {
            id: game.id.clone(),
            name: game.name.clone(),
            year_published: game.year_published,
            bgg_id: game.bgg_id,
            description: game.description.clone(),
            source: game.source.clone(),
        }
    }
}

impl From<GameDto> for Game {
    fn from(dto: GameDto) -> Self {
        Self::new_for_db(
            dto.name.clone(),
            dto.year_published,
            dto.bgg_id,
            dto.description.clone(),
            dto.source,
        )
        .unwrap_or_else(|_| Self {
            id: dto.id,
            rev: String::new(), // Let ArangoDB set this
            name: dto.name,
            year_published: dto.year_published,
            bgg_id: dto.bgg_id,
            description: dto.description,
            source: dto.source,
        })
    }
}

impl GameDto {
    /// Updates a game with the DTO's values
    pub fn update_game(&self, game: &mut Game) {
        game.name = self.name.clone();
        game.year_published = self.year_published;
        game.bgg_id = self.bgg_id;
        game.description = self.description.clone();
        game.source = self.source.clone();
    }

    /// Validates the DTO and converts to Game if valid
    pub fn try_into_game(self) -> std::result::Result<Game, validator::ValidationErrors> {
        self.validate()?;
        Ok(Game::from(self))
    }
}

fn validate_description_len(text: &String) -> Result<(), validator::ValidationError> {
    if text.len() > 4000 {
        let mut err = validator::ValidationError::new("length");
        err.message = Some("Description must be at most 4000 characters".into());
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_game_dto_validation_empty_name() {
        let dto = GameDto {
            id: "game/1".to_string(),
            name: "".to_string(),
            year_published: None,
            bgg_id: None,
            description: None,
            source: GameSource::BGG,
        };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_game_dto_validation_valid_data() {
        let dto = GameDto {
            id: "game/1".to_string(),
            name: "Valid Name".to_string(),
            year_published: Some(2020),
            bgg_id: Some(12345),
            description: Some("A valid game".to_string()),
            source: GameSource::BGG,
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn test_game_dto_validation_minimal_data() {
        let dto = GameDto {
            id: "game/1".to_string(),
            name: "Valid Name".to_string(),
            year_published: None,
            bgg_id: None,
            description: None,
            source: GameSource::BGG,
        };
        assert!(dto.validate().is_ok());
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;

    #[test]
    fn test_game_dto_to_model_invalid() {
        let dto = GameDto {
            id: "game/1".to_string(),
            name: "".to_string(), // Invalid: empty name
            year_published: None,
            bgg_id: None,
            description: None,
            source: GameSource::BGG,
        };
        let result = dto.try_into_game();
        assert!(result.is_err());
    }
}
