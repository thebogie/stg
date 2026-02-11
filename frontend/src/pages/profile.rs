use crate::analytics::client_manager::ClientAnalyticsManager;
use crate::api::games::get_all_games;
use crate::api::utils::authenticated_get;
use crate::api::venues::get_all_venues;
use crate::components::contests_modal::ContestsModal;
use crate::components::profile::comparison_tab::ComparisonTab;
use crate::components::profile::achievements_tab::AchievementsTab;
use crate::components::profile::game_performance_tab::GamePerformanceTab;
use crate::components::profile::nemesis_tab::NemesisTab;
use crate::components::profile::overall_stats_tab::OverallStatsTab;
use crate::components::profile::owned_tab::OwnedTab;
use crate::components::profile::profile_tabs::ProfileTabs;
use crate::components::profile::ratings_tab::RatingsTab;
use crate::components::profile::settings_tab::SettingsTab;
use crate::components::profile::trends_tab::TrendsTab;
use chrono::DateTime;
use js_sys::encode_uri_component;
use serde_json::Value;
use shared::dto::analytics::{
    GamePerformanceDto, HeadToHeadRecordDto, PerformanceTrendDto, PlayerOpponentDto,
};
use shared::models::client_analytics::{
    AnalyticsQuery, CoreStats, GamePerformance, PerformanceTrend,
};
use shared::{GameDto, VenueDto};
use shared::dto::analytics::PlayerAchievementsDto;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;
use yew::prelude::*;
use yew::use_effect_with;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct StreakData {
    current_streak: i32,
    longest_streak: i32,
}

#[allow(dead_code)]
async fn fetch_contest_details_for_streaks(
    contest_ids: Vec<String>,
    player_id: &str,
    core_stats: yew::UseStateHandle<Option<CoreStats>>,
) {
    let mut all_contests = Vec::new();

    console::log_1(&format!("🔄 Fetching details for {} contests", contest_ids.len()).into());

    // Fetch individual contest details
    for contest_id in contest_ids {
        // Extract just the numeric part from contest IDs like "contest/4127490"
        let numeric_id = if contest_id.starts_with("contest/") {
            contest_id.strip_prefix("contest/").unwrap_or(&contest_id)
        } else {
            &contest_id
        };

        let contest_url = format!("/api/contests/{}", numeric_id);
        console::log_1(
            &format!(
                "🔍 Fetching contest details: {} -> {}",
                contest_id, contest_url
            )
            .into(),
        );

        match authenticated_get(&contest_url).send().await {
            Ok(response) => {
                if response.ok() {
                    match response.json::<Value>().await {
                        Ok(contest_data) => {
                            console::log_1(
                                &format!("✅ Fetched contest {}: {:?}", contest_id, contest_data)
                                    .into(),
                            );
                            all_contests.push(contest_data);
                        }
                        Err(e) => {
                            console::log_1(
                                &format!("Failed to parse contest {}: {}", contest_id, e).into(),
                            );
                        }
                    }
                } else {
                    console::log_1(
                        &format!(
                            "Failed to fetch contest {}: {}",
                            contest_id,
                            response.status()
                        )
                        .into(),
                    );
                }
            }
            Err(e) => {
                console::log_1(&format!("Failed to fetch contest {}: {}", contest_id, e).into());
            }
        }
    }

    console::log_1(
        &format!(
            "📊 Successfully fetched {} contest details",
            all_contests.len()
        )
        .into(),
    );

    if !all_contests.is_empty() {
        let streaks = calculate_streaks_from_contests(&all_contests, player_id);
        console::log_1(
            &format!(
                "✅ Calculated streaks: current={}, longest={}",
                streaks.current_streak, streaks.longest_streak
            )
            .into(),
        );

        // Update core stats with calculated streaks
        if let Some(current_stats) = core_stats.as_ref() {
            let mut updated_stats = current_stats.clone();
            updated_stats.current_streak = streaks.current_streak;
            updated_stats.longest_streak = streaks.longest_streak;
            console::log_1(
                &format!(
                    "🔄 Updating core_stats with streaks: current={}, longest={}",
                    updated_stats.current_streak, updated_stats.longest_streak
                )
                .into(),
            );
            core_stats.set(Some(updated_stats));
            console::log_1(&format!("✅ Core stats updated successfully").into());
        } else {
            console::log_1(&format!("❌ No current_stats found to update").into());
        }
    }
}

