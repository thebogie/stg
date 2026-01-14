use crate::contest::name_generator::generate_contest_name;
use crate::game::repository::GameRepositoryImpl;
use crate::game::usecase::{GameUseCase, GameUseCaseImpl};
use crate::player::repository::{PlayerRepository, PlayerRepositoryImpl};
use crate::player::usecase::{PlayerUseCase, PlayerUseCaseImpl};
use crate::venue::repository::VenueRepositoryImpl;
use crate::venue::usecase::{VenueUseCase, VenueUseCaseImpl};
use arangors::client::reqwest::ReqwestClient;
use arangors::document::options::InsertOptions;
use arangors::Database;
use argon2::{Argon2, PasswordHasher};
use async_trait::async_trait;
use shared::dto::contest::{ContestDto, OutcomeDto};
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use shared::models::contest::Contest;
use shared::models::relations::{PlayedAt, PlayedWith, ResultedIn};
use shared::SharedError;
use uuid::Uuid;

pub struct ContestRepositoryImpl {
    pub db: Database<ReqwestClient>,
    pub google_config: Option<(String, String)>,
    pub venue_usecase: VenueUseCaseImpl<VenueRepositoryImpl>,
    pub game_usecase: GameUseCaseImpl<GameRepositoryImpl>,
    pub player_usecase: PlayerUseCaseImpl<PlayerRepositoryImpl>,
}

