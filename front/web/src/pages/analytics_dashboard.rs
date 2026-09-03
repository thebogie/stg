mod links;
mod types;

use links::*;
use types::{AnalyticsDashboardProps, AnalyticsTab, GameRecommendation, VenuePerformance};

use crate::api::games::get_game_analytics;
use crate::api::games::search_games;
use crate::api::utils::authenticated_get;
use crate::components::chart_renderer::ChartRenderer;
use crate::pages::components::head_to_head_modal::HeadToHeadModal;
use crate::Route;
use gloo_net::http::Request;
use serde_json::Value;
use shared::dto::analytics::HeadToHeadRecordDto;
use shared::dto::game::GameDto;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::*;

#[wasm_bindgen(module = "/src/js/timezone.js")]
extern "C" {
    fn getBrowserIanaTimezone() -> String;
}

#[function_component(AnalyticsDashboard)]
pub fn analytics_dashboard(_props: &AnalyticsDashboardProps) -> Html {
    let auth = use_context::<crate::auth::AuthContext>().expect("Auth context not found");
    let navigator = use_navigator().unwrap();
    let platform_stats = use_state(|| None::<Value>);
    let contest_trends_chart = use_state(|| None::<String>);

    let _contest_analysis_chart = use_state(|| None::<String>);
    let game_popularity_chart = use_state(|| None::<String>);
    let insights = use_state(|| None::<Value>);
    let activity_metrics_chart = use_state(|| None::<String>);
    let glicko_leaderboard = use_state(|| None::<Vec<Value>>);
    let glicko_loading = use_state(|| false);
    let glicko_error = use_state(|| None::<String>);

    // Enhanced analytics state
    let venue_performance = use_state(|| None::<Vec<VenuePerformance>>);
    let venue_loading = use_state(|| false);
    let game_recommendations = use_state(|| None::<Vec<GameRecommendation>>);
    let recommendations_loading = use_state(|| false);
    let gaming_communities = use_state(|| None::<Value>);
    let communities_loading = use_state(|| false);
    let player_networking = use_state(|| None::<Value>);
    let networking_loading = use_state(|| false);

    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);

    // Tabs state
    let current_tab = use_state(|| AnalyticsTab::Overview);

    // Contests heatmap state
    let contest_heatmap = use_state(|| None::<Value>);
    let contest_heatmap_loading = use_state(|| false);
    let contest_heatmap_error = use_state(|| None::<String>);
    let heatmap_weeks = use_state(|| 8i32);
    let recent_contests = use_state(|| None::<Vec<Value>>);
    let recent_contests_loading = use_state(|| false);
    let recent_contests_error = use_state(|| None::<String>);

    // Games tab state
    let game_id_input = use_state(|| String::new());
    let game_analytics = use_state(|| None::<Value>);
    let game_analytics_loading = use_state(|| false);
    let game_analytics_error = use_state(|| None::<String>);

    // Games tab search state
    let game_search_query = use_state(|| String::new());
    let game_search_loading = use_state(|| false);
    let game_search_error = use_state(|| None::<String>);
    let game_search_results = use_state(|| Vec::<GameDto>::new());

    // Per-tab extended analytics
    let tab_analytics = use_state(|| None::<Value>);
    let tab_analytics_loading = use_state(|| false);
    let tab_analytics_error = use_state(|| None::<String>);
    let player_timezone = use_state(|| String::from("UTC"));
    let system_health = use_state(|| None::<Value>);
    let system_health_loading = use_state(|| false);
    let h2h_modal_open = use_state(|| false);
    let h2h_modal_loading = use_state(|| false);
    let h2h_modal_error = use_state(|| None::<String>);
    let h2h_modal_record = use_state(|| None::<HeadToHeadRecordDto>);
    let h2h_modal_opponent = use_state(|| (String::new(), String::new()));

    {
        let player_timezone = player_timezone.clone();
        use_effect_with((), move |_| {
            let tz = shared::timezone::normalize_iana_timezone(&getBrowserIanaTimezone());
            player_timezone.set(tz);
            || ()
        });
    }

    let on_select_tab = {
        let current_tab = current_tab.clone();
        Callback::from(move |tab: AnalyticsTab| {
            current_tab.set(tab);
        })
    };

    // Load extended analytics when the active tab changes
    {
        let current_tab = current_tab.clone();
        let tab_analytics = tab_analytics.clone();
        let tab_analytics_loading = tab_analytics_loading.clone();
        let tab_analytics_error = tab_analytics_error.clone();
        let auth = auth.clone();
        let player_timezone = player_timezone.clone();
        use_effect_with(((*current_tab).clone(), (*player_timezone).clone()), move |(tab, tz)| {
            let tab = (*tab).clone();
            let tz = (*tz).clone();
            let (path, needs_auth) = match tab {
                AnalyticsTab::Overview => ("/api/analytics/tabs/overview", false),
                AnalyticsTab::Contests => ("/api/analytics/tabs/contests", false),
                AnalyticsTab::Venues => ("/api/analytics/tabs/venues", false),
                AnalyticsTab::Games => ("/api/analytics/tabs/games", false),
                AnalyticsTab::Players => ("/api/analytics/tabs/players", true),
            };
            let url = format!("{}?{}", path, player_timezone_query(&tz));
            tab_analytics_loading.set(true);
            tab_analytics_error.set(None);
            tab_analytics.set(None);
            let tab_analytics = tab_analytics.clone();
            let tab_analytics_loading = tab_analytics_loading.clone();
            let tab_analytics_error = tab_analytics_error.clone();
            let path = url;
            wasm_bindgen_futures::spawn_local(async move {
                let result = if needs_auth {
                    if auth.state.player.is_none() {
                        tab_analytics_error.set(Some("Sign in to view player analytics".to_string()));
                        tab_analytics.set(None);
                        tab_analytics_loading.set(false);
                        return;
                    }
                    authenticated_get(&path).send().await
                } else {
                    Request::get(&path).send().await
                };
                match result {
                    Ok(response) if response.ok() => {
                        if let Ok(data) = response.json::<Value>().await {
                            tab_analytics.set(Some(data));
                        } else {
                            tab_analytics_error.set(Some("Failed to parse tab analytics".to_string()));
                        }
                    }
                    Ok(response) => {
                        tab_analytics_error.set(Some(format!("Tab analytics failed: {}", response.status())));
                    }
                    Err(e) => {
                        tab_analytics_error.set(Some(format!("Failed to load tab analytics: {}", e)));
                    }
                }
                tab_analytics_loading.set(false);
            });
            || ()
        });
    }

    let on_game_id_input = {
        let game_id_input = game_id_input.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            game_id_input.set(input.value());
        })
    };

    let on_load_game_analytics = {
        let game_id_input = game_id_input.clone();
        let game_analytics = game_analytics.clone();
        let game_analytics_loading = game_analytics_loading.clone();
        let game_analytics_error = game_analytics_error.clone();
        Callback::from(move |_| {
            let game_id = (*game_id_input).clone();
            if game_id.is_empty() {
                return;
            }
            game_analytics_loading.set(true);
            game_analytics_error.set(None);
            let game_analytics = game_analytics.clone();
            let game_analytics_loading = game_analytics_loading.clone();
            let game_analytics_error = game_analytics_error.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match get_game_analytics(&game_id).await {
                    Ok(data) => {
                        game_analytics.set(Some(data));
                    }
                    Err(e) => {
                        game_analytics_error.set(Some(e));
                        game_analytics.set(None);
                    }
                }
                game_analytics_loading.set(false);
            });
        })
    };

    let on_game_search_input = {
        let game_search_query = game_search_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            game_search_query.set(input.value());
        })
    };

    let on_game_search = {
        let game_search_query = game_search_query.clone();
        let game_search_results = game_search_results.clone();
        let game_search_loading = game_search_loading.clone();
        let game_search_error = game_search_error.clone();
        Callback::from(move |_| {
            let query = (*game_search_query).clone();
            game_search_loading.set(true);
            game_search_error.set(None);
            let game_search_results = game_search_results.clone();
            let game_search_loading = game_search_loading.clone();
            let game_search_error = game_search_error.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if query.trim().is_empty() {
                    game_search_results.set(Vec::new());
                    game_search_loading.set(false);
                    return;
                }
                match search_games(&query).await {
                    Ok(results) => {
                        game_search_results.set(results);
                    }
                    Err(e) => {
                        game_search_error.set(Some(e));
                        game_search_results.set(Vec::new());
                    }
                }
                game_search_loading.set(false);
            });
        })
    };

    // Load platform stats (Overview tab only)
    {
        let platform_stats = platform_stats.clone();
        let loading = loading.clone();
        let error = error.clone();
        let current_tab = current_tab.clone();

        use_effect_with((*current_tab).clone(), move |tab| {
            if *tab == AnalyticsTab::Overview {
            loading.set(true);
            error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                match Request::get("/api/analytics/platform").send().await {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(stats) = response.json::<Value>().await {
                                platform_stats.set(Some(stats));
                            } else {
                                error.set(Some("Failed to parse platform stats".to_string()));
                            }
                        } else {
                            let status = response.status();
                            let text = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Unknown error".to_string());
                            error.set(Some(format!(
                                "Platform stats request failed: {} - {}",
                                status, text
                            )));
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch platform stats: {}", e)));
                    }
                }
                loading.set(false);
            });
            }
            || ()
        });
    }

    // Load Glicko2 leaderboard (Players tab only)
    {
        let glicko_leaderboard = glicko_leaderboard.clone();
        let glicko_loading = glicko_loading.clone();
        let glicko_error = glicko_error.clone();
        let current_tab = current_tab.clone();

        use_effect_with((*current_tab).clone(), move |tab| {
            if *tab == AnalyticsTab::Players {
            glicko_loading.set(true);
            glicko_error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_get(
                    "/api/ratings/leaderboard?scope=global&min_games=3&limit=10",
                )
                .send()
                .await
                {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(leaderboard) = response.json::<Vec<Value>>().await {
                                glicko_leaderboard.set(Some(leaderboard));
                            } else {
                                glicko_error
                                    .set(Some("Failed to parse Glicko2 leaderboard".to_string()));
                            }
                        } else {
                            let status = response.status();
                            let text = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Unknown error".to_string());
                            glicko_error.set(Some(format!(
                                "Glicko2 leaderboard request failed: {} - {}",
                                status, text
                            )));
                        }
                    }
                    Err(e) => {
                        glicko_error
                            .set(Some(format!("Failed to fetch Glicko2 leaderboard: {}", e)));
                    }
                }
                glicko_loading.set(false);
            });
            }
            || ()
        });
    }

    // Load contest trends chart (Contests tab)
    {
        let contest_trends_chart = contest_trends_chart.clone();
        let error = error.clone();
        let current_tab = current_tab.clone();
        let player_timezone = player_timezone.clone();

        use_effect_with(
            ((*current_tab).clone(), (*player_timezone).clone()),
            move |(tab, tz)| {
                if *tab == AnalyticsTab::Contests {
                let tz = (*tz).clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match Request::get(&format!(
                        "/api/analytics/charts/contest-trends?months=12&title=Contest%20Trends%20Over%20Time&{}",
                        player_timezone_query(&tz)
                    ))
                    .send()
                    .await
                    {
                        Ok(response) => {
                            if let Ok(chart_data) = response.text().await {
                                contest_trends_chart.set(Some(chart_data));
                            } else {
                                error.set(Some("Failed to parse contest trends chart".to_string()));
                            }
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to fetch contest trends chart: {}", e)));
                        }
                    }
                });
                }
                || ()
            },
        );
    }

    // Load contest heatmap data (Contests tab only)
    {
        let contest_heatmap = contest_heatmap.clone();
        let contest_heatmap_loading = contest_heatmap_loading.clone();
        let contest_heatmap_error = contest_heatmap_error.clone();
        let heatmap_weeks = heatmap_weeks.clone();
        let player_timezone = player_timezone.clone();
        let current_tab = current_tab.clone();
        use_effect_with(
            (
                (*current_tab).clone(),
                heatmap_weeks.clone(),
                (*player_timezone).clone(),
            ),
            move |(tab, weeks, tz)| {
                if *tab == AnalyticsTab::Contests {
                let w = **weeks;
                let tz = (*tz).clone();
                contest_heatmap_loading.set(true);
                contest_heatmap_error.set(None);
                wasm_bindgen_futures::spawn_local(async move {
                    match Request::get(&format!(
                        "/api/analytics/contests/heatmap?weeks={}&{}",
                        w,
                        player_timezone_query(&tz)
                    ))
                    .send()
                    .await
                    {
                        Ok(resp) => {
                            if resp.ok() {
                                match resp.json::<Value>().await {
                                    Ok(data) => contest_heatmap.set(Some(data)),
                                    Err(e) => contest_heatmap_error
                                        .set(Some(format!("Failed to parse heatmap: {}", e))),
                                }
                            } else {
                                contest_heatmap_error
                                    .set(Some(format!("Heatmap request failed: {}", resp.status())));
                            }
                        }
                        Err(e) => {
                            contest_heatmap_error
                                .set(Some(format!("Failed to fetch heatmap: {}", e)))
                        }
                    }
                    contest_heatmap_loading.set(false);
                });
                }
                || ()
            },
        );
    }

    // Load recent contests for Contests tab
    {
        let current_tab = current_tab.clone();
        let recent_contests = recent_contests.clone();
        let recent_contests_loading = recent_contests_loading.clone();
        let recent_contests_error = recent_contests_error.clone();
        use_effect_with((*current_tab).clone(), move |tab| {
            if *tab == AnalyticsTab::Contests {
            recent_contests_loading.set(true);
            recent_contests_error.set(None);
            let recent_contests = recent_contests.clone();
            let recent_contests_loading = recent_contests_loading.clone();
            let recent_contests_error = recent_contests_error.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get("/api/analytics/contests/recent?limit=15").send().await {
                    Ok(resp) if resp.ok() => {
                        match resp.json::<Vec<Value>>().await {
                            Ok(rows) => recent_contests.set(Some(rows)),
                            Err(e) => recent_contests_error
                                .set(Some(format!("Failed to parse recent contests: {}", e))),
                        }
                    }
                    Ok(resp) => {
                        recent_contests_error
                            .set(Some(format!("Recent contests failed: {}", resp.status())));
                    }
                    Err(e) => {
                        recent_contests_error
                            .set(Some(format!("Failed to load recent contests: {}", e)));
                    }
                }
                recent_contests_loading.set(false);
            });
            }
            || ()
        });
    }

    // Load platform insights (Overview tab)
    {
        let insights_state = insights.clone();
        let error = error.clone();
        let current_tab = current_tab.clone();
        use_effect_with((*current_tab).clone(), move |tab| {
            if *tab == AnalyticsTab::Overview {
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get("/api/analytics/insights").send().await {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(data) = response.json::<Value>().await {
                                insights_state.set(Some(data));
                            }
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch insights: {}", e)));
                    }
                }
            });
            }
            || ()
        });
    }

    // Load games by player count distribution (Games tab)
    {
        let game_popularity_chart = game_popularity_chart.clone();
        let error = error.clone();
        let current_tab = current_tab.clone();
        use_effect_with((*current_tab).clone(), move |tab| {
            if *tab == AnalyticsTab::Games {
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get("/api/analytics/charts/game-popularity?title=Games%20by%20Player%20Count%20Distribution")
                                .send()
                                .await
                            {
                                Ok(response) => {
                                    if let Ok(chart_data) = response.text().await {
                                        game_popularity_chart.set(Some(chart_data));
                                    } else {
                                        error.set(Some("Failed to parse games by player count chart".to_string()));
                                    }
                                }
                                Err(e) => {
                                    error.set(Some(format!("Failed to fetch games by player count chart: {}", e)));
                                }
                            }
            });
            }
            || ()
        });
    }

    // Load activity metrics chart (Overview tab)
    {
        let activity_metrics_chart = activity_metrics_chart.clone();
        let error = error.clone();
        let current_tab = current_tab.clone();
        let player_timezone = player_timezone.clone();
        use_effect_with(
            ((*current_tab).clone(), (*player_timezone).clone()),
            move |(tab, tz)| {
                if *tab == AnalyticsTab::Overview {
                let tz = (*tz).clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match Request::get(&format!(
                        "/api/analytics/charts/activity-metrics?days=180&title=Monthly%20Activity&{}",
                        player_timezone_query(&tz)
                    ))
                    .send()
                    .await
                    {
                        Ok(response) => {
                            if let Ok(chart_data) = response.text().await {
                                activity_metrics_chart.set(Some(chart_data));
                            } else {
                                error.set(Some("Failed to parse activity metrics chart".to_string()));
                            }
                        }
                        Err(e) => {
                            error.set(Some(format!(
                                "Failed to fetch activity metrics chart: {}",
                                e
                            )));
                        }
                    }
                });
                }
                || ()
            },
        );
    }

    // Load system health (Overview tab)
    {
        let system_health = system_health.clone();
        let system_health_loading = system_health_loading.clone();
        let current_tab = current_tab.clone();
        use_effect_with((*current_tab).clone(), move |tab| {
            if *tab == AnalyticsTab::Overview {
            system_health_loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get("/health/detailed").send().await {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(data) = response.json::<Value>().await {
                                system_health.set(Some(data));
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to fetch system health: {}", e);
                    }
                }
                system_health_loading.set(false);
            });
            }
            || ()
        });
    }

    // Load enhanced analytics data
    {
        let venue_performance = venue_performance.clone();
        let venue_loading = venue_loading.clone();
        let game_recommendations = game_recommendations.clone();
        let recommendations_loading = recommendations_loading.clone();
        let gaming_communities = gaming_communities.clone();
        let communities_loading = communities_loading.clone();
        let player_networking = player_networking.clone();
        let networking_loading = networking_loading.clone();

        let auth = auth.clone();
        let current_tab = current_tab.clone();
        use_effect_with(
            (auth.state.player.clone(), (*current_tab).clone()),
            move |(player, tab)| {
                if *tab == AnalyticsTab::Players {
                    if let Some(user_id) = player.as_ref().map(|p| p.id.clone()) {
            // Load venue performance for the current user
            let set_venue_performance = venue_performance.clone();
            let set_venue_loading = venue_loading.clone();
            set_venue_loading.set(true);

            let user_id_venue = user_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match crate::api::utils::authenticated_get(&format!(
                    "/api/analytics-enhanced/venues/player-stats/{}",
                    user_id_venue
                ))
                .send()
                .await
                {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(data) = response.json::<Value>().await {
                                // Normalize to an array of venue stat entries
                                let performance_array_opt = data
                                    .get("player_performance")
                                    .and_then(|v| v.as_array())
                                    .cloned()
                                    .or_else(|| {
                                        // Some APIs return [{ player_id, venue_stats: [...] }]
                                        if let Some(arr) = data.as_array() {
                                            if let Some(first) = arr.first() {
                                                return first
                                                    .get("venue_stats")
                                                    .and_then(|v| v.as_array())
                                                    .cloned();
                                            }
                                        }
                                        data.as_array().cloned()
                                    });
                                if let Some(performance_array) = performance_array_opt {
                                    let performance: Vec<VenuePerformance> = performance_array
                                        .iter()
                                        .filter_map(|v| {
                                            if let (
                                                Some(venue_id),
                                                Some(venue_name),
                                                Some(total_contests),
                                                Some(win_rate),
                                            ) = (
                                                v.get("venue_id").and_then(|n| n.as_str()),
                                                v.get("venue_name").and_then(|n| n.as_str()),
                                                v.get("total_contests").and_then(|c| c.as_u64()),
                                                v.get("win_rate").and_then(|w| w.as_f64()),
                                            ) {
                                                Some(VenuePerformance {
                                                    venue_id: venue_id.to_string(),
                                                    venue_name: venue_name.to_string(),
                                                    total_contests,
                                                    win_rate,
                                                })
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    set_venue_performance.set(Some(performance));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to fetch venue performance: {}", e);
                    }
                }
                set_venue_loading.set(false);
            });

            // Load game recommendations
            let set_game_recommendations = game_recommendations.clone();
            let set_recommendations_loading = recommendations_loading.clone();
            set_recommendations_loading.set(true);

            let user_id_rec = user_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match crate::api::utils::authenticated_get(&format!(
                    "/api/analytics-enhanced/games/recommendations/{}?limit=5",
                    user_id_rec
                ))
                .send()
                .await
                {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(data) = response.json::<Value>().await {
                                let rec_array_opt = data
                                    .get("recommendations")
                                    .and_then(|v| v.as_array())
                                    .cloned()
                                    .or_else(|| data.as_array().cloned());
                                if let Some(rec_array) = rec_array_opt {
                                    let recs: Vec<GameRecommendation> = rec_array
                                        .iter()
                                        .filter_map(|v| {
                                            let game_id = v
                                                .get("game_id")
                                                .and_then(|n| n.as_str())
                                                .unwrap_or("")
                                                .to_string();
                                            let game_name = v
                                                .get("game_name")
                                                .and_then(|n| n.as_str())
                                                .or_else(|| v.get("name").and_then(|n| n.as_str()));
                                            let reason = v
                                                .get("reason")
                                                .and_then(|r| r.as_str())
                                                .or(Some("Recommended"));
                                            let score = v
                                                .get("score")
                                                .and_then(|s| s.as_f64())
                                                .or(Some(0.0));
                                            if let (Some(game_name), Some(reason), Some(score)) =
                                                (game_name, reason, score)
                                            {
                                                Some(GameRecommendation {
                                                    game_id,
                                                    game_name: game_name.to_string(),
                                                    reason: reason.to_string(),
                                                    score,
                                                })
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    set_game_recommendations.set(Some(recs));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to fetch game recommendations: {}", e);
                    }
                }
                set_recommendations_loading.set(false);
            });

            // Load gaming communities
            let set_gaming_communities = gaming_communities.clone();
            let set_communities_loading = communities_loading.clone();
            set_communities_loading.set(true);

            let user_id_comm = user_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match crate::api::utils::authenticated_get(&format!(
                    "/api/analytics-enhanced/communities/{}?min_contests=2",
                    user_id_comm
                ))
                .send()
                .await
                {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(data) = response.json::<Value>().await {
                                // Normalize nested communities shape if needed
                                if let Some(arr) =
                                    data.get("gaming_communities").and_then(|v| v.as_array())
                                {
                                    if let Some(first) = arr.first() {
                                        if let Some(inner) = first
                                            .get("gaming_communities")
                                            .and_then(|v| v.as_array())
                                        {
                                            let normalized = serde_json::json!({
                                                "gaming_communities": inner
                                            });
                                            set_gaming_communities.set(Some(normalized));
                                            return;
                                        }
                                    }
                                }
                                set_gaming_communities.set(Some(data));
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to fetch gaming communities: {}", e);
                    }
                }
                set_communities_loading.set(false);
            });

            // Load player networking
            let set_player_networking = player_networking.clone();
            let set_networking_loading = networking_loading.clone();
            set_networking_loading.set(true);

            let user_id_net = user_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match crate::api::utils::authenticated_get(&format!(
                    "/api/analytics-enhanced/networking/{}",
                    user_id_net
                ))
                .send()
                .await
                {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(data) = response.json::<Value>().await {
                                set_player_networking.set(Some(data));
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to fetch player networking: {}", e);
                    }
                }
                set_networking_loading.set(false);
            });

                    }
                }
                || ()
            },
        );
    }

    let on_open_h2h_history = {
        let h2h_modal_open = h2h_modal_open.clone();
        let h2h_modal_loading = h2h_modal_loading.clone();
        let h2h_modal_error = h2h_modal_error.clone();
        let h2h_modal_record = h2h_modal_record.clone();
        let h2h_modal_opponent = h2h_modal_opponent.clone();
        Callback::from(move |(opponent_id, opponent_handle): (String, String)| {
            h2h_modal_open.set(true);
            h2h_modal_loading.set(true);
            h2h_modal_error.set(None);
            h2h_modal_record.set(None);
            h2h_modal_opponent.set((opponent_id.clone(), opponent_handle.clone()));
            let h2h_modal_record = h2h_modal_record.clone();
            let h2h_modal_loading = h2h_modal_loading.clone();
            let h2h_modal_error = h2h_modal_error.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_get(&format!(
                    "/api/analytics/player/head-to-head/{}",
                    opponent_id
                ))
                .send()
                .await
                {
                    Ok(response) if response.ok() => {
                        match response.json::<HeadToHeadRecordDto>().await {
                            Ok(record) => h2h_modal_record.set(Some(record)),
                            Err(e) => {
                                h2h_modal_error.set(Some(format!("Failed to parse H2H record: {}", e)))
                            }
                        }
                    }
                    Ok(response) => {
                        h2h_modal_error.set(Some(format!("H2H request failed: {}", response.status())))
                    }
                    Err(e) => {
                        h2h_modal_error.set(Some(format!("Failed to load H2H record: {}", e)))
                    }
                }
                h2h_modal_loading.set(false);
            });
        })
    };

    let on_close_h2h_modal = {
        let h2h_modal_open = h2h_modal_open.clone();
        Callback::from(move |_| h2h_modal_open.set(false))
    };

    html! {
        <div class="analytics-dashboard">
            <div class="dashboard-header">
                <h1>{"Analytics Statistics"}</h1>
                <p>{"Comprehensive analytics and visualizations for gaming tournament data"}</p>
            </div>

            // Tabs
            <div class="flex space-x-2 border-b border-gray-200 mb-6">
                <button class={classes!(
                        "inline-flex", "items-center", "px-3", "py-2", "text-sm", "font-medium", "border-b-2",
                        if *current_tab == AnalyticsTab::Overview {
                            classes!("border-blue-500", "text-blue-600")
                        } else {
                            classes!("border-transparent", "text-gray-500", "hover:text-gray-700", "hover:border-gray-300")
                        }
                    )}
                    onclick={
                        let on_select_tab = on_select_tab.clone();
                        Callback::from(move |_| on_select_tab.emit(AnalyticsTab::Overview))
                    }>
                    {"Overview"}
                </button>
                <button class={classes!(
                        "inline-flex", "items-center", "px-3", "py-2", "text-sm", "font-medium", "border-b-2",
                        if *current_tab == AnalyticsTab::Contests {
                            classes!("border-blue-500", "text-blue-600")
                        } else {
                            classes!("border-transparent", "text-gray-500", "hover:text-gray-700", "hover:border-gray-300")
                        }
                    )}
                    onclick={
                        let on_select_tab = on_select_tab.clone();
                        Callback::from(move |_| on_select_tab.emit(AnalyticsTab::Contests))
                    }>
                    {"Contests"}
                </button>
                <button class={classes!(
                        "inline-flex", "items-center", "px-3", "py-2", "text-sm", "font-medium", "border-b-2",
                        if *current_tab == AnalyticsTab::Venues {
                            classes!("border-blue-500", "text-blue-600")
                        } else {
                            classes!("border-transparent", "text-gray-500", "hover:text-gray-700", "hover:border-gray-300")
                        }
                    )}
                    onclick={
                        let on_select_tab = on_select_tab.clone();
                        Callback::from(move |_| on_select_tab.emit(AnalyticsTab::Venues))
                    }>
                    {"Venues"}
                </button>
                <button class={classes!(
                        "inline-flex", "items-center", "px-3", "py-2", "text-sm", "font-medium", "border-b-2",
                        if *current_tab == AnalyticsTab::Games {
                            classes!("border-blue-500", "text-blue-600")
                        } else {
                            classes!("border-transparent", "text-gray-500", "hover:text-gray-700", "hover:border-gray-300")
                        }
                    )}
                    onclick={
                        let on_select_tab = on_select_tab.clone();
                        Callback::from(move |_| on_select_tab.emit(AnalyticsTab::Games))
                    }>
                    {"Games"}
                </button>
                <button class={classes!(
                        "inline-flex", "items-center", "px-3", "py-2", "text-sm", "font-medium", "border-b-2",
                        if *current_tab == AnalyticsTab::Players {
                            classes!("border-blue-500", "text-blue-600")
                        } else {
                            classes!("border-transparent", "text-gray-500", "hover:text-gray-700", "hover:border-gray-300")
                        }
                    )}
                    onclick={
                        let on_select_tab = on_select_tab.clone();
                        Callback::from(move |_| on_select_tab.emit(AnalyticsTab::Players))
                    }>
                    {"Players"}
                </button>
            </div>

            if let Some(error_msg) = (*error).as_ref() {
                <div class="error-message">
                    <p>{"Error: "}{error_msg}</p>
                </div>
            }

            if let Some(err) = &*tab_analytics_error {
                <div class="error-message">
                    <p>{"Tab analytics: "}{err}</p>
                </div>
            }

            if *loading {
                // Global skeleton while first load occurs
                <div class="space-y-6">
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                        {for (0..6).map(|_| html!{<div class="h-24 rounded-lg bg-gray-100 animate-pulse"></div>})}
                    </div>
                    <div class="h-96 rounded-lg bg-gray-100 animate-pulse"></div>
                </div>
            } else {
                <div class="dashboard-content">
                    // Overview Tab
                    if *current_tab == AnalyticsTab::Overview {
                    // Platform Overview Section
                    <div class="dashboard-section">
                        <h2>{"🏆 Platform Overview"}</h2>
                        {section_guide(
                            "Headline counts for the whole community — registered players, contests played, active users, games in the library, and venues on the map.",
                            "See whether the scene around you is growing before you commit to a new league night or venue. A healthy player base usually means easier matchmaking and more table options."
                        )}
                        <div class="stats-grid">
                            if let Some(stats) = (*platform_stats).as_ref() {
                                <div class="stat-card primary">
                                    <h3>{"👥 Total Players"}</h3>
                                    <div class="stat-value">{stats["total_players"].as_i64().unwrap_or(0)}</div>
                                    <div class="stat-subtitle">{"Registered users"}</div>
                                </div>
                                <div class="stat-card primary">
                                    <h3>{"🎮 Total Contests"}</h3>
                                    <div class="stat-value">{stats["total_contests"].as_i64().unwrap_or(0)}</div>
                                    <div class="stat-subtitle">{"Games played"}</div>
                                </div>
                                <div class="stat-card success">
                                    <h3>{"🔥 Active Players (7d)"}</h3>
                                    <div class="stat-value">{stats["active_players_7d"].as_i64().unwrap_or(0)}</div>
                                    <div class="stat-subtitle">{"Recent activity"}</div>
                                </div>
                                <div class="stat-card success">
                                    <h3>{"📈 Active Players (30d)"}</h3>
                                    <div class="stat-value">{stats["active_players_30d"].as_i64().unwrap_or(0)}</div>
                                    <div class="stat-subtitle">{"Monthly engagement"}</div>
                                </div>
                                <div class="stat-card info">
                                    <h3>{"🎲 Total Games"}</h3>
                                    <div class="stat-value">{stats["total_games"].as_i64().unwrap_or(0)}</div>
                                    <div class="stat-subtitle">{"Game library"}</div>
                                </div>
                                <div class="stat-card info">
                                    <h3>{"🏟️ Total Venues"}</h3>
                                    <div class="stat-value">{stats["total_venues"].as_i64().unwrap_or(0)}</div>
                                    <div class="stat-subtitle">{"Play locations"}</div>
                                </div>
                            } else {
                                // Skeletons for overview KPIs
                                {for (0..6).map(|_| html!{<div class="h-24 rounded-lg bg-gray-100 animate-pulse"></div>})}
                            }
                        </div>
                    </div>

                    if *tab_analytics_loading && *current_tab == AnalyticsTab::Overview {
                        <div class="dashboard-section">
                            <div class="h-24 rounded-lg bg-gray-100 animate-pulse"></div>
                        </div>
                    } else if let Some(data) = &*tab_analytics {
                        if *current_tab == AnalyticsTab::Overview {
                            <div class="dashboard-section">
                                <h2>{"📈 Player Engagement"}</h2>
                                {section_guide(
                                    "How many people joined in the last 30 days versus came back for another contest, plus the share of contests that finished with recorded results.",
                                    "Spot whether the community is attracting newcomers and keeping them. A high completion rate means posted results you can trust for ratings and rivalries."
                                )}
                                <div class="stats-grid">
                                    <div class="stat-card success">
                                        <h3>{"New Players (30d)"}</h3>
                                        <div class="stat-value">{data["new_players_30d"].as_i64().unwrap_or(0)}</div>
                                        <div class="stat-subtitle">{"First contest in period"}</div>
                                    </div>
                                    <div class="stat-card success">
                                        <h3>{"Returning (30d)"}</h3>
                                        <div class="stat-value">{data["returning_players_30d"].as_i64().unwrap_or(0)}</div>
                                        <div class="stat-subtitle">{"Played before + active now"}</div>
                                    </div>
                                    <div class="stat-card warning">
                                        <h3>{"Completion Rate"}</h3>
                                        <div class="stat-value">{format!("{:.0}%", data["contest_completion_rate_pct"].as_f64().unwrap_or(0.0))}</div>
                                        <div class="stat-subtitle">{"Contests with recorded results"}</div>
                                    </div>
                                </div>
                            </div>
                            <div class="dashboard-section">
                                <h2>{"📊 Week-over-Week Growth"}</h2>
                                {section_guide(
                                    "Contests and unique active players this calendar week compared with last week, both in your timezone. The mini bar chart shows contest volume by ISO week.",
                                    "Know if this is a busy week to find open tables or a quiet one to organize your own game. Rising activity often means more opponents at your skill level."
                                )}
                                {if let Some(wow) = data.get("week_over_week") {
                                    html! {
                                        <>
                                            <div class="metrics-grid">
                                                <div class="metric-card">
                                                    <h3>{"Contests"}</h3>
                                                    <div class="metric-value">{wow["contests_this_week"].as_i64().unwrap_or(0)}</div>
                                                    <div class="metric-description">{format!("vs {} last week ({:+.0}%)", wow["contests_last_week"].as_i64().unwrap_or(0), wow["contests_change_pct"].as_f64().unwrap_or(0.0))}</div>
                                                </div>
                                                <div class="metric-card">
                                                    <h3>{"Active Players"}</h3>
                                                    <div class="metric-value">{wow["players_this_week"].as_i64().unwrap_or(0)}</div>
                                                    <div class="metric-description">{format!("vs {} last week ({:+.0}%)", wow["players_last_week"].as_i64().unwrap_or(0), wow["players_change_pct"].as_f64().unwrap_or(0.0))}</div>
                                                </div>
                                            </div>
                                            {if let Some(spark) = wow.get("weekly_contest_sparkline").and_then(|v| v.as_array()) {
                                                html! {
                                                    <div class="mt-4 flex flex-wrap gap-2 items-end">
                                                        {for spark.iter().map(|pt| {
                                                            let count = pt["count"].as_i64().unwrap_or(0);
                                                            let h = (count as f64).sqrt() * 8.0 + 4.0;
                                                            html! {
                                                                <div class="text-center" title={format!("{}: {} contests", pt["label"].as_str().unwrap_or(""), count)}>
                                                                    <div class="bg-blue-500 rounded-t mx-auto" style={format!("width:2rem;height:{}px", h as i32)}></div>
                                                                    <div class="text-[10px] text-gray-500 mt-1">{pt["label"].as_str().unwrap_or("").chars().rev().take(3).collect::<String>().chars().rev().collect::<String>()}</div>
                                                                </div>
                                                            }
                                                        })}
                                                    </div>
                                                }
                                            } else { html!{} }}
                                        </>
                                    }
                                } else { html!{<div class="no-data">{"No growth data"}</div>} }}
                            </div>
                        }
                    }

                    // Engagement Metrics Section
                    <div class="dashboard-section">
                        <h2>{"📊 Platform Health Metrics"}</h2>
                        {section_guide(
                            "Recent contest volume, typical table size, contests per player, what share of players were active this month, and how this month compares to the yearly average.",
                            "Gauge whether people are actually playing or just signed up. Steady contests per player and activity rate suggest you'll find regular events, not one-off gatherings."
                        )}
                        <div class="metrics-grid">
                            if let Some(stats) = (*platform_stats).as_ref() {
                                <div class="metric-card">
                                    <h3>{"🎯 Contest Activity"}</h3>
                                    <div class="metric-value">
                                        {stats["contests_30d"].as_i64().unwrap_or(0)}
                                    </div>
                                    <div class="metric-description">{"Contests this month"}</div>
                                </div>
                                <div class="metric-card">
                                    <h3>{"👥 Contest Size"}</h3>
                                    <div class="metric-value">
                                        {format!("{:.1}", stats["average_participants_per_contest"].as_f64().unwrap_or(0.0))}
                                    </div>
                                    <div class="metric-description">{"Avg players per contest"}</div>
                                </div>
                                if let Some(ins) = (*insights).as_ref() {
                                    <div class="metric-card">
                                        <h3>{"📈 Engagement"}</h3>
                                        <div class="metric-value">
                                            {format!("{:.1}", ins["metrics"]["contests_per_player"].as_f64().unwrap_or(0.0))}
                                        </div>
                                        <div class="metric-description">{"Contests per player"}</div>
                                    </div>
                                    <div class="metric-card">
                                        <h3>{"🧭 Activity Rate"}</h3>
                                        <div class="metric-value">
                                            {format!("{:.0}%", ins["metrics"]["activity_rate"].as_f64().unwrap_or(0.0))}
                                        </div>
                                        <div class="metric-description">{"Active players (30d) / total"}</div>
                                    </div>
                                    <div class="metric-card">
                                        <h3>{"📈 Monthly Growth"}</h3>
                                        <div class="metric-value">
                                            {format!("{:.0}%", ins["metrics"]["monthly_growth"].as_f64().unwrap_or(0.0))}
                                        </div>
                                        <div class="metric-description">{"Contests vs 12-mo avg"}</div>
                                    </div>
                                    <div class="metric-card">
                                        <h3>{"🏥 Health"}</h3>
                                        <div class="metric-value">
                                            {ins["insights"]["platform_health"].as_str().unwrap_or("--")}
                                        </div>
                                        <div class="metric-description">{ins["metrics"]["growth_trend"].as_str().unwrap_or("")}</div>
                                    </div>
                                }
                            }
                        </div>
                    </div>

                    // Platform Insights Section
                    <div class="dashboard-section">
                        <h2>{"💡 Platform Insights"}</h2>
                        {section_guide(
                            "Quick read on game and venue variety plus overall contest throughput across the platform.",
                            "A wider game library and more venues mean more chances to try new titles and meet players without traveling far. Use this when planning what to learn or where to host."
                        )}
                        <div class="insights-grid">
                            if let Some(stats) = (*platform_stats).as_ref() {
                                <div class="insight-card">
                                    <h3>{"🎮 Game Diversity"}</h3>
                                    <div class="insight-content">
                                        <div class="insight-stat">
                                            <span class="insight-value">{stats["total_games"].as_i64().unwrap_or(0)}</span>
                                            <span class="insight-label">{"Total Games"}</span>
                                        </div>
                                        <div class="insight-stat">
                                            <span class="insight-value">{stats["total_venues"].as_i64().unwrap_or(0)}</span>
                                            <span class="insight-label">{"Total Venues"}</span>
                                        </div>
                                        <div class="insight-description">
                                            {"Your platform offers a diverse selection of games and venues for players to explore."}
                                        </div>
                                    </div>
                                </div>
                                <div class="insight-card">
                                    <h3>{"📊 Activity Analysis"}</h3>
                                    <div class="insight-content">
                                        <div class="insight-stat">
                                            <span class="insight-value">{stats["total_contests"].as_i64().unwrap_or(0)}</span>
                                            <span class="insight-label">{"Total Contests"}</span>
                                        </div>
                                        <div class="insight-stat">
                                            <span class="insight-value">{stats["total_players"].as_i64().unwrap_or(0)}</span>
                                            <span class="insight-label">{"Total Players"}</span>
                                        </div>
                                        <div class="insight-description">
                                            {"Strong contest activity with "}{stats["total_contests"].as_i64().unwrap_or(0)}{" contests across "}{stats["total_players"].as_i64().unwrap_or(0)}{" players."}
                                        </div>
                                    </div>
                                </div>
                                <div class="insight-card">
                                    <h3>{"🚀 Performance Trends"}</h3>
                                    <div class="insight-content">
                                        <div class="insight-stat">
                                            <span class="insight-value">{stats["contests_30d"].as_i64().unwrap_or(0)}</span>
                                            <span class="insight-label">{"Recent Contests"}</span>
                                        </div>
                                        <div class="insight-stat">
                                            <span class="insight-value">{format!("{:.1}", stats["average_participants_per_contest"].as_f64().unwrap_or(0.0))}</span>
                                            <span class="insight-label">{"Avg Participants"}</span>
                                        </div>
                                        <div class="insight-description">
                                            {"Recent activity shows "}{stats["contests_30d"].as_i64().unwrap_or(0)}{" contests with an average of "}{format!("{:.1}", stats["average_participants_per_contest"].as_f64().unwrap_or(0.0))}{" participants each."}
                                        </div>
                                    </div>
                                </div>
                            } else {
                                <div class="h-40 rounded-lg bg-gray-100 animate-pulse"></div>
                                <div class="h-40 rounded-lg bg-gray-100 animate-pulse"></div>
                                <div class="h-40 rounded-lg bg-gray-100 animate-pulse"></div>
                            }
                        </div>
                    </div>

                    // Top Games & Venues Section
                    <div class="dashboard-section">
                        <h2>{"🏆 Popular Games & Venues"}</h2>
                        {section_guide(
                            "The games and locations with the most recorded plays on the platform right now.",
                            "Follow the crowd when you want a guaranteed pickup game, or avoid the top entries when you're looking for something less crowded. Busy venues are good bets for finding opponents."
                        )}
                        <div class="popularity-grid">
                            if let Some(stats) = (*platform_stats).as_ref() {
                                <div class="popularity-card">
                                    <h3>{"🎮 Top Games"}</h3>
                                    if let Some(top_games) = stats["top_games"].as_array() {
                                        if !top_games.is_empty() {
                                            <div class="popularity-list">
                                                {top_games.iter().enumerate().map(|(i, game)| {
                                                    html! {
                                                        <div class="popularity-item">
                                                            <span class="rank">{i + 1}</span>
                                                            <span class="name">{game_link_from(game, "game_id", "game_name", "Unknown")}</span>
                                                            <span class="count">{game["plays"].as_i64().unwrap_or(0)} {"plays"}</span>
                                                        </div>
                                                    }
                                                }).collect::<Html>()}
                                            </div>
                                        } else {
                                            <div class="no-data">{"No game data available"}</div>
                                        }
                                    } else {
                                        <div class="no-data">{"No game data available"}</div>
                                    }
                                </div>
                                <div class="popularity-card">
                                    <h3>{"🏟️ Top Venues"}</h3>
                                    if let Some(top_venues) = stats["top_venues"].as_array() {
                                        if !top_venues.is_empty() {
                                            <div class="popularity-list">
                                                {top_venues.iter().enumerate().map(|(i, venue)| {
                                                    html! {
                                                        <div class="popularity-item">
                                                            <span class="rank">{i + 1}</span>
                                                            <span class="name">{venue_link_from(venue, "venue_id", "venue_name", "Unknown")}</span>
                                                            <span class="count">{venue["contests_held"].as_i64().unwrap_or(0)} {"contests"}</span>
                                                        </div>
                                                    }
                                                }).collect::<Html>()}
                                            </div>
                                        } else {
                                            <div class="no-data">{"No venue data available"}</div>
                                        }
                                    } else {
                                        <div class="no-data">{"No venue data available"}</div>
                                    }
                                </div>
                            } else {
                                <div class="h-48 rounded-lg bg-gray-100 animate-pulse"></div>
                                <div class="h-48 rounded-lg bg-gray-100 animate-pulse"></div>
                            }
                        </div>
                    </div>

                    // Growth Trends Section
                    <div class="dashboard-section">
                        <h2>{"📈 Activity Trends"}</h2>
                        {section_guide(
                            "Side-by-side snapshots of this month's contests and active players, total platform scale, and simple engagement ratios.",
                            "Compare short-term buzz (30 days) with long-term size. If monthly contests are rising while average table size holds steady, the scene is growing without thinning out your games."
                        )}
                        <div class="trends-grid">
                            if let Some(stats) = (*platform_stats).as_ref() {
                                <div class="trend-card">
                                    <h3>{"🎯 Monthly Activity"}</h3>
                                    <div class="trend-metrics">
                                        <div class="trend-item">
                                            <span class="trend-label">{"Contests This Month"}</span>
                                            <span class="trend-value positive">
                                                {stats["contests_30d"].as_i64().unwrap_or(0)}
                                            </span>
                                        </div>
                                        <div class="trend-item">
                                            <span class="trend-label">{"Active Players"}</span>
                                            <span class="trend-value">
                                                {stats["active_players_30d"].as_i64().unwrap_or(0)}
                                            </span>
                                        </div>
                                        <div class="trend-item">
                                            <span class="trend-label">{"Avg Contest Size"}</span>
                                            <span class="trend-value">
                                                {format!("{:.1}", stats["average_participants_per_contest"].as_f64().unwrap_or(0.0))}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                                <div class="trend-card">
                                    <h3>{"📊 Platform Scale"}</h3>
                                    <div class="trend-metrics">
                                        <div class="trend-item">
                                            <span class="trend-label">{"Total Players"}</span>
                                            <span class="trend-value">
                                                {stats["total_players"].as_i64().unwrap_or(0)}
                                            </span>
                                        </div>
                                        <div class="trend-item">
                                            <span class="trend-label">{"Total Contests"}</span>
                                            <span class="trend-value">
                                                {stats["total_contests"].as_i64().unwrap_or(0)}
                                            </span>
                                        </div>
                                        <div class="trend-item">
                                            <span class="trend-label">{"Total Games"}</span>
                                            <span class="trend-value">
                                                {stats["total_games"].as_i64().unwrap_or(0)}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                                <div class="trend-card">
                                    <h3>{"🚀 Performance Indicators"}</h3>
                                    <div class="trend-metrics">
                                        <div class="trend-item">
                                            <span class="trend-label">{"Player Engagement"}</span>
                                            <span class="trend-value">
                                                {if stats["total_players"].as_i64().unwrap_or(0) > 0 {
                                                    let contests_per_player = stats["total_contests"].as_i64().unwrap_or(0) as f64 /
                                                                           stats["total_players"].as_i64().unwrap_or(1) as f64;
                                                    format!("{:.1}", contests_per_player)
                                                } else {
                                                    "0.0".to_string()
                                                }}
                                            </span>
                                        </div>
                                        <div class="trend-item">
                                            <span class="trend-label">{"Activity Rate"}</span>
                                            <span class="trend-value">
                                                {if stats["total_players"].as_i64().unwrap_or(0) > 0 {
                                                    let activity_rate = (stats["active_players_30d"].as_i64().unwrap_or(0) as f64 /
                                                                      stats["total_players"].as_i64().unwrap_or(1) as f64) * 100.0;
                                                    format!("{:.0}%", activity_rate)
                                                } else {
                                                    "0%".to_string()
                                                }}
                                            </span>
                                        </div>
                                        <div class="trend-item">
                                            <span class="trend-label">{"Monthly Growth"}</span>
                                            <span class="trend-value">
                                                {if stats["total_contests"].as_i64().unwrap_or(0) > 0 {
                                                    let monthly_avg = stats["total_contests"].as_i64().unwrap_or(0) as f64 / 12.0;
                                                    let current_month = stats["contests_30d"].as_i64().unwrap_or(0) as f64;
                                                    if current_month > monthly_avg * 1.2 {
                                                        "↗️ Above Avg".to_string()
                                                    } else if current_month < monthly_avg * 0.8 {
                                                        "↘️ Below Avg".to_string()
                                                    } else {
                                                        "→ On Track".to_string()
                                                    }
                                                } else {
                                                    "N/A".to_string()
                                                }}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            } else {
                                <div class="trend-card">
                                    <h3>{"📊 Activity Trends"}</h3>
                                    <div class="trend-metrics">
                                        <div class="trend-item">
                                            <span class="trend-label">{"No data available"}</span>
                                            <span class="trend-value">{"--"}</span>
                                        </div>
                                    </div>
                                </div>
                            }
                        </div>
                    </div>

                    // Activity Metrics Chart (DAU & Contests per day)
                    if let Some(chart_data) = (*activity_metrics_chart).as_ref() {
                        <div class="dashboard-section">
                            <h2>{"Activity Metrics"}</h2>
                            {section_guide(
                                "A line chart of monthly active players and monthly contests over recent months.",
                                "See whether participation is climbing or cooling off before you invest in a new game night. Align your schedule with upward trends to meet more players."
                            )}
                            <div class="chart-container">
                                <ChartRenderer
                                    chart_data={chart_data.clone()}
                                    chart_id={"activity-metrics-chart".to_string()}
                                    width={Some(800)}
                                    height={Some(400)}
                                />
                            </div>
                        </div>
                    }

                    // Quick Actions
                    <div class="dashboard-section">
                        <h2>{"⚡ Quick Actions"}</h2>
                        <div class="actions-grid">
                            <button class="action-button primary" onclick={|_| {
                                gloo_utils::window().location().reload().unwrap();
                            }}>
                                {"🔄 Refresh Dashboard"}
                            </button>
                        </div>
                    </div>

                    // System Health Section
                    <div class="dashboard-section">
                        <h2>{"💚 System Health"}</h2>
                        if *system_health_loading {
                            <div class="h-20 rounded-lg bg-gray-100 animate-pulse"></div>
                        } else if let Some(health) = &*system_health {
                            {{
                                let db_ok = health
                                    .get("services")
                                    .and_then(|s| s.get("database"))
                                    .and_then(|d| d.get("status"))
                                    .and_then(|s| s.as_str())
                                    == Some("healthy");
                                let redis_ok = health
                                    .get("services")
                                    .and_then(|s| s.get("redis"))
                                    .and_then(|d| d.get("status"))
                                    .and_then(|s| s.as_str())
                                    == Some("healthy");
                                let api_ok = health.get("status").and_then(|s| s.as_str()) == Some("ok");
                                html! {
                                    <div class="health-grid">
                                        <div class="health-card">
                                            <h3>{"Database Status"}</h3>
                                            <div class={classes!("health-indicator", if db_ok { "online" } else { "offline" })}>
                                                {if db_ok { "🟢 Online" } else { "🔴 Offline" }}
                                            </div>
                                            <div class="health-details">
                                                {health["services"]["database"]["message"].as_str().unwrap_or("Database check")}
                                            </div>
                                        </div>
                                        <div class="health-card">
                                            <h3>{"Cache Status"}</h3>
                                            <div class={classes!("health-indicator", if redis_ok { "online" } else { "offline" })}>
                                                {if redis_ok { "🟢 Online" } else { "🔴 Offline" }}
                                            </div>
                                            <div class="health-details">
                                                {health["services"]["redis"]["message"].as_str().unwrap_or("Redis check")}
                                            </div>
                                        </div>
                                        <div class="health-card">
                                            <h3>{"API Response"}</h3>
                                            <div class={classes!("health-indicator", if api_ok { "online" } else { "offline" })}>
                                                {if api_ok { "🟢 Online" } else { "🔴 Degraded" }}
                                            </div>
                                            <div class="health-details">{"Platform health endpoint"}</div>
                                        </div>
                                    </div>
                                }
                            }}
                        } else {
                            <div class="no-data"><p>{"Health status unavailable"}</p></div>
                        }
                    </div>
                    }

                    // Contests Tab
                    if *current_tab == AnalyticsTab::Contests {
                        if *tab_analytics_loading {
                            <div class="dashboard-section"><div class="h-24 rounded-lg bg-gray-100 animate-pulse"></div></div>
                        } else if let Some(data) = &*tab_analytics {
                            <div class="dashboard-section">
                                <h2>{"⏱️ Contest Metrics"}</h2>
                                {section_guide(
                                    "Typical contest length from start to finish, and how long it usually takes for a posted contest to reach its start time.",
                                    "Plan your evening — know if games run two hours or four, and whether you need to sign up days ahead or can jump in same-day."
                                )}
                                <div class="stats-grid">
                                    <div class="stat-card primary">
                                        <h3>{"Avg Duration"}</h3>
                                        <div class="stat-value">{format!("{:.0}", data["avg_duration_minutes"].as_f64().unwrap_or(0.0))}</div>
                                        <div class="stat-subtitle">{"minutes"}</div>
                                    </div>
                                    <div class="stat-card info">
                                        <h3>{"Time to Fill"}</h3>
                                        <div class="stat-value">{format!("{:.1}", data["avg_time_to_fill_hours"].as_f64().unwrap_or(0.0))}</div>
                                        <div class="stat-subtitle">{"hours (created → start)"}</div>
                                    </div>
                                </div>
                            </div>
                            {if let Some(sizes) = data.get("size_distribution").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"👥 Contest Size Distribution"}</h2>
                                        {section_guide(
                                            "How many contests were played at each player count (2-player, 3–4, 5–6, etc.).",
                                            "Pick games and events that match how people actually play here. If most tables are 4-player, prioritize titles that shine at that count."
                                        )}
                                        <div class="popularity-list">
                                            {for sizes.iter().map(|row| html! {
                                                <div class="popularity-item">
                                                    <span class="name">{row["label"].as_str().unwrap_or("")}</span>
                                                    <span class="count">{row["count"].as_i64().unwrap_or(0)} {"contests"}</span>
                                                </div>
                                            })}
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                            {if let Some(cells) = data.get("peak_participants_heatmap").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"📊 Peak Capacity by Time (avg participants)"}</h2>
                                        {section_guide(
                                            &format!(
                                                "A weekday × hour grid showing average players per contest in {}. Darker green = fuller tables.",
                                                timezone_label(&player_timezone)
                                            ),
                                            "Find the sweet spots when tables actually fill up — great for competitive play — versus quieter hours when you can learn a new game without pressure."
                                        )}
                                        <div class="overflow-x-auto">
                                            <div class="inline-grid gap-1" style="grid-template-columns: auto repeat(24, 1.5rem);">
                                                <div></div>
                                                {for (0..24).map(|h| html!{<div class="w-6 text-[10px] text-gray-500 text-center">{format!("{:02}", h)}</div>})}
                                                {for (0..7).map(|day_idx| {
                                                    let day_label = match day_idx { 0=>"Sun",1=>"Mon",2=>"Tue",3=>"Wed",4=>"Thu",5=>"Fri", _=>"Sat" };
                                                    html! {
                                                        <>
                                                            <div class="text-[10px] text-gray-500 pr-1">{day_label}</div>
                                                            {for (0..24).map(|hour| {
                                                                let val = cells.iter().find(|c| c["day"].as_i64().unwrap_or(-1) == day_idx && c["hour"].as_i64().unwrap_or(-1) == hour)
                                                                    .and_then(|c| c["value"].as_f64()).unwrap_or(0.0);
                                                                let intensity = if val == 0.0 { 0.0 } else { (val / 6.0).min(1.0) };
                                                                let bg = if intensity == 0.0 { "bg-gray-100" } else if intensity < 0.25 { "bg-green-100" } else if intensity < 0.5 { "bg-green-200" } else if intensity < 0.75 { "bg-green-400" } else { "bg-green-600" };
                                                                html!{<div class={classes!("w-6","h-6","rounded", bg)} title={format!("{} {:02}:00 — {:.1} avg players", day_label, hour, val)}></div>}
                                                            })}
                                                        </>
                                                    }
                                                })}
                                            </div>
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                        }
                        if let Some(chart_data) = (*contest_trends_chart).as_ref() {
                            <div class="dashboard-section">
                                <h2>{"Contest Trends"}</h2>
                                {section_guide(
                                    "Monthly contest counts over the past year — how many games were logged each month.",
                                    "Spot busy seasons and slow months. Schedule your big events when activity is already high, or fill a gap when the community goes quiet."
                                )}
                                <div class="chart-container">
                                    <ChartRenderer
                                        chart_data={chart_data.clone()}
                                        chart_id={"contest-trends-chart".to_string()}
                                        width={Some(800)}
                                        height={Some(400)}
                                    />
                                </div>
                            </div>
                        }

                        // Contest Heatmap (weekday x hour)
                        <div class="dashboard-section">
                            <div class="flex items-center justify-between">
                                <h2>{"When People Play (Heatmap)"}</h2>
                                <div class="flex items-center space-x-2 text-sm">
                                    <span class="text-gray-600">{"Window:"}</span>
                                    {for [8, 12, 26, 52].iter().map(|w| {
                                        let selected = *w == *heatmap_weeks;
                                        let heatmap_weeks = heatmap_weeks.clone();
                                        html!{
                                            <button
                                                class={classes!(
                                                    "px-2", "py-1", "rounded",
                                                    if selected { "bg-blue-600 text-white" } else { "bg-gray-100 text-gray-700 hover:bg-gray-200" }
                                                )}
                                                onclick={Callback::from(move |_| heatmap_weeks.set(*w))}
                                            >{format!("{}w", w)}</button>
                                        }
                                    })}
                                </div>
                            </div>
                            {section_guide(
                                &format!(
                                    "Each cell is a day-of-week row and hour column in {}. Color shows how many contests started in that slot over the selected window (darker blue = more).",
                                    timezone_label(&player_timezone)
                                ),
                                "Compare community rhythm with your own schedule. Dark bands reveal when players are online — line up your regular game night or discover underused slots with less competition for tables."
                            )}
                            if *contest_heatmap_loading {
                                <div class="h-64 rounded-lg bg-gray-100 animate-pulse"></div>
                            } else if let Some(err) = &*contest_heatmap_error {
                                <div class="error-message"><p>{err}</p></div>
                            } else if let Some(data) = &*contest_heatmap {
                                // Expecting shape: { buckets: [[u64;24];7] } where 0=Sun..6=Sat
                                { if let Some(week_rows) = data.get("buckets").and_then(|v| v.as_array()) {
                                    html!{
                                            <div>
                                                <div class="overflow-x-auto">
                                                <div class="inline-grid gap-1" style="grid-template-columns: auto repeat(24, 1.5rem);">
                                                    <div></div>
                                                    {for (0..24).map(|h| html!{<div class="w-6 text-[10px] text-gray-500 text-center">{format!("{:02}", h)}</div>})}
                                                {for week_rows.iter().enumerate().map(|(day_idx, row)| {
                                                    let day_label = match day_idx { 0=>"Sun",1=>"Mon",2=>"Tue",3=>"Wed",4=>"Thu",5=>"Fri", _=>"Sat" };
                                                    let hours = row.as_array().unwrap_or(&vec![]).clone();
                                                    html!{
                                                        <>
                                                            <div class="text-[10px] text-gray-500 pr-1">{day_label}</div>
                                                            {for hours.iter().enumerate().map(|(h, val)| {
                                                                let raw = val.as_u64().unwrap_or(0);
                                                                let v = raw as f64;
                                                                let intensity = if v == 0.0 { 0.0 } else { (v.log10()+1.0).min(4.0)/4.0 };
                                                                let bg = if intensity == 0.0 { "bg-gray-100" } else if intensity < 0.25 { "bg-blue-100" } else if intensity < 0.5 { "bg-blue-200" } else if intensity < 0.75 { "bg-blue-400" } else { "bg-blue-600" };
                                                                let title = format!("{} {:02}:00 — {} contests", day_label, h, raw);
                                                                html!{<div class={classes!("w-6","h-6","rounded", bg)} title={title}></div>}
                                                            })}
                                                        </>
                                                    }
                                                })}
                                            </div>
                                                </div>
                                                <div class="mt-3 flex items-center space-x-2 text-xs text-gray-600">
                                                <span>{"Fewer"}</span>
                                                <div class="w-6 h-3 bg-gray-100 rounded"></div>
                                                <div class="w-6 h-3 bg-blue-100 rounded"></div>
                                                <div class="w-6 h-3 bg-blue-200 rounded"></div>
                                                <div class="w-6 h-3 bg-blue-400 rounded"></div>
                                                <div class="w-6 h-3 bg-blue-600 rounded"></div>
                                                <span>{"More"}</span>
                                                <span class="ml-4">{format!("Window: last {} weeks", data.get("weeks").and_then(|w| w.as_i64()).unwrap_or(8))}</span>
                                                </div>
                                            </div>
                                    }
                                } else { html!{<div class="text-sm text-gray-500">{"No heatmap data available"}</div>} } }
                            } else {
                                <div class="text-sm text-gray-500">{"No heatmap data available"}</div>
                            }
                        </div>

                        <div class="dashboard-section">
                            <h2>{"🕹️ Recent Contests"}</h2>
                            {section_guide(
                                "The latest contests logged on the platform, with game, player count, and duration.",
                                "Jump straight into a finished or in-progress event — open any row to see results, players, and venue details."
                            )}
                            if *recent_contests_loading {
                                <div class="h-32 rounded-lg bg-gray-100 animate-pulse"></div>
                            } else if let Some(err) = &*recent_contests_error {
                                <div class="error-message"><p>{err}</p></div>
                            } else if let Some(rows) = &*recent_contests {
                                if rows.is_empty() {
                                    <div class="no-data"><p>{"No recent contests found"}</p></div>
                                } else {
                                    <div class="overflow-x-auto">
                                        <table class="min-w-full divide-y divide-gray-200 text-sm">
                                            <thead class="bg-gray-50">
                                                <tr>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Contest"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"When"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Players"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Duration"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Top Game"}</th>
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white divide-y divide-gray-200">
                                                {for rows.iter().map(|c| {
                                                    let contest_id = c["contest_id"].as_str().unwrap_or("");
                                                    let label = contest_label_from_json(c);
                                                    let when = c.get("started_at")
                                                        .and_then(|v| v.as_str())
                                                        .map(|s| format_in_player_timezone(s, &player_timezone))
                                                        .unwrap_or_else(|| "—".to_string());
                                                    let players = c["participant_count"].as_i64().unwrap_or(0);
                                                    let duration = c["duration_minutes"].as_i64().unwrap_or(0);
                                                    let game_name = c.get("most_popular_game").and_then(|v| v.as_str()).unwrap_or("—");
                                                    let game_cell = if let Some(gid) = c.get("most_popular_game_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                                                        html! { <td class="px-4 py-2">{game_link(gid, game_name)}</td> }
                                                    } else {
                                                        html! { <td class="px-4 py-2 text-gray-600">{game_name}</td> }
                                                    };
                                                    html! {
                                                        <tr class="hover:bg-gray-50">
                                                            <td class="px-4 py-2">{contest_link(contest_id, &label)}</td>
                                                            <td class="px-4 py-2 text-gray-600">{when}</td>
                                                            <td class="px-4 py-2">{players}</td>
                                                            <td class="px-4 py-2">{format!("{} min", duration)}</td>
                                                            {game_cell}
                                                        </tr>
                                                    }
                                                })}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                            } else {
                                <div class="no-data"><p>{"Recent contests not loaded"}</p></div>
                            }
                        </div>
                    }



                    // Venues Tab
                    if *current_tab == AnalyticsTab::Venues {
                        if let Some(stats) = (*platform_stats).as_ref() {
                            <div class="dashboard-section">
                                <h2>{"🏟️ Venue Overview"}</h2>
                                {section_guide(
                                    "Total registered play locations on the platform.",
                                    "More venues usually means games closer to home and different atmospheres — cafes, shops, clubs — to match how you like to play."
                                )}
                                <div class="stats-grid">
                                    <div class="stat-card info">
                                        <h3>{"Total Venues"}</h3>
                                        <div class="stat-value">{stats["total_venues"].as_i64().unwrap_or(0)}</div>
                                        <div class="stat-subtitle">{"Registered play locations"}</div>
                                    </div>
                                </div>
                            </div>
                        }

                        <div class="dashboard-section">
                            <h2>{"🏟️ Top Venues"}</h2>
                            {section_guide(
                                "Venues ranked by how many contests have been held at each location.",
                                "The busiest spots are where you're most likely to walk in and find a game. Use this when choosing where to spend your first visit."
                            )}
                            if let Some(stats) = (*platform_stats).as_ref() {
                                if let Some(top_venues) = stats["top_venues"].as_array() {
                                    if !top_venues.is_empty() {
                                        <div class="popularity-list">
                                            {top_venues.iter().enumerate().map(|(i, venue)| {
                                                html! {
                                                    <div class="popularity-item">
                                                        <span class="rank">{i + 1}</span>
                                                        <span class="name">{venue_link_from(venue, "venue_id", "venue_name", "Unknown")}</span>
                                                        <span class="count">{venue["contests_held"].as_i64().unwrap_or(0)} {"contests"}</span>
                                                    </div>
                                                }
                                            }).collect::<Html>()}
                                        </div>
                                    } else {
                                        <div class="no-data">{"No venue data available"}</div>
                                    }
                                } else {
                                    <div class="no-data">{"No venue data available"}</div>
                                }
                            }
                        </div>

                        if *tab_analytics_loading {
                            <div class="dashboard-section"><div class="h-24 rounded-lg bg-gray-100 animate-pulse"></div></div>
                        } else if let Some(data) = &*tab_analytics {
                            <div class="dashboard-section">
                                <h2>{"🔄 Venue Retention"}</h2>
                                {section_guide(
                                    "The percentage of players who return to the same venue for another contest after their first visit.",
                                    "High retention means a venue people genuinely come back to — not just a one-time meetup spot. Favor those places for building a local group."
                                )}
                                <div class="stat-card success">
                                    <h3>{"Return Rate"}</h3>
                                    <div class="stat-value">{format!("{:.0}%", data["venue_retention_rate_pct"].as_f64().unwrap_or(0.0))}</div>
                                    <div class="stat-subtitle">{"Players who revisit the same venue"}</div>
                                </div>
                            </div>
                            {if let Some(rows) = data.get("utilization").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"📈 Venue Utilization (30d)"}</h2>
                                        {section_guide(
                                            "Contests held at each venue in the last 30 days.",
                                            "See which locations are hot right now versus fading. A venue with steady recent activity is a safer bet for finding players this month."
                                        )}
                                        <div class="popularity-list">
                                            {for rows.iter().map(|v| html! {
                                                <div class="popularity-item">
                                                    <span class="name">{venue_link_from(v, "venue_id", "venue_name", "Unknown")}</span>
                                                    <span class="count">{v["contests_30d"].as_i64().unwrap_or(0)} {"contests"}</span>
                                                </div>
                                            })}
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                            {if let Some(rows) = data.get("diverse_venues").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"🎲 Most Diverse Venues"}</h2>
                                        {section_guide(
                                            "Venues that host the widest variety of games, with total contest counts.",
                                            "Great for explorers — these locations rotate through many titles so you can sample new games without committing to a single league."
                                        )}
                                        <div class="overflow-x-auto">
                                            <table class="min-w-full divide-y divide-gray-200">
                                                <thead class="bg-gray-50"><tr>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Venue"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Unique Games"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Contests"}</th>
                                                </tr></thead>
                                                <tbody class="bg-white divide-y divide-gray-200">
                                                    {for rows.iter().map(|v| html! {
                                                        <tr>
                                                            <td class="px-4 py-2 text-sm">{venue_link_from(v, "venue_id", "venue_name", "")}</td>
                                                            <td class="px-4 py-2 text-sm">{v["unique_games"].as_i64().unwrap_or(0)}</td>
                                                            <td class="px-4 py-2 text-sm">{v["total_contests"].as_i64().unwrap_or(0)}</td>
                                                        </tr>
                                                    })}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                            {if let Some(rows) = data.get("timeslot_breakdown").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"🕐 Venue × Timeslot Activity"}</h2>
                                        {section_guide(
                                            &format!(
                                                "Which venues see the most contests in Morning, Afternoon, or Evening buckets, using {} local time.",
                                                timezone_label(&player_timezone)
                                            ),
                                            "Match a venue to your lifestyle — morning coffee-shop sessions versus evening weeknight games. Pick the combo that fits when you can actually play."
                                        )}
                                        <div class="overflow-x-auto">
                                            <table class="min-w-full divide-y divide-gray-200 text-sm">
                                                <thead class="bg-gray-50"><tr>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Venue"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Timeslot"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Contests"}</th>
                                                </tr></thead>
                                                <tbody class="bg-white divide-y divide-gray-200">
                                                    {for rows.iter().take(15).map(|v| html! {
                                                        <tr>
                                                            <td class="px-4 py-2">{venue_link_from(v, "venue_id", "venue_name", "")}</td>
                                                            <td class="px-4 py-2">{v["timeslot"].as_str().unwrap_or("")}</td>
                                                            <td class="px-4 py-2">{format!("{:.0}", v["contest_count"].as_f64().unwrap_or(0.0))}</td>
                                                        </tr>
                                                    })}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                        }

                        <div class="dashboard-section">
                            <h2>{"📍 Your Venue Performance"}</h2>
                            {section_guide(
                                "Every venue you've played at, with your contest count and win rate at each.",
                                "Discover your home turf — where you play most and where you perform best. Lean into strong venues for tournaments and try new spots where your win rate suggests room to grow."
                            )}
                            if *venue_loading {
                                <div class="loading-container"><p>{"Loading venue performance..."}</p></div>
                            } else if let Some(performance) = (*venue_performance).as_ref() {
                                if !performance.is_empty() {
                                    <div class="overflow-x-auto">
                                        <table class="min-w-full divide-y divide-gray-200">
                                            <thead class="bg-gray-50">
                                                <tr>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Venue"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Contests"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Win Rate"}</th>
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white divide-y divide-gray-200">
                                                {performance.iter().map(|v| {
                                                    html! {
                                                        <tr>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{venue_link(&v.venue_id, &v.venue_name)}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{v.total_contests}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{format!("{:.1}%", v.win_rate)}</td>
                                                        </tr>
                                                    }
                                                }).collect::<Html>()}
                                            </tbody>
                                        </table>
                                    </div>
                                } else {
                                    <div class="no-data"><p>{"No venue performance data yet. Play contests at venues to see stats here."}</p></div>
                                }
                            } else {
                                <div class="no-data"><p>{"Venue performance not loaded"}</p></div>
                            }
                        </div>
                    }

                    // Games Tab
                    if *current_tab == AnalyticsTab::Games {
                        if *tab_analytics_loading {
                            <div class="dashboard-section"><div class="h-24 rounded-lg bg-gray-100 animate-pulse"></div></div>
                        } else if let Some(data) = &*tab_analytics {
                            <div class="dashboard-section">
                                <h2>{"🏆 Platform Top Games"}</h2>
                                {section_guide(
                                    "Games with the most recorded plays across the whole platform.",
                                    "See what's trending in your community before buying or learning a title. Popular games are easier to get to the table."
                                )}
                                {if let Some(games) = data.get("top_games").and_then(|v| v.as_array()) {
                                    html! {
                                        <div class="popularity-list">
                                            {for games.iter().enumerate().map(|(i, g)| html! {
                                                <div class="popularity-item">
                                                    <span class="rank">{i + 1}</span>
                                                    <span class="name">{game_link_from(g, "game_id", "game_name", "Unknown")}</span>
                                                    <span class="count">{g["plays"].as_i64().unwrap_or(0)} {"plays"}</span>
                                                </div>
                                            })}
                                        </div>
                                    }
                                } else { html!{<div class="no-data">{"No game data"}</div>} }}
                            </div>
                            <div class="dashboard-section">
                                <h2>{"🎯 Player Count Fit"}</h2>
                                {section_guide(
                                    "How often contests use the most common table size on the platform (e.g. 4-player).",
                                    "A high score means posted games usually match typical group sizes — less friction finding a seat that fits the rules. Low scores may mean more niche player counts."
                                )}
                                <div class="stat-value text-3xl font-bold mt-2">{format!("{:.0}%", data["player_count_fit_score_pct"].as_f64().unwrap_or(0.0))}</div>
                            </div>
                            {if let Some(rows) = data.get("cross_venue_popularity").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"🌍 Cross-Venue Popularity"}</h2>
                                        {section_guide(
                                            "Games played across multiple venues — how many locations and total plays each title has.",
                                            "Titles that travel well are safe picks wherever you go. Single-venue hits might be local favorites worth trying at that specific spot."
                                        )}
                                        <div class="overflow-x-auto">
                                            <table class="min-w-full divide-y divide-gray-200 text-sm">
                                                <thead class="bg-gray-50"><tr>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Game"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Venues"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Plays"}</th>
                                                </tr></thead>
                                                <tbody class="bg-white divide-y divide-gray-200">
                                                    {for rows.iter().map(|g| html! {
                                                        <tr>
                                                            <td class="px-4 py-2">{game_link_from(g, "game_id", "game_name", "")}</td>
                                                            <td class="px-4 py-2">{g["venue_count"].as_i64().unwrap_or(0)}</td>
                                                            <td class="px-4 py-2">{g["total_plays"].as_i64().unwrap_or(0)}</td>
                                                        </tr>
                                                    })}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                            {if let Some(trends) = data.get("longevity_trends").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"📈 Game Longevity Trends"}</h2>
                                        {section_guide(
                                            "Month-by-month play counts for selected games — whether interest is rising, steady, or fading.",
                                            "Avoid investing in a dying fad or catch a rising title early. Pair this with your own taste to choose what to learn next."
                                        )}
                                        {for trends.iter().map(|game| {
                                            let game_id = game["game_id"].as_str().unwrap_or("");
                                            let name = game["game_name"].as_str().unwrap_or("Unknown");
                                            html! {
                                                <div class="mb-4">
                                                    <h3 class="text-sm font-medium text-gray-900 mb-1">{game_link(game_id, name)}</h3>
                                                    {if let Some(months) = game.get("monthly_plays").and_then(|v| v.as_array()) {
                                                        html! {
                                                            <div class="flex flex-wrap gap-2 text-xs text-gray-600">
                                                                {for months.iter().map(|m| html! {
                                                                    <span class="bg-gray-100 px-2 py-1 rounded">{m["period"].as_str().unwrap_or("")}{": "}{m["plays"].as_i64().unwrap_or(0)}</span>
                                                                })}
                                                            </div>
                                                        }
                                                    } else { html!{} }}
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            } else { html!{} }}
                        }
                        <div class="dashboard-section">
                            <h2>{"🎮 Game Analytics"}</h2>
                            {section_guide(
                                "Look up a specific game to see total plays, unique players and venues, average duration, top players, and popular locations.",
                                "Research before you show up — know if a game is a quick filler or an all-night affair, and who the regulars are if you want a friendly table."
                            )}
                            <div class="games-analytics-controls">
                                <input
                                    class="input"
                                    placeholder="Enter game ID (e.g., game/123...)"
                                    value={(*game_id_input).clone()}
                                    oninput={on_game_id_input}
                                />
                                <button class="action-button primary" onclick={on_load_game_analytics.clone()} disabled={*game_analytics_loading}>
                                    { if *game_analytics_loading { "Loading..." } else { "Load Analytics" } }
                                </button>
                                <div class="spacer"></div>
                                <input
                                    class="input"
                                    placeholder="Search games by name"
                                    value={(*game_search_query).clone()}
                                    oninput={on_game_search_input}
                                />
                                <button class="action-button secondary" onclick={on_game_search.clone()} disabled={*game_search_loading}>
                                    { if *game_search_loading { "Searching..." } else { "Search" } }
                                </button>
                                if auth.state.player.as_ref().map(|p| p.is_admin).unwrap_or(false) {
                                    <button class="action-button" onclick={
                                        let navigator = navigator.clone();
                                        Callback::from(move |_| navigator.push(&Route::Games))
                                    }>
                                        {"Go to Games Admin"}
                                    </button>
                                }
                            </div>
                            // Search results
                            if let Some(err) = &*game_search_error { <div class="error-message"><p>{err}</p></div> }
                            if *game_search_loading {
                                <div class="overflow-x-auto mt-4">
                                    <div class="space-y-2">
                                        {for (0..3).map(|_| html!{<div class="h-10 rounded bg-gray-100 animate-pulse"></div>})}
                                    </div>
                                </div>
                            } else if !(*game_search_results).is_empty() {
                                <div class="overflow-x-auto mt-4">
                                    <table class="min-w-full divide-y divide-gray-200">
                                        <thead class="bg-gray-50">
                                            <tr>
                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Game"}</th>
                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Year"}</th>
                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"BGG"}</th>
                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Actions"}</th>
                                            </tr>
                                        </thead>
                                        <tbody class="bg-white divide-y divide-gray-200">
                                            {for (*game_search_results).iter().map(|g| {
                                                let gid = g.id.clone();
                                                let gid_for_analytics = g.id.clone();
                                                let navigator = navigator.clone();
                                                html! {
                                                    <tr>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{game_link(&gid, &g.name)}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{g.year_published.map(|y| y.to_string()).unwrap_or_else(|| "".to_string())}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{g.bgg_id.map(|id| id.to_string()).unwrap_or_else(|| "".to_string())}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700 space-x-2">
                                                            <button class="link" onclick={
                                                                let navigator = navigator.clone();
                                                                let gid = gid.clone();
                                                                Callback::from(move |_| navigator.push(&Route::GameDetails { game_id: gid.clone() }))
                                                            }>{"Open Details"}</button>
                                                            <button class="link" onclick={
                                                                let game_id_input = game_id_input.clone();
                                                                let gid_for_analytics = gid_for_analytics.clone();
                                                                Callback::from(move |_| game_id_input.set(gid_for_analytics.clone()))
                                                            }>{"Use for Analytics"}</button>
                                                        </td>
                                                    </tr>
                                                }
                                            })}
                                        </tbody>
                                    </table>
                                </div>
                            }
                            if let Some(err) = &*game_analytics_error {
                                <div class="error-message"><p>{err}</p></div>
                            }
                            if *game_analytics_loading {
                                <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mt-6">
                                    {for (0..4).map(|_| html!{<div class="h-24 rounded-lg bg-gray-100 animate-pulse"></div>})}
                                </div>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mt-6">
                                    <div class="h-48 rounded-lg bg-gray-100 animate-pulse"></div>
                                    <div class="h-48 rounded-lg bg-gray-100 animate-pulse"></div>
                                </div>
                            } else if let Some(analytics_data) = &*game_analytics {
                                <div class="stats-grid">
                                    <div class="stat-card primary">
                                        <h3>{"Total Plays"}</h3>
                                        <div class="stat-value">{analytics_data.get("total_plays").and_then(|v| v.as_u64()).unwrap_or(0)}</div>
                                    </div>
                                    <div class="stat-card success">
                                        <h3>{"Unique Players"}</h3>
                                        <div class="stat-value">{analytics_data.get("unique_players").and_then(|v| v.as_u64()).unwrap_or(0)}</div>
                                    </div>
                                    <div class="stat-card info">
                                        <h3>{"Unique Venues"}</h3>
                                        <div class="stat-value">{analytics_data.get("unique_venues").and_then(|v| v.as_u64()).unwrap_or(0)}</div>
                                    </div>
                                    <div class="stat-card warning">
                                        <h3>{"Avg Duration (min)"}</h3>
                                        <div class="stat-value">{analytics_data.get("avg_duration_minutes").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32}</div>
                                    </div>
                                </div>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                    <div class="bg-gray-50 rounded-lg p-4">
                                        <h3 class="text-sm font-medium text-gray-900 mb-2">{"Top Players"}</h3>
                                        <div class="space-y-2">
                                            {if let Some(top_players) = analytics_data.get("top_players").and_then(|v| v.as_array()) {
                                                html! { {for top_players.iter().take(5).map(|player| {
                                                    let name = player.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                                    let player_id = player.get("player_id")
                                                        .or_else(|| player.get("id"))
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("");
                                                    html! {
                                                        <div class="flex justify-between text-sm">
                                                            <span class="text-gray-700">{player_link_from_id(player_id, name)}</span>
                                                            <span class="text-gray-500">{player.get("plays").and_then(|v| v.as_u64()).unwrap_or(0)} {"plays"}</span>
                                                        </div>
                                                    }
                                                })} }
                                            } else { html! { <p class="text-sm text-gray-500">{"No player data available"}</p> } }}
                                        </div>
                                    </div>
                                    <div class="bg-gray-50 rounded-lg p-4">
                                        <h3 class="text-sm font-medium text-gray-900 mb-2">{"Popular Venues"}</h3>
                                        <div class="space-y-2">
                                            {if let Some(top_venues) = analytics_data.get("top_venues").and_then(|v| v.as_array()) {
                                                html! { {for top_venues.iter().take(5).map(|venue| {
                                                    html! {
                                                        <div class="flex justify-between text-sm">
                                                            <span class="text-gray-700">{venue_link_from(venue, "venue_id", "name", "Unknown")}</span>
                                                            <span class="text-gray-500">{venue.get("plays").and_then(|v| v.as_u64()).unwrap_or(0)} {"plays"}</span>
                                                        </div>
                                                    }
                                                })} }
                                            } else { html! { <p class="text-sm text-gray-500">{"No venue data available"}</p> } }}
                                        </div>
                                    </div>
                                </div>
                            } else {
                                <div class="no-data"><p>{"Enter a game ID to view analytics"}</p></div>
                            }
                        </div>

                        if let Some(chart_data) = (*game_popularity_chart).as_ref() {
                            <div class="dashboard-section">
                                <h2>{"Games by Player Count Distribution"}</h2>
                                {section_guide(
                                    "A chart of how contests break down by number of players — which table sizes are most common on the platform.",
                                    "Bring games that fit the local meta. If the chart peaks at 4 players, your 2-player duel might need a dedicated partner rather than an open night."
                                )}
                                <div class="chart-container">
                                    <ChartRenderer
                                        chart_data={chart_data.clone()}
                                        chart_id={"game-popularity-chart".to_string()}
                                        width={Some(800)}
                                        height={Some(400)}
                                    />
                                </div>
                            </div>
                        }
                    }

                    // Players Tab
                    if *current_tab == AnalyticsTab::Players {
                        if auth.state.player.is_none() {
                            <div class="dashboard-section">
                                <h2>{"👤 Player Analytics"}</h2>
                                <div class="no-data">
                                    <p>{"Sign in to view your personal analytics, head-to-head records, and recommendations."}</p>
                                    <button class="action-button primary mt-4" onclick={{
                                        let navigator = navigator.clone();
                                        Callback::from(move |_| { navigator.push(&Route::Login); })
                                    }}>
                                        {"Sign in"}
                                    </button>
                                </div>
                            </div>
                        } else if *tab_analytics_loading {
                            <div class="dashboard-section"><div class="h-24 rounded-lg bg-gray-100 animate-pulse"></div></div>
                        } else if let Some(data) = &*tab_analytics {
                            <div class="dashboard-section">
                                <h2>{"📊 Your Activity"}</h2>
                                {section_guide(
                                    "Your current win streak, best-ever streak, and how many days since your last recorded contest.",
                                    "Track consistency and spot when you've gone quiet. Streaks reward regular play; a long gap might mean it's time to post or join a contest."
                                )}
                                <div class="stats-grid">
                                    <div class="stat-card primary">
                                        <h3>{"Current Streak"}</h3>
                                        <div class="stat-value">{data["current_streak"].as_i64().unwrap_or(0)}</div>
                                    </div>
                                    <div class="stat-card success">
                                        <h3>{"Longest Streak"}</h3>
                                        <div class="stat-value">{data["longest_streak"].as_i64().unwrap_or(0)}</div>
                                    </div>
                                    <div class="stat-card info">
                                        <h3>{"Days Since Last Contest"}</h3>
                                        <div class="stat-value">{data["days_since_last_contest"].as_i64().unwrap_or(-1)}</div>
                                        {if let Some(last_id) = data.get("last_contest_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                                            html! {
                                                <div class="stat-subtitle mt-2">
                                                    {"Last played: "}{contest_link(last_id, "View contest")}
                                                </div>
                                            }
                                        } else { html!{} }}
                                    </div>
                                </div>
                            </div>
                            {if let Some(buckets) = data.get("rating_distribution").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"📈 Rating Distribution (Platform)"}</h2>
                                        {section_guide(
                                            "How many rated players fall into each skill band on the platform.",
                                            "See where you sit in the field — crowded middle bands mean lots of peers at your level; thin tails mean fewer extreme beginners or experts."
                                        )}
                                        <div class="flex flex-wrap gap-3 items-end mt-2">
                                            {for buckets.iter().map(|b| {
                                                let count = b["player_count"].as_i64().unwrap_or(0);
                                                let h = (count as f64).sqrt() * 6.0 + 4.0;
                                                html! {
                                                    <div class="text-center">
                                                        <div class="bg-purple-500 rounded-t mx-auto" style={format!("width:3rem;height:{}px", h as i32)}></div>
                                                        <div class="text-xs text-gray-600 mt-1">{b["range_label"].as_str().unwrap_or("")}</div>
                                                        <div class="text-xs text-gray-500">{count}</div>
                                                    </div>
                                                }
                                            })}
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                            {if let Some(points) = data.get("skill_trajectory").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"📉 Skill Trajectory"}</h2>
                                        {section_guide(
                                            "Your Glicko-2 rating over recent rating periods, with games played in each window (times shown in your timezone).",
                                            "Watch whether you're improving, plateauing, or slipping. Pair rating moves with games played to see if changes are meaningful or just noise from inactivity."
                                        )}
                                        <div class="space-y-1 text-sm">
                                            {for points.iter().map(|p| html! {
                                                <div class="flex justify-between border-b border-gray-100 py-1">
                                                    <span>{p["period"].as_str().unwrap_or("")}</span>
                                                    <span>{format!("{:.0} rating", p["rating"].as_f64().unwrap_or(0.0))}</span>
                                                    <span class="text-gray-500">{p["games_played"].as_i64().unwrap_or(0)}{" games"}</span>
                                                </div>
                                            })}
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                            {if let Some(h2h) = data.get("head_to_head_top").and_then(|v| v.as_array()) {
                                html! {
                                    <div class="dashboard-section">
                                        <h2>{"⚔️ Head-to-Head (Top Opponents)"}</h2>
                                        {section_guide(
                                            "Players you've faced most often, with your wins and win rate against each.",
                                            "Identify rivals and nemeses — someone you beat often is a confidence booster; a tough opponent shows where to study. Great for picking rematches or balanced tables."
                                        )}
                                        <div class="overflow-x-auto">
                                            <table class="min-w-full divide-y divide-gray-200 text-sm">
                                                <thead class="bg-gray-50"><tr>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Opponent"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Contests"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Your Wins"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"Win %"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{"History"}</th>
                                                </tr></thead>
                                                <tbody class="bg-white divide-y divide-gray-200">
                                                    {for h2h.iter().map(|o| {
                                                        let opponent_id = o["opponent_id"].as_str().unwrap_or("").to_string();
                                                        let opponent_handle = o["opponent_handle"].as_str().unwrap_or("").to_string();
                                                        let on_open_h2h_history = on_open_h2h_history.clone();
                                                        html! {
                                                        <tr>
                                                            <td class="px-4 py-2">{player_link_from_id(&opponent_id, &opponent_handle)}</td>
                                                            <td class="px-4 py-2">{o["total_contests"].as_i64().unwrap_or(0)}</td>
                                                            <td class="px-4 py-2">{o["my_wins"].as_i64().unwrap_or(0)}</td>
                                                            <td class="px-4 py-2">{format!("{:.0}%", o["my_win_rate"].as_f64().unwrap_or(0.0))}</td>
                                                            <td class="px-4 py-2">
                                                                <button class="text-blue-600 hover:underline text-sm" onclick={{
                                                                    let opponent_id = opponent_id.clone();
                                                                    let opponent_handle = opponent_handle.clone();
                                                                    Callback::from(move |_| {
                                                                        on_open_h2h_history.emit((opponent_id.clone(), opponent_handle.clone()));
                                                                    })
                                                                }}>
                                                                    {"View contests"}
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }})}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                }
                            } else { html!{} }}
                        }
                    // Glicko2 Ratings Leaderboard Section
                    <div class="dashboard-section">
                        <h2>{"🏆 Glicko2 Ratings Leaderboard"}</h2>
                        {section_guide(
                            "Top-rated players by Glicko-2 skill estimate, with rating deviation (RD) showing confidence and when each player was last active.",
                            "Benchmark yourself against the best in the community. Lower RD means a steadier, battle-tested rating — useful when scouting serious opponents or tournament fields."
                        )}
                        <div class="glicko-leaderboard-container">
                            if *glicko_loading {
                                <div class="overflow-x-auto">
                                    <div class="space-y-2">
                                        {for (0..8).map(|_| html!{<div class="h-10 rounded bg-gray-100 animate-pulse"></div>})}
                                    </div>
                                </div>
                            } else if let Some(err) = (*glicko_error).as_ref() {
                                <div class="error-container">
                                    <p class="error-text">{"Error loading ratings: "}{err}</p>
                                </div>
                            } else if let Some(leaderboard) = (*glicko_leaderboard).as_ref() {
                                if leaderboard.is_empty() {
                                    <div class="no-data-container">
                                        <p>{"No ratings available yet. Players need to participate in contests to get rated."}</p>
                                    </div>
                                } else {
                                    <div class="overflow-x-auto">
                                        <table class="min-w-full divide-y divide-gray-200">
                                            <thead class="bg-gray-50">
                                                <tr>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Rank"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Player"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Rating"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"RD"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Games"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Last Active"}</th>
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white divide-y divide-gray-200">
                                                {leaderboard.iter().enumerate().map(|(index, player)| {
                                                    let rank = index + 1;
                                                    let player_id = player["player_id"].as_str().unwrap_or("Unknown");
                                                    let player_key = player_id
                                                        .split('/')
                                                        .last()
                                                        .unwrap_or(player_id)
                                                        .trim_matches('`')
                                                        .to_string();
                                                    let handle = player["handle"].as_str().unwrap_or("Unknown");
                                                    let rating = player["rating"].as_f64().unwrap_or(1500.0);
                                                    let rd = player["rd"].as_f64().unwrap_or(350.0);
                                                    let games_played = player["games_played"].as_i64().unwrap_or(0);
                                                    let last_active = player["last_active"]
                                                        .as_str()
                                                        .map(|s| format_in_player_timezone(s, &player_timezone))
                                                        .unwrap_or_else(|| "Unknown".to_string());

                                                    let row_class = if rank == 1 { "bg-yellow-50" } else if rank == 2 { "bg-gray-50" } else if rank == 3 { "bg-orange-50" } else { "" };

                                                    html! {
                                                        <tr class={classes!(row_class, "hover:bg-gray-50")}>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{rank}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap">
                                                                <Link<Route>
                                                                    to={Route::PlayerProfile { player_id: player_key.clone() }}
                                                                    classes="text-blue-600 hover:text-blue-800 hover:underline"
                                                                >
                                                                    <div class="text-sm font-medium">{handle}</div>
                                                                    <div class="text-xs text-gray-500 font-normal">{format!("player/{}", player_key)}</div>
                                                                </Link<Route>>
                                                            </td>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{format!("{:.0}", rating)}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{format!("{:.0}", rd)}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{games_played}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{last_active}</td>
                                                        </tr>
                                                    }
                                                }).collect::<Html>()}
                                            </tbody>
                                        </table>
                                    </div>
                                    <div class="glicko-leaderboard-note">
                                        <p>{"📊 Ratings are recalculated monthly based on contest results. "}
                                            {"Minimum 3 games required to appear on leaderboard. "}
                                            {"Lower RD (Rating Deviation) means higher confidence in the rating."}
                                        </p>
                                    </div>
                                }
                            } else {
                                <div class="loading-container">
                                    <p>{"Loading Glicko2 ratings..."}</p>
                                </div>
                            }
                        </div>
                    </div>

                    // Game Recommendations Section
                    <div class="dashboard-section">
                        <h2>{"🎮 Game Recommendations"}</h2>
                        {section_guide(
                            "Games suggested for you based on who you play with, how often you play, and inferred preferences from your history.",
                            "Skip the research spiral — get pointed at titles you're likely to enjoy and can actually get played with your usual group."
                        )}
                        if *recommendations_loading {
                            <div class="loading-container"><p>{"Loading game recommendations..."}</p></div>
                        } else if let Some(recommendations) = (*game_recommendations).as_ref() {
                            if !recommendations.is_empty() {
                                <div class="overflow-x-auto">
                                    <table class="min-w-full divide-y divide-gray-200">
                                        <thead class="bg-gray-50">
                                            <tr>
                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Game"}</th>
                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Reason"}</th>
                                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Score"}</th>
                                            </tr>
                                        </thead>
                                        <tbody class="bg-white divide-y divide-gray-200">
                                            {recommendations.iter().map(|g| {
                                                html! {
                                                    <tr>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{game_link(&g.game_id, &g.game_name)}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{g.reason.clone()}</td>
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{format!("{:.0}%", g.score)}</td>
                                                    </tr>
                                                }
                                            }).collect::<Html>()}
                                        </tbody>
                                    </table>
                                </div>
                            } else {
                                <div class="no-data">
                                    <p class="mb-1">{"No game recommendations available"}</p>
                                    <p class="text-xs text-gray-600">{"Recommendations appear after the player has enough contest history and opponent overlap."}</p>
                                </div>
                            }
                        } else {
                            <div class="no-data"><p>{"Game recommendations not loaded"}</p></div>
                        }
                    </div>

                    // Gaming Communities Section
                    <div class="dashboard-section">
                        <h2>{"👥 Gaming Communities"}</h2>
                        {section_guide(
                            "Groups of players who frequently share tables, with a community leader and a strength score for the cluster.",
                            "Find your people — these are the circles you're already orbiting. Joining a strong community makes recurring game nights and invites much easier."
                        )}
                        if *communities_loading {
                            <div class="loading-container"><p>{"Loading gaming communities..."}</p></div>
                        } else if let Some(communities_data) = (*gaming_communities).as_ref() {
                            if let Some(communities) = communities_data["gaming_communities"].as_array() {
                                if !communities.is_empty() {
                                    <div class="overflow-x-auto">
                                        <table class="min-w-full divide-y divide-gray-200">
                                            <thead class="bg-gray-50">
                                                <tr>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Leader"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Members"}</th>
                                                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Strength"}</th>
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white divide-y divide-gray-200">
                                                {communities.iter().map(|c| {
                                                    let leader = &c["community_leader"];
                                                    let leader_id = leader["player_id"].as_str().unwrap_or("");
                                                    let leader_name = leader["opponent_handle"].as_str().unwrap_or("Unknown");
                                                    let total_members = c["total_members"].as_i64().unwrap_or(0);
                                                    let community_strength = c["community_strength"].as_f64().unwrap_or(0.0);
                                                    html! {
                                                        <tr>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{player_link_from_id(leader_id, leader_name)}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{total_members}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{format!("{:.0}", community_strength)}</td>
                                                        </tr>
                                                    }
                                                }).collect::<Html>()}
                                            </tbody>
                                        </table>
                                    </div>
                                } else {
                                    <div class="no-data">
                                        <p class="mb-1">{"No gaming communities available"}</p>
                                        <p class="text-xs text-gray-600">{"Communities emerge when a player has recurring opponents across multiple contests."}</p>
                                    </div>
                                }
                            } else {
                                <div class="no-data"><p>{"No gaming communities available"}</p></div>
                            }
                        } else {
                            <div class="no-data"><p>{"Gaming communities not loaded"}</p></div>
                        }
                    </div>

                    // Player Networking Section
                    <div class="dashboard-section">
                        <h2>{"📊 Social Network"}</h2>
                        {section_guide(
                            "Your most frequent opponents — contests played together, wins and losses, and when you last shared a table (in your timezone).",
                            "Map your network at a glance. Reconnect with familiar faces, notice one-sided matchups worth revisiting, and see who you haven't played in a while."
                        )}
                        if *networking_loading {
                            <div class="loading-container">
                                <p>{"Loading social network data..."}</p>
                            </div>
                        } else if let Some(networking_data) = (*player_networking).as_ref() {
                            <div class="networking-grid">
                                if let Some(opponents) = networking_data["opponent_analysis"].as_array() {
                                    {opponents.iter().take(5).map(|opponent| {
                                        let opponent_id = opponent["opponent_id"].as_str().unwrap_or("");
                                        let opponent_handle = opponent["opponent_handle"].as_str().unwrap_or("Unknown");
                                        let total_contests = opponent["total_contests"].as_i64().unwrap_or(0);
                                        let win_rate = opponent["win_rate"].as_f64().unwrap_or(0.0);
                                        let last_played = opponent["last_played"]
                                            .as_str()
                                            .map(|s| format_in_player_timezone(s, &player_timezone))
                                            .unwrap_or_else(|| "Never".to_string());

                                        html! {
                                            <div class="opponent-card">
                                                <h3>{player_link_from_id(opponent_id, opponent_handle)}</h3>
                                                <div class="opponent-stats">
                                                    <div class="stat">
                                                        <span class="label">{"Games:"}</span>
                                                        <span class="value">{total_contests}</span>
                                                    </div>
                                                    <div class="stat">
                                                        <span class="label">{"Your Win Rate:"}</span>
                                                        <span class="value">{format!("{:.1}%", win_rate)}</span>
                                                    </div>
                                                    <div class="stat">
                                                        <span class="label">{"Last Played:"}</span>
                                                        <span class="value">{last_played}</span>
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Html>()}
                                } else {
                                    <div class="no-data">
                                        <p>{"No networking data available"}</p>
                                    </div>
                                }
                            </div>
                        } else {
                            <div class="no-data">
                                <p>{"Networking data not loaded"}</p>
                            </div>
                        }
                    </div>
                    }
                </div>
            }
            if *h2h_modal_open {
                <HeadToHeadModal
                    record={(*h2h_modal_record).clone()}
                    opponent_handle={h2h_modal_opponent.1.clone()}
                    opponent_name={h2h_modal_opponent.1.clone()}
                    loading={*h2h_modal_loading}
                    error={(*h2h_modal_error).clone()}
                    on_close={on_close_h2h_modal}
                />
            }
        </div>
    }
}
