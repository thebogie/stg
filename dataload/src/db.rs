use anyhow::{Result, Context, anyhow};
use arangors::{Connection, Document, Database, ClientError};
use arangors::client::reqwest::ReqwestClient;
use arangors::document::options::InsertOptions;
use std::collections::HashMap;
use std::env;
use log::{info, debug, warn};
use chrono::Local;
use reqwest::Client;
use serde_json::json;
use crate::models::{StgContest, StgVenue, StgGame, StgOutcome, DocumentCache};
use shared::{Player, Game, Venue, Contest, PlayedAt, PlayedWith, ResultedIn};
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHasher, SaltString,
    },
    Argon2,
    Params,
    Version,
};

pub struct DbClient {
    db: Database<ReqwestClient>,
    cache: DocumentCache,
}

impl DbClient {
    pub async fn new() -> Result<Self> {
        let host = env::var("ARANGO_URL").context("ARANGO_URL not set")?;
        let db_name = env::var("ARANGO_DB").context("ARANGO_DB not set")?;
        let user = env::var("ARANGO_USERNAME").context("ARANGO_USERNAME not set")?;
        let password = env::var("ARANGO_PASSWORD").context("ARANGO_PASSWORD not set")?;

        let conn = Connection::establish_basic_auth(&host, &user, &password)
            .await
            .context("Failed to connect to ArangoDB")?;

        // Try to drop the database if it exists using REST API
        let client = Client::new();
        let url = format!("{}/_db/_system/_api/database/{}", host, db_name);
        let response = client
            .delete(&url)
            .basic_auth(&user, Some(&password))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                info!("Dropped existing database {}", db_name);
            }
            Ok(resp) if resp.status().as_u16() == 404 => {
                info!("Database {} does not exist, will create it", db_name);
            }
            Ok(resp) => {
                let error = resp.text().await?;
                warn!("Error dropping database: {}", error);
            }
            Err(e) => warn!("Error dropping database: {}", e),
        }

        // Create new database using the REST API
        let url = format!("{}/_db/_system/_api/database", host);
        let response = client
            .post(&url)
            .basic_auth(&user, Some(&password))
            .json(&json!({
                "name": db_name,
                "users": [{
                    "username": user,
                    "passwd": password,
                    "active": true
                }]
            }))
            .send()
            .await
            .context("Failed to create database")?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to create database: {}", error));
        }
        info!("Created database {}", db_name);

        // Connect to the new database
        let db = conn.db(&db_name)
            .await
            .context("Failed to get database")?;

        // Create collections
        let mut collections = HashMap::new();
        let document_collections = ["player", "game", "venue", "contest"];
        let edge_collections = ["played_at", "played_with", "resulted_in"];

        // Create document collections
        for name in document_collections.iter() {
            // Try to drop collection if it exists
            match db.drop_collection(name).await {
                Ok(_) => info!("Dropped existing collection {}", name),
                Err(ClientError::Arango(arango_error)) if arango_error.error_num() == 1203 => {
                    info!("Collection {} does not exist, will create it", name)
                }
                Err(e) => warn!("Error dropping collection {}: {}", name, e),
            }

            // Create new document collection
            let collection = db.create_collection(name)
                .await
                .context(format!("Failed to create collection {}", name))?;
            info!("Created document collection {}", name);
            collections.insert(name.to_string(), collection);
        }

        // Create edge collections
        for name in edge_collections.iter() {
            // Try to drop collection if it exists
            match db.drop_collection(name).await {
                Ok(_) => info!("Dropped existing collection {}", name),
                Err(ClientError::Arango(arango_error)) if arango_error.error_num() == 1203 => {
                    info!("Collection {} does not exist, will create it", name)
                }
                Err(e) => warn!("Error dropping collection {}: {}", name, e),
            }

            // Create new edge collection
            let collection = db.create_edge_collection(name)
                .await
                .context(format!("Failed to create edge collection {}", name))?;
            info!("Created edge collection {}", name);
            collections.insert(name.to_string(), collection);
        }

        Ok(Self { 
            db, 
            cache: DocumentCache::new(),
        })
    }

    pub async fn load_records(&mut self, contests: Vec<StgContest>) -> Result<()> {
        info!("Starting to load {} contests at {}", 
            contests.len(), 
            Local::now().format("%Y-%m-%d %H:%M:%S"));
        
        let mut total_games = 0;
        let mut total_outcomes = 0;
        let mut total_venues = 0;
        
        for (i, contest) in contests.iter().enumerate() {
            info!("=== Processing contest {}/{} ===", i + 1, contests.len());
            debug!("Contest details: name='{}', start='{}', stop='{}', venue='{}'", 
                contest.name,
                contest.start.format("%Y-%m-%d %H:%M:%S"),
                contest.stop.format("%Y-%m-%d %H:%M:%S"),
                contest.venue.display_name);
            
            self.process_contest(contest).await?;
            
            total_games += contest.games.len();
            total_outcomes += contest.outcome.len();
            total_venues += 1;
        }
        
        info!("=== Load Summary ===");
        info!("Total contests processed: {}", contests.len());
        info!("Total games processed: {}", total_games);
        info!("Total outcomes processed: {}", total_outcomes);
        info!("Total venues processed: {}", total_venues);
        info!("Average games per contest: {:.2}", total_games as f64 / contests.len() as f64);
        info!("Average outcomes per contest: {:.2}", total_outcomes as f64 / contests.len() as f64);
        info!("Load completed at {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
        Ok(())
    }

    async fn create_edge<T: serde::Serialize + for<'de> serde::Deserialize<'de>>(
        &self,
        collection: &str,
        from: &str,
        to: &str,
        edge: T,
    ) -> Result<String> {
        let edge_collection = self.db.collection(collection).await?;
        let edge_doc = Document::new(edge);
        let result = edge_collection
            .create_document(edge_doc, InsertOptions::default())
            .await
            .context(format!("Failed to create edge in {:?}", collection))?;
        let header = result.header().unwrap();
        info!(
            "Created edge in {:?} from {} to {} (key: {})",
            collection, from, to, header._key
        );
        Ok(header._key.clone())
    }

    async fn process_venue(&mut self, venue: &StgVenue) -> Result<Venue> {
        let place_id = venue.place_id.clone();

        // Check if venue exists in cache using place_id
        if let Some(key) = self.cache.get_venue(&place_id) {
            info!("Found existing venue in cache: {} (key: {})", venue.place_id, key);
            // Fetch the venue from the database using the key
            let collection = self.db.collection("venue").await?;
            let doc = collection.document::<Venue>(&key).await?;
            return Ok(doc.document);
        }

        // Check if venue exists in database using parameterized query
        let collection = self.db.collection("venue").await?;
        let query = arangors::AqlQuery::builder()
            .query("FOR v IN venue FILTER v.place_id == @place_id RETURN v")
            .bind_var("place_id", place_id.clone())
            .build();
        let cursor: Vec<Document<Venue>> = self.db.aql_query(query).await?;
        let venues: Vec<Venue> = cursor.into_iter().map(|doc| doc.document).collect();

        if let Some(existing) = venues.first() {
            info!("Found existing venue in database: {} (id: {})", venue.display_name, existing.id);
            // Store in cache for future use - store just the key
            let key = existing.id.clone();
            self.cache.store_venue(place_id, key);
            return Ok(existing.clone());
        }

        // Create new venue - let ArangoDB set id and rev
        let venue = Venue::new_for_db(
            venue.display_name.clone(),
            venue.formatted_address.clone(),
            venue.place_id.clone(),
            venue.lat,
            venue.lng,
            "UTC".to_string(),
            shared::models::venue::VenueSource::Database,
        )?;
        let doc = collection.create_document(venue.clone(), InsertOptions::default()).await?;
        let header = doc.header().unwrap();
        let venue_id = header._id.clone();
        info!("Created new venue: {} (id: {})", venue.display_name, venue_id);
        
        // Store in cache for future use - store just the key
        let key = header._key.clone();
        self.cache.store_venue(place_id, key);

        // Return the venue with the database ID
        Ok(Venue {
            id: venue_id,
            rev: header._rev.clone(),
            display_name: venue.display_name.clone(),
            formatted_address: venue.formatted_address.clone(),
            place_id: venue.place_id.clone(),
            lat: venue.lat,
            lng: venue.lng,
            timezone: venue.timezone.clone(),
            source: venue.source,
        })
    }

    fn hash_password(password: &str) -> Result<String> {
        // Generate a random salt
        let salt = SaltString::generate(&mut OsRng);
        
        // Configure Argon2 with strong parameters
        let params = Params::new(
            65536,  // Memory cost (64MB)
            3,      // Time cost (3 iterations)
            1,      // Parallelism factor
            None,   // Output length (use default)
        ).map_err(|e| anyhow!("Failed to create Argon2 parameters: {}", e))?;
        
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,  // Use Argon2id variant (recommended)
            Version::V0x13,               // Use latest version
            params,
        );
        
        // Hash the password
        Ok(argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?
            .to_string())
    }

    async fn process_game(&mut self, game: &StgGame) -> Result<Game> {
        // Check if game exists in cache
        if let Some(key) = self.cache.get_game(&game.name) {
            info!("Found existing game in cache: {} (key: {})", game.name, key);
            // Fetch the game from the database using the key
            let collection = self.db.collection("game").await?;
            let doc = collection.document::<Game>(&key).await?;
            return Ok(doc.document);
        }

        // Check if game exists in database using parameterized query
        let collection = self.db.collection("game").await?;
        let query = arangors::AqlQuery::builder()
            .query("FOR g IN game FILTER g.name == @name RETURN g")
            .bind_var("name", game.name.clone())
            .build();
        let cursor: Vec<Document<Game>> = self.db.aql_query(query).await?;
        let games: Vec<Game> = cursor.into_iter().map(|doc| doc.document).collect();

        if let Some(existing) = games.first() {
            info!("Found existing game in database: {} (id: {})", game.name, existing.id);
            // Store in cache for future use
            let key = existing.id.clone();
            self.cache.store_game(game.name.clone(), key);
            return Ok(existing.clone());
        }

        // Create new game
        let game = Game::new_for_db(
            game.name.clone(),
            Some(game.year_published),
            game.bgg_id,
            None, // description is optional
            shared::models::game::GameSource::Database,
        )?;
        let doc = collection.create_document(game.clone(), InsertOptions::default()).await?;
        let header = doc.header().unwrap();
        let game_id = header._id.clone();
        info!("Created new game: {} (id: {})", game.name, game_id);
        
        // Store in cache for future use
        let key = header._key.clone();
        self.cache.store_game(game.name.clone(), key);

        // Return the game with the database ID
        Ok(Game {
            id: game_id,
            rev: header._rev.clone(),
            name: game.name,
            year_published: game.year_published,
            bgg_id: game.bgg_id,
            description: game.description,
            source: game.source,
        })
    }

    async fn process_player(&mut self, outcome: &StgOutcome) -> Result<Player> {
        // Sanitize player ID by replacing spaces with underscores and converting to lowercase
        let sanitized_id = outcome.player_id.to_lowercase().replace(' ', "_");
        // Use sanitized ID for email
        let email = format!("{}@example.com", sanitized_id);

        // Check if player exists in cache using email
        if let Some(key) = self.cache.get_player(&email) {
            info!("Found existing player in cache: {} (key: {})", outcome.player_id, key);
            // Fetch the player from the database using the key
            let collection = self.db.collection("player").await?;
            let doc = collection.document::<Player>(&key).await?;
            return Ok(doc.document);
        }

        // Check if player exists in database using parameterized query
        let collection = self.db.collection("player").await?;
        let query = arangors::AqlQuery::builder()
            .query("FOR p IN player FILTER p.email == @email RETURN p")
            .bind_var("email", email.clone())
            .build();
        let cursor: Vec<Document<Player>> = self.db.aql_query(query).await?;
        let players: Vec<Player> = cursor.into_iter().map(|doc| doc.document).collect();

        if let Some(existing) = players.first() {
            info!("Found existing player in database: {} (id: {})", outcome.player_id, existing.id);
            // Store in cache for future use - store just the key
            let key = existing.id.clone();
            self.cache.store_player(email, key);
            return Ok(existing.clone());
        }

        // Hash the default password using Argon2
        let hashed_password = Self::hash_password("letmein")?;

        // Create new player with hashed password
        let player = Player::new_for_db(
            outcome.player_id.clone(),
            outcome.player_id.clone(),
            email.clone(),
            hashed_password,
            chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
            false,
        )?;
        let doc = collection.create_document(player.clone(), InsertOptions::default()).await?;
        let header = doc.header().unwrap();
        let player_id = header._id.clone();
        info!("Created new player: {} (id: {})", outcome.player_id, player_id);
        
        // Store in cache for future use - store just the key
        let key = header._key.clone();
        self.cache.store_player(email, key);

        // Return the player with the database ID
        Ok(Player {
            id: player_id,
            rev: header._rev.clone(),
            firstname: player.firstname,
            handle: player.handle,
            email: player.email,
            password: player.password,
            created_at: player.created_at,
            is_admin: false,
        })
    }

    pub async fn process_contest(&mut self, contest: &StgContest) -> Result<Contest> {
        let start_time = Local::now();
        info!("=== Processing contest: {} ===", contest.name);
        debug!("Contest metadata: start='{}', stop='{}', games={}, outcomes={}", 
            contest.start.format("%Y-%m-%d %H:%M:%S"),
            contest.stop.format("%Y-%m-%d %H:%M:%S"),
            contest.games.len(),
            contest.outcome.len());
        
        // Process venue (using cache)
        let venue = self.process_venue(&contest.venue).await?;
        info!("Processing venue: {} (id: {})", venue.display_name, venue.id);
        debug!("Venue details: address='{}', lat={}, lng={}, place_id={}", 
            venue.formatted_address, venue.lat, venue.lng, venue.place_id);

        // Process games (using cache)
        info!("Processing {} games for contest", contest.games.len());
        let mut games = Vec::new();
        for (i, game) in contest.games.iter().enumerate() {
            let game = self.process_game(game).await?;
            info!("Processing game {}/{}: {} (id: {})", 
                i + 1, contest.games.len(), game.name, game.id);
            debug!("Game details: year={:?}, bgg_id={:?}", 
                game.year_published, game.bgg_id);
            games.push(game);
        }

        // Create new contest - let ArangoDB set key, id, and rev
        let contest_doc: Contest = contest.into();
        let collection = self.db.collection("contest").await?;
        let doc = collection.create_document(contest_doc.clone(), InsertOptions::default()).await?;
        let header = doc.header().unwrap();
        let contest_id = header._id.clone();
        info!("Created new contest: {} (id: {})", contest.name, contest_id);

        // Create edges (no caching needed)
        // Create played_at edge
        let played_at = PlayedAt::new(
            String::new(), // Let ArangoDB set this
            String::new(), // Let ArangoDB set this
            venue.id.clone(),  // Using venue's _id
            contest_id.clone(), // Using contest's _id
        )?;
        self.create_edge("played_at", &contest_id, &venue.id, played_at).await?;

        // Create played_with edges
        for game in &games {
            let played_with = PlayedWith::new(
                String::new(), // Let ArangoDB set this
                String::new(), // Let ArangoDB set this
                game.id.clone(),       // Using game's _id
                contest_id.clone(), // Using contest's _id
            )?;
            self.create_edge("played_with", &contest_id, &game.id, played_with).await?;
        }

        // Process outcomes and create resulted_in edges
        info!("Processing {} outcomes for contest", contest.outcome.len());
        for (i, outcome) in contest.outcome.iter().enumerate() {
            // Get or create player (using cache)
            let player = self.process_player(outcome).await?;
            info!("Processing outcome {}/{}: player {} (place: {})", 
                i + 1, contest.outcome.len(), player.handle, outcome.place);
            debug!("Player details: firstname='{}', email='{}', created_at='{}'", 
                player.firstname, 
                player.email,
                player.created_at.format("%Y-%m-%d %H:%M:%S"));

            // Create resulted_in edge (no caching needed)
            let resulted_in = ResultedIn::new(
                String::new(), // Let ArangoDB set this
                String::new(), // Let ArangoDB set this
                player.id.clone(),     // Using player's _id
                contest_id.clone(), // Using contest's _id
                outcome.place,
                outcome.result.clone(),
            )?;
            self.create_edge("resulted_in", &contest_id, &player.id, resulted_in).await?;
        }

        let duration = Local::now().signed_duration_since(start_time);
        info!("=== Successfully processed contest '{}' ===", contest.name);
        info!("Summary: games={}, outcomes={}, duration={}s", 
            contest.games.len(), 
            contest.outcome.len(),
            duration.num_seconds());
        debug!("Contest processing complete: id={}, venue={}, games={:?}", 
            contest_id, 
            venue.display_name,
            games.iter().map(|g| g.name.clone()).collect::<Vec<_>>());
        Ok(contest_doc)
    }
} 