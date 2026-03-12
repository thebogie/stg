use crate::contest::name_generator::generate_contest_name;
use crate::db::Db;
use crate::game::repository::GameRepositoryImpl;
use crate::game::usecase::{GameUseCase, GameUseCaseImpl};
use crate::player::repository::{PlayerRepository, PlayerRepositoryImpl};
use crate::player::usecase::{PlayerUseCase, PlayerUseCaseImpl};
use crate::surreal_helpers::{
    record_id_from_field, record_id_from_row, record_id_to_key,
    select_one_by_record_id,
};
use surrealdb::types::SurrealValue;
use crate::venue::repository::VenueRepositoryImpl;
use crate::venue::usecase::{VenueUseCase, VenueUseCaseImpl};
use argon2::{Argon2, PasswordHasher};
use async_trait::async_trait;
use shared::dto::contest::{ContestDto, OutcomeDto};
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use shared::models::contest::Contest;
use shared::SharedError;
use std::collections::HashSet;
use uuid::Uuid;

/// Build Vec<Thing> from "table/key" or "table:key" ids for INSIDE bindings (SurrealDB v2 matches record id to Thing array).
fn strings_to_thing_array(ids: &[String]) -> Vec<surrealdb::types::RecordId> {
    ids.iter()
        .filter_map(|s| {
            let s = s.trim().replace('`', "");
            let (tb, key) = s.split_once(':').or_else(|| s.split_once('/'))?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            Some(surrealdb::types::RecordId::new(tb.trim(), key))
        })
        .collect()
}

pub struct ContestRepositoryImpl {
    pub db: Db,
    pub google_config: Option<(String, String)>,
    pub venue_usecase: VenueUseCaseImpl<VenueRepositoryImpl>,
    pub game_usecase: GameUseCaseImpl<GameRepositoryImpl>,
    pub player_usecase: PlayerUseCaseImpl<PlayerRepositoryImpl>,
}

impl ContestRepositoryImpl {
    pub fn new_with_google_config(db: Db, google_config: Option<(String, String)>) -> Self {
        let venue_repo = VenueRepositoryImpl::new(db.clone(), google_config.clone());
        let venue_usecase = VenueUseCaseImpl { repo: venue_repo };
        let game_repo = GameRepositoryImpl::new(db.clone());
        let game_usecase = GameUseCaseImpl { repo: game_repo };
        let player_repo = PlayerRepositoryImpl::new(db.clone());
        let player_usecase = PlayerUseCaseImpl { repo: player_repo };
        Self { db, google_config, venue_usecase, game_usecase, player_usecase }
    }
}

#[async_trait]
pub trait ContestRepository: Send + Sync {
    async fn create_contest(
        &self,
        contest_dto: ContestDto,
        creator_id: String,
    ) -> Result<ContestDto, String>;
    async fn find_by_id(&self, id: &str) -> Option<Contest>;
    async fn find_all(&self) -> Vec<Contest>;
    async fn search(&self, query: &str) -> Vec<Contest>;
    async fn update(&self, contest: Contest) -> Result<Contest, String>;
    async fn delete(&self, id: &str) -> Result<(), String>;
    async fn find_contests_by_player_and_game(
        &self,
        player_id: &str,
        game_id: &str,
    ) -> Result<Vec<serde_json::Value>, String>;
}