#[allow(dead_code)]
fn calculate_streaks_from_contests(contests: &[Value], player_id: &str) -> StreakData {
    let mut player_contests = Vec::new();

    console::log_1(
        &format!(
            "🔍 Analyzing {} contests for player {}",
            contests.len(),
            player_id
        )
        .into(),
    );

    // Extract player's contest results from the contest data
    for (i, contest) in contests.iter().enumerate() {
        console::log_1(&format!("Contest {}: {:?}", i, contest).into());

        // Try different possible data structures
        if let Some(participants) = contest.get("participants").and_then(|p| p.as_array()) {
            console::log_1(
                &format!(
                    "Found participants array with {} entries",
                    participants.len()
                )
                .into(),
            );
            for (j, participant) in participants.iter().enumerate() {
                console::log_1(&format!("Participant {}: {:?}", j, participant).into());
                if let Some(pid) = participant.get("player_id").and_then(|p| p.as_str()) {
                    console::log_1(
                        &format!("Checking player_id: {} against {}", pid, player_id).into(),
                    );
                    if pid == player_id || pid.ends_with(player_id) {
                        if let Some(place) = participant.get("place").and_then(|p| p.as_i64()) {
                            if let Some(start_time) = contest.get("start").and_then(|s| s.as_str())
                            {
                                console::log_1(
                                    &format!(
                                        "Found match! Player {} placed {} at {}",
                                        pid, place, start_time
                                    )
                                    .into(),
                                );
                                // Parse the start time to sort contests chronologically
                                if let Ok(dt) = DateTime::parse_from_rfc3339(start_time) {
                                    player_contests.push((dt, place as i32));
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(outcomes) = contest.get("outcomes").and_then(|o| o.as_array()) {
            console::log_1(&format!("Found outcomes array with {} entries", outcomes.len()).into());
            for (j, outcome) in outcomes.iter().enumerate() {
                console::log_1(&format!("Outcome {}: {:?}", j, outcome).into());
                if let Some(pid) = outcome.get("player_id").and_then(|p| p.as_str()) {
                    console::log_1(
                        &format!("Checking player_id: {} against {}", pid, player_id).into(),
                    );
                    if pid == player_id || pid.ends_with(player_id) {
                        if let Some(place_str) = outcome.get("place").and_then(|p| p.as_str()) {
                            if let Ok(place) = place_str.parse::<i32>() {
                                if let Some(start_time) =
                                    contest.get("start").and_then(|s| s.as_str())
                                {
                                    console::log_1(
                                        &format!(
                                            "Found match! Player {} placed {} at {}",
                                            pid, place, start_time
                                        )
                                        .into(),
                                    );
                                    // Parse the start time to sort contests chronologically
                                    if let Ok(dt) = DateTime::parse_from_rfc3339(start_time) {
                                        player_contests.push((dt, place));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Try alternative data structure - maybe the contest data is structured differently
            console::log_1(
                &format!("No participants or outcomes array found, trying alternative structure")
                    .into(),
            );

            // Check if this is a direct player result
            if let Some(pid) = contest.get("player_id").and_then(|p| p.as_str()) {
                if pid == player_id || pid.ends_with(player_id) {
                    if let Some(place) = contest.get("place").and_then(|p| p.as_i64()) {
                        if let Some(start_time) = contest.get("start").and_then(|s| s.as_str()) {
                            console::log_1(
                                &format!(
                                    "Found direct match! Player {} placed {} at {}",
                                    pid, place, start_time
                                )
                                .into(),
                            );
                            if let Ok(dt) = DateTime::parse_from_rfc3339(start_time) {
                                player_contests.push((dt, place as i32));
                            }
                        }
                    }
                }
            }
        }
    }

    console::log_1(&format!("📊 Found {} player contests", player_contests.len()).into());

    if player_contests.is_empty() {
        console::log_1(&"❌ No player contests found - cannot calculate streaks".into());
        return StreakData {
            current_streak: 0,
            longest_streak: 0,
        };
    }

    // Sort contests by start time (oldest first)
    player_contests.sort_by(|a, b| a.0.cmp(&b.0));

    // Log the sorted contests
    for (i, (dt, place)) in player_contests.iter().enumerate() {
        console::log_1(
            &format!(
                "Contest {}: {} - Place {}",
                i,
                dt.format("%Y-%m-%d %H:%M"),
                place
            )
            .into(),
        );
    }

    // Calculate streaks
    let mut current_streak = 0;
    let mut longest_streak = 0;

    for (_, place) in &player_contests {
        if *place == 1 {
            // Win
            current_streak += 1;
            longest_streak = longest_streak.max(current_streak);
            console::log_1(
                &format!(
                    "🏆 Win! Current streak: {}, Longest: {}",
                    current_streak, longest_streak
                )
                .into(),
            );
        } else {
            // Loss or tie - reset streak
            if current_streak > 0 {
                console::log_1(
                    &format!(
                        "💔 Loss/Tie (place {}). Streak of {} ended",
                        place, current_streak
                    )
                    .into(),
                );
            }
            current_streak = 0;
        }
    }

    console::log_1(
        &format!(
            "✅ Final streaks - Current: {}, Longest: {}",
            current_streak, longest_streak
        )
        .into(),
    );

    StreakData {
        current_streak,
        longest_streak,
    }
}

use crate::auth::AuthContext;
use gloo_storage::Storage;
use gloo_utils;

#[derive(Properties, PartialEq)]
pub struct ProfilePageProps {
    #[prop_or_default]
    pub player_id: Option<String>,
}

#[derive(PartialEq, Clone)]
pub enum ProfileTab {
    OverallStats,
    Ratings,
    Achievements,
    Nemesis,
    Owned,
    GamePerformance,
    Trends,
    Comparison,
    Settings,
}

#[function_component(ProfilePage)]
pub fn profile_page(props: &ProfilePageProps) -> Html {
    let auth_context = use_context::<AuthContext>().expect("AuthContext not found");
    let viewing_other_player = props.player_id.is_some();
    let player_id_override = props.player_id.clone();
    // Restore last selected tab from LocalStorage, default to Ratings
    let current_tab = {
        let mut initial = if let Ok(val) = gloo_storage::LocalStorage::get::<String>("profile_last_tab")
        {
            match val.as_str() {
                "OverallStats" => ProfileTab::OverallStats,
                "Ratings" => ProfileTab::Ratings,
                "Achievements" => ProfileTab::Achievements,
                "Nemesis" => ProfileTab::Nemesis,
                "Owned" => ProfileTab::Owned,
                "GamePerformance" => ProfileTab::GamePerformance,
                "Trends" => ProfileTab::Trends,
                "Comparison" => ProfileTab::Comparison,
                "Settings" => ProfileTab::Settings,
                _ => ProfileTab::OverallStats,
            }
        } else {
            ProfileTab::OverallStats
        };
        if viewing_other_player && initial == ProfileTab::Settings {
            initial = ProfileTab::OverallStats;
        }
        use_state(|| initial)
    };
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);

    // Analytics data states
    let opponents_who_beat_me = use_state(|| None::<Vec<HeadToHeadRecordDto>>);
    let opponents_i_beat = use_state(|| None::<Vec<HeadToHeadRecordDto>>);
    let game_performance = use_state(|| None::<Vec<GamePerformance>>);
    let performance_trends = use_state(|| None::<Vec<PerformanceTrend>>);
    let trends_loading = use_state(|| false);
    let trends_error = use_state(|| None::<String>);
    let core_stats = use_state(|| None::<CoreStats>);
    let achievements = use_state(|| None::<PlayerAchievementsDto>);
    let achievements_loading = use_state(|| false);
    let achievements_error = use_state(|| None::<String>);

    // Glicko2 ratings states
    let glicko_ratings = use_state(|| None::<Vec<serde_json::Value>>);
    let glicko_loading = use_state(|| false);
    let glicko_error = use_state(|| None::<String>);

    // Ratings history states
    let rating_history = use_state(|| None::<Vec<serde_json::Value>>);
    let rating_history_loading = use_state(|| false);
    let rating_history_error = use_state(|| None::<String>);

    // Trends filters + lookups
    let games = use_state(|| None::<Vec<GameDto>>);
    let venues = use_state(|| None::<Vec<VenueDto>>);
    let selected_game_id = use_state(|| None::<String>);
    let selected_venue_id = use_state(|| None::<String>);

    // Contest details modal states
    let contest_modal_open = use_state(|| false);
    let contest_modal_loading = use_state(|| false);
    let contest_modal_error = use_state(|| None::<String>);
    let contest_details = use_state(|| None::<Vec<Value>>);
    let selected_opponent = use_state(|| None::<(String, String, String)>); // (id, handle, name)

    // Tab click handler
    let on_tab_click = {
        let current_tab = current_tab.clone();
        Callback::from(move |tab: ProfileTab| {
            // Clone before moving into state update
            let t = tab.clone();
            current_tab.set(tab);
            // Persist selection for navigation/back behavior
            let _ = match t {
                ProfileTab::OverallStats => {
                    gloo_storage::LocalStorage::set("profile_last_tab", "OverallStats")
                }
                ProfileTab::Ratings => {
                    gloo_storage::LocalStorage::set("profile_last_tab", "Ratings")
                }
                ProfileTab::Achievements => {
                    gloo_storage::LocalStorage::set("profile_last_tab", "Achievements")
                }
                ProfileTab::Nemesis => {
                    gloo_storage::LocalStorage::set("profile_last_tab", "Nemesis")
                }
                ProfileTab::Owned => gloo_storage::LocalStorage::set("profile_last_tab", "Owned"),
                ProfileTab::GamePerformance => {
                    gloo_storage::LocalStorage::set("profile_last_tab", "GamePerformance")
                }
                ProfileTab::Trends => gloo_storage::LocalStorage::set("profile_last_tab", "Trends"),
                ProfileTab::Comparison => {
                    gloo_storage::LocalStorage::set("profile_last_tab", "Comparison")
                }
                ProfileTab::Settings => {
                    gloo_storage::LocalStorage::set("profile_last_tab", "Settings")
                }
            };
        })
    };

    // Load initial data
    {
        let loading = loading.clone();
        let error = error.clone();
        let auth_context = auth_context.clone();
        let player_id_override = player_id_override.clone();
        let viewing_other_player = viewing_other_player;
        let opponents_who_beat_me = opponents_who_beat_me.clone();
        let opponents_i_beat = opponents_i_beat.clone();
        let game_performance = game_performance.clone();
        let performance_trends = performance_trends.clone();
        let core_stats = core_stats.clone();
        let glicko_ratings = glicko_ratings.clone();
        let glicko_loading = glicko_loading.clone();
        let glicko_error = glicko_error.clone();
        let rating_history = rating_history.clone();
        let rating_history_loading = rating_history_loading.clone();
        let rating_history_error = rating_history_error.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                loading.set(true);
                error.set(None);

                // Resolve target player ID (override or current auth player)
                let player_id = if let Some(override_id) = &player_id_override {
                    if override_id.starts_with("player/") {
                        override_id.trim_start_matches("player/").to_string()
                    } else {
                        override_id.clone()
                    }
                } else if let Some(player) = &auth_context.state.player {
                    if player.id.starts_with("player/") {
                        player.id.trim_start_matches("player/").to_string()
                    } else {
                        player.id.clone()
                    }
                } else {
                    error.set(Some("Player not authenticated".to_string()));
                    loading.set(false);
                    return;
                };
                let player_id_param = encode_uri_component(&player_id)
                    .as_string()
                    .unwrap_or_else(|| player_id.clone());

                // Try client analytics first (only for the current player)
                let mut analytics_manager = ClientAnalyticsManager::new();
                let analytics_result = if viewing_other_player {
                    Err("Client analytics not available for other players".to_string())
                } else {
                    analytics_manager
                        .get_analytics(
                            "current_player",
                            AnalyticsQuery {
                                date_range: None,
                                games: None,
                                venues: None,
                                opponents: None,
                                min_players: None,
                                max_players: None,
                            },
                        )
                        .await
                };
                match analytics_result {
                    Ok(analytics) => {
                        console::log_1(
                            &format!("Client analytics data received: {:?}", analytics).into(),
                        );

                        // Set core stats
                        core_stats.set(Some(analytics.stats));

                        // Convert opponent performance data to HeadToHeadRecordDto
                        let opponents_who_beat_me_data: Vec<HeadToHeadRecordDto> = analytics
                            .opponent_performance
                            .iter()
                            .filter(|opp| opp.head_to_head.my_win_rate < 50.0)
                            .map(|opp| HeadToHeadRecordDto {
                                opponent_id: opp.opponent.player_id.clone(),
                                opponent_handle: opp.opponent.handle.clone(),
                                opponent_name: opp.opponent.name.clone(),
                                total_contests: opp.head_to_head.total_contests,
                                my_wins: opp.head_to_head.my_wins,
                                opponent_wins: opp.head_to_head.opponent_wins,
                                my_win_rate: opp.head_to_head.my_win_rate,
                                contest_history: vec![],
                            })
                            .collect();

                        let opponents_i_beat_data: Vec<HeadToHeadRecordDto> = analytics
                            .opponent_performance
                            .iter()
                            .filter(|opp| opp.head_to_head.my_win_rate >= 50.0)
                            .map(|opp| HeadToHeadRecordDto {
                                opponent_id: opp.opponent.player_id.clone(),
                                opponent_handle: opp.opponent.handle.clone(),
                                opponent_name: opp.opponent.name.clone(),
                                total_contests: opp.head_to_head.total_contests,
                                my_wins: opp.head_to_head.my_wins,
                                opponent_wins: opp.head_to_head.opponent_wins,
                                my_win_rate: opp.head_to_head.my_win_rate,
                                contest_history: vec![],
                            })
                            .collect();

                        // Set opponent data
                        opponents_who_beat_me.set(Some(opponents_who_beat_me_data));
                        opponents_i_beat.set(Some(opponents_i_beat_data));

                        // Set game performance and trends
                        game_performance.set(Some(analytics.game_performance));
                        performance_trends.set(Some(analytics.trends));

                        console::log_1(&"✅ Successfully set all client analytics data".into());
                    }
                    Err(e) => {
                        console::log_1(
                            &format!(
                                "Client analytics not available, falling back to API calls: {}",
                                e
                            )
                            .into(),
                        );

                        // Fallback to API calls
                        // Since the stats endpoint is having issues, we'll calculate core stats from other working endpoints
                        console::log_1(
                            &"⚠️ Stats endpoint unavailable, calculating from other data sources"
                                .into(),
                        );

                        // We'll fetch contest history after game performance to ensure proper timing

                        // Get game performance
                        let game_perf_url = if viewing_other_player {
                            format!(
                                "/api/analytics/player/game-performance?player_id={}",
                                player_id_param
                            )
                        } else {
                            "/api/analytics/player/game-performance".to_string()
                        };
                        match authenticated_get(&game_perf_url).send().await {
                            Ok(response) => {
                                if response.ok() {
                                    // First try to parse as an array of DTOs (backend returns a raw array)
                                    match response.json::<Vec<GamePerformanceDto>>().await {
                                        Ok(dto_list) => {
                                            let mapped: Vec<GamePerformance> = dto_list.into_iter().map(|dto| {
                                                GamePerformance {
                                                    game: shared::models::client_analytics::ClientGame {
                                                        id: dto.game_id,
                                                        name: dto.game_name,
                                                        year_published: None,
                                                    },
                                                    total_plays: dto.total_plays,
                                                    wins: dto.wins,
                                                    losses: dto.losses,
                                                    win_rate: dto.win_rate,
                                                    average_placement: dto.average_placement,
                                                    best_placement: dto.best_placement,
                                                    worst_placement: dto.worst_placement,
                                                    last_played: dto.last_played,
                                                    days_since_last_play: dto.days_since_last_play,
                                                    favorite_venue: None,
                                                }
                                            }).collect();
                                            game_performance.set(Some(mapped.clone()));

                                            // Calculate core stats from game performance data
                                            let total_contests: i32 =
                                                mapped.iter().map(|gp| gp.total_plays).sum();
                                            let total_wins: i32 =
                                                mapped.iter().map(|gp| gp.wins).sum();
                                            let total_losses: i32 =
                                                mapped.iter().map(|gp| gp.losses).sum();
                                            let win_rate = if total_contests > 0 {
                                                (total_wins as f64 / total_contests as f64) * 100.0
                                            } else {
                                                0.0
                                            };
                                            let average_placement = if !mapped.is_empty() {
                                                mapped
                                                    .iter()
                                                    .map(|gp| gp.average_placement)
                                                    .sum::<f64>()
                                                    / mapped.len() as f64
                                            } else {
                                                0.0
                                            };
                                            let best_placement = mapped
                                                .iter()
                                                .map(|gp| gp.best_placement)
                                                .min()
                                                .unwrap_or(0);

                                            let core_stats_data =
                                                shared::models::client_analytics::CoreStats {
                                                    total_contests,
                                                    total_wins,
                                                    total_losses,
                                                    win_rate,
                                                    average_placement,
                                                    best_placement,
                                                    worst_placement: mapped
                                                        .iter()
                                                        .map(|gp| gp.worst_placement)
                                                        .max()
                                                        .unwrap_or(0),
                                                    current_streak: if win_rate >= 100.0 {
                                                        total_wins
                                                    } else {
                                                        0
                                                    },
                                                    longest_streak: if win_rate >= 100.0 {
                                                        total_wins
                                                    } else {
                                                        0
                                                    },
                                                    skill_rating: 1200.0, // Default, will be updated from ratings
                                                    total_points: total_wins * 10, // Simple calculation
                                                };
                                            core_stats.set(Some(core_stats_data));
                                            console::log_1(&format!("✅ Calculated core stats: {} contests, {} wins, {:.1}% win rate", total_contests, total_wins, win_rate).into());

                                            console::log_1(&format!("✅ Core stats set with streaks: current={}, longest={}", 
                                                if win_rate >= 100.0 { total_wins } else { 0 },
                                                if win_rate >= 100.0 { total_wins } else { 0 }
                                            ).into());
                                        }
                                        Err(_) => {
                                            // Backward compatibility: parse generic JSON and look for wrapped key
                                            match authenticated_get(&game_perf_url).send().await {
                                                Ok(resp2) => {
                                                    if resp2.ok() {
                                                        match resp2.json::<Value>().await {
                                                            Ok(data) => {
                                                                if let Some(perf_data) = data
                                                                    .get("game_performance")
                                                                    .and_then(|v| v.as_array())
                                                                {
                                                                    let mut game_perf_vec =
                                                                        Vec::new();
                                                                    for game_data in perf_data {
                                                                        if let Ok(game_perf) =
                                                                            serde_json::from_value::<
                                                                                GamePerformance,
                                                                            >(
                                                                                game_data.clone()
                                                                            )
                                                                        {
                                                                            game_perf_vec
                                                                                .push(game_perf);
                                                                        }
                                                                    }
                                                                    game_performance.set(Some(
                                                                        game_perf_vec.clone(),
                                                                    ));

                                                                    // Calculate core stats from game performance data
                                                                    let total_contests: i32 =
                                                                        game_perf_vec
                                                                            .iter()
                                                                            .map(|gp| {
                                                                                gp.total_plays
                                                                            })
                                                                            .sum();
                                                                    let total_wins: i32 =
                                                                        game_perf_vec
                                                                            .iter()
                                                                            .map(|gp| gp.wins)
                                                                            .sum();
                                                                    let total_losses: i32 =
                                                                        game_perf_vec
                                                                            .iter()
                                                                            .map(|gp| gp.losses)
                                                                            .sum();
                                                                    let win_rate = if total_contests
                                                                        > 0
                                                                    {
                                                                        (total_wins as f64
                                                                            / total_contests as f64)
                                                                            * 100.0
                                                                    } else {
                                                                        0.0
                                                                    };
                                                                    let average_placement =
                                                                        if !game_perf_vec.is_empty()
                                                                        {
                                                                            game_perf_vec.iter().map(|gp| gp.average_placement).sum::<f64>() / game_perf_vec.len() as f64
                                                                        } else {
                                                                            0.0
                                                                        };
                                                                    let best_placement =
                                                                        game_perf_vec
                                                                            .iter()
                                                                            .map(|gp| {
                                                                                gp.best_placement
                                                                            })
                                                                            .min()
                                                                            .unwrap_or(0);

                                                                    let core_stats_data = shared::models::client_analytics::CoreStats {
                                                                        total_contests,
                                                                        total_wins,
                                                                        total_losses,
                                                                        win_rate,
                                                                        average_placement,
                                                                        best_placement,
                                                                        worst_placement: game_perf_vec.iter().map(|gp| gp.worst_placement).max().unwrap_or(0),
                                                                        current_streak: 0, // Requires individual contest data
                                                                        longest_streak: 0, // Requires individual contest data
                                                                        skill_rating: 1200.0, // Default, will be updated from ratings
                                                                        total_points: total_wins * 10, // Simple calculation
                                                                    };
                                                                    core_stats
                                                                        .set(Some(core_stats_data));
                                                                    console::log_1(&format!("✅ Calculated core stats (fallback): {} contests, {} wins, {:.1}% win rate", total_contests, total_wins, win_rate).into());
                                                                }
                                                            }
                                                            Err(e) => {
                                                                console::log_1(&format!("Failed to parse game performance (wrapped): {}", e).into());
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    console::log_1(&format!("Failed to refetch game performance: {}", e).into());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                console::log_1(
                                    &format!("Failed to fetch game performance: {}", e).into(),
                                );
                            }
                        }

                        // Get opponents who beat me
                        let opponents_who_beat_url = if viewing_other_player {
                            format!(
                                "/api/analytics/player/opponents-who-beat-me?player_id={}",
                                player_id_param
                            )
                        } else {
                            "/api/analytics/player/opponents-who-beat-me".to_string()
                        };
                        match authenticated_get(&opponents_who_beat_url).send().await {
                            Ok(response) => {
                                if response.ok() {
                                    match response.json::<Value>().await {
                                        Ok(data) => {
                                            // Map PlayerOpponentDto -> HeadToHeadRecordDto for UI consumption
                                            if let Ok(opponents) =
                                                serde_json::from_value::<Vec<PlayerOpponentDto>>(
                                                    data.clone(),
                                                )
                                            {
                                                let mapped: Vec<HeadToHeadRecordDto> = opponents
                                                    .into_iter()
                                                    .map(|o| HeadToHeadRecordDto {
                                                        opponent_id: o.player_id,
                                                        opponent_handle: o.player_handle,
                                                        opponent_name: o.player_name,
                                                        total_contests: o.contests_played,
                                                        my_wins: o.losses_to_me,
                                                        opponent_wins: o.wins_against_me,
                                                        my_win_rate: 100.0 - o.win_rate_against_me,
                                                        contest_history: vec![],
                                                    })
                                                    .collect();
                                                opponents_who_beat_me.set(Some(mapped));
                                            } else if let Ok(opponents_h2h) =
                                                serde_json::from_value::<Vec<HeadToHeadRecordDto>>(
                                                    data,
                                                )
                                            {
                                                opponents_who_beat_me.set(Some(opponents_h2h));
                                            }
                                        }
                                        Err(e) => {
                                            console::log_1(
                                                &format!(
                                                    "Failed to parse opponents who beat me: {}",
                                                    e
                                                )
                                                .into(),
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                console::log_1(
                                    &format!("Failed to fetch opponents who beat me: {}", e).into(),
                                );
                            }
                        }

                        // Get opponents I beat
                        let opponents_i_beat_url = if viewing_other_player {
                            format!(
                                "/api/analytics/player/opponents-i-beat?player_id={}",
                                player_id_param
                            )
                        } else {
                            "/api/analytics/player/opponents-i-beat".to_string()
                        };
                        match authenticated_get(&opponents_i_beat_url).send().await {
                            Ok(response) => {
                                if response.ok() {
                                    match response.json::<Value>().await {
                                        Ok(data) => {
                                            if let Ok(opponents) =
                                                serde_json::from_value::<Vec<PlayerOpponentDto>>(
                                                    data.clone(),
                                                )
                                            {
                                                let mapped: Vec<HeadToHeadRecordDto> = opponents
                                                    .into_iter()
                                                    .map(|o| HeadToHeadRecordDto {
                                                        opponent_id: o.player_id,
                                                        opponent_handle: o.player_handle,
                                                        opponent_name: o.player_name,
                                                        total_contests: o.contests_played,
                                                        my_wins: o.wins_against_me,
                                                        opponent_wins: o.losses_to_me,
                                                        my_win_rate: o.win_rate_against_me,
                                                        contest_history: vec![],
                                                    })
                                                    .collect();
                                                opponents_i_beat.set(Some(mapped));
                                            } else if let Ok(opponents_h2h) =
                                                serde_json::from_value::<Vec<HeadToHeadRecordDto>>(
                                                    data,
                                                )
                                            {
                                                opponents_i_beat.set(Some(opponents_h2h));
                                            }
                                        }
                                        Err(e) => {
                                            console::log_1(
                                                &format!("Failed to parse opponents i beat: {}", e)
                                                    .into(),
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                console::log_1(
                                    &format!("Failed to fetch opponents i beat: {}", e).into(),
                                );
                            }
                        }

                        // Get performance trends
                        let trends_url = if viewing_other_player {
                            format!(
                                "/api/analytics/player/performance-trends?player_id={}",
                                player_id_param
                            )
                        } else {
                            "/api/analytics/player/performance-trends".to_string()
                        };
                        match authenticated_get(&trends_url).send().await {
                            Ok(response) => {
                                if response.ok() {
                                    match response.json::<Value>().await {
                                        Ok(data) => {
                                            // Parse as array of PerformanceTrendDto and map to PerformanceTrend
                                            if let Ok(trends_dto) = serde_json::from_value::<
                                                Vec<shared::dto::analytics::PerformanceTrendDto>,
                                            >(
                                                data.clone()
                                            ) {
                                                let mapped: Vec<PerformanceTrend> = trends_dto
                                                    .into_iter()
                                                    .map(|dto| PerformanceTrend {
                                                        period: dto.month,
                                                        contests_played: dto.contests_played,
                                                        wins: dto.wins,
                                                        win_rate: dto.win_rate,
                                                        average_placement: dto.average_placement,
                                                        skill_rating: dto.skill_rating,
                                                    })
                                                    .collect();
                                                performance_trends.set(Some(mapped));
                                            } else if let Ok(trends) =
                                                serde_json::from_value::<Vec<PerformanceTrend>>(
                                                    data,
                                                )
                                            {
                                                // Fallback to direct parsing if already in correct format
                                                performance_trends.set(Some(trends));
                                            }
                                        }
                                        Err(e) => {
                                            console::log_1(
                                                &format!(
                                                    "Failed to parse performance trends: {}",
                                                    e
                                                )
                                                .into(),
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                console::log_1(
                                    &format!("Failed to fetch performance trends: {}", e).into(),
                                );
                            }
                        }
                    }
                }

                // Player ID already extracted above

                // Fetch Glicko2 ratings
                glicko_loading.set(true);
                let ratings_url = format!("/api/ratings/player/{}", player_id);
                match authenticated_get(&ratings_url).send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Vec<serde_json::Value>>().await {
                                Ok(data) => {
                                    console::log_1(
                                        &format!("Glicko2 ratings data received: {:?}", data)
                                            .into(),
                                    );
                                    glicko_ratings.set(Some(data.clone()));

                                    // Update skill rating in core stats if available
                                    if let Some(global_rating) = data.iter().find(|r| {
                                        r.get("scope")
                                            .and_then(|s| s.get("type"))
                                            .and_then(|t| t.as_str())
                                            == Some("Global")
                                    }) {
                                        if let Some(rating_value) =
                                            global_rating.get("rating").and_then(|r| r.as_f64())
                                        {
                                            // Update the core stats with the real skill rating
                                            if let Some(current_stats) = core_stats.as_ref() {
                                                let mut updated_stats = current_stats.clone();
                                                updated_stats.skill_rating = rating_value;
                                                core_stats.set(Some(updated_stats));
                                                console::log_1(
                                                    &format!(
                                                        "✅ Updated skill rating to {:.0}",
                                                        rating_value
                                                    )
                                                    .into(),
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(_e) => {
                                    console::log_1(&"Failed to parse Glicko2 ratings".into());
                                    glicko_error
                                        .set(Some("Failed to parse Glicko2 ratings".to_string()));
                                }
                            }
                        } else {
                            glicko_error.set(Some(format!(
                                "Failed to fetch ratings: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => {
                        glicko_error.set(Some(format!("Failed to fetch ratings: {}", e)));
                    }
                }
                glicko_loading.set(false);

                // Fetch rating history (global scope)
                rating_history_loading.set(true);
                let history_url = "/api/ratings/history?scope=global";
                match authenticated_get(history_url).send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Vec<serde_json::Value>>().await {
                                Ok(hist) => {
                                    console::log_1(
                                        &format!("Ratings history points: {}", hist.len()).into(),
                                    );
                                    rating_history.set(Some(hist));
                                }
                                Err(_) => {
                                    rating_history_error
                                        .set(Some("Failed to parse ratings history".to_string()));
                                }
                            }
                        } else {
                            let status = response.status();
                            if status == 404 {
                                // Treat as no history yet
                                rating_history.set(Some(vec![]));
                            } else {
                                rating_history_error.set(Some(format!(
                                    "Failed to fetch ratings history: {}",
                                    status
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        rating_history_error
                            .set(Some(format!("Failed to fetch ratings history: {}", e)));
                    }
                }
                rating_history_loading.set(false);

                loading.set(false);
            });
        });
    }

    // Load achievements
    {
        let achievements = achievements.clone();
        let achievements_loading = achievements_loading.clone();
        let achievements_error = achievements_error.clone();
        let auth_context = auth_context.clone();
        let player_id_override = player_id_override.clone();

        use_effect_with(
            (player_id_override.clone(), auth_context.state.player.clone()),
            move |(override_id, player)| {
            let override_id = override_id.clone();
            let player = player.clone();
            achievements_loading.set(true);
            achievements_error.set(None);

            spawn_local(async move {
                let player_id = if let Some(override_id) = override_id {
                    if override_id.starts_with("player/") {
                        override_id.trim_start_matches("player/").to_string()
                    } else {
                        override_id
                    }
                } else if let Some(player) = player {
                    if player.id.starts_with("player/") {
                        player.id.trim_start_matches("player/").to_string()
                    } else {
                        player.id.clone()
                    }
                } else {
                    achievements_loading.set(false);
                    return;
                };

                let url = format!("/api/analytics/players/{}/achievements", player_id);
                match authenticated_get(&url).send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<PlayerAchievementsDto>().await {
                                Ok(data) => achievements.set(Some(data)),
                                Err(e) => achievements_error
                                    .set(Some(format!("Failed to parse achievements: {}", e))),
                            }
                        } else {
                            achievements_error.set(Some(format!(
                                "Failed to fetch achievements: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => {
                        achievements_error
                            .set(Some(format!("Failed to fetch achievements: {}", e)))
                    }
                }

                achievements_loading.set(false);
            });

            || ()
        });
    }

    // Load games and venues for trends filters
    {
        let games = games.clone();
        let venues = venues.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(all_games) = get_all_games().await {
                    games.set(Some(all_games));
                }
                if let Ok(all_venues) = get_all_venues().await {
                    venues.set(Some(all_venues));
                }
            });

            || ()
        });
    }

    // Load performance trends with filters
    {
        let performance_trends = performance_trends.clone();
        let trends_loading = trends_loading.clone();
        let trends_error = trends_error.clone();
        let selected_game_id = selected_game_id.clone();
        let selected_venue_id = selected_venue_id.clone();
        let player_id_override = player_id_override.clone();
        let viewing_other_player = viewing_other_player;

        use_effect_with(
            (selected_game_id.clone(), selected_venue_id.clone()),
            move |(game_id, venue_id)| {
                let game_id = game_id.clone();
                let venue_id = venue_id.clone();

                trends_loading.set(true);
                trends_error.set(None);

                spawn_local(async move {
                    let mut params = Vec::new();
                    if viewing_other_player {
                        if let Some(player_id) = player_id_override.as_ref() {
                            let encoded = encode_uri_component(player_id)
                                .as_string()
                                .unwrap_or_else(|| player_id.clone());
                            params.push(format!("player_id={}", encoded));
                        }
                    }
                    if let Some(id) = &*game_id {
                        if !id.is_empty() {
                            let encoded = encode_uri_component(id)
                                .as_string()
                                .unwrap_or_else(|| id.clone());
                            params.push(format!("game_id={}", encoded));
                        }
                    }
                    if let Some(id) = &*venue_id {
                        if !id.is_empty() {
                            let encoded = encode_uri_component(id)
                                .as_string()
                                .unwrap_or_else(|| id.clone());
                            params.push(format!("venue_id={}", encoded));
                        }
                    }
                    let url = if params.is_empty() {
                        "/api/analytics/player/performance-trends".to_string()
                    } else {
                        format!(
                            "/api/analytics/player/performance-trends?{}",
                            params.join("&")
                        )
                    };

                    match authenticated_get(&url).send().await {
                        Ok(response) => {
                            if response.ok() {
                                match response.json::<Vec<PerformanceTrendDto>>().await {
                                    Ok(data) => {
                                        let mapped: Vec<PerformanceTrend> = data
                                            .into_iter()
                                            .map(|dto| PerformanceTrend {
                                                period: dto.month,
                                                contests_played: dto.contests_played,
                                                wins: dto.wins,
                                                win_rate: dto.win_rate,
                                                average_placement: dto.average_placement,
                                                skill_rating: dto.skill_rating,
                                            })
                                            .collect();
                                        performance_trends.set(Some(mapped));
                                    }
                                    Err(e) => trends_error
                                        .set(Some(format!("Failed to parse trends: {}", e))),
                                }
                            } else {
                                trends_error.set(Some(format!(
                                    "Failed to fetch trends: {}",
                                    response.status()
                                )));
                            }
                        }
                        Err(e) => trends_error.set(Some(format!("Failed to fetch trends: {}", e))),
                    }

                    trends_loading.set(false);
                });

                || ()
            },
        );
    }

    // Fetch contest details for head-to-head
    let fetch_contest_details = {
        let contest_modal_open = contest_modal_open.clone();
        let contest_modal_loading = contest_modal_loading.clone();
        let contest_modal_error = contest_modal_error.clone();
        let contest_details = contest_details.clone();
        let selected_opponent = selected_opponent.clone();
        let viewing_other_player = viewing_other_player;
        let player_id_override = player_id_override.clone();

        Callback::from(move |opponent: (String, String, String)| {
            let contest_modal_open = contest_modal_open.clone();
            let contest_modal_loading = contest_modal_loading.clone();
            let contest_modal_error = contest_modal_error.clone();
            let contest_details = contest_details.clone();
            let selected_opponent = selected_opponent.clone();
            let player_id_param = player_id_override.as_ref().cloned();

            spawn_local(async move {
                contest_modal_open.set(true);
                contest_modal_loading.set(true);
                contest_modal_error.set(None);
                selected_opponent.set(Some(opponent.clone()));

                let mut url = format!("/api/analytics/player/head-to-head/{}", opponent.0);
                if viewing_other_player {
                    if let Some(player_id) = player_id_param {
                        let encoded = encode_uri_component(&player_id)
                            .as_string()
                            .unwrap_or_else(|| player_id.clone());
                        url = format!("{}?player_id={}", url, encoded);
                    }
                }
                match authenticated_get(&url).send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Value>().await {
                                Ok(data) => {
                                    // Backend returns `contest_history`
                                    if let Some(history) =
                                        data.get("contest_history").and_then(|v| v.as_array())
                                    {
                                        contest_details.set(Some(history.clone()));
                                    } else if let Some(contests) =
                                        data.get("contests").and_then(|v| v.as_array())
                                    {
                                        // Backward compatibility if older key is present
                                        contest_details.set(Some(contests.clone()));
                                    } else {
                                        contest_details.set(Some(vec![]));
                                    }
                                }
                                Err(e) => {
                                    contest_modal_error
                                        .set(Some(format!("Failed to parse contests: {}", e)));
                                }
                            }
                        } else {
                            contest_modal_error.set(Some(format!(
                                "Failed to fetch contests: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => {
                        contest_modal_error.set(Some(format!("Failed to fetch contests: {}", e)));
                    }
                }

                contest_modal_loading.set(false);
            });
        })
    };

    if *loading {
        html! {
            <div class="min-h-screen bg-gray-50 flex items-center justify-center">
                <div class="text-center">
                    <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
                    <p class="mt-4 text-gray-600">{"Loading profile..."}</p>
                </div>
            </div>
        }
    } else if let Some(ref error_msg) = *error {
        html! {
            <div class="min-h-screen bg-gray-50 flex items-center justify-center">
                <div class="text-center">
                    <div class="text-red-600 text-6xl mb-4">{"⚠️"}</div>
                    <h1 class="text-2xl font-bold text-gray-900 mb-2">{"Error Loading Profile"}</h1>
                    <p class="text-gray-600 mb-4">{error_msg}</p>
                    <button
                        class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
                        onclick={Callback::from(|_| {
                            gloo_utils::window().location().reload().unwrap();
                        })}
                    >
                        {"Retry"}
                    </button>
                </div>
            </div>
        }
    } else {
        html! {
            <div class="min-h-screen bg-gray-50">
                <div class="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
                    // Header
                    <div class="mb-8">
                        <h1 class="text-3xl font-bold text-gray-900">{"Player Profile"}</h1>
                        <p class="mt-2 text-gray-600">{"Manage your profile, view statistics, and track your gaming progress"}</p>
                    </div>

                    // Profile Tabs
                    <ProfileTabs
                        current_tab={(*current_tab).clone()}
                        on_tab_click={on_tab_click}
                        show_settings={!viewing_other_player}
                    />

                    // Tab Content
                    <div class="mt-8">
                        {match *current_tab {
                            ProfileTab::OverallStats => html! {
                                <OverallStatsTab
                                    core_stats={(*core_stats).clone()}
                                    game_performance={(*game_performance).clone()}
                                />
                            },
                            ProfileTab::Ratings => html! {
                                <RatingsTab
                                    glicko_ratings={(*glicko_ratings).clone()}
                                    glicko_loading={*glicko_loading}
                                    glicko_error={(*glicko_error).clone()}
                                    rating_history={(*rating_history).clone()}
                                    rating_history_loading={*rating_history_loading}
                                    rating_history_error={(*rating_history_error).clone()}
                                />
                            },
                            ProfileTab::Achievements => html! {
                                <AchievementsTab
                                    achievements={(*achievements).clone()}
                                    loading={*achievements_loading}
                                    error={(*achievements_error).clone()}
                                />
                            },
                            ProfileTab::Nemesis => html! {
                                <NemesisTab
                                    opponents_who_beat_me={(*opponents_who_beat_me).clone()}
                                    on_open_contest_modal={fetch_contest_details.clone()}
                                />
                            },
                            ProfileTab::Owned => html! {
                                <OwnedTab
                                    opponents_i_beat={(*opponents_i_beat).clone()}
                                    on_open_contest_modal={fetch_contest_details.clone()}
                                />
                            },
                            ProfileTab::GamePerformance => html! {
                                <GamePerformanceTab game_performance={(*game_performance).clone()} />
                            },
                            ProfileTab::Trends => {
                                let current_rating = if let Some(ratings) = &*glicko_ratings {
                                    if let Some(global_rating) = ratings.iter().find(|r| {
                                        r.get("scope").and_then(|s| s.get("type")).and_then(|t| t.as_str()) == Some("Global")
                                    }) {
                                        global_rating.get("rating").and_then(|r| r.as_f64())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                let on_game_change = {
                                    let selected_game_id = selected_game_id.clone();
                                    Callback::from(move |value: Option<String>| {
                                        selected_game_id.set(value);
                                    })
                                };
                                let on_venue_change = {
                                    let selected_venue_id = selected_venue_id.clone();
                                    Callback::from(move |value: Option<String>| {
                                        selected_venue_id.set(value);
                                    })
                                };

                                html! {
                                    <TrendsTab
                                        performance_trends={(*performance_trends).clone()}
                                        current_rating={current_rating}
                                        games={(*games).clone()}
                                        venues={(*venues).clone()}
                                        selected_game_id={(*selected_game_id).clone()}
                                        selected_venue_id={(*selected_venue_id).clone()}
                                        on_game_change={on_game_change}
                                        on_venue_change={on_venue_change}
                                        trends_loading={*trends_loading}
                                        trends_error={(*trends_error).clone()}
                                    />
                                }
                            },
                            ProfileTab::Comparison => html! {
                                <ComparisonTab player_id={player_id_override.clone()} />
                            },
                            ProfileTab::Settings => html! {
                                {if viewing_other_player {
                                    html! {
                                        <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                                            <h2 class="text-2xl font-bold text-gray-900 mb-2">{"Settings"}</h2>
                                            <p class="text-gray-600">{"Settings are only available on your own profile."}</p>
                                        </div>
                                    }
                                } else {
                                    html! { <SettingsTab /> }
                                }}
                            },
                        }}
                    </div>
                </div>

                // Head-to-Head Contest Modal
                <ContestsModal
                    is_open={*contest_modal_open}
                    on_close={Callback::from(move |_| contest_modal_open.set(false))}
                    title={format!("⚔️ Contests vs {}", (*selected_opponent).as_ref().map(|opp| opp.2.clone()).unwrap_or_default())}
                    subtitle={Some("Head-to-head contest history".to_string())}
                    contests={(*contest_details).clone()}
                    loading={*contest_modal_loading}
                    error={(*contest_modal_error).clone()}
                    show_bgg_link={None::<String>}
                />
            </div>
        }
    }
}
