use anyhow::Result;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use random_word;
use serde::Deserialize;
use shared::{Contest, Game, Player, Venue};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct StgContest {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(rename = "_rev", default)]
    pub rev: String,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_utc_datetime")]
    pub start: DateTime<FixedOffset>,
    pub startoffset: String,
    #[serde(deserialize_with = "deserialize_utc_datetime")]
    pub stop: DateTime<FixedOffset>,
    pub stopoffset: String,
    pub venue: StgVenue,
    #[serde(deserialize_with = "deserialize_games")]
    pub games: Vec<StgGame>,
    pub outcome: Vec<StgOutcome>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StgVenue {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(rename = "_rev", default)]
    pub rev: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "formattedAddress")]
    pub formatted_address: String,
    pub lat: f64,
    pub lng: f64,
    #[serde(default)]
    pub place_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StgGame {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(rename = "_rev", default)]
    pub rev: String,
    pub name: String,
    #[serde(rename = "year_published")]
    pub year_published: i32,
    pub bgg_id: Option<i32>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_min_players")]
    pub min_players: i32,
    #[serde(default = "default_max_players")]
    pub max_players: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StgOutcome {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(rename = "_rev", default)]
    pub rev: String,
    pub player_id: String,
    pub place: i32,
    pub result: String,
}

fn default_name() -> String {
    let adj = random_word::get(random_word::Lang::En);
    let noun = random_word::get(random_word::Lang::En);

    // Capitalize first letter of each word
    let adj_capitalized = adj
        .chars()
        .next()
        .unwrap()
        .to_uppercase()
        .collect::<String>()
        + &adj[1..];
    let noun_capitalized = noun
        .chars()
        .next()
        .unwrap()
        .to_uppercase()
        .collect::<String>()
        + &noun[1..];

    format!("{} {}", adj_capitalized, noun_capitalized)
}

fn default_min_players() -> i32 {
    1
}
fn default_max_players() -> i32 {
    99
}

fn deserialize_games<'de, D>(deserializer: D) -> Result<Vec<StgGame>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum GameOrString {
        Game(StgGame),
        String(String),
    }

    let games = Vec::<GameOrString>::deserialize(deserializer)?;
    let mut result = Vec::with_capacity(games.len());

    for game in games {
        match game {
            GameOrString::Game(g) => result.push(g),
            GameOrString::String(name) => {
                // Create a minimal StgGame from the string, letting ArangoDB handle the key
                result.push(StgGame {
                    id: String::new(),  // Let ArangoDB set this
                    rev: String::new(), // Let ArangoDB set this
                    name,
                    description: String::new(),
                    year_published: 0,
                    bgg_id: None,
                    min_players: default_min_players(),
                    max_players: default_max_players(),
                });
            }
        }
    }

    Ok(result)
}

fn deserialize_utc_datetime<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Clean up the datetime string
    let s = s.trim();

    // Handle common datetime format issues
    let s = if s.len() > 19 {
        // If longer than "YYYY-MM-DDThh:mm:ss"
        let (base, extra) = s.split_at(19);
        if extra.chars().all(|c| c == '0') {
            base.to_string()
        } else {
            // Try to fix common format issues
            let fixed = base.to_string();
            if extra.starts_with('.') || extra.starts_with(',') {
                // Remove milliseconds/microseconds
                fixed
            } else {
                s.to_string()
            }
        }
    } else {
        s.to_string()
    };

    // Try to fix malformed dates like "2015-0620:45:00" -> "2015-06-20:45:00"
    let s = if s.len() >= 10 && &s[7..8] != "-" {
        let mut fixed = s[..7].to_string();
        fixed.push('-');
        fixed.push_str(&s[7..]);
        fixed
    } else {
        s
    };

    // Try to fix missing 'T' separator
    let s = if s.len() >= 10 && &s[10..11] != "T" {
        let mut fixed = s[..10].to_string();
        fixed.push('T');
        fixed.push_str(&s[10..]);
        fixed
    } else {
        s
    };

    // Try parsing with chrono's DateTime parser first since it's more lenient
    match DateTime::parse_from_rfc3339(&s) {
        Ok(dt) => return Ok(dt),
        Err(_) => {
            // Fall back to our custom parsing attempts
            let naive_dt = match NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
                Ok(dt) => dt,
                Err(e1) => {
                    // Try format without seconds
                    match NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M") {
                        Ok(dt) => dt,
                        Err(e2) => {
                            // Try format with just date
                            match NaiveDateTime::parse_from_str(&format!("{}T00:00:00", &s[..10]), "%Y-%m-%dT%H:%M:%S") {
                                Ok(dt) => dt,
                                Err(e3) => return Err(serde::de::Error::custom(format!(
                                    "Failed to parse datetime '{}': {}. Tried formats:\n1. RFC3339\n2. YYYY-MM-DDThh:mm:ss: {}\n3. YYYY-MM-DDThh:mm: {}\n4. YYYY-MM-DD: {}",
                                    s, e1, e1, e2, e3
                                )))
                            }
                        }
                    }
                }
            };

            // Convert to UTC DateTime
            let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);

            // Return as UTC (offset will be applied later)
            Ok(utc_dt.with_timezone(&FixedOffset::east_opt(0).unwrap()))
        }
    }
}

