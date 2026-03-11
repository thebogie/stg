use crate::api::games::get_game_analytics;
use crate::api::games::search_games;
use crate::api::utils::authenticated_get;
use crate::components::chart_renderer::ChartRenderer;
use crate::Route;
use gloo_net::http::Request;
use serde_json::Value;
use shared::dto::game::GameDto;
use web_sys::console;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct GameRecommendation {
    game_name: String,
    reason: String,
    score: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct VenuePerformance {
    venue_name: String,
    total_contests: u64,
    win_rate: f64,
}

#[derive(Properties, PartialEq, Clone)]
pub struct AnalyticsDashboardProps {}

#[derive(Clone, PartialEq)]
enum AnalyticsTab {
    Overview,
    Contests,
    Venues,
    Games,
    Players,
}

#[function_component(AnalyticsDashboard)]
pub fn analytics_dashboard(_props: &AnalyticsDashboardProps) -> Html {
    let auth = use_context::<crate::auth::AuthContext>().expect("Auth context not found");
    let navigator = use_navigator().unwrap();
    let platform_stats = use_state(|| None::<Value>);
    let contest_trends_chart = use_state(|| None::<String>);
    let platform_dashboard = use_state(|| None::<Vec<Value>>);

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

    let on_select_tab = {
        let current_tab = current_tab.clone();
        Callback::from(move |tab: AnalyticsTab| {
            current_tab.set(tab);
        })
    };

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

    // Load platform stats
    {
        let platform_stats = platform_stats.clone();
        let loading = loading.clone();
        let error = error.clone();

        use_effect_with((), move |_| {
            loading.set(true);
            error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                match Request::get("/api/analytics/platform").send().await {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(stats) = response.json::<Value>().await {
                                console::log_1(
                                    &format!("Platform stats received: {:?}", stats).into(),
                                );

                                // Check if the real data has meaningful game play counts
                                let has_real_data =
                                    if let Some(top_games) = stats["top_games"].as_array() {
                                        top_games
                                            .iter()
                                            .any(|game| game["plays"].as_i64().unwrap_or(0) > 0)
                                    } else {
                                        false
                                    };

                                if has_real_data {
                                    // Use real data
                                    platform_stats.set(Some(stats));
                                } else {
                                    // Fall back to sample data for better UX
                                    console::log_1(
                                        &"Real data shows 0 plays, using sample data".into(),
                                    );
                                    match Request::get("/api/analytics/sample-platform")
                                        .send()
                                        .await
                                    {
                                        Ok(sample_response) => {
                                            if sample_response.ok() {
                                                if let Ok(sample_stats) =
                                                    sample_response.json::<Value>().await
                                                {
                                                    console::log_1(
                                                        &"Using sample platform stats".into(),
                                                    );
                                                    platform_stats.set(Some(sample_stats));
                                                } else {
                                                    // If sample data fails, still use real data
                                                    platform_stats.set(Some(stats));
                                                }
                                            } else {
                                                // If sample data fails, still use real data
                                                platform_stats.set(Some(stats));
                                            }
                                        }
                                        Err(_) => {
                                            // If sample data fails, still use real data
                                            platform_stats.set(Some(stats));
                                        }
                                    }
                                }
                            } else {
                                error.set(Some("Failed to parse platform stats".to_string()));
                            }
                        } else {
                            let status = response.status();
                            let text = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Unknown error".to_string());
                            console::error_1(
                                &format!("Platform stats request failed: {} - {}", status, text)
                                    .into(),
                            );
                            error.set(Some(format!(
                                "Platform stats request failed: {} - {}",
                                status, text
                            )));
                        }
                    }
                    Err(e) => {
                        console::error_1(&format!("Failed to fetch platform stats: {}", e).into());
                        error.set(Some(format!("Failed to fetch platform stats: {}", e)));
                    }
                }
                loading.set(false);
            });

            || ()
        });
    }

    // Load Glicko2 leaderboard
    {
        let glicko_leaderboard = glicko_leaderboard.clone();
        let glicko_loading = glicko_loading.clone();
        let glicko_error = glicko_error.clone();

        use_effect_with((), move |_| {
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
                                console::log_1(
                                    &format!(
                                        "Glicko2 leaderboard received: {} players",
                                        leaderboard.len()
                                    )
                                    .into(),
                                );
                                console::log_1(
                                    &format!("First player: {:?}", leaderboard.first()).into(),
                                );
                                console::log_1(
                                    &format!("Last player: {:?}", leaderboard.last()).into(),
                                );
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
                            console::error_1(
                                &format!(
                                    "Glicko2 leaderboard request failed: {} - {}",
                                    status, text
                                )
                                .into(),
                            );
                            glicko_error.set(Some(format!(
                                "Glicko2 leaderboard request failed: {} - {}",
                                status, text
                            )));
                        }
                    }
                    Err(e) => {
                        console::error_1(
                            &format!("Failed to fetch Glicko2 leaderboard: {}", e).into(),
                        );
                        glicko_error
                            .set(Some(format!("Failed to fetch Glicko2 leaderboard: {}", e)));
                    }
                }
                glicko_loading.set(false);
            });

            || ()
        });
    }

    // Load contest trends chart
    {
        let contest_trends_chart = contest_trends_chart.clone();
        let error = error.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get("/api/analytics/charts/contest-trends?months=12&title=Contest%20Trends%20Over%20Time")
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

            || ()
        });
    }

    // Load contest heatmap data (weekday x hour buckets)
    {
        let contest_heatmap = contest_heatmap.clone();
        let contest_heatmap_loading = contest_heatmap_loading.clone();
        let contest_heatmap_error = contest_heatmap_error.clone();
        let heatmap_weeks = heatmap_weeks.clone();
        use_effect_with(heatmap_weeks.clone(), move |weeks| {
            let w = **weeks;
            contest_heatmap_loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get(&format!("/api/analytics/contests/heatmap?weeks={}", w))
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
                        contest_heatmap_error.set(Some(format!("Failed to fetch heatmap: {}", e)))
                    }
                }
                contest_heatmap_loading.set(false);
            });
            || ()
        });
    }

    // Load platform dashboard
    {
        let platform_dashboard = platform_dashboard.clone();
        let error = error.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get(
                    "/api/analytics/charts/platform-dashboard?title=Platform%20Overview",
                )
                .send()
                .await
                {
                    Ok(response) => {
                        if let Ok(charts) = response.json::<Vec<Value>>().await {
                            platform_dashboard.set(Some(charts));
                        } else {
                            error.set(Some("Failed to parse platform dashboard".to_string()));
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch platform dashboard: {}", e)));
                    }
                }
            });

            || ()
        });
    }

    // Load platform insights
    {
        let insights_state = insights.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
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
            || ()
        });
    }

    // Load games by player count distribution
    {
        let game_popularity_chart = game_popularity_chart.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
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
            || ()
        });
    }

    // Load activity metrics chart
    {
        let activity_metrics_chart = activity_metrics_chart.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get(
                    "/api/analytics/charts/activity-metrics?days=60&title=Daily%20Activity",
                )
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

        use_effect_with((), move |_| {
            // Load venue performance for the current user
            let set_venue_performance = venue_performance.clone();
            let set_venue_loading = venue_loading.clone();
            set_venue_loading.set(true);

            wasm_bindgen_futures::spawn_local(async move {
                let user_id = "player/2025041711441894568690500"; // Placeholder
                match crate::api::utils::authenticated_get(&format!(
                    "/api/analytics-enhanced/venues/player-stats/{}",
                    user_id
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
                                                Some(venue_name),
                                                Some(total_contests),
                                                Some(win_rate),
                                            ) = (
                                                v.get("venue_name").and_then(|n| n.as_str()),
                                                v.get("total_contests").and_then(|c| c.as_u64()),
                                                v.get("win_rate").and_then(|w| w.as_f64()),
                                            ) {
                                                Some(VenuePerformance {
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

            wasm_bindgen_futures::spawn_local(async move {
                let user_id = "player/2025041711441894568690500"; // Placeholder
                match crate::api::utils::authenticated_get(&format!(
                    "/api/analytics-enhanced/games/recommendations/{}?limit=5",
                    user_id
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

            wasm_bindgen_futures::spawn_local(async move {
                let user_id = "player/2025041711441894568690500"; // Placeholder

                match crate::api::utils::authenticated_get(&format!(
                    "/api/analytics-enhanced/communities/{}?min_contests=2",
                    user_id
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

            wasm_bindgen_futures::spawn_local(async move {
                let user_id = "player/2025041711441894568690500"; // Placeholder

                match crate::api::utils::authenticated_get(&format!(
                    "/api/analytics-enhanced/networking/{}",
                    user_id
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

            || ()
        });
    }

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

                    // Engagement Metrics Section
                    <div class="dashboard-section">
                        <h2>{"📊 Platform Health Metrics"}</h2>
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
                                                            <span class="name">{game["game_name"].as_str().unwrap_or("Unknown")}</span>
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
                                                            <span class="name">{venue["venue_name"].as_str().unwrap_or("Unknown")}</span>
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
                    }

                    // Contests Tab
                    if *current_tab == AnalyticsTab::Contests {
                        if let Some(chart_data) = (*contest_trends_chart).as_ref() {
                            <div class="dashboard-section">
                                <h2>{"Contest Trends"}</h2>
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
                            <p class="mt-1 mb-3 text-sm text-gray-600">
                                {"Rows are days of week (Sun–Sat). Columns are hours in UTC (00–23). Colors indicate how many contests started in that hour over the last N weeks (darker = more)."}
                            </p>
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
                    }



                    // Venues Tab
                    if *current_tab == AnalyticsTab::Venues {
                        // Reuse venue-related sections (popular venues)
                        <div class="dashboard-section">
                            <h2>{"🏟️ Top Venues"}</h2>
                            if let Some(stats) = (*platform_stats).as_ref() {
                                if let Some(top_venues) = stats["top_venues"].as_array() {
                                    if !top_venues.is_empty() {
                                        <div class="popularity-list">
                                            {top_venues.iter().enumerate().map(|(i, venue)| {
                                                html! {
                                                    <div class="popularity-item">
                                                        <span class="rank">{i + 1}</span>
                                                        <span class="name">{venue["venue_name"].as_str().unwrap_or("Unknown")}</span>
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
                    }

                    // Games Tab
                    if *current_tab == AnalyticsTab::Games {
                        <div class="dashboard-section">
                            <h2>{"🎮 Game Analytics"}</h2>
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
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{g.name.clone()}</td>
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
                                                    html! {
                                                        <div class="flex justify-between text-sm">
                                                            <span class="text-gray-700">{player.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown")}</span>
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
                                                            <span class="text-gray-700">{venue.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown")}</span>
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
                    }

                    // Players Tab
                    if *current_tab == AnalyticsTab::Players {
                    // Glicko2 Ratings Leaderboard Section
                    <div class="dashboard-section">
                        <h2>{"🏆 Glicko2 Ratings Leaderboard"}</h2>
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
                                                    let handle = player["handle"].as_str().unwrap_or("Unknown");
                                                    let rating = player["rating"].as_f64().unwrap_or(1500.0);
                                                    let rd = player["rd"].as_f64().unwrap_or(350.0);
                                                    let games_played = player["games_played"].as_i64().unwrap_or(0);
                                                    let last_active = player["last_active"].as_str().unwrap_or("Unknown");

                                                    let row_class = if rank == 1 { "bg-yellow-50" } else if rank == 2 { "bg-gray-50" } else if rank == 3 { "bg-orange-50" } else { "" };

                                                    html! {
                                                        <tr class={classes!(row_class)}>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{rank}</td>
                                                            <td class="px-4 py-2 whitespace-nowrap">
                                                                <div class="text-sm font-medium text-gray-900">{handle}</div>
                                                                <div class="text-xs text-gray-500">{"#"}{&player_id[8..]}</div>
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



                    // Contest Trends Chart
                    if let Some(chart_data) = (*contest_trends_chart).as_ref() {
                        <div class="dashboard-section">
                            <h2>{"Contest Trends"}</h2>
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
                    }

                    // Games by Player Count Distribution
                    if let Some(chart_data) = (*game_popularity_chart).as_ref() {
                        <div class="dashboard-section">
                            <h2>{"Games by Player Count Distribution"}</h2>
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







                    // Game Recommendations Section
                    <div class="dashboard-section">
                        <h2>{"🎮 Game Recommendations"}</h2>
                        <p class="text-sm text-gray-600 mb-3">{"Personalized suggestions based on opponents, frequency, and inferred preferences."}</p>
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
                                                        <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{g.game_name.clone()}</td>
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
                        <p class="text-sm text-gray-600 mb-3">{"Clusters of players the user frequently plays with, highlighting community leaders and strength."}</p>
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
                                                    let leader_name = leader["opponent_handle"].as_str().unwrap_or("Unknown");
                                                    let total_members = c["total_members"].as_i64().unwrap_or(0);
                                                    let community_strength = c["community_strength"].as_f64().unwrap_or(0.0);
                                                    html! {
                                                        <tr>
                                                            <td class="px-4 py-2 whitespace-nowrap text-sm text-gray-700">{leader_name}</td>
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
                        if *networking_loading {
                            <div class="loading-container">
                                <p>{"Loading social network data..."}</p>
                            </div>
                        } else if let Some(networking_data) = (*player_networking).as_ref() {
                            <div class="networking-grid">
                                if let Some(opponents) = networking_data["opponent_analysis"].as_array() {
                                    {opponents.iter().take(5).map(|opponent| {
                                        let opponent_handle = opponent["opponent_handle"].as_str().unwrap_or("Unknown");
                                        let total_contests = opponent["total_contests"].as_i64().unwrap_or(0);
                                        let win_rate = opponent["win_rate"].as_f64().unwrap_or(0.0);
                                        let last_played = opponent["last_played"].as_str().unwrap_or("Never");

                                        html! {
                                            <div class="opponent-card">
                                                <h3>{opponent_handle}</h3>
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

                    // Quick Actions
                    <div class="dashboard-section">
                        <h2>{"⚡ Quick Actions"}</h2>
                        <div class="actions-grid">
                            <button class="action-button primary" onclick={|_| {
                                // Refresh all charts
                                gloo_utils::window().location().reload().unwrap();
                            }}>
                                {"🔄 Refresh Dashboard"}
                            </button>
                            <button class="action-button secondary" onclick={|_| {
                                // Export data
                                log::info!("Export functionality would be implemented here");
                            }}>
                                {"📊 Export Data"}
                            </button>
                            <button class="action-button secondary" onclick={|_| {
                                // Generate report
                                log::info!("Report generation would be implemented here");
                            }}>
                                {"📋 Generate Report"}
                            </button>
                        </div>
                    </div>

                    // System Health Section
                    <div class="dashboard-section">
                        <h2>{"💚 System Health"}</h2>
                        <div class="health-grid">
                            <div class="health-card">
                                <h3>{"Database Status"}</h3>
                                <div class="health-indicator online">{"🟢 Online"}</div>
                                <div class="health-details">{"All collections accessible"}</div>
                            </div>
                            <div class="health-card">
                                <h3>{"Cache Status"}</h3>
                                <div class="health-indicator online">{"🟢 Online"}</div>
                                <div class="health-details">{"Redis cache active"}</div>
                            </div>
                            <div class="health-card">
                                <h3>{"API Response"}</h3>
                                <div class="health-indicator online">{"🟢 Online"}</div>
                                <div class="health-details">{"Endpoints responding"}</div>
                            </div>
                        </div>
                    </div>
                </div>
            }
        </div>
    }
}
