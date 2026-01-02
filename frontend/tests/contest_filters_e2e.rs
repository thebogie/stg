//! Frontend E2E-style API tests for contest filters. Require BACKEND_BASE_URL.
//! 
//! Note: These tests are only compiled for non-WASM targets as they use
//! server-side libraries (reqwest, tokio) that are not compatible with WASM.

#![cfg(not(target_arch = "wasm32"))]

use std::env;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ContestItem { pub _id: String, pub name: String }

#[derive(Debug, Deserialize)]
struct ContestResponse { pub items: Vec<ContestItem>, pub total: u64, pub page: u32, pub page_size: u32 }

fn base_url() -> Option<String> { env::var("BACKEND_URL").ok() }

#[tokio::test]
async fn e2e_contest_filters_basic_pagination() {
	if base_url().is_none() { return; }
	let base = base_url().unwrap();
	let url = format!("{}/api/contests/search", base);

	let res = reqwest::Client::new()
		.get(&url)
		.query(&[("page", "1"), ("per_page", "5"), ("scope", "all")])
		.send().await.expect("request ok");
	assert!(res.status().is_success());
	let body: ContestResponse = res.json().await.expect("json ok");
	assert!(body.total >= body.items.len() as u64);
}

#[tokio::test]
async fn e2e_contest_filters_game_ids_any_semantics_shape() {
	if base_url().is_none() { return; }
	let base = base_url().unwrap();
	let url = format!("{}/api/contests/search", base);

	let res = reqwest::Client::new()
		.get(&url)
		.query(&[("game_ids", "game/__nonexistent__"), ("scope", "all")])
		.send().await.expect("request ok");
	assert!(res.status().is_success());
}

#[tokio::test]
async fn e2e_contest_filters_venue_and_player_params_shape() {
	if base_url().is_none() { return; }
	let base = base_url().unwrap();
	let url = format!("{}/api/contests/search", base);

	let res = reqwest::Client::new()
		.get(&url)
		.query(&[
			("venue_id", "venue/__nonexistent__"),
			("player_id", "player/__nonexistent__"),
			("scope", "all"),
		])
		.send().await.expect("request ok");
	assert!(res.status().is_success());
}


