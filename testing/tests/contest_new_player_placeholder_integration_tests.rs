//! Record-contest: client sends a placeholder `player/{uuid}` for a new guest (email/handle set).
//! Backend must create the player document, not attach `resulted_in` to a non-existent id.

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

#[tokio::test]
#[serial_test::serial]
async fn contest_create_creates_player_when_client_sends_placeholder_uuid() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let legacy_key = format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    );
    let creator_id = format!("player/{}", legacy_key);

    let sql = format!(
        "USE NS {}; USE DB {}; CREATE type::record('player', $key) CONTENT {{ \
         firstname: 'Creator', handle: $h, email: $e, password: 'x', isAdmin: false, createdAt: time::now() }};",
        env.surrealdb_ns, env.surrealdb_db
    );
    let mut res = app_data
        .db
        .query(&sql)
        .bind(("key", legacy_key.clone()))
        .bind(("h", format!("creator_{}", legacy_key)))
        .bind(("e", format!("creator_{}@example.com", legacy_key)))
        .await?;
    let _: Vec<serde_json::Value> = res.take(2)?;

    let venue_uc = VenueUseCaseImpl {
        repo: app_data.venue_repo.as_ref().clone(),
    };
    let game_uc = GameUseCaseImpl {
        repo: app_data.game_repo.as_ref().clone(),
    };

    let venue = venue_uc
        .create_venue(VenueDto {
            id: String::new(),
            display_name: "Placeholder Player Venue".to_string(),
            formatted_address: "1 Test Ln".to_string(),
            place_id: format!("plc_{}", legacy_key),
            lat: 40.0,
            lng: -70.0,
            timezone: "UTC".to_string(),
            source: VenueSource::Database,
        })
        .await
        .map_err(|e| anyhow::anyhow!("create_venue: {}", e))?;

    let game = game_uc
        .create_game(GameDto {
            id: String::new(),
            name: "Placeholder Flow Game".to_string(),
            year_published: Some(2024),
            bgg_id: None,
            description: None,
            source: GameSource::Database,
        })
        .await
        .map_err(|e| anyhow::anyhow!("create_game: {}", e))?;

    let placeholder_uuid = "bf11e185-31ae-4bf0-b497-c88d8a041c63";
    let guest_email = format!("bri_{}@gmail.com", legacy_key);
    let guest_handle = "bri".to_string();

    let contest_dto = ContestDto {
        id: String::new(),
        name: "Placeholder guest contest".to_string(),
        start: Utc::now().into(),
        stop: (Utc::now() + chrono::Duration::hours(2)).into(),
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
            player_id: format!("player/{}", placeholder_uuid),
            place: "1".to_string(),
            result: "won".to_string(),
            score: String::new(),
            email: guest_email.clone(),
            handle: guest_handle.clone(),
        }],
        creator_id: String::new(),
        creator_handle: None,
        created_at: None,
        moderation_status: String::new(),
        moderated_at: None,
        moderated_by: None,
        moderation_note: None,
        has_image: false,
        image_url: None,
        image_detail_url: None,
    };
    contest_dto.validate()?;

    let saved = app_data
        .contest_repo
        .create_contest(contest_dto, creator_id)
        .await
        .map_err(|e| anyhow::anyhow!("create_contest: {}", e))?;

    assert_eq!(saved.outcomes.len(), 1);
    let outcome = &saved.outcomes[0];
    assert_eq!(outcome.email, guest_email);
    assert_eq!(outcome.handle, guest_handle);
    assert!(
        outcome.player_id.starts_with("player/"),
        "expected canonical player id, got {}",
        outcome.player_id
    );
    assert_ne!(
        outcome.player_id,
        format!("player/{}", placeholder_uuid),
        "must not keep client placeholder id"
    );

    let loaded = app_data
        .contest_repo
        .player_usecase
        .get_player(&outcome.player_id)
        .await
        .map_err(|e| anyhow::anyhow!("get_player: {}", e))?;
    assert_eq!(loaded.email, guest_email);

    Ok(())
}
