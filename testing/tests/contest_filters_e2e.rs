//! Frontend E2E-style API tests for contest filters. Require BACKEND_URL.
//! 
//! These tests exercise the contest search/filter API endpoints.

use std::env;
use serde::Deserialize;
use anyhow::Result;

#[derive(Debug, Deserialize)]
struct ContestItem { 
    pub _id: String, 
    pub name: String 
}

#[derive(Debug, Deserialize)]
struct ContestResponse { 
    pub items: Vec<ContestItem>, 
    pub total: u64, 
    pub page: u32, 
    pub page_size: u32 
}

fn base_url() -> Option<String> { 
    env::var("BACKEND_URL").ok() 
}

#[tokio::test]
async fn e2e_contest_filters_basic_pagination() -> Result<()> {
    if base_url().is_none() { 
        eprintln!("Skipping test: BACKEND_URL not set");
        return Ok(());
    }
    let base = base_url().unwrap();
    let url = format!("{}/api/contests/search", base);

    let res = reqwest::Client::new()
        .get(&url)
        .query(&[("page", "1"), ("per_page", "5"), ("scope", "all")])
        .send().await?;
    
    assert!(res.status().is_success(), "Expected success status, got: {}", res.status());
    let body: ContestResponse = res.json().await?;
    assert!(body.total >= body.items.len() as u64);
    
    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_game_ids_any_semantics_shape() -> Result<()> {
    if base_url().is_none() { 
        eprintln!("Skipping test: BACKEND_URL not set");
        return Ok(());
    }
    let base = base_url().unwrap();
    let url = format!("{}/api/contests/search", base);

    let res = reqwest::Client::new()
        .get(&url)
        .query(&[("game_ids", "game/__nonexistent__"), ("scope", "all")])
        .send().await?;
    
    assert!(res.status().is_success(), "Expected success status, got: {}", res.status());
    
    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_venue_and_player_params_shape() -> Result<()> {
    if base_url().is_none() { 
        eprintln!("Skipping test: BACKEND_URL not set");
        return Ok(());
    }
    let base = base_url().unwrap();
    let url = format!("{}/api/contests/search", base);

    let res = reqwest::Client::new()
        .get(&url)
        .query(&[
            ("venue_id", "venue/__nonexistent__"),
            ("player_id", "player/__nonexistent__"),
            ("scope", "all"),
        ])
        .send().await?;
    
    assert!(res.status().is_success(), "Expected success status, got: {}", res.status());
    
    Ok(())
}