#[async_trait]
impl ContestRepository for ContestRepositoryImpl {
    async fn create_contest(
        &self,
        mut contest_dto: ContestDto,
        creator_id: String,
    ) -> Result<ContestDto, String> {
        log::info!("🎯 Starting contest creation process");
        log::info!(
            "🎯 Contest DTO received: name='{}', start='{}', stop='{}'",
            contest_dto.name,
            contest_dto.start,
            contest_dto.stop
        );
        log::info!(
            "🎯 Contest has {} outcomes and {} games",
            contest_dto.outcomes.len(),
            contest_dto.games.len()
        );

        // Log the player IDs we're working with
        for (i, outcome) in contest_dto.outcomes.iter().enumerate() {
            log::info!(
                "  Outcome {}: player_id='{}', handle='{}', email='{}'",
                i,
                outcome.player_id,
                outcome.handle,
                outcome.email
            );
        }

        // Log games
        for (i, game) in contest_dto.games.iter().enumerate() {
            log::info!(
                "  Game {}: id='{}', name='{}', source='{:?}'",
                i,
                game.id,
                game.name,
                game.source
            );
        }

        // Generate a random name for the contest if one wasn't provided
        if contest_dto.name.is_empty() {
            log::info!("📝 Contest name is empty, attempting to generate random name...");
            log::info!("📝 About to call generate_contest_name()...");

            contest_dto.name = match std::panic::catch_unwind(|| {
                log::info!("📝 Inside generate_contest_name() function");
                let result = generate_contest_name();
                log::info!("📝 generate_contest_name() returned: '{}'", result);
                result
            }) {
                Ok(name) if !name.is_empty() => {
                    log::info!("✅ Successfully generated contest name: '{}'", name);
                    name
                }
                Ok(empty_name) => {
                    log::warn!(
                        "⚠️ generate_contest_name() returned empty string: '{}'",
                        empty_name
                    );
                    let fallback_name = format!("Contest {}", chrono::Utc::now().timestamp());
                    log::info!("📝 Using fallback name: '{}'", fallback_name);
                    fallback_name
                }
                Err(panic_info) => {
                    log::error!("💥 generate_contest_name() panicked: {:?}", panic_info);
                    let fallback_name = format!("Contest {}", chrono::Utc::now().timestamp());
                    log::info!("📝 Using fallback name after panic: '{}'", fallback_name);
                    fallback_name
                }
            };
        } else {
            log::info!("📝 Using provided contest name: '{}'", contest_dto.name);
        }

        log::info!("📝 Final contest name: '{}'", contest_dto.name);

        // Create the contest document
        log::info!("📄 Creating contest document in database...");
        let contest = Contest {
            id: contest_dto.id.clone(),
            rev: "1".to_string(),
            name: contest_dto.name.clone(),
            start: contest_dto.start.into(),
            stop: contest_dto.stop.into(),
            creator_id: creator_id.clone(),
            created_at: chrono::Utc::now(),
        };

        log::info!("📄 Contest model created: id='{}', name='{}', start='{}', stop='{}', creator='{}', created_at='{}'", 
            contest.id, contest.name, contest.start, contest.stop, contest.creator_id, contest.created_at);

        let contest_key = Uuid::new_v4().to_string();
        let contest_id = format!("contest/{}", contest_key);
        let doc = serde_json::json!({
            "name": contest.name,
            "start": contest.start.to_rfc3339(),
            "stop": contest.stop.to_rfc3339(),
            "creator_id": contest.creator_id,
            "created_at": contest.created_at.to_rfc3339(),
        });
        log::info!("💾 Inserting contest document...");
        self.db
            .query("CREATE type::record('contest', $key) CONTENT $doc")
            .bind(("key", contest_key.clone()))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to create contest: {}", e))?;

        let created_contest = Contest {
            id: contest_id.clone(),
            rev: "1".to_string(),
            name: contest.name,
            start: contest.start,
            stop: contest.stop,
            creator_id: contest.creator_id,
            created_at: contest.created_at,
        };

        log::info!("✅ Contest document created successfully: id='{}', name='{}'", created_contest.id, created_contest.name);

        // Handle venue based on source and ID
        log::info!("🏢 Processing venue...");
        log::info!(
            "🏢 Venue source: {:?}, ID: '{}'",
            contest_dto.venue.source,
            contest_dto.venue.id
        );

        let venue = if contest_dto.venue.source == shared::models::venue::VenueSource::Database
            && !contest_dto.venue.id.is_empty()
        {
            log::info!(
                "🏢 Venue is from database, looking up by ID: {}",
                contest_dto.venue.id
            );
            // Venue is from database with a valid ID, search by ID to get the full venue object
            match self.venue_usecase.get_venue(&contest_dto.venue.id).await {
                Ok(venue) => {
                    log::info!(
                        "✅ Successfully found existing venue: {} ({})",
                        venue.display_name,
                        venue.id
                    );
                    venue
                }
                Err(e) => {
                    log::error!("💥 Failed to get venue by ID: {}", e);
                    return Err(format!("Failed to get venue: {}", e));
                }
            }
        } else {
            log::info!("🏢 Venue is new, creating venue...");
            // Venue is new (either Google source or Database source without ID), create it and use it directly
            match self
                .venue_usecase
                .create_venue(contest_dto.venue.clone())
                .await
            {
                Ok(venue) => {
                    log::info!(
                        "✅ Successfully created new venue: {} ({})",
                        venue.display_name,
                        venue.id
                    );
                    venue
                }
                Err(e) => {
                    log::error!("💥 Failed to create venue: {}", e);
                    return Err(format!("Failed to create venue: {}", e));
                }
            }
        };

        let venue_id = venue.id.clone();
        log::info!("🏢 Final venue ID: {}", venue_id);

        // Create the played_at relationship
        log::info!("🔗 Creating PLAYED_AT edge...");
        log::info!(
            "🔗 Edge details: contest_id={}, venue_id={}",
            created_contest.id,
            venue_id
        );
        match self
            .create_played_at_relation(&created_contest.id, &venue_id)
            .await
        {
            Ok(_) => log::info!("✅ Successfully created PLAYED_AT edge"),
            Err(e) => {
                log::error!("💥 Failed to create played_at edge: {:?}", e);
                return Err(format!("Failed to create played_at edge: {:?}", e));
            }
        }

        // Handle games based on source
        log::info!("🎮 Processing games...");
        log::info!("🎮 Total games to process: {}", contest_dto.games.len());

        let mut processed_games = Vec::new();
        for (i, game_dto) in contest_dto.games.iter().enumerate() {
            log::info!(
                "🎮 Processing game {}/{}: {} (id: '{}')",
                i + 1,
                contest_dto.games.len(),
                game_dto.name,
                game_dto.id
            );

            let game = if game_dto.id.starts_with("game/") {
                log::info!(
                    "🎮 Game is from database, looking up by ID: {}",
                    game_dto.id
                );
                // Game is from database, search by ID to get the full game object
                match self.game_usecase.get_game(&game_dto.id).await {
                    Ok(game) => {
                        log::info!(
                            "✅ Successfully found existing game: {} ({})",
                            game.name,
                            game.id
                        );
                        game
                    }
                    Err(e) => {
                        log::error!("💥 Failed to get game by ID: {}", e);
                        return Err(format!("Failed to get game: {}", e));
                    }
                }
            } else {
                log::info!("🎮 Game is new (BGG), creating game...");
                // Game is new (BGG), create it and use it directly
                match self.game_usecase.create_game(game_dto.clone()).await {
                    Ok(game) => {
                        log::info!(
                            "✅ Successfully created new game: {} ({})",
                            game.name,
                            game.id
                        );
                        game
                    }
                    Err(e) => {
                        log::error!("💥 Failed to create game: {}", e);
                        return Err(format!("Failed to create game: {}", e));
                    }
                }
            };
            let game_name = game.name.clone();
            let game_id = game.id.clone();
            processed_games.push(game);
            log::info!(
                "🎮 Game {}/{} processed successfully: {} ({})",
                i + 1,
                contest_dto.games.len(),
                game_name,
                game_id
            );
        }

        // Create the played_with relationships for each processed game
        log::info!("🔗 Creating PLAYED_WITH edges...");
        log::info!(
            "🔗 Total PLAYED_WITH edges to create: {}",
            processed_games.len()
        );

        for (i, game) in processed_games.iter().enumerate() {
            log::info!(
                "🔗 Creating PLAYED_WITH edge {}/{}: contest_id={}, game_id={}",
                i + 1,
                processed_games.len(),
                created_contest.id,
                game.id
            );
            match self
                .create_played_with_relation(&created_contest.id, &game.id)
                .await
            {
                Ok(_) => log::info!(
                    "✅ Successfully created PLAYED_WITH edge {}/{}",
                    i + 1,
                    processed_games.len()
                ),
                Err(e) => {
                    log::error!(
                        "💥 Failed to create played_with edge {}/{}: {:?}",
                        i + 1,
                        processed_games.len(),
                        e
                    );
                    return Err(format!("Failed to create played_with edge: {:?}", e));
                }
            }
        }

        // Process players in outcomes
        log::info!("👥 Processing outcomes/players...");
        log::info!(
            "👥 Total outcomes to process: {}",
            contest_dto.outcomes.len()
        );

        let mut processed_outcomes = Vec::new();
        for (i, outcome) in contest_dto.outcomes.iter().enumerate() {
            log::info!(
                "👥 Processing outcome {}/{}: player_id='{}', handle='{}', email='{}'",
                i + 1,
                contest_dto.outcomes.len(),
                outcome.player_id,
                outcome.handle,
                outcome.email
            );

            let player_id = outcome.player_id.clone();

            // Helper: check if player_id is a real DB record id (SurrealDB format)
            fn is_real_player_id(player_id: &str) -> bool {
                player_id.starts_with("player/") && player_id.len() > 7
            }

            let player = if !player_id.is_empty() && is_real_player_id(&player_id) {
                log::info!("👥 Looking up existing player with ID: {}", player_id);
                // Fetch existing player by ID (SurrealDB record id)
                match self.player_usecase.get_player(&player_id).await {
                    Ok(player) => {
                        log::info!(
                            "✅ Found existing player: {} ({})",
                            player.handle,
                            player.id
                        );
                        player
                    }
                    Err(e) => {
                        log::error!("💥 Failed to find player with ID '{}': {}", player_id, e);
                        return Err(format!("Player not found: {}", e));
                    }
                }
            } else {
                log::info!(
                    "👥 Creating new player from outcome data: handle='{}', email='{}'",
                    outcome.handle,
                    outcome.email
                );
                // Create new player from outcome data
                let _player_dto = shared::dto::player::PlayerDto {
                    id: String::new(), // Will be set by SurrealDB
                    firstname: outcome.handle.clone(),
                    handle: outcome.handle.clone(),
                    email: outcome.email.clone(),
                    created_at: chrono::Utc::now().fixed_offset(),
                    is_admin: false,
                };

                // Create player with a default password
                let default_password = "letmein"; // TODO: Generate random password or require email verification
                let salt_string = argon2::password_hash::SaltString::generate(
                    &mut argon2::password_hash::rand_core::OsRng,
                );
                let hashed_password = Argon2::default()
                    .hash_password(default_password.as_bytes(), &salt_string)
                    .map_err(|e| format!("Failed to hash password: {}", e))?
                    .to_string();

                let player = shared::models::player::Player::new_for_db(
                    outcome.handle.clone(),
                    outcome.handle.clone(),
                    outcome.email.clone(),
                    hashed_password,
                    chrono::Utc::now().fixed_offset(),
                    false,
                )
                .map_err(|e| format!("Failed to create player: {}", e))?;

                // Save to DB
                match self.player_usecase.repo.create(player).await {
                    Ok(player) => {
                        log::info!("✅ Created new player: {} ({})", player.handle, player.id);
                        player
                    }
                    Err(e) => {
                        log::error!("💥 Failed to create player: {}", e);
                        return Err(format!("Failed to create player: {}", e));
                    }
                }
            };

            // Update OutcomeDto with correct player_id
            let mut updated_outcome = outcome.clone();
            let final_player_id = player.id.clone();
            updated_outcome.player_id = final_player_id.clone();
            processed_outcomes.push(updated_outcome);
            log::info!(
                "👥 Outcome {}/{} processed successfully: player_id='{}', handle='{}'",
                i + 1,
                contest_dto.outcomes.len(),
                final_player_id,
                outcome.handle
            );
        }

        // Create the resulted_in relationships for each processed outcome
        log::info!("🔗 Creating RESULTED_IN edges...");
        log::info!(
            "🔗 Total RESULTED_IN edges to create: {}",
            processed_outcomes.len()
        );

        for (i, outcome) in processed_outcomes.iter().enumerate() {
            log::info!("🔗 Creating RESULTED_IN edge {}/{}: contest_id={}, player_id={}, place='{}', result='{}'", 
                i + 1, processed_outcomes.len(), created_contest.id, outcome.player_id, outcome.place, outcome.result);
            match self
                .create_resulted_in_relation(&created_contest.id, outcome)
                .await
            {
                Ok(_) => log::info!(
                    "✅ Successfully created RESULTED_IN edge {}/{}",
                    i + 1,
                    processed_outcomes.len()
                ),
                Err(e) => {
                    log::error!(
                        "💥 Failed to create resulted_in edge {}/{}: {:?}",
                        i + 1,
                        processed_outcomes.len(),
                        e
                    );
                    return Err(format!("Failed to create resulted_in edge: {:?}", e));
                }
            }
        }

        // Return the created contest as a DTO
        log::info!("📋 Creating final response DTO...");

        let venue_dto = VenueDto::from(&venue);
        log::info!(
            "📋 Venue DTO created: {} ({})",
            venue_dto.display_name,
            venue_dto.id
        );

        let game_dtos: Vec<GameDto> = processed_games.iter().map(|g| GameDto::from(g)).collect();
        log::info!("📋 Game DTOs created: {} games", game_dtos.len());

        let created_dto = ContestDto {
            id: created_contest.id.clone(),
            name: created_contest.name.clone(),
            start: created_contest.start.into(),
            stop: created_contest.stop.into(),
            venue: venue_dto,
            games: game_dtos,
            outcomes: processed_outcomes,
            creator_id: created_contest.creator_id.clone(),
            created_at: Some(created_contest.created_at.into()),
        };

        log::info!("✅ Contest creation process completed successfully!");
        log::info!("✅ Final contest DTO: id='{}', name='{}', {} games, {} outcomes, creator='{}', created_at='{}'",
            created_dto.id, created_dto.name, created_dto.games.len(), created_dto.outcomes.len(),
            created_dto.creator_id, created_dto.created_at.unwrap_or_else(|| chrono::Utc::now().fixed_offset()));

        Ok(created_dto)
    }
    async fn find_by_id(&self, id: &str) -> Option<Contest> {
        log::info!("🔍 Finding contest by ID: {}", id);
        let key = id.trim_start_matches("contest/").trim_start_matches("contest:").to_string();
        let key_clone = key.clone();

        // Prefer SurrealDB function when applied (docs/surreal-functions.surql)
        if let Ok(mut res) = self.db.query("SELECT fn::contest_row($key) AS result FROM [1]").bind(("key", key.clone())).await {
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            if let Some(first) = rows.into_iter().next() {
                let v = first.get("result").or_else(|| first.get("fn::contest_row($key)")).cloned().unwrap_or(first);
                if v.is_object() && !v.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    let id_str = record_id_from_row(&v, None).unwrap_or_else(|| format!("contest/{}", key_clone));
                    let parse_dt = |v: &serde_json::Value, key: &str| -> chrono::DateTime<chrono::FixedOffset> {
                        v.get(key).and_then(|x| x.as_str()).and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
                            .unwrap_or_else(|| chrono::Utc::now().fixed_offset())
                    };
                    let contest = Contest {
                        id: id_str,
                        rev: v.get("_rev").or_else(|| v.get("rev")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        start: parse_dt(&v, "start").into(),
                        stop: parse_dt(&v, "stop").into(),
                        creator_id: v.get("creator_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        created_at: parse_dt(&v, "created_at").into(),
                    };
                    log::info!("✅ Found contest via fn::contest_row: {}", contest.name);
                    return Some(contest);
                }
            }
        }

        let mut res = match self.db.query("SELECT * FROM contest WHERE id = type::record('contest', $key)").bind(("key", key)).await {
            Ok(r) => r,
            Err(e) => {
                log::error!("💥 Failed to get contest: {}", e);
                return None;
            }
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let v = rows.into_iter().next()?;
        let id_str = record_id_from_row(&v, None).unwrap_or_else(|| format!("contest/{}", key_clone));
        let parse_dt = |v: &serde_json::Value, key: &str| -> chrono::DateTime<chrono::FixedOffset> {
            v.get(key).and_then(|x| x.as_str()).and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
                .unwrap_or_else(|| chrono::Utc::now().fixed_offset())
        };
        let contest = Contest {
            id: id_str,
            rev: v.get("_rev").or_else(|| v.get("rev")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
            name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            start: parse_dt(&v, "start").into(),
            stop: parse_dt(&v, "stop").into(),
            creator_id: v.get("creator_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            created_at: parse_dt(&v, "created_at").into(),
        };
        log::info!("✅ Found contest: {}", contest.name);
        Some(contest)
    }
    async fn find_all(&self) -> Vec<Contest> {
        unimplemented!()
    }
    async fn search(&self, _query: &str) -> Vec<Contest> {
        unimplemented!()
    }
    async fn update(&self, _contest: Contest) -> Result<Contest, String> {
        unimplemented!()
    }
    async fn delete(&self, _id: &str) -> Result<(), String> {
        unimplemented!()
    }

    async fn find_contests_by_player_and_game(
        &self,
        player_id: &str,
        game_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Finding contests for player {} and game {}", player_id, game_id);
        let player_key = record_id_to_key(player_id, "player");
        let game_key = record_id_to_key(game_id, "game");

        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct OutRow {
            contest_id: Option<surrealdb::types::RecordId>,
        }
        fn rid_from_row(r: &OutRow) -> Option<String> {
            let id = crate::surreal_helpers::thing_to_record_id(&r.contest_id);
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        }

        let mut res = self
            .db
            .query("SELECT `in` AS contest_id FROM resulted_in WHERE `out` = type::record('player', $player_key)")
            .bind(("player_key", player_key.clone()))
            .await
            .map_err(|e| e.to_string())?;
        let player_contests: Vec<OutRow> = res.take(0).map_err(|e| e.to_string())?;

        let mut res2 = self
            .db
            .query("SELECT `in` AS contest_id FROM played_with WHERE `out` = type::record('game', $game_key)")
            .bind(("game_key", game_key))
            .await
            .map_err(|e| e.to_string())?;
        let game_contests: Vec<OutRow> = res2.take(0).map_err(|e| e.to_string())?;

        let pc: std::collections::HashSet<String> = player_contests.iter().filter_map(rid_from_row).collect();
        let gc: std::collections::HashSet<String> = game_contests.iter().filter_map(rid_from_row).collect();
        let contest_ids: Vec<String> = pc.intersection(&gc).cloned().collect();

        if contest_ids.is_empty() {
            log::info!("✅ Found 0 contests for player {} and game {}", player_id, game_id);
            return Ok(Vec::new());
        }

        // Load each contest with venue, games, outcomes and build same shape as before
        let mut results = Vec::new();
        for cid in contest_ids {
            let id_norm = cid.replace("contest:", "contest/"); // already "contest/key" from rid_from_row
            if let Some(dto) = self.find_details_by_id(&id_norm).await {
                let venue = &dto.venue;
                let games = &dto.games;
                let outcomes = &dto.outcomes;
                let my_outcome = outcomes.iter().find(|o| o.player_id == player_id);
                let game = games.iter().find(|g| g.id == game_id);
                let contest_json = serde_json::json!({
                    "contest_id": dto.id,
                    "contest_name": dto.name,
                    "contest_date": dto.start,
                    "contest_description": serde_json::Value::Null,
                    "contest_status": serde_json::Value::Null,
                    "game_id": game.map(|g| g.id.as_str()).unwrap_or(""),
                    "game_name": game.map(|g| g.name.as_str()).unwrap_or(""),
                    "game_year_published": game.and_then(|g| g.year_published),
                    "venue_id": venue.id,
                    "venue_name": venue.display_name,
                    "venue_display_name": venue.display_name,
                    "venue_address": venue.formatted_address,
                    "my_placement": my_outcome.map(|o| o.place.as_str()).unwrap_or(""),
                    "my_result": my_outcome.map(|o| o.result.as_str()).unwrap_or(""),
                    "total_players": outcomes.len(),
                    "players": outcomes.iter().map(|o| serde_json::json!({
                        "player_id": o.player_id,
                        "player_handle": o.handle,
                        "placement": o.place,
                        "result": o.result
                    })).collect::<Vec<_>>()
                });
                results.push(contest_json);
            }
        }
        results.sort_by(|a, b| {
            let sa = a.get("contest_date").and_then(|v| v.as_str()).unwrap_or("");
            let sb = b.get("contest_date").and_then(|v| v.as_str()).unwrap_or("");
            sb.cmp(sa)
        });
        log::info!("✅ Found {} contests for player {} and game {}", results.len(), player_id, game_id);
        Ok(results)
    }
}

impl ContestRepositoryImpl {
    /// Create a played_at relation: [Contest]-in->(played_at)-out->[Venue] (IN=subject, OUT=object).
    async fn create_played_at_relation(
        &self,
        contest_id: &str,
        venue_id: &str,
    ) -> Result<(), SharedError> {
        log::info!("🔗 Creating PLAYED_AT edge: contest='{}' -> venue='{}'", contest_id, venue_id);
        let in_id = contest_id.replace("contest/", "contest:");
        let out_id = venue_id.replace("venue/", "venue:");
        self.db
            .query("INSERT INTO played_at (`in`, `out`) VALUES (type::record($in), type::record($out))")
            .bind(("in", in_id))
            .bind(("out", out_id))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to create played_at relation: {}", e)))?;
        log::info!("✅ PLAYED_AT edge creation completed successfully");
        Ok(())
    }

    /// Create a played_with relation: [Contest]-in->(played_with)-out->[Game].
    async fn create_played_with_relation(
        &self,
        contest_id: &str,
        game_id: &str,
    ) -> Result<(), SharedError> {
        log::info!("🔗 Creating PLAYED_WITH edge: contest='{}' -> game='{}'", contest_id, game_id);
        let in_id = contest_id.replace("contest/", "contest:");
        let out_id = game_id.replace("game/", "game:");
        self.db
            .query("INSERT INTO played_with (`in`, `out`) VALUES (type::record($in), type::record($out))")
            .bind(("in", in_id))
            .bind(("out", out_id))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to create played_with relation: {}", e)))?;
        log::info!("✅ PLAYED_WITH edge creation completed successfully");
        Ok(())
    }

    /// Create a resulted_in relation: [Contest]-in->(resulted_in)-out->[Player] (with place/result).
    async fn create_resulted_in_relation(
        &self,
        contest_id: &str,
        outcome: &OutcomeDto,
    ) -> Result<(), SharedError> {
        log::info!("🔗 Creating RESULTED_IN edge: contest='{}' -> player='{}', place='{}', result='{}'", contest_id, outcome.player_id, outcome.place, outcome.result);
        let place = outcome.place.parse::<i32>().map_err(|e| SharedError::Validation(format!("Invalid place value: {}", e)))?;
        let in_id = contest_id.replace("contest/", "contest:");
        let out_key = outcome
            .player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`');
        let out_id = format!("player:{}", out_key);
        let result_str = outcome.result.clone();
        self.db
            .query("INSERT INTO resulted_in (`in`, `out`, place, result) VALUES (type::record($in), type::record($out), $place, $result)")
            .bind(("in", in_id))
            .bind(("out", out_id))
            .bind(("place", place))
            .bind(("result", result_str))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to create resulted_in relation: {}", e)))?;
        log::info!("✅ RESULTED_IN edge creation completed successfully");
        Ok(())
    }
}

impl ContestRepositoryImpl {
    /// Look up contest details using the given Db. Use the app's shared Db (e.g. from web::Data<Db>)
    /// so the same NS/DB as analytics is used and by-id lookup sees the same data.
    pub async fn find_details_by_id_using(&self, id: &str, db: &crate::db::Db) -> Option<ContestDto> {
        self.find_details_by_id_impl(id, db).await
    }

    pub async fn find_details_by_id(&self, id: &str) -> Option<ContestDto> {
        self.find_details_by_id_impl(id, &self.db).await
    }

    /// Uses SurrealDB function fn::contest_with_edges when available (apply docs/surreal-functions.surql).
    /// Returns None if the function is not defined or returns no contest (fallback to multi-query path).
    async fn find_details_via_function(
        &self,
        db: &crate::db::Db,
        id: &str,
        key: &str,
    ) -> Option<ContestDto> {
        let mut res = match db
            .query("SELECT fn::contest_with_edges($key) AS result FROM [1]")
            .bind(("key", key.to_string()))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::debug!("contest_with_edges function not available or failed: {}", e);
                return None;
            }
        };
        let rows: Vec<serde_json::Value> = res.take(0).ok()?;
        let first = rows.into_iter().next()?;
        // SELECT fn::contest_with_edges($key) AS result → one row with key "result" = { contest, venue_id, game_ids, outcomes }
        let row = first
            .get("result")
            .or_else(|| first.get("fn::contest_with_edges($key)"))
            .cloned()
            .unwrap_or(first);
        let contest_data = row.get("contest")?.as_object()?;
        let contest_data = serde_json::Value::Object(contest_data.clone());

        let id_str = record_id_from_row(&contest_data, None).unwrap_or_else(|| id.to_string());
        let name = contest_data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let start_str = contest_data.get("start").and_then(|v| v.as_str()).unwrap_or("");
        let stop_str = contest_data.get("stop").and_then(|v| v.as_str()).unwrap_or("");
        let start = chrono::DateTime::parse_from_rfc3339(start_str)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));
        let stop = chrono::DateTime::parse_from_rfc3339(stop_str)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));
        let (start, stop) = match (start, stop) {
            (Some(s), Some(st)) => (s, st),
            _ => return None,
        };

        let venue_id_opt: Option<String> = row
            .get("venue_id")
            .and_then(|v| record_id_from_field(&serde_json::json!({ "venue_id": v }), "venue_id"));
        let venue_dto = if let Some(venue_rid) = venue_id_opt {
            let vkey = record_id_to_key(&venue_rid, "venue");
            let vrows: Vec<serde_json::Value> = db
                .query("SELECT * FROM venue WHERE id = type::record('venue', $key)")
                .bind(("key", vkey.clone()))
                .await
                .ok()
                .and_then(|mut r| r.take(0).ok())
                .unwrap_or_default();
            vrows.into_iter().next().map(|v| {
                let id = record_id_from_row(&v, None).unwrap_or_else(|| format!("venue/{}", vkey));
                VenueDto {
                    id,
                    display_name: v.get("displayName").or(v.get("display_name")).and_then(|x| x.as_str()).unwrap_or("Unknown Venue").to_string(),
                    formatted_address: v.get("formattedAddress").or(v.get("formatted_address")).and_then(|x| x.as_str()).unwrap_or("Address not available").to_string(),
                    place_id: v.get("placeId").or(v.get("place_id")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    lat: v.get("lat").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    lng: v.get("lng").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    timezone: v.get("timezone").and_then(|x| x.as_str()).unwrap_or("UTC").to_string(),
                    source: shared::models::venue::VenueSource::Database,
                }
            }).unwrap_or_else(|| VenueDto {
                id: format!("venue/{}", vkey),
                display_name: "Unknown Venue".to_string(),
                formatted_address: "Address not available".to_string(),
                place_id: String::new(),
                lat: 0.0,
                lng: 0.0,
                timezone: "UTC".to_string(),
                source: shared::models::venue::VenueSource::Database,
            })
        } else {
            VenueDto {
                id: String::new(),
                display_name: "Unknown Venue".to_string(),
                formatted_address: "Address not available".to_string(),
                place_id: String::new(),
                lat: 0.0,
                lng: 0.0,
                timezone: "UTC".to_string(),
                source: shared::models::venue::VenueSource::Database,
            }
        };

        let game_ids: Vec<String> = row
            .get("game_ids")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| record_id_from_field(&serde_json::json!({ "game_id": v }), "game_id"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let game_things = strings_to_thing_array(&game_ids);
        let games: Vec<GameDto> = if game_things.is_empty() {
            vec![]
        } else {
            let grows: Vec<serde_json::Value> = db
                .query("SELECT * FROM game WHERE id INSIDE $ids")
                .bind(("ids", game_things))
                .await
                .ok()
                .and_then(|mut r| r.take(0).ok())
                .unwrap_or_default();
            grows
                .into_iter()
                .filter_map(|v| {
                    let gkey = record_id_from_row(&v, None)?;
                    Some(GameDto {
                        id: gkey,
                        name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        year_published: v.get("year_published").and_then(|x| x.as_i64()).map(|y| y as i32),
                        bgg_id: v.get("bgg_id").and_then(|x| x.as_i64()).map(|b| b as i32),
                        description: v.get("description").and_then(|x| x.as_str()).map(String::from),
                        source: shared::models::game::GameSource::Database,
                    })
                })
                .collect()
        };

        let outcomes_arr = row.get("outcomes").and_then(|v| v.as_array()).map(|a| a.as_slice()).unwrap_or(&[]);
        let player_ids: Vec<String> = outcomes_arr
            .iter()
            .filter_map(|o| record_id_from_field(o, "player_id"))
            .collect();
        let player_things = strings_to_thing_array(&player_ids);
        let player_map: std::collections::HashMap<String, (String, String)> = if player_things.is_empty() {
            std::collections::HashMap::new()
        } else {
            let prows: Vec<serde_json::Value> = db
                .query("SELECT * FROM player WHERE id INSIDE $ids")
                .bind(("ids", player_things))
                .await
                .ok()
                .and_then(|mut r| r.take(0).ok())
                .unwrap_or_default();
            prows
                .into_iter()
                .filter_map(|p| {
                    let pid = record_id_from_row(&p, None)?;
                    let handle = p.get("handle").and_then(|x| x.as_str()).map(String::from).unwrap_or_else(|| record_id_to_key(&pid, "player"));
                    let email = p.get("email").and_then(|x| x.as_str()).map(String::from).unwrap_or_default();
                    Some((pid, (handle, email)))
                })
                .collect()
        };
        let outcomes: Vec<OutcomeDto> = outcomes_arr
            .iter()
            .filter_map(|o| {
                let player_id = record_id_from_field(o, "player_id")?;
                let (handle, email) = player_map.get(&player_id).cloned().unwrap_or_else(|| (record_id_to_key(&player_id, "player"), String::new()));
                let place = o.get("place").and_then(|x| x.as_i64()).map(|p| p.to_string()).or_else(|| o.get("place").and_then(|x| x.as_str()).map(String::from)).unwrap_or_default();
                let result = o.get("result").and_then(|x| x.as_str()).unwrap_or("").to_string();
                Some(OutcomeDto { player_id, handle, email, place, result })
            })
            .collect();

        let creator_id = contest_data.get("creator_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let created_at = contest_data
            .get("created_at")
            .and_then(|x| x.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

        log::info!("✅ Contest details loaded via fn::contest_with_edges for: {}", id_str);
        Some(ContestDto {
            id: id_str,
            name,
            start,
            stop,
            venue: venue_dto,
            games,
            outcomes,
            creator_id,
            created_at,
        })
    }

    async fn find_details_by_id_impl(&self, id: &str, db: &crate::db::Db) -> Option<ContestDto> {
        log::info!("🔍 Finding comprehensive contest details by ID: {}", id);
        let key = record_id_to_key(id, "contest");
        if key.is_empty() {
            log::warn!("❌ Empty key extracted from contest id: {}", id);
            return None;
        }
        // Optional: use SurrealDB function for one round-trip (contest + edges). Apply docs/surreal-functions.surql to enable.
        if let Some(dto) = self.find_details_via_function(db, id, &key).await {
            return Some(dto);
        }
        // Prefer fn::contest_row when applied, then string key, then numeric key (Arango import).
        let mut contest_data: Option<serde_json::Value> = None;
        let mut edge_key_int: Option<i64> = None;
        if let Ok(mut r) = db.query("SELECT fn::contest_row($key) AS result FROM [1]").bind(("key", key.clone())).await {
            let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
            if let Some(first) = rows.into_iter().next() {
                let v = first.get("result").or_else(|| first.get("fn::contest_row($key)")).cloned().unwrap_or(first);
                if v.is_object() && !v.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    contest_data = Some(v);
                }
            }
        }
        if contest_data.is_none() {
            contest_data = match db
                .query("SELECT * FROM contest WHERE id = type::record('contest', $key) LIMIT 1")
                .bind(("key", key.clone()))
                .await
            {
                Ok(mut r) => {
                    let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
                    rows.into_iter().next()
                }
                Err(e) => {
                    log::debug!("contest by-id query failed: {}", e);
                    None
                }
            };
        }
        if contest_data.is_none() && key.parse::<i64>().is_ok() {
            if let Ok(k) = key.parse::<i64>() {
                if let Ok(mut r) = db
                    .query("SELECT * FROM contest WHERE id = type::record('contest', $key) LIMIT 1")
                    .bind(("key", k))
                    .await
                {
                    let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
                    if let Some(row) = rows.into_iter().next() {
                        contest_data = Some(row);
                        edge_key_int = Some(k);
                    }
                }
            }
        }
        let contest_data = if let Some(row) = contest_data {
            row
        } else if let Some(row) = select_one_by_record_id(db, "contest", id).await {
            row
        } else {
            log::warn!("❌ No contest found with ID: {} (key={})", id, key);
            return None;
        };

        let id = record_id_from_row(&contest_data, None).unwrap_or_else(|| id.to_string());
        let name = contest_data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let start_str = contest_data.get("start").and_then(|v| v.as_str()).unwrap_or("");
        let stop_str = contest_data.get("stop").and_then(|v| v.as_str()).unwrap_or("");

        let start = chrono::DateTime::parse_from_rfc3339(start_str).ok().map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));
        let stop = chrono::DateTime::parse_from_rfc3339(stop_str).ok().map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));
        let (start, stop) = match (start, stop) {
            (Some(s), Some(st)) => (s, st),
            _ => {
                log::error!("❌ Failed to parse contest start/stop dates");
                return None;
            }
        };

        // Venue: played_at has in=contest, out=venue (use same key type that matched contest)
        let venue_res = if let Some(k) = edge_key_int {
            db.query("SELECT `out` AS venue_id FROM played_at WHERE `in` = type::record('contest', $key) LIMIT 1")
                .bind(("key", k))
        } else {
            db.query("SELECT `out` AS venue_id FROM played_at WHERE `in` = type::record('contest', $key) LIMIT 1")
                .bind(("key", key.clone()))
        }
            .await
            .ok();
        let venue_rows: Vec<serde_json::Value> = venue_res.and_then(|mut r| r.take(0).ok()).unwrap_or_default();
        let venue_id_opt: Option<String> = venue_rows.into_iter().next().and_then(|v: serde_json::Value| v.get("venue_id").and_then(|x| x.as_str()).map(String::from));

        let venue_dto = if let Some(venue_rid) = venue_id_opt {
            let vkey = record_id_to_key(&venue_rid, "venue");
            let vres = db.query("SELECT * FROM venue WHERE id = type::record('venue', $key)").bind(("key", vkey.clone())).await.ok();
            let vrows: Vec<serde_json::Value> = vres.and_then(|mut r| r.take(0).ok()).unwrap_or_default();
            vrows.into_iter().next().map(|v: serde_json::Value| {
                let id = record_id_from_row(&v, None).unwrap_or_else(|| format!("venue/{}", vkey));
                VenueDto {
                    id,
                    display_name: v.get("displayName").or(v.get("display_name")).and_then(|x| x.as_str()).unwrap_or("Unknown Venue").to_string(),
                    formatted_address: v.get("formattedAddress").or(v.get("formatted_address")).and_then(|x| x.as_str()).unwrap_or("Address not available").to_string(),
                    place_id: v.get("placeId").or(v.get("place_id")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    lat: v.get("lat").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    lng: v.get("lng").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    timezone: v.get("timezone").and_then(|x| x.as_str()).unwrap_or("UTC").to_string(),
                    source: shared::models::venue::VenueSource::Database,
                }
            }).unwrap_or_else(|| VenueDto {
                id: format!("venue/{}", vkey),
                display_name: "Unknown Venue".to_string(),
                formatted_address: "Address not available".to_string(),
                place_id: String::new(),
                lat: 0.0,
                lng: 0.0,
                timezone: "UTC".to_string(),
                source: shared::models::venue::VenueSource::Database,
            })
        } else {
            VenueDto {
                id: String::new(),
                display_name: "Unknown Venue".to_string(),
                formatted_address: "Address not available".to_string(),
                place_id: String::new(),
                lat: 0.0,
                lng: 0.0,
                timezone: "UTC".to_string(),
                source: shared::models::venue::VenueSource::Database,
            }
        };

        // Games: played_with has in=contest, out=game
        let games_res = if let Some(k) = edge_key_int {
            db.query("SELECT `out` AS game_id FROM played_with WHERE `in` = type::record('contest', $key)")
                .bind(("key", k))
        } else {
            db.query("SELECT `out` AS game_id FROM played_with WHERE `in` = type::record('contest', $key)")
                .bind(("key", key.clone()))
        }
            .await
            .ok();
        let game_rows: Vec<serde_json::Value> = games_res.and_then(|mut r| r.take(0).ok()).unwrap_or_default();
        let game_rids: Vec<String> = game_rows.into_iter().filter_map(|v| v.get("game_id").and_then(|x| x.as_str()).map(String::from)).collect();

        let mut games = Vec::new();
        for rid in game_rids {
            let gkey = record_id_to_key(&rid, "game");
            let qres = db.query("SELECT * FROM game WHERE id = type::record('game', $key)").bind(("key", gkey.clone())).await.ok();
            let rows: Vec<serde_json::Value> = qres.and_then(|mut r| r.take(0).ok()).unwrap_or_default();
            if let Some(v) = rows.into_iter().next() {
                games.push(GameDto {
                    id: record_id_from_row(&v, None).unwrap_or_else(|| format!("game/{}", gkey)),
                    name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    year_published: v.get("year_published").and_then(|x| x.as_i64()).map(|y| y as i32),
                    bgg_id: v.get("bgg_id").and_then(|x| x.as_i64()).map(|b| b as i32),
                    description: v.get("description").and_then(|x| x.as_str()).map(String::from),
                    source: shared::models::game::GameSource::Database,
                });
            }
        }

        // Outcomes: resulted_in has in=contest, out=player
        let out_res = if let Some(k) = edge_key_int {
            db.query("SELECT `out` AS player_id, place, result FROM resulted_in WHERE `in` = type::record('contest', $key) ORDER BY place ASC")
                .bind(("key", k))
        } else {
            db.query("SELECT `out` AS player_id, place, result FROM resulted_in WHERE `in` = type::record('contest', $key) ORDER BY place ASC")
                .bind(("key", key.clone()))
        }
            .await
            .ok();
        let outcome_rows: Vec<serde_json::Value> = out_res.and_then(|mut r| r.take(0).ok()).unwrap_or_default();

        let mut outcomes = Vec::new();
        for row in outcome_rows {
            let player_rid = match row.get("player_id").and_then(|x| x.as_str()) {
                Some(r) => r.to_string(),
                None => continue,
            };
            let pkey = record_id_to_key(&player_rid, "player");
            let pres = db.query("SELECT * FROM player WHERE id = type::record('player', $key)").bind(("key", pkey.clone())).await.ok();
            let prow: Vec<serde_json::Value> = pres.and_then(|mut r| r.take(0).ok()).unwrap_or_default();
            let player = prow.into_iter().next();
            let handle = player.as_ref().and_then(|p| p.get("handle").and_then(|x| x.as_str()).map(String::from)).unwrap_or_else(|| pkey.clone());
            let email = player.as_ref().and_then(|p| p.get("email").and_then(|x| x.as_str()).map(String::from)).unwrap_or_default();
            let place = row.get("place").and_then(|x| x.as_i64()).map(|p| p.to_string()).or_else(|| row.get("place").and_then(|x| x.as_str()).map(String::from)).unwrap_or_default();
            let result = row.get("result").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let player_id = format!("player/{}", pkey);
            outcomes.push(OutcomeDto { player_id, handle, email, place, result });
        }

        let contest_dto = ContestDto {
            id: id.to_string(),
            name,
            start,
            stop,
            venue: venue_dto,
            games,
            outcomes,
            creator_id: contest_data.get("creator_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            created_at: contest_data.get("created_at").and_then(|x| x.as_str()).and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
        };
        log::info!("✅ Successfully created ContestDto for contest: {}", id);
        Some(contest_dto)
    }
}

impl ContestRepositoryImpl {
    /// SurrealQL: contest has at least one played_with edge with `in` in game_ids.
    pub(crate) fn build_game_filter_clause(_game_ids_full: &Vec<String>) -> Option<String> {
        if _game_ids_full.is_empty() {
            return None;
        }
        Some("id IN (SELECT VALUE in FROM played_with WHERE out IN $game_rids)".to_string())
    }

    pub async fn search_contests(
        &self,
        q: &str,
        start_from: Option<&str>,
        start_to: Option<&str>,
        stop_from: Option<&str>,
        stop_to: Option<&str>,
        venue_id: Option<&str>,
        game_ids: &Vec<String>,
        sort_by: &str,
        sort_dir: &str,
        page: u32,
        page_size: u32,
        scope: &str,
        player_id: &str,
        filter_player_id: Option<&str>,
        db: Option<&crate::db::Db>,
    ) -> Result<serde_json::Value, String> {
        let db = db.unwrap_or(&self.db);
        let venue_key = venue_id.map(|v| record_id_to_key(v, "venue"));
        let game_keys: Vec<String> = game_ids
            .iter()
            .map(|g| record_id_to_key(g, "game"))
            .filter(|k| !k.is_empty())
            .collect();
        let game_things: Vec<surrealdb::types::RecordId> = game_keys
            .iter()
            .map(|k| surrealdb::types::RecordId::new("game", k.as_str()))
            .collect();
        let player_key = if player_id.is_empty() {
            None
        } else {
            let k = record_id_to_key(player_id, "player");
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        };
        let filter_player_key = filter_player_id.and_then(|pid| {
            let k = record_id_to_key(pid, "player");
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        });

        let skip = ((page.saturating_sub(1)) as u64) * (page_size as u64);
        let sort_col = match sort_by {
            "stop" => "stop",
            "created_at" => "id",
            _ => "start",
        };
        let order_dir = if sort_dir.eq_ignore_ascii_case("asc") { "ASC" } else { "DESC" };

        // Fast path: no filters (scope=all, no venue/game/player filter) — query contest table directly so we don't depend on played_at/played_with edges or INSIDE binding.
        let no_filters = scope == "all"
            && venue_key.is_none()
            && game_things.is_empty()
            && filter_player_key.is_none();
        if no_filters {
            log::info!("contest search: fast path (scope=all, no venue/game/player filter)");
            let mut where_parts: Vec<String> = vec![];
            if !q.is_empty() {
                where_parts.push("string::contains(string::lowercase(name), string::lowercase($q))".to_string());
            }
            if start_from.is_some() {
                where_parts.push("start >= $start_from".to_string());
            }
            if start_to.is_some() {
                where_parts.push("start <= $start_to".to_string());
            }
            if stop_from.is_some() {
                where_parts.push("stop >= $stop_from".to_string());
            }
            if stop_to.is_some() {
                where_parts.push("stop <= $stop_to".to_string());
            }
            let where_clause = if where_parts.is_empty() {
                "true".to_string()
            } else {
                where_parts.join(" AND ")
            };
            let count_sql = format!("SELECT count() FROM contest WHERE {} GROUP ALL", where_clause);
            let mut count_q = db.query(&count_sql);
            if !q.is_empty() {
                count_q = count_q.bind(("q", q.to_string()));
            }
            if let Some(sf) = start_from {
                count_q = count_q.bind(("start_from", sf.to_string()));
            }
            if let Some(st) = start_to {
                count_q = count_q.bind(("start_to", st.to_string()));
            }
            if let Some(ef) = stop_from {
                count_q = count_q.bind(("stop_from", ef.to_string()));
            }
            if let Some(et) = stop_to {
                count_q = count_q.bind(("stop_to", et.to_string()));
            }
            let count_res: Vec<serde_json::Value> = count_q.await.map_err(|e| {
                log::error!("contest search count query failed: {}", e);
                e.to_string()
            })?.take(0).unwrap_or_default();
            let total: u64 = count_res
                .into_iter()
                .next()
                .and_then(|v| v.as_u64().or_else(|| v.get("count").and_then(|c| c.as_u64())))
                .unwrap_or(0);
            log::info!("contest search fast path: total={} (page={}, page_size={})", total, page, page_size);

            let items_sql = format!(
                "SELECT string::concat(id) AS id, name, start, stop FROM contest WHERE {} ORDER BY {} {} LIMIT {} START {}",
                where_clause, sort_col, order_dir, page_size, skip
            );
            let mut items_q = db.query(&items_sql);
            if !q.is_empty() {
                items_q = items_q.bind(("q", q.to_string()));
            }
            if let Some(sf) = start_from {
                items_q = items_q.bind(("start_from", sf.to_string()));
            }
            if let Some(st) = start_to {
                items_q = items_q.bind(("start_to", st.to_string()));
            }
            if let Some(ef) = stop_from {
                items_q = items_q.bind(("stop_from", ef.to_string()));
            }
            if let Some(et) = stop_to {
                items_q = items_q.bind(("stop_to", et.to_string()));
            }
            let rows: Vec<serde_json::Value> = items_q.await.map_err(|e| {
                log::error!("contest search items query failed: {}", e);
                e.to_string()
            })?.take(0).unwrap_or_default();
            log::info!("contest search fast path: rows from DB={}", rows.len());
            if let Some(first) = rows.first() {
                log::debug!("contest search: first row id raw={:?}", first.get("id"));
            }

            let mut items = Vec::new();
            for row in &rows {
                let id_norm = record_id_from_row(row, None);
                if let Some(ref id_norm) = id_norm {
                    let key = record_id_to_key(id_norm, "contest");
                    if key.is_empty() {
                        log::warn!("contest search: record_id_to_key returned empty for id_norm={}", id_norm);
                        continue;
                    }
                    let id_canonical = format!("contest/{}", key);
                    match self.find_details_by_id_using(&id_canonical, db).await {
                        Some(dto) => {
                            items.push(serde_json::json!({
                                "_id": dto.id,
                                "name": dto.name,
                                "start": dto.start,
                                "stop": dto.stop,
                                "venue": dto.venue,
                                "games": dto.games,
                                "outcomes": dto.outcomes
                            }));
                        }
                        None => {
                            // Single-record lookup failed (e.g. id format mismatch); use list row so table still shows contests
                            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            items.push(serde_json::json!({
                                "_id": id_norm,
                                "name": name,
                                "start": row.get("start").cloned().unwrap_or(serde_json::Value::Null),
                                "stop": row.get("stop").cloned().unwrap_or(serde_json::Value::Null),
                                "venue": serde_json::json!({}),
                                "games": [],
                                "outcomes": []
                            }));
                        }
                    }
                } else {
                    log::warn!("contest search: record_id_from_row returned None for row keys={:?}", row.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                }
            }
            log::info!("🔍 Search (fast path) returned {} items (page {} of size {}), total {}", items.len(), page, page_size, total);
            return Ok(serde_json::json!({"items": items, "total": total, "page": page, "page_size": page_size}));
        }

        // Two-query approach: get contest ids from edges (in=contest, out=venue/game). Use Thing bindings so SurrealDB v2 matches.
        let played_at_sql = if venue_key.is_some() {
            "SELECT `in` AS out FROM played_at WHERE `out` = type::record('venue', $venue_key)"
        } else {
            "SELECT `in` AS out FROM played_at"
        };
        let mut pa_q = db.query(played_at_sql);
        if let Some(ref k) = venue_key {
            pa_q = pa_q.bind(("venue_key", k.clone()));
        }
        let pa_rows: Vec<serde_json::Value> = pa_q.await.map_err(|e| e.to_string())?.take(0).unwrap_or_default();
        let played_at_rids: HashSet<String> = pa_rows.iter().filter_map(|row| record_id_from_field(row, "out")).collect();

        let played_with_sql = if game_things.is_empty() {
            "SELECT `in` AS out FROM played_with"
        } else {
            "SELECT `in` AS out FROM played_with WHERE `out` INSIDE $game_things"
        };
        let mut pw_q = db.query(played_with_sql);
        if !game_things.is_empty() {
            pw_q = pw_q.bind(("game_things", game_things.clone()));
        }
        let pw_rows: Vec<serde_json::Value> = pw_q.await.map_err(|e| e.to_string())?.take(0).unwrap_or_default();
        let played_with_rids: HashSet<String> = pw_rows.iter().filter_map(|row| record_id_from_field(row, "out")).collect();

        let mut contest_ids: HashSet<String> = played_at_rids.intersection(&played_with_rids).cloned().collect();

        // When no venue/game filters and edge intersection is empty (e.g. fresh DB or import without edges), list all contests from the contest table
        if contest_ids.is_empty() && venue_key.is_none() && game_keys.is_empty() {
            let all_res: Vec<serde_json::Value> = db
                .query("SELECT string::concat(id) AS id FROM contest")
                .await
                .map_err(|e| e.to_string())?
                .take(0)
                .unwrap_or_default();
            let all_ids: HashSet<String> = all_res
                .iter()
                .filter_map(|row| record_id_from_row(row, None))
                .collect();
            if !all_ids.is_empty() {
                contest_ids = all_ids;
            }
        }

        if scope != "all" {
            if let Some(ref pk) = player_key {
                let ri_q = db
                    .query("SELECT `in` AS out FROM resulted_in WHERE `out` = type::record('player', $player_key)")
                    .bind(("player_key", pk.clone()));
                let ri_rows: Vec<serde_json::Value> = ri_q.await.map_err(|e| e.to_string())?.take(0).unwrap_or_default();
                let result_rids: HashSet<String> = ri_rows.iter().filter_map(|row| record_id_from_field(row, "out")).collect();
                contest_ids = contest_ids.intersection(&result_rids).cloned().collect();
            } else {
                contest_ids.clear();
            }
        }

        if let Some(ref fpk) = &filter_player_key {
            let ri_q = db
                .query("SELECT `in` AS out FROM resulted_in WHERE `out` = type::record('player', $filter_player_key)")
                .bind(("filter_player_key", fpk.clone()));
            let ri_rows: Vec<serde_json::Value> = ri_q.await.map_err(|e| e.to_string())?.take(0).unwrap_or_default();
            let result_rids: HashSet<String> = ri_rows.iter().filter_map(|row| record_id_from_field(row, "out")).collect();
            contest_ids = contest_ids.intersection(&result_rids).cloned().collect();
        }

        let contest_ids_vec: Vec<String> = contest_ids.into_iter().collect();
        let contest_things = strings_to_thing_array(&contest_ids_vec);
        if contest_things.is_empty() {
            log::info!("🔍 Search: no contest ids after edge intersection, returning empty");
            return Ok(serde_json::json!({
                "items": [],
                "total": 0u64,
                "page": page,
                "page_size": page_size
            }));
        }

        let mut where_parts = vec!["id INSIDE $contest_ids".to_string()];
        if !q.is_empty() {
            where_parts.push("string::contains(string::lowercase(name), string::lowercase($q))".to_string());
        }
        if start_from.is_some() {
            where_parts.push("start >= $start_from".to_string());
        }
        if start_to.is_some() {
            where_parts.push("start <= $start_to".to_string());
        }
        if stop_from.is_some() {
            where_parts.push("stop >= $stop_from".to_string());
        }
        if stop_to.is_some() {
            where_parts.push("stop <= $stop_to".to_string());
        }
        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT count() FROM contest WHERE {}", where_clause);
        let mut count_q = db.query(&count_sql).bind(("contest_ids", contest_things.clone()));
        if !q.is_empty() {
            count_q = count_q.bind(("q", q.to_string()));
        }
        if let Some(sf) = start_from {
            count_q = count_q.bind(("start_from", sf.to_string()));
        }
        if let Some(st) = start_to {
            count_q = count_q.bind(("start_to", st.to_string()));
        }
        if let Some(ef) = stop_from {
            count_q = count_q.bind(("stop_from", ef.to_string()));
        }
        if let Some(et) = stop_to {
            count_q = count_q.bind(("stop_to", et.to_string()));
        }
        let count_res: Vec<serde_json::Value> = count_q.await.map_err(|e| e.to_string())?.take(0).unwrap_or_default();
        let total: u64 = count_res.into_iter().next().and_then(|v: serde_json::Value| v.get("count").and_then(|c| c.as_u64())).unwrap_or(0);

        let items_sql = format!(
            "SELECT string::concat(id) AS id, name, start, stop FROM contest WHERE {} ORDER BY {} {} LIMIT {} START {}",
            where_clause, sort_col, order_dir, page_size, skip
        );
        let mut items_q = db.query(&items_sql).bind(("contest_ids", contest_things));
        if !q.is_empty() {
            items_q = items_q.bind(("q", q.to_string()));
        }
        if let Some(sf) = start_from {
            items_q = items_q.bind(("start_from", sf.to_string()));
        }
        if let Some(st) = start_to {
            items_q = items_q.bind(("start_to", st.to_string()));
        }
        if let Some(ef) = stop_from {
            items_q = items_q.bind(("stop_from", ef.to_string()));
        }
        if let Some(et) = stop_to {
            items_q = items_q.bind(("stop_to", et.to_string()));
        }
        let rows: Vec<serde_json::Value> = items_q.await.map_err(|e| e.to_string())?.take(0).unwrap_or_default();

        let mut items = Vec::new();
        for row in rows {
            let id_norm = record_id_from_row(&row, None);
            if let Some(id_norm) = id_norm {
                let key = record_id_to_key(&id_norm, "contest");
                if key.is_empty() {
                    continue;
                }
                let id_canonical = format!("contest/{}", key);
                if let Some(dto) = self.find_details_by_id_using(&id_canonical, db).await {
                    items.push(serde_json::json!({
                        "_id": dto.id,
                        "name": dto.name,
                        "start": dto.start,
                        "stop": dto.stop,
                        "venue": dto.venue,
                        "games": dto.games,
                        "outcomes": dto.outcomes
                    }));
                }
            }
        }

        log::info!("🔍 Search query returned {} items (page {} of size {}), total {}", items.len(), page, page_size, total);
        Ok(serde_json::json!({"items": items, "total": total, "page": page, "page_size": page_size}))
    }
}

#[cfg(test)]
mod repository_unit_tests {
    use super::ContestRepositoryImpl;

    #[test]
    fn game_filter_clause_empty_is_none() {
        let ids: Vec<String> = vec![];
        assert!(ContestRepositoryImpl::build_game_filter_clause(&ids).is_none());
    }

    #[test]
    fn game_filter_clause_non_empty_uses_any_semantics() {
        let ids = vec!["game/abc".to_string(), "game/def".to_string()];
        let clause = ContestRepositoryImpl::build_game_filter_clause(&ids).expect("some");
        assert!(clause.contains("played_with"));
        assert!(clause.contains("game_rids"));
    }
}