impl ContestRepositoryImpl {
    pub fn new_with_google_config(
        db: Database<ReqwestClient>,
        google_config: Option<(String, String)>,
    ) -> Self {
        let venue_repo = VenueRepositoryImpl::new(db.clone(), google_config.clone());
        let venue_usecase = VenueUseCaseImpl { repo: venue_repo };
        let game_repo = GameRepositoryImpl::new(db.clone());
        let game_usecase = GameUseCaseImpl { repo: game_repo };
        let player_repo = PlayerRepositoryImpl::new(db.clone());
        let player_usecase = PlayerUseCaseImpl { repo: player_repo };
        Self {
            db,
            google_config,
            venue_usecase,
            game_usecase,
            player_usecase,
        }
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
        let now = chrono::Utc::now().fixed_offset();
        let contest = Contest {
            id: contest_dto.id.clone(),
            rev: "1".to_string(),
            name: contest_dto.name.clone(),
            start: contest_dto.start,
            stop: contest_dto.stop,
            creator_id: creator_id.clone(),
            created_at: now,
        };

        log::info!("📄 Contest model created: id='{}', name='{}', start='{}', stop='{}', creator='{}', created_at='{}'", 
            contest.id, contest.name, contest.start, contest.stop, contest.creator_id, contest.created_at);

        // Insert the contest document
        log::info!("💾 Getting contest collection from database...");
        let contest_collection = match self.db.collection("contest").await {
            Ok(collection) => {
                log::info!("✅ Successfully got contest collection");
                collection
            }
            Err(e) => {
                log::error!("💥 Failed to get contest collection: {}", e);
                return Err(format!("Failed to get contest collection: {}", e));
            }
        };

        log::info!("💾 Inserting contest document with options: return_new=true");
        let insert_options = InsertOptions::builder().return_new(true).build();
        let result = match contest_collection
            .create_document(contest, insert_options)
            .await
        {
            Ok(result) => {
                log::info!("✅ Successfully inserted contest document");
                result
            }
            Err(e) => {
                log::error!("💥 Failed to create contest document: {}", e);
                return Err(format!("Failed to create contest: {}", e));
            }
        };

        let created_contest: Contest = match result.new_doc() {
            Some(doc) => {
                log::info!("✅ Got new document from insert result");
                doc.clone()
            }
            None => {
                log::error!("💥 No document returned after creation");
                return Err(format!("No document returned after creation"));
            }
        };

        log::info!(
            "✅ Contest document created successfully: id='{}', name='{}'",
            created_contest.id,
            created_contest.name
        );

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

            // Helper: check if player_id is a real DB id (ArangoDB format)
            fn is_real_player_id(player_id: &str) -> bool {
                player_id.starts_with("player/") && player_id.len() > 7
            }

            let player = if !player_id.is_empty() && is_real_player_id(&player_id) {
                log::info!("👥 Looking up existing player with ID: {}", player_id);
                // Fetch existing player by ID (ArangoDB _id format)
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
                    id: String::new(), // Will be set by ArangoDB
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
            start: created_contest.start,
            stop: created_contest.stop,
            venue: venue_dto,
            games: game_dtos,
            outcomes: processed_outcomes,
            creator_id: created_contest.creator_id.clone(),
            created_at: Some(created_contest.created_at),
        };

        log::info!("✅ Contest creation process completed successfully!");
        log::info!("✅ Final contest DTO: id='{}', name='{}', {} games, {} outcomes, creator='{}', created_at='{}'",
            created_dto.id, created_dto.name, created_dto.games.len(), created_dto.outcomes.len(),
            created_dto.creator_id, created_dto.created_at.unwrap_or_else(|| chrono::Utc::now().fixed_offset()));

        Ok(created_dto)
    }
    async fn find_by_id(&self, id: &str) -> Option<Contest> {
        log::info!("🔍 Finding contest by ID: {}", id);

        // Extract collection and key from the full ID
        let (collection, key) = if id.contains('/') {
            let parts: Vec<&str> = id.splitn(2, '/').collect();
            if parts.len() == 2 {
                (parts[0], parts[1])
            } else {
                log::error!("💥 Invalid ID format: {}", id);
                return None;
            }
        } else {
            ("contest", id)
        };

        log::debug!("🔍 Using collection: {}, key: {}", collection, key);

        // Get the contest document
        let contest_collection = match self.db.collection(collection).await {
            Ok(collection) => collection,
            Err(e) => {
                log::error!("💥 Failed to get contest collection: {}", e);
                return None;
            }
        };

        let contest_doc = match contest_collection.document(key).await {
            Ok(doc) => doc,
            Err(e) => {
                log::error!("💥 Failed to get contest document: {}", e);
                return None;
            }
        };

        let contest: Contest = contest_doc.document;

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
        log::info!(
            "🔍 Finding contests for player {} and game {}",
            player_id,
            game_id
        );

        log::info!(
            "🔍 Executing AQL query for player {} and game {}",
            player_id,
            game_id
        );

        // First, let's debug what contests this player has
        let debug_query = arangors::AqlQuery::builder()
            .query(
                r#"
        FOR contest IN 1..1 OUTBOUND @player_id resulted_in
        RETURN {
            contest_id: contest._id,
            contest_name: contest.name,
            contest_date: contest.start
        }
        "#,
            )
            .bind_var("player_id", player_id)
            .build();

        match self.db.aql_query::<serde_json::Value>(debug_query).await {
            Ok(debug_results) => {
                log::info!(
                    "🔍 Debug: Player has {} total contests",
                    debug_results.len()
                );
                if debug_results.len() > 0 {
                    log::info!("🔍 Debug: First contest: {:?}", debug_results[0]);
                }
            }
            Err(e) => {
                log::warn!("🔍 Debug query failed: {}", e);
            }
        }

        // Let's also debug what games exist with this game_id
        let game_debug_query = arangors::AqlQuery::builder()
            .query(
                r#"
        FOR game IN game
        FILTER game._id == @game_id
        RETURN {
            game_id: game._id,
            game_name: game.name,
            game_key: game._key
        }
        "#,
            )
            .bind_var("game_id", game_id)
            .build();

        match self
            .db
            .aql_query::<serde_json::Value>(game_debug_query)
            .await
        {
            Ok(game_results) => {
                log::info!(
                    "🔍 Debug: Found {} games with game_id {}",
                    game_results.len(),
                    game_id
                );
                if game_results.len() > 0 {
                    log::info!("🔍 Debug: Game details: {:?}", game_results[0]);
                } else {
                    log::warn!("🔍 Debug: No game found with game_id {}", game_id);
                }
            }
            Err(e) => {
                log::warn!("🔍 Game debug query failed: {}", e);
            }
        }

        let query = arangors::AqlQuery::builder()
            .query(r#"
        FOR contest IN contest
        LET my_outcome = FIRST(FOR r IN resulted_in FILTER r._from == contest._id AND r._to == @player_id RETURN r)
        LET game = FIRST(FOR e IN played_with FILTER e._from == contest._id RETURN DOCUMENT(e._to))
        FILTER my_outcome != null AND game != null AND game._id == @game_id
        LET venue_edge = FIRST(FOR e IN played_at FILTER e._from == contest._id RETURN e)
        LET venue = venue_edge != null ? DOCUMENT(venue_edge._to) : null
        LET all_outcomes = (
            FOR outcome IN resulted_in
            FILTER outcome._from == contest._id
            LET player = DOCUMENT(outcome._to)
            SORT TO_NUMBER(outcome.place)
            RETURN {
                player_id: player._key,
                player_name: CONCAT(player.firstname, ' ', player.lastname),
                player_handle: player.handle,
                placement: outcome.place,
                result: outcome.result
            }
        )
        SORT contest.start DESC
        RETURN {
            contest_id: contest._id,
            contest_name: contest.name,
            contest_date: contest.start,
            contest_description: contest.description,
            contest_status: contest.status,
            game_id: game._key,
            game_name: game.name,
            game_year_published: game.year_published,
            venue_id: venue != null ? venue._key : null,
            venue_name: venue != null ? venue.name : "Unknown Venue",
            venue_display_name: venue != null ? venue.displayName : null,
            venue_address: venue != null ? venue.formattedAddress : null,
            my_placement: my_outcome.place,
            my_result: my_outcome.result,
            total_players: LENGTH(all_outcomes),
            players: all_outcomes
        }
        "#)
            .bind_var("player_id", player_id)
            .bind_var("game_id", game_id)
            .build();

        match self.db.aql_query::<serde_json::Value>(query).await {
            Ok(results) => {
                log::info!(
                    "✅ Found {} contests for player {} and game {}",
                    results.len(),
                    player_id,
                    game_id
                );
                if results.is_empty() {
                    log::warn!("⚠️ No contests found - this might indicate a data issue");
                    log::info!("🔍 Debug: player_id={}, game_id={}", player_id, game_id);
                }
                Ok(results)
            }
            Err(e) => {
                log::error!(
                    "💥 Failed to query contests for player {} and game {}: {}",
                    player_id,
                    game_id,
                    e
                );
                Err(format!("Database query failed: {}", e))
            }
        }
    }
}

impl ContestRepositoryImpl {
    /// Create a played_at relation between a contest and a venue
    async fn create_played_at_relation(
        &self,
        contest_id: &str,
        venue_id: &str,
    ) -> Result<(), SharedError> {
        log::info!("🔗 Creating PLAYED_AT edge...");
        log::info!(
            "🔗 Edge details: from='{}', to='{}', label='PLAYED_AT'",
            contest_id,
            venue_id
        );

        let edge_id = format!("played_at/{}", Uuid::new_v4());
        log::info!("🔗 Generated edge ID: {}", edge_id);

        let played_at = PlayedAt {
            id: edge_id.clone(),
            rev: String::new(),
            from: contest_id.to_string(),
            to: venue_id.to_string(),
            label: "PLAYED_AT".to_string(),
        };

        log::info!("🔗 Getting played_at collection...");
        let played_at_collection = match self.db.collection("played_at").await {
            Ok(collection) => {
                log::info!("✅ Successfully got played_at collection");
                collection
            }
            Err(e) => {
                log::error!("💥 Failed to get played_at collection: {}", e);
                return Err(SharedError::Database(format!(
                    "Failed to get played_at collection: {}",
                    e
                )));
            }
        };

        log::info!("🔗 Inserting PLAYED_AT edge document...");
        match played_at_collection
            .create_document(played_at, InsertOptions::default())
            .await
        {
            Ok(result) => {
                log::info!("✅ Successfully inserted PLAYED_AT edge document");
                if let Some(_doc) = result.new_doc() {
                    log::info!("✅ Got new document from edge creation");
                }
                log::info!("✅ PLAYED_AT edge creation completed successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("💥 Failed to create played_at relation: {}", e);
                Err(SharedError::Database(format!(
                    "Failed to create played_at relation: {}",
                    e
                )))
            }
        }
    }

    /// Create a played_with relation between a contest and a game
    async fn create_played_with_relation(
        &self,
        contest_id: &str,
        game_id: &str,
    ) -> Result<(), SharedError> {
        log::info!("🔗 Creating PLAYED_WITH edge...");
        log::info!(
            "🔗 Edge details: from='{}', to='{}', label='PLAYED_WITH'",
            contest_id,
            game_id
        );

        let edge_id = format!("played_with/{}", Uuid::new_v4());
        log::info!("🔗 Generated edge ID: {}", edge_id);

        let played_with = PlayedWith {
            id: edge_id.clone(),
            rev: String::new(),
            from: contest_id.to_string(),
            to: game_id.to_string(),
            label: "PLAYED_WITH".to_string(),
        };

        log::info!("🔗 Getting played_with collection...");
        let played_with_collection = match self.db.collection("played_with").await {
            Ok(collection) => {
                log::info!("✅ Successfully got played_with collection");
                collection
            }
            Err(e) => {
                log::error!("💥 Failed to get played_with collection: {}", e);
                return Err(SharedError::Database(format!(
                    "Failed to get played_with collection: {}",
                    e
                )));
            }
        };

        log::info!("🔗 Inserting PLAYED_WITH edge document...");
        match played_with_collection
            .create_document(played_with, InsertOptions::default())
            .await
        {
            Ok(_result) => {
                log::info!("✅ Successfully inserted PLAYED_WITH edge document");
                log::info!("✅ PLAYED_WITH edge creation completed successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("💥 Failed to create played_with relation: {}", e);
                Err(SharedError::Database(format!(
                    "Failed to create played_with relation: {}",
                    e
                )))
            }
        }
    }

    /// Create a resulted_in relation between a contest and a player
    async fn create_resulted_in_relation(
        &self,
        contest_id: &str,
        outcome: &OutcomeDto,
    ) -> Result<(), SharedError> {
        log::info!("🔗 Creating RESULTED_IN edge...");
        log::info!(
            "🔗 Edge details: from='{}', to='{}', label='RESULTED_IN', place='{}', result='{}'",
            contest_id,
            outcome.player_id,
            outcome.place,
            outcome.result
        );

        // Convert place from string to i32
        log::info!("🔗 Parsing place value: '{}'", outcome.place);
        let place = match outcome.place.parse::<i32>() {
            Ok(p) => {
                log::info!("✅ Successfully parsed place value: {}", p);
                p
            }
            Err(e) => {
                log::error!("💥 Failed to parse place value '{}': {}", outcome.place, e);
                return Err(SharedError::Validation(format!(
                    "Invalid place value: {}",
                    e
                )));
            }
        };

        let edge_id = format!("resulted_in/{}", Uuid::new_v4());
        log::info!("🔗 Generated edge ID: {}", edge_id);

        let resulted_in = ResultedIn {
            id: edge_id.clone(),
            rev: String::new(),
            from: contest_id.to_string(),
            to: outcome.player_id.clone(),
            label: "RESULTED_IN".to_string(),
            place,
            result: outcome.result.clone(),
        };

        log::info!("🔗 Getting resulted_in collection...");
        let resulted_in_collection = match self.db.collection("resulted_in").await {
            Ok(collection) => {
                log::info!("✅ Successfully got resulted_in collection");
                collection
            }
            Err(e) => {
                log::error!("💥 Failed to get resulted_in collection: {}", e);
                return Err(SharedError::Database(format!(
                    "Failed to get resulted_in collection: {}",
                    e
                )));
            }
        };

        log::info!("🔗 Inserting RESULTED_IN edge document...");
        match resulted_in_collection
            .create_document(resulted_in, InsertOptions::default())
            .await
        {
            Ok(_result) => {
                log::info!("✅ Successfully inserted RESULTED_IN edge document");
                log::info!("✅ RESULTED_IN edge creation completed successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("💥 Failed to create resulted_in relation: {}", e);
                Err(SharedError::Database(format!(
                    "Failed to create resulted_in relation: {}",
                    e
                )))
            }
        }
    }
}

impl ContestRepositoryImpl {
    pub async fn find_details_by_id(&self, id: &str) -> Option<ContestDto> {
        log::info!("🔍 Finding comprehensive contest details by ID: {}", id);

        // Use AQL to fetch comprehensive contest data
        let query = arangors::AqlQuery::builder()
            .query(r#"
                FOR contest IN contest
                FILTER contest._id == @contest_id
                LET venue = (
                    FOR played_at IN played_at
                    FILTER played_at._from == contest._id
                    FOR venue IN venue
                    FILTER played_at._to == venue._id
                    RETURN {
                        id: venue._id,
                        display_name: venue.displayName || venue.name || venue.title || "Unknown Venue",
                        formatted_address: venue.formattedAddress || venue.address || "Address not available",
                        place_id: venue.placeId,
                        lat: venue.lat,
                        lng: venue.lng,
                        timezone: venue.timezone || "UTC"
                    }
                )[0]
                LET games = (
                    FOR played_with IN played_with
                    FILTER played_with._from == contest._id
                    FOR game IN game
                    FILTER played_with._to == game._id
                    RETURN {
                        id: game._id,
                        name: game.name,
                        year_published: game.year_published,
                        bgg_id: game.bgg_id,
                        description: game.description
                    }
                )
                LET outcomes = (
                    FOR result IN resulted_in
                    FILTER result._from == contest._id
                    FOR player IN player
                    FILTER result._to == player._id
                    RETURN {
                        player_id: player._id,
                        handle: player.handle,
                        email: player.email,
                        place: TO_STRING(result.place),
                        result: result.result
                    }
                )
                RETURN {
                    _id: contest._id,
                    name: contest.name,
                    start: contest.start,
                    stop: contest.stop,
                    venue: venue,
                    games: games,
                    outcomes: outcomes
                }
            "#)
            .bind_var("contest_id", id)
            .build();

        match self.db.aql_query::<serde_json::Value>(query).await {
            Ok(mut cursor) => {
                log::info!("🔍 Contest details query returned {} results", cursor.len());
                if cursor.is_empty() {
                    log::warn!("❌ No contest found with ID: {}", id);
                    // Let's check if the contest exists at all
                    let check_query = arangors::AqlQuery::builder()
                        .query(r#"FOR contest IN contest FILTER contest._id == @contest_id RETURN contest._id"#)
                        .bind_var("contest_id", id)
                        .build();
                    match self.db.aql_query::<serde_json::Value>(check_query).await {
                        Ok(check_cursor) => {
                            log::info!("🔍 Contest exists check: {} results", check_cursor.len());
                            if !check_cursor.is_empty() {
                                log::warn!(
                                    "❌ Contest exists but missing relationships for ID: {}",
                                    id
                                );
                            } else {
                                log::warn!("❌ Contest does not exist at all for ID: {}", id);
                            }
                        }
                        Err(e) => log::error!("❌ Error checking contest existence: {}", e),
                    }
                    return None;
                }
                if let Some(result) = cursor.pop() {
                    log::info!("✅ Found comprehensive contest details for: {}", id);
                    log::debug!("📊 Raw AQL result: {:?}", result);

                    let contest_data: serde_json::Value = result;

                    // Extract contest data
                    log::debug!("🔍 Extracting contest data...");
                    let id = match contest_data["_id"].as_str() {
                        Some(id) => {
                            log::debug!("✅ Contest ID: {}", id);
                            id.to_string()
                        }
                        None => {
                            log::error!(
                                "❌ Failed to extract contest ID from: {:?}",
                                contest_data["_id"]
                            );
                            return None;
                        }
                    };

                    let name = match contest_data["name"].as_str() {
                        Some(name) => {
                            log::debug!("✅ Contest name: {}", name);
                            name.to_string()
                        }
                        None => {
                            log::error!(
                                "❌ Failed to extract contest name from: {:?}",
                                contest_data["name"]
                            );
                            return None;
                        }
                    };

                    let start_str = match contest_data["start"].as_str() {
                        Some(start) => {
                            log::debug!("✅ Contest start: {}", start);
                            start
                        }
                        None => {
                            log::error!(
                                "❌ Failed to extract contest start from: {:?}",
                                contest_data["start"]
                            );
                            return None;
                        }
                    };

                    let stop_str = match contest_data["stop"].as_str() {
                        Some(stop) => {
                            log::debug!("✅ Contest stop: {}", stop);
                            stop
                        }
                        None => {
                            log::error!(
                                "❌ Failed to extract contest stop from: {:?}",
                                contest_data["stop"]
                            );
                            return None;
                        }
                    };

                    // Parse dates
                    log::debug!("🔍 Parsing dates...");

                    // Get timezone from venue (will be processed later when we have venue data)
                    let _venue_timezone = "UTC".to_string(); // Default, will be updated from venue

                    let start = match chrono::DateTime::parse_from_rfc3339(start_str) {
                        Ok(dt) => {
                            log::debug!("✅ Start date parsed (RFC3339): {}", dt);
                            dt
                        }
                        Err(e) => {
                            log::error!(
                                "❌ Failed to parse start date '{}' as RFC3339: {}",
                                start_str,
                                e
                            );
                            return None;
                        }
                    };

                    let stop = match chrono::DateTime::parse_from_rfc3339(stop_str) {
                        Ok(dt) => {
                            log::debug!("✅ Stop date parsed (RFC3339): {}", dt);
                            dt
                        }
                        Err(e) => {
                            log::error!(
                                "❌ Failed to parse stop date '{}' as RFC3339: {}",
                                stop_str,
                                e
                            );
                            return None;
                        }
                    };

                    // Extract venue data
                    log::debug!("🔍 Extracting venue data...");
                    let venue_data = match contest_data["venue"].as_object() {
                        Some(venue) => {
                            log::debug!("✅ Venue data found: {:?}", venue);
                            venue
                        }
                        None => {
                            log::error!("❌ No venue data found");
                            return None;
                        }
                    };

                    let venue_dto = match (
                        venue_data["id"].as_str(),
                        venue_data["display_name"].as_str(),
                        venue_data["formatted_address"].as_str(),
                        venue_data["place_id"].as_str(),
                        venue_data["lat"].as_f64(),
                        venue_data["lng"].as_f64(),
                        venue_data["timezone"].as_str(),
                    ) {
                        (
                            Some(id),
                            Some(display_name),
                            Some(formatted_address),
                            Some(place_id),
                            Some(lat),
                            Some(lng),
                            timezone,
                        ) => {
                            log::debug!("✅ Venue fields extracted: id={}, display_name={}, formatted_address={}, place_id={}, lat={}, lng={}, timezone={}", 
                                id, display_name, formatted_address, place_id, lat, lng, timezone.unwrap_or("UTC"));
                            VenueDto {
                                id: id.to_string(),
                                display_name: display_name.to_string(),
                                formatted_address: formatted_address.to_string(),
                                place_id: place_id.to_string(),
                                lat,
                                lng,
                                timezone: timezone.unwrap_or("UTC").to_string(),
                                source: shared::models::venue::VenueSource::Database,
                            }
                        }
                        (
                            Some(id),
                            display_name,
                            formatted_address,
                            Some(place_id),
                            Some(lat),
                            Some(lng),
                            timezone,
                        ) => {
                            // Handle null display_name and formatted_address with defaults
                            let display_name = display_name.unwrap_or("Unknown Venue");
                            let formatted_address =
                                formatted_address.unwrap_or("Address not available");
                            log::debug!("✅ Venue fields extracted (with defaults): id={}, display_name={}, formatted_address={}, place_id={}, lat={}, lng={}, timezone={}", 
                                id, display_name, formatted_address, place_id, lat, lng, timezone.unwrap_or("UTC"));
                            VenueDto {
                                id: id.to_string(),
                                display_name: display_name.to_string(),
                                formatted_address: formatted_address.to_string(),
                                place_id: place_id.to_string(),
                                lat,
                                lng,
                                timezone: timezone.unwrap_or("UTC").to_string(),
                                source: shared::models::venue::VenueSource::Database,
                            }
                        }
                        (
                            Some(id),
                            display_name,
                            formatted_address,
                            place_id,
                            lat,
                            lng,
                            timezone,
                        ) => {
                            // More flexible handling - only require id, others can be null
                            let display_name = display_name.unwrap_or("Unknown Venue");
                            let formatted_address =
                                formatted_address.unwrap_or("Address not available");
                            let place_id = place_id.unwrap_or("Unknown Place");
                            let lat = lat.unwrap_or(0.0);
                            let lng = lng.unwrap_or(0.0);

                            log::debug!("✅ Venue fields extracted (flexible): id={}, display_name={}, formatted_address={}, place_id={}, lat={}, lng={}, timezone={}", 
                                id, display_name, formatted_address, place_id, lat, lng, timezone.unwrap_or("UTC"));
                            VenueDto {
                                id: id.to_string(),
                                display_name: display_name.to_string(),
                                formatted_address: formatted_address.to_string(),
                                place_id: place_id.to_string(),
                                lat,
                                lng,
                                timezone: timezone.unwrap_or("UTC").to_string(),
                                source: shared::models::venue::VenueSource::Database,
                            }
                        }
                        _ => {
                            log::error!("❌ Failed to extract venue fields: id={:?}, display_name={:?}, formatted_address={:?}, place_id={:?}, lat={:?}, lng={:?}", 
                                venue_data["id"], venue_data["display_name"], venue_data["formatted_address"], 
                                venue_data["place_id"], venue_data["lat"], venue_data["lng"]);
                            return None;
                        }
                    };

                    log::debug!("✅ Final venue DTO created: {:?}", venue_dto);

                    // Extract games data
                    log::debug!("🔍 Extracting games data...");
                    let games: Vec<GameDto> = match contest_data["games"].as_array() {
                        Some(games_array) => {
                            log::debug!("✅ Found {} games", games_array.len());
                            games_array.iter()
                                .enumerate()
                                .filter_map(|(i, game_json)| {
                                    log::debug!("🔍 Processing game {}: {:?}", i, game_json);
                                    let id = game_json["id"].as_str()?.to_string();
                                    let name = game_json["name"].as_str()?.to_string();
                                    let year_published = game_json["year_published"].as_i64().map(|y| y as i32);
                                    let bgg_id = game_json["bgg_id"].as_i64().map(|b| b as i32);
                                    let description = game_json["description"].as_str().map(|s| s.to_string());

                                    log::debug!("✅ Game {} extracted: id={}, name={}, year_published={:?}, bgg_id={:?}, description={:?}",
                                        i, id, name, year_published, bgg_id, description);

                                    Some(GameDto {
                                        id,
                                        name,
                                        year_published,
                                        bgg_id,
                                        description,
                                        source: shared::models::game::GameSource::Database,
                                    })
                                })
                                .collect()
                        }
                        None => {
                            log::warn!("⚠️ No games array found, using empty vector");
                            Vec::new()
                        }
                    };

                    log::debug!("✅ Extracted {} games", games.len());

                    // Extract outcomes data
                    log::debug!("🔍 Extracting outcomes data...");
                    let outcomes: Vec<OutcomeDto> = match contest_data["outcomes"].as_array() {
                        Some(outcomes_array) => {
                            log::debug!("✅ Found {} outcomes", outcomes_array.len());
                            outcomes_array.iter()
                                .enumerate()
                                .filter_map(|(i, outcome_json)| {
                                    log::debug!("🔍 Processing outcome {}: {:?}", i, outcome_json);
                                    let player_id = outcome_json["player_id"].as_str()?.to_string();
                                    let handle = outcome_json["handle"].as_str()?.to_string();
                                    let email = outcome_json["email"].as_str()?.to_string();
                                    let place = outcome_json["place"].as_str()?.to_string();
                                    let result = outcome_json["result"].as_str()?.to_string();

                                    log::debug!("✅ Outcome {} extracted: player_id={}, handle={}, email={}, place={}, result={}",
                                        i, player_id, handle, email, place, result);

                                    Some(OutcomeDto {
                                        player_id,
                                        place,
                                        result,
                                        email,
                                        handle,
                                    })
                                })
                                .collect()
                        }
                        None => {
                            log::warn!("⚠️ No outcomes array found, using empty vector");
                            Vec::new()
                        }
                    };

                    log::debug!("✅ Extracted {} outcomes", outcomes.len());

                    // Create ContestDto
                    log::debug!("🔍 Creating ContestDto...");
                    let contest_dto = ContestDto {
                        id: id.clone(),
                        name,
                        start,
                        stop,
                        venue: venue_dto,
                        games,
                        outcomes,
                        creator_id: String::new(), // Will be populated from contest data
                        created_at: None,          // Will be populated from contest data
                    };

                    log::info!("✅ Successfully created ContestDto for contest: {}", id);
                    Some(contest_dto)
                } else {
                    log::warn!("⚠️ No comprehensive contest details found for: {}", id);
                    None
                }
            }
            Err(e) => {
                log::error!("💥 Failed to query comprehensive contest details: {}", e);
                None
            }
        }
    }
}

impl ContestRepositoryImpl {
    /// Build the AQL filter clause for game_ids. Returns None when no game_ids provided.
    pub(crate) fn build_game_filter_clause(game_ids_full: &Vec<String>) -> Option<String> {
        if game_ids_full.is_empty() {
            return None;
        }
        Some("LENGTH(FOR e IN played_with FILTER e._from == contest._id AND e._to IN @game_ids RETURN 1) > 0".to_string())
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
    ) -> Result<serde_json::Value, String> {
        let venue_full = venue_id.map(|v| {
            if v.contains('/') {
                v.to_string()
            } else {
                format!("venue/{}", v)
            }
        });
        let game_full: Vec<String> = game_ids
            .iter()
            .map(|g| {
                if g.contains('/') {
                    g.clone()
                } else {
                    format!("game/{}", g)
                }
            })
            .collect();
        let player_full = if player_id.is_empty() {
            None
        } else {
            Some(if player_id.contains('/') {
                player_id.to_string()
            } else {
                format!("player/{}", player_id)
            })
        };

        let mut filters = Vec::new();
        if !q.is_empty() {
            filters.push("LIKE(contest.name, @q, true)".to_string());
        }
        if start_from.is_some() {
            filters.push("contest.start >= DATE_ISO8601(@start_from)".to_string());
        }
        if start_to.is_some() {
            filters.push("contest.start <= DATE_ISO8601(@start_to)".to_string());
        }
        if stop_from.is_some() {
            filters.push("contest.stop >= DATE_ISO8601(@stop_from)".to_string());
        }
        if stop_to.is_some() {
            filters.push("contest.stop <= DATE_ISO8601(@stop_to)".to_string());
        }
        if venue_full.is_some() {
            filters.push("venue_edge != null && venue_edge._to == @venue_id".to_string());
        }
        if let Some(game_clause) = Self::build_game_filter_clause(&game_full) {
            filters.push(game_clause);
        }

        let _scope_filter = match (scope, &player_full) {
            ("mine", Some(_)) | ("my_games", Some(_)) | ("my_venues", Some(_)) => "my_scope",
            ("all", _) => "all",
            _ => "my_scope",
        };

        let filter_clause = if filters.is_empty() {
            String::new()
        } else {
            format!("FILTER {}", filters.join(" AND "))
        };

        let sort_field = match sort_by {
            "stop" => "contest.stop",
            "created_at" => "contest._key",
            _ => "contest.start",
        };
        let sort_dir = if sort_dir.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };
        let skip = ((page.saturating_sub(1)) as u64) * (page_size as u64);

        let aql = format!(
            r#"
LET my_player = @player_id
FOR contest IN contest
    LET venue_edge = FIRST(FOR e IN played_at FILTER e._from == contest._id RETURN e)
    LET game_edge = FIRST(FOR e IN played_with FILTER e._from == contest._id RETURN e)
    LET mine = @scope == "all" ? true : (
        my_player != null && LENGTH(FOR r IN resulted_in FILTER r._from == contest._id AND r._to == my_player RETURN 1) > 0
    )
    // If scope is 'all', 'mine' evaluates true for everyone; otherwise enforce player-based filter
    FILTER (@scope == "all" || mine) AND venue_edge != null AND game_edge != null
    {filter_clause}
    SORT {sort_field} {sort_dir}
    LIMIT @skip, @limit
    LET venue = venue_edge != null ? DOCUMENT(venue_edge._to) : null
    LET games = (FOR e IN played_with FILTER e._from == contest._id 
        LET game = DOCUMENT(e._to)
        RETURN {{ id: game._id, name: game.name, year_published: game.year_published, bgg_id: game.bgg_id, description: game.description }})
    LET outcomes = (
        FOR r IN resulted_in FILTER r._from == contest._id
        LET player = DOCUMENT(r._to)
        RETURN {{ player_id: player._id, handle: player.handle, email: player.email, place: TO_STRING(r.place), result: r.result }}
    )
    RETURN {{
        _id: contest._id,
        name: contest.name,
        start: contest.start,
        stop: contest.stop,
        venue: venue == null ? null : {{
            id: venue._id,
            displayName: venue.displayName,
            formattedAddress: venue.formattedAddress,
            place_id: venue.placeId,
            lat: venue.lat,
            lng: venue.lng,
            timezone: venue.timezone || "UTC"
        }},
        games: games,
        outcomes: outcomes
    }}
"#
        );

        let mut bind_vars: std::collections::HashMap<&str, serde_json::Value> =
            std::collections::HashMap::new();
        if !q.is_empty() {
            bind_vars.insert("q", serde_json::Value::String(format!("%{}%", q)));
        }
        bind_vars.insert("scope", serde_json::Value::String(scope.to_string()));
        bind_vars.insert(
            "skip",
            serde_json::Value::Number(serde_json::Number::from(skip as i64)),
        );
        bind_vars.insert(
            "limit",
            serde_json::Value::Number(serde_json::Number::from(page_size as i64)),
        );
        // Use null when there is no player context so AQL 'my_player != null' works
        if let Some(player) = player_full.clone() {
            bind_vars.insert("player_id", serde_json::Value::String(player));
        } else {
            bind_vars.insert("player_id", serde_json::Value::Null);
        }

        if let Some(sf) = start_from {
            bind_vars.insert("start_from", serde_json::Value::String(sf.to_string()));
        }
        if let Some(st) = start_to {
            bind_vars.insert("start_to", serde_json::Value::String(st.to_string()));
        }
        if let Some(ef) = stop_from {
            bind_vars.insert("stop_from", serde_json::Value::String(ef.to_string()));
        }
        if let Some(et) = stop_to {
            bind_vars.insert("stop_to", serde_json::Value::String(et.to_string()));
        }
        if let Some(ref v) = venue_full {
            bind_vars.insert("venue_id", serde_json::Value::String(v.clone()));
        }
        if !game_full.is_empty() {
            bind_vars.insert(
                "game_ids",
                serde_json::Value::Array(
                    game_full
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        // First, compute the full total without LIMIT using a lightweight COUNT query
        let count_aql = format!(
            r#"
LET my_player = @player_id
FOR contest IN contest
    LET venue_edge = FIRST(FOR e IN played_at FILTER e._from == contest._id RETURN e)
    LET game_edge = FIRST(FOR e IN played_with FILTER e._from == contest._id RETURN e)
    LET mine = @scope == "all" ? true : (
        my_player != null && LENGTH(FOR r IN resulted_in FILTER r._from == contest._id AND r._to == my_player RETURN 1) > 0
    )
    FILTER (@scope == "all" || mine) AND venue_edge != null AND game_edge != null
    {filter_clause}
    COLLECT WITH COUNT INTO total
    RETURN total
"#
        );

        // Build a lean set of bind vars for the count query (exclude pagination-only vars like skip/limit)
        let mut count_bind_vars: std::collections::HashMap<&str, serde_json::Value> =
            std::collections::HashMap::new();
        if !q.is_empty() {
            count_bind_vars.insert("q", serde_json::Value::String(format!("%{}%", q)));
        }
        count_bind_vars.insert("scope", serde_json::Value::String(scope.to_string()));
        if let Some(ref pf) = player_full {
            count_bind_vars.insert("player_id", serde_json::Value::String(pf.clone()));
        } else {
            count_bind_vars.insert("player_id", serde_json::Value::Null);
        }
        if let Some(sf) = start_from {
            count_bind_vars.insert("start_from", serde_json::Value::String(sf.to_string()));
        }
        if let Some(st) = start_to {
            count_bind_vars.insert("start_to", serde_json::Value::String(st.to_string()));
        }
        if let Some(ef) = stop_from {
            count_bind_vars.insert("stop_from", serde_json::Value::String(ef.to_string()));
        }
        if let Some(et) = stop_to {
            count_bind_vars.insert("stop_to", serde_json::Value::String(et.to_string()));
        }
        if let Some(ref v) = venue_full {
            count_bind_vars.insert("venue_id", serde_json::Value::String(v.clone()));
        }
        if !game_full.is_empty() {
            count_bind_vars.insert(
                "game_ids",
                serde_json::Value::Array(
                    game_full
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        let count_query = arangors::AqlQuery::builder()
            .query(count_aql.as_str())
            .bind_vars(count_bind_vars)
            .build();
        let count_result = self
            .db
            .aql_query::<i64>(count_query)
            .await
            .map_err(|e| e.to_string())?;
        let total: u64 = count_result.first().cloned().unwrap_or(0) as u64;

        // Then fetch the paginated items
        let items_query = arangors::AqlQuery::builder()
            .query(aql.as_str())
            .bind_vars(bind_vars)
            .build();
        let result = self
            .db
            .aql_query::<serde_json::Value>(items_query)
            .await
            .map_err(|e| e.to_string())?;
        log::info!(
            "🔍 Search query returned {} items (page {} of size {}), total {}",
            result.len(),
            page,
            page_size,
            total
        );
        let items: Vec<serde_json::Value> = result.into_iter().collect();
        Ok(
            serde_json::json!({"items": items, "total": total, "page": page, "page_size": page_size}),
        )
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
        assert!(clause.contains("e._to IN @game_ids"));
        assert!(clause.contains("LENGTH("));
        assert!(clause.contains("> 0"));
    }
}
