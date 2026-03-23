//! Contest creation when the creator (and outcome participant) use **legacy numeric** `player` record
//! keys (e.g. migrated from Arango). These keys must not be passed through `type::uuid(...)` in
//! SurrealQL — regression test for: creator_id + `resulted_in.out` with `type::record('player', $key)`.
//!
//! Requires SurrealDB + Redis (same as other `testing` integration tests): `./deploy/stack.sh start`

use anyhow::Result;
use chrono::Utc;
use shared::dto::contest::{ContestDto, OutcomeDto};
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use shared::models::game::GameSource;
use shared::models::venue::VenueSource;
use testing::{app_setup, TestEnvironment};
use validator::Validate;

use backend::contest::repository::ContestRepository;
use backend::game::usecase::{GameUseCase, GameUseCaseImpl};
use backend::player::usecase::PlayerUseCase;
use backend::venue::usecase::{VenueUseCase, VenueUseCaseImpl};

/// Unique numeric-style key (same shape as production legacy ids).
fn unique_legacy_player_key() -> String {
    format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_millis()
    )
}

/// Seed a player row whose record id key is a **non-UUID** string (digits only).
async fn seed_legacy_player_with_numeric_key(
    db: &backend::db::Db,
    ns: &str,
    db_name: &str,
    key: &str,
) -> Result<()> {
    let handle = format!("legacy_h_{}", key);
    let email = format!("legacy_{}@example.com", key);
    // `createdAt` must be Surreal datetime, not a JSON string.
    let sql = format!(
        "USE NS {}; USE DB {}; CREATE type::record('player', $key) CONTENT {{ \
         firstname: $fn, \
         handle: $h, \
         email: $e, \
         password: $p, \
         isAdmin: false, \
         createdAt: time::now() \
         }};",
        ns, db_name
    );
    let mut res = db
        .query(&sql)
        .bind(("key", key.to_string()))
        .bind(("fn", "Legacy".to_string()))
        .bind(("h", handle))
        .bind(("e", email))
        .bind(("p", "x".to_string()))
        .await
        .map_err(|e| anyhow::anyhow!("seed legacy player: {}", e))?;
    let _: Vec<serde_json::Value> = res.take(2).map_err(|e| anyhow::anyhow!("take: {}", e))?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn contest_create_succeeds_with_numeric_player_ids() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let legacy_key = unique_legacy_player_key();
    let player_canonical = format!("player/{}", legacy_key);

    seed_legacy_player_with_numeric_key(
        app_data.db.as_ref(),
        &env.surrealdb_ns,
        &env.surrealdb_db,
        &legacy_key,
    )
    .await?;

    // Sanity: repository can load this player by canonical id
    let loaded = app_data
        .contest_repo
        .player_usecase
        .get_player(&player_canonical)
        .await;
    assert!(
        loaded.is_ok(),
        "seeded legacy player should be readable: {:?}",
        loaded.err()
    );

    let venue_uc = VenueUseCaseImpl {
        repo: app_data.venue_repo.as_ref().clone(),
    };
    let game_uc = GameUseCaseImpl {
        repo: app_data.game_repo.as_ref().clone(),
    };

    let place_tail = format!("plc_{}", legacy_key);
    let venue_dto = VenueDto {
        id: String::new(),
        display_name: "Legacy Flow Venue".to_string(),
        formatted_address: "1 Legacy Ln".to_string(),
        place_id: place_tail,
        lat: 40.0,
        lng: -70.0,
        timezone: "UTC".to_string(),
        source: VenueSource::Database,
    };
    let venue = venue_uc
        .create_venue(venue_dto)
        .await
        .map_err(|e| anyhow::anyhow!("create_venue: {}", e))?;

    let game_dto = GameDto {
        id: String::new(),
        name: "Legacy Flow Game".to_string(),
        year_published: Some(2024),
        bgg_id: None,
        description: None,
        source: GameSource::Database,
    };
    let game = game_uc
        .create_game(game_dto)
        .await
        .map_err(|e| anyhow::anyhow!("create_game: {}", e))?;

    let start = Utc::now().into();
    let stop = (Utc::now() + chrono::Duration::hours(2)).into();

    let contest_dto = ContestDto {
        id: String::new(),
        name: "Legacy key contest".to_string(),
        start,
        stop,
        venue: VenueDto {
            id: venue.id.clone(),
            display_name: venue.display_name.clone(),
            formatted_address: venue.formatted_address.clone(),
            place_id: venue.place_id.clone(),
            lat: venue.lat,
            lng: venue.lng,
            timezone: venue.timezone.clone(),
            source: VenueSource::Database,
        },
        games: vec![GameDto {
            id: game.id.clone(),
            name: game.name.clone(),
            year_published: game.year_published,
            bgg_id: game.bgg_id,
            description: game.description.clone(),
            source: GameSource::Database,
        }],
        outcomes: vec![OutcomeDto {
            player_id: player_canonical.clone(),
            place: "1".to_string(),
            result: "win".to_string(),
            email: String::new(),
            handle: String::new(),
        }],
        creator_id: String::new(),
        creator_handle: None,
        created_at: None,
    };

    contest_dto.validate()?;

    let created = app_data
        .contest_repo
        .create_contest(contest_dto, player_canonical.clone())
        .await
        .map_err(|e| anyhow::anyhow!("create_contest: {}", e))?;

    assert!(created.id.starts_with("contest/"));
    assert_eq!(created.creator_id, player_canonical);
    assert_eq!(created.outcomes.len(), 1);
    assert_eq!(created.outcomes[0].player_id, player_canonical);

    Ok(())
}
