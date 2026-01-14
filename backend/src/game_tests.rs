#[cfg(test)]
mod game_tests {
    use shared::dto::game::GameDto;
    use shared::models::game::{Game, GameSource};

    #[test]
    fn test_game_dto_creation() {
        let game_dto = GameDto {
            id: "game/test".to_string(),
            name: "Test Game".to_string(),
            year_published: Some(2020),
            bgg_id: Some(12345),
            description: Some("A test game".to_string()),
            source: GameSource::Database,
        };

        assert_eq!(game_dto.name, "Test Game");
        assert_eq!(game_dto.year_published, Some(2020));
        assert_eq!(game_dto.bgg_id, Some(12345));
    }

    #[test]
    fn test_game_model_creation() {
        let game = Game {
            id: "game/test".to_string(),
            rev: "1".to_string(),
            name: "Test Game".to_string(),
            year_published: Some(2020),
            bgg_id: Some(12345),
            description: Some("A test game".to_string()),
            source: GameSource::Database,
        };

        assert_eq!(game.name, "Test Game");
        assert_eq!(game.year_published, Some(2020));
        assert_eq!(game.bgg_id, Some(12345));
        assert_eq!(game.source, GameSource::Database);
    }

    #[test]
    fn test_game_serialization() {
        let game = Game {
            id: "game/test".to_string(),
            rev: "1".to_string(),
            name: "Test Game".to_string(),
            year_published: Some(2020),
            bgg_id: Some(12345),
            description: Some("A test game".to_string()),
            source: GameSource::Database,
        };

        let json = serde_json::to_string(&game).unwrap();
        let deserialized: Game = serde_json::from_str(&json).unwrap();

        assert_eq!(game.id, deserialized.id);
        assert_eq!(game.name, deserialized.name);
        assert_eq!(game.source, deserialized.source);
    }

    #[test]
    fn test_game_source_enum() {
        // Test that GameSource enum variants exist
        match GameSource::Database {
            GameSource::Database => assert!(true),
            _ => assert!(false),
        }

        match GameSource::BGG {
            GameSource::BGG => assert!(true),
            _ => assert!(false),
        }
    }
}
