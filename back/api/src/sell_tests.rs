//! Unit tests for sell listing shared types.

use shared::dto::sell_listing::status_after_checkpoint;
use shared::models::sell_listing::{checkpoint, listing_status, SellListing};

#[test]
fn status_after_photos_checkpoint() {
    assert_eq!(
        status_after_checkpoint(checkpoint::PHOTOS),
        Some(listing_status::PHOTOS_UPLOADED)
    );
}

#[test]
fn listing_has_checkpoint() {
    use chrono::Utc;
    let mut listing = SellListing {
        id: "sell_listing/x".to_string(),
        rev: String::new(),
        seller_id: "player/1".to_string(),
        status: listing_status::DRAFT_CREATED.to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        expires_at: Utc::now(),
        title: None,
        description: None,
        condition_notes: None,
        condition: None,
        price_cents: None,
        currency: None,
        shipping_notes: None,
        bgg_id: None,
        game_name: None,
        edition_notes: None,
        missing_components: vec![],
        ai_confidence: None,
        ai_questions: vec![],
        ai_warnings: vec![],
        bgg_listing_url: None,
        checkpoint_approvals: vec![],
        photo_count: 0,
    };
    assert!(!listing.has_checkpoint(checkpoint::PHOTOS));
    listing.checkpoint_approvals.push(shared::models::sell_listing::CheckpointApproval {
        checkpoint: checkpoint::PHOTOS.to_string(),
        approved_at: Utc::now(),
        approved_by: "player/1".to_string(),
    });
    assert!(listing.has_checkpoint(checkpoint::PHOTOS));
}
