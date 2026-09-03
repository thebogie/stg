//! HTTP integration tests for analytics tab endpoints.
//! Skips gracefully when the API is not reachable at API_BASE_URL (default http://127.0.0.1:50002).

use serde_json::Value;

fn api_base() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:50002".to_string())
}

async fn fetch_tab(path: &str) -> Option<(reqwest::StatusCode, Value)> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", api_base(), path);
    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Skipping analytics tab test (API unreachable at {}): {}", url, e);
            return None;
        }
    };
    let status = response.status();
    let body: Value = response.json().await.ok()?;
    Some((status, body))
}

#[tokio::test]
async fn overview_tab_returns_timezone_and_wow() {
    let Some((status, body)) =
        fetch_tab("/api/analytics/tabs/overview?timezone=America/Chicago").await
    else {
        return;
    };
    assert!(status.is_success(), "overview tab: {}", body);
    assert_eq!(
        body.get("timezone").and_then(|v| v.as_str()),
        Some("America/Chicago")
    );
    assert!(body.get("week_over_week").is_some());
}

#[tokio::test]
async fn contests_tab_returns_timezone() {
    let Some((status, body)) =
        fetch_tab("/api/analytics/tabs/contests?timezone=UTC").await
    else {
        return;
    };
    assert!(status.is_success(), "contests tab: {}", body);
    assert_eq!(body.get("timezone").and_then(|v| v.as_str()), Some("UTC"));
}

#[tokio::test]
async fn venues_and_games_tabs_return_timezone() {
    for path in [
        "/api/analytics/tabs/venues?timezone=UTC",
        "/api/analytics/tabs/games?timezone=UTC",
    ] {
        let Some((status, body)) = fetch_tab(path).await else {
            return;
        };
        assert!(status.is_success(), "{}: {}", path, body);
        assert_eq!(body.get("timezone").and_then(|v| v.as_str()), Some("UTC"));
    }
}

#[tokio::test]
async fn activity_metrics_chart_accepts_timezone_query() {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/analytics/charts/activity-metrics?days=60&timezone=America/New_York",
        api_base()
    );
    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Skipping activity metrics chart test: {}", e);
            return;
        }
    };
    assert!(response.status().is_success());
    let body: Value = response.json().await.expect("chart json");
    assert_eq!(body.get("chart_type").and_then(|v| v.as_str()), Some("Line"));
}