// Cache to store document IDs
#[derive(Default)]
pub struct DocumentCache {
    venues: HashMap<String, String>,  // (place_id, id)
    games: HashMap<String, String>,   // (name, id)
    players: HashMap<String, String>, // (player_id, id)
}

impl DocumentCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_venue(&self, place_id: &str) -> Option<String> {
        self.venues.get(place_id).cloned()
    }

    pub fn get_game(&self, name: &str) -> Option<String> {
        self.games.get(name).cloned()
    }

    pub fn get_player(&self, player_id: &str) -> Option<String> {
        self.players.get(player_id).cloned()
    }

    pub fn store_venue(&mut self, place_id: String, id: String) {
        self.venues.insert(place_id, id);
    }

    pub fn store_game(&mut self, name: String, id: String) {
        self.games.insert(name, id);
    }

    pub fn store_player(&mut self, player_id: String, id: String) {
        self.players.insert(player_id, id);
    }
}

impl From<&StgVenue> for Venue {
    fn from(venue: &StgVenue) -> Self {
        Venue::new(
            venue.id.clone(),
            venue.rev.clone(),
            venue.display_name.clone(),
            venue.formatted_address.clone(),
            venue.place_id.clone(),
            venue.lat,
            venue.lng,
            "UTC".to_string(),
            shared::models::venue::VenueSource::Database,
        )
        .unwrap_or_else(|_| {
            Venue::new_for_db(
                venue.display_name.clone(),
                venue.formatted_address.clone(),
                venue.place_id.clone(),
                venue.lat,
                venue.lng,
                "UTC".to_string(),
                shared::models::venue::VenueSource::Database,
            )
            .unwrap()
        })
    }
}

impl From<StgGame> for Game {
    fn from(game: StgGame) -> Self {
        Game::new(
            String::new(), // Let ArangoDB set this
            String::new(), // Let ArangoDB set this
            game.name.clone(),
            Some(game.year_published),
            game.bgg_id,
            None, // description is optional
            shared::models::game::GameSource::Database,
        )
        .unwrap_or_else(|_| {
            Game::new(
                String::new(), // Let ArangoDB set this
                String::new(), // Let ArangoDB set this
                game.name,
                Some(game.year_published),
                game.bgg_id,
                None, // description is optional
                shared::models::game::GameSource::Database,
            )
            .unwrap()
        })
    }
}

impl From<&StgContest> for Contest {
    fn from(contest: &StgContest) -> Self {
        // Create minimal contest (relationships managed through edge collections)
        let contest_doc = Contest {
            id: contest.id.clone(),
            rev: contest.rev.clone(),
            start: contest.start,
            stop: contest.stop,
            name: contest.name.clone(),
            creator_id: String::new(),
            created_at: chrono::Utc::now().fixed_offset(),
        };

        contest_doc
    }
}

impl From<&StgOutcome> for Player {
    fn from(outcome: &StgOutcome) -> Self {
        Player::new(
            outcome.id.clone(),
            outcome.rev.clone(),
            outcome.player_id.clone(),
            outcome.player_id.clone(),
            String::new(), // Empty email - must be set separately
            String::new(), // Empty password - must be set separately
            chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
            false,
        )
        .unwrap_or_else(|_| {
            Player::new_for_db(
                outcome.player_id.clone(),
                outcome.player_id.clone(),
                String::new(), // Empty email - must be set separately
                String::new(), // Empty password - must be set separately
                chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
                false,
            )
            .unwrap()
        })
    }
}
