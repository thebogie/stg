use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::error::Result;
use uuid::Uuid;

/// Represents the source of game data
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GameSource {
    #[serde(rename = "database")]
    Database,
    #[serde(rename = "bgg")]
    BGG,
}

/// Represents a board game
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
pub struct Game {
    /// Game's ID
    #[serde(rename = "_id")]
    pub id: String,

    /// Game's revision
    #[serde(rename = "_rev")]
    pub rev: String,

    /// Game's name
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,

    /// Year the game was published
    #[serde(rename = "year_published")]
    pub year_published: Option<i32>,

    /// BoardGameGeek ID
    #[serde(rename = "bgg_id")]
    pub bgg_id: Option<i32>,

    /// Game's description
    pub description: Option<String>,

    /// Source of the game data
    pub source: GameSource,
}

impl Game {
    /// Creates a new game
    pub fn new(
        id: String,
        rev: String,
        name: String,
        year_published: Option<i32>,
        bgg_id: Option<i32>,
        description: Option<String>,
        source: GameSource,
    ) -> Result<Self> {
        let game = Self {
            id,
            rev,
            name,
            year_published,
            bgg_id,
            description,
            source,
        };
        game.validate()?;
        Ok(game)
    }

    /// Creates a new game for database insertion (ArangoDB will set id and rev)
    pub fn new_for_db(
        name: String,
        year_published: Option<i32>,
        bgg_id: Option<i32>,
        description: Option<String>,
        source: GameSource,
    ) -> Result<Self> {
        let game = Self {
            id: String::new(), // Will be set by ArangoDB
            rev: String::new(), // Will be set by ArangoDB
            name,
            year_published,
            bgg_id,
            description,
            source,
        };
        game.validate()?;
        Ok(game)
    }

    /// Creates a new game with a randomly generated ID
    pub fn new_random(name: String, description: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            rev: String::new(),
            name,
            year_published: None,
            bgg_id: None,
            description,
            source: GameSource::Database,
        }
    }
} 
