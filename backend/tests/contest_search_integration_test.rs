//! Integration tests for contest search endpoint. These tests assume a running backend.
//! They will auto-skip unless the env var BACKEND_BASE_URL is set (e.g., http://localhost:8080).

use std::env;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ContestSearchItemDto {
	pub id: String,
	pub name: String,
}

#[derive(Debug, Deserialize)]
struct ContestSearchResponseDto {
	pub items: Vec<ContestSearchItemDto>,
	pub count: u64,
}

fn base_url() -> Option<String> {
	env::var("BACKEND_URL").ok()
}

fn has_base_url() -> bool {
	base_url().is_some()
}

#[tokio::test]
async fn contests_search_accepts_empty_query_and_returns_json() {
	if !has_base_url() {
		// Auto-skip if no integration target is configured
		return;
	}
	let base = base_url().unwrap();
	let url = format!("{}/api/contests/search", base);

	let client = reqwest::Client::new();
	let res = client
		.get(&url)
		.query(&[("page", "1"), ("per_page", "10")])
		.send()
		.await
		.expect("request to succeed");

	assert!(res.status().is_success(), "unexpected status: {}", res.status());
	let body: ContestSearchResponseDto = res.json().await.expect("valid json response");
	assert!(body.count >= body.items.len() as u64);
}

#[tokio::test]
async fn contests_search_handles_game_ids_param() {
	if !has_base_url() {
		return;
	}
	let base = base_url().unwrap();
	let url = format!("{}/api/contests/search", base);

	// Use a clearly non-existent ID to ensure the backend does not 500 on parsing.
	let non_existent_game_id = "game/__does_not_exist__";
	let client = reqwest::Client::new();
	let res = client
		.get(&url)
		.query(&[("game_ids", non_existent_game_id), ("page", "1"), ("per_page", "5")])
		.send()
		.await
		.expect("request to succeed");

	assert!(res.status().is_success(), "unexpected status: {}", res.status());
	let body: ContestSearchResponseDto = res.json().await.expect("valid json response");
	// With an invalid game id, zero or more items may be returned depending on data; only assert shape.
	assert!(body.count >= body.items.len() as u64);
}

#[tokio::test]
async fn contests_search_supports_date_range_params() {
	if !has_base_url() {
		return;
	}
	let base = base_url().unwrap();
	let url = format!("{}/api/contests/search", base);

	let client = reqwest::Client::new();
	let res = client
		.get(&url)
		.query(&[
			("start_from", "2020-01-01T00:00:00Z"),
			("start_to", "2100-01-01T00:00:00Z"),
			("page", "1"),
			("per_page", "5"),
		])
		.send()
		.await
		.expect("request to succeed");

	assert!(res.status().is_success(), "unexpected status: {}", res.status());
	let body: ContestSearchResponseDto = res.json().await.expect("valid json response");
	assert!(body.count >= body.items.len() as u64);
}

#[tokio::test]
async fn contests_search_supports_venue_param() {
	if !has_base_url() {
		return;
	}
	let base = base_url().unwrap();
	let url = format!("{}/api/contests/search", base);

	let client = reqwest::Client::new();
	let res = client
		.get(&url)
		.query(&[("venue_id", "venue/__does_not_exist__"), ("page", "1"), ("per_page", "5")])
		.send()
		.await
		.expect("request to succeed");

	assert!(res.status().is_success(), "unexpected status: {}", res.status());
	let body: ContestSearchResponseDto = res.json().await.expect("valid json response");
	assert!(body.count >= body.items.len() as u64);
}

#[tokio::test]
async fn contests_search_supports_player_param() {
	if !has_base_url() {
		return;
	}
	let base = base_url().unwrap();
	let url = format!("{}/api/contests/search", base);

	let client = reqwest::Client::new();
	let res = client
		.get(&url)
		.query(&[("player_id", "player/__does_not_exist__"), ("page", "1"), ("per_page", "5")])
		.send()
		.await
		.expect("request to succeed");

	assert!(res.status().is_success(), "unexpected status: {}", res.status());
	let body: ContestSearchResponseDto = res.json().await.expect("valid json response");
	assert!(body.count >= body.items.len() as u64);
}


