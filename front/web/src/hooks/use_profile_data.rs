//! Single source for Player Profile data (Tauri-standard: one load path for bundle + ratings).
//! Profile is always "view this player": from leaderboard we pass that player's id; from nav we pass
//! None and the backend resolves "me" to the current user. Same page, same API shape.
//! Uses ProfileCacheContext when available so returning to the profile page shows cached data.
//!
//! Data strategy: progressive + parallel (see docs/PROFILE_PAGE_DATA_ARCHITECTURE.md).
//! Critical path: summary → first paint; then achievements, opponents, and (bundle + ratings + history) in parallel.

use crate::api::utils::authenticated_get;
use crate::context::profile_cache::{CachedProfileEntry, ProfileCacheContextValue};
use futures::future::join3;
use js_sys::{encode_uri_component, Date};
use shared::dto::analytics::{
    HeadToHeadRecordDto, ProfileBundleDto, ProfileOpponentsDto, ProfileSummaryDto,
};
use shared::models::client_analytics::{CoreStats, GamePerformance, PerformanceTrend};
use shared::PlayerAchievementsDto;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::UseStateHandle;

/// Handles to all profile-related state. Pass these (or their values) to profile tab components.
#[derive(Clone)]
pub struct ProfileDataHandles {
    pub display_label: UseStateHandle<Option<String>>,
    pub core_stats: UseStateHandle<Option<CoreStats>>,
    pub achievements: UseStateHandle<Option<PlayerAchievementsDto>>,
    pub game_performance: UseStateHandle<Option<Vec<GamePerformance>>>,
    pub performance_trends: UseStateHandle<Option<Vec<PerformanceTrend>>>,
    pub opponents_who_beat_me: UseStateHandle<Option<Vec<HeadToHeadRecordDto>>>,
    pub opponents_i_beat: UseStateHandle<Option<Vec<HeadToHeadRecordDto>>>,
    pub glicko_ratings: UseStateHandle<Option<Vec<serde_json::Value>>>,
    pub glicko_loading: UseStateHandle<bool>,
    pub glicko_error: UseStateHandle<Option<String>>,
    pub rating_history: UseStateHandle<Option<Vec<serde_json::Value>>>,
    pub rating_history_loading: UseStateHandle<bool>,
    pub rating_history_error: UseStateHandle<Option<String>>,
}

/// Resolve (player_id for URLs, profile param, viewing_other_player).
fn resolve_player(
    player_id_override: &Option<String>,
    is_authenticated: bool,
) -> Option<(String, String, bool)> {
    if let Some(ref override_id) = player_id_override {
        let id = override_id
            .strip_prefix("player/")
            .unwrap_or(override_id)
            .to_string();
        let param = encode_uri_component(&id)
            .as_string()
            .unwrap_or_else(|| id.clone());
        return Some((id, param, true));
    }
    if is_authenticated {
        return Some(("me".to_string(), "me".to_string(), false));
    }
    None
}

/// Hydrate profile UI state from a cached bundle (no network).
/// Only sets core_stats when bundle has real stats so we never overwrite with zeros.
fn hydrate_from_bundle(
    bundle: &ProfileBundleDto,
    _viewing_other_player: bool,
    display_label: &UseStateHandle<Option<String>>,
    core_stats: &UseStateHandle<Option<CoreStats>>,
    achievements: &UseStateHandle<Option<PlayerAchievementsDto>>,
    game_performance: &UseStateHandle<Option<Vec<GamePerformance>>>,
    performance_trends: &UseStateHandle<Option<Vec<PerformanceTrend>>>,
    opponents_who_beat_me: &UseStateHandle<Option<Vec<HeadToHeadRecordDto>>>,
    opponents_i_beat: &UseStateHandle<Option<Vec<HeadToHeadRecordDto>>>,
) {
    display_label.set(bundle.display_label.clone());
    if bundle.player_stats.total_contests > 0 {
        core_stats.set(Some(CoreStats {
            total_contests: bundle.player_stats.total_contests,
            total_wins: bundle.player_stats.total_wins,
            total_losses: bundle.player_stats.total_losses,
            win_rate: bundle.player_stats.win_rate,
            average_placement: bundle.player_stats.average_placement,
            best_placement: bundle.player_stats.best_placement,
            worst_placement: 0,
            current_streak: bundle.player_stats.current_streak,
            longest_streak: bundle.player_stats.longest_streak,
            skill_rating: bundle.player_stats.skill_rating,
            total_points: bundle.player_stats.total_points,
        }));
    }
    achievements.set(Some(bundle.achievements.clone()));
    game_performance.set(Some(
        bundle
            .game_performance
            .iter()
            .map(|dto| GamePerformance {
                game: shared::models::client_analytics::ClientGame {
                    id: dto.game_id.clone(),
                    name: dto.game_name.clone(),
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
            })
            .collect(),
    ));
    performance_trends.set(Some(
        bundle
            .performance_trends
            .iter()
            .map(|dto| PerformanceTrend {
                period: dto.month.clone(),
                contests_played: dto.contests_played,
                wins: dto.wins,
                win_rate: dto.win_rate,
                average_placement: dto.average_placement,
                skill_rating: dto.skill_rating,
            })
            .collect(),
    ));
    opponents_who_beat_me.set(Some(
        bundle
            .opponents_who_beat_me
            .iter()
            .map(|o| HeadToHeadRecordDto {
                opponent_id: o.player_id.clone(),
                opponent_handle: o.player_handle.clone(),
                opponent_name: o.player_name.clone(),
                total_contests: o.contests_played,
                my_wins: o.losses_to_me,
                opponent_wins: o.wins_against_me,
                my_win_rate: 100.0 - o.win_rate_against_me,
                contest_history: vec![],
            })
            .collect(),
    ));
    opponents_i_beat.set(Some(
        bundle
            .opponents_i_beat
            .iter()
            .map(|o| HeadToHeadRecordDto {
                opponent_id: o.player_id.clone(),
                opponent_handle: o.player_handle.clone(),
                opponent_name: o.player_name.clone(),
                total_contests: o.contests_played,
                my_wins: o.losses_to_me,
                opponent_wins: o.wins_against_me,
                my_win_rate: 100.0 - o.win_rate_against_me,
                contest_history: vec![],
            })
            .collect(),
    ));
}

/// Load profile bundle + ratings + rating history (one Tauri-standard entry point).
/// Returns (loading, error, handles). When not authenticated, error is set and effect no-ops.
#[hook]
pub fn use_profile_data(
    player_id_override: Option<String>,
    is_authenticated: bool,
) -> (
    UseStateHandle<bool>,
    UseStateHandle<Option<String>>,
    ProfileDataHandles,
) {
    let loading: UseStateHandle<bool> = use_state(|| true);
    let error: UseStateHandle<Option<String>> = use_state(|| None);
    let display_label: UseStateHandle<Option<String>> = use_state(|| None);
    let core_stats: UseStateHandle<Option<CoreStats>> = use_state(|| None);
    let achievements: UseStateHandle<Option<PlayerAchievementsDto>> = use_state(|| None);
    let game_performance: UseStateHandle<Option<Vec<GamePerformance>>> = use_state(|| None);
    let performance_trends: UseStateHandle<Option<Vec<PerformanceTrend>>> = use_state(|| None);
    let opponents_who_beat_me: UseStateHandle<Option<Vec<HeadToHeadRecordDto>>> =
        use_state(|| None);
    let opponents_i_beat: UseStateHandle<Option<Vec<HeadToHeadRecordDto>>> = use_state(|| None);
    let glicko_ratings: UseStateHandle<Option<Vec<serde_json::Value>>> = use_state(|| None);
    let glicko_loading: UseStateHandle<bool> = use_state(|| false);
    let glicko_error: UseStateHandle<Option<String>> = use_state(|| None);
    let rating_history: UseStateHandle<Option<Vec<serde_json::Value>>> = use_state(|| None);
    let rating_history_loading: UseStateHandle<bool> = use_state(|| false);
    let rating_history_error: UseStateHandle<Option<String>> = use_state(|| None);
    let cache_ctx = use_context::<ProfileCacheContextValue>();

    {
        let loading = loading.clone();
        let error = error.clone();
        let player_id_override = player_id_override.clone();
        let display_label = display_label.clone();
        let core_stats = core_stats.clone();
        let achievements = achievements.clone();
        let game_performance = game_performance.clone();
        let performance_trends = performance_trends.clone();
        let opponents_who_beat_me = opponents_who_beat_me.clone();
        let opponents_i_beat = opponents_i_beat.clone();
        let glicko_ratings = glicko_ratings.clone();
        let glicko_loading = glicko_loading.clone();
        let glicko_error = glicko_error.clone();
        let rating_history = rating_history.clone();
        let rating_history_loading = rating_history_loading.clone();
        let rating_history_error = rating_history_error.clone();
        let cache_ctx = cache_ctx.clone();

        use_effect_with(
            (
                player_id_override.clone(),
                is_authenticated,
                cache_ctx.clone(),
            ),
            move |(override_id, _, ctx)| {
                let loading = loading.clone();
                let error = error.clone();
                let display_label = display_label.clone();
                let core_stats = core_stats.clone();
                let achievements = achievements.clone();
                let game_performance = game_performance.clone();
                let performance_trends = performance_trends.clone();
                let opponents_who_beat_me = opponents_who_beat_me.clone();
                let opponents_i_beat = opponents_i_beat.clone();
                let glicko_ratings = glicko_ratings.clone();
                let glicko_loading = glicko_loading.clone();
                let _glicko_error = glicko_error.clone();
                let rating_history = rating_history.clone();
                let rating_history_loading = rating_history_loading.clone();
                let _rating_history_error = rating_history_error.clone();

                let Some((player_id, profile_param, viewing_other_player)) =
                    resolve_player(override_id, is_authenticated)
                else {
                    error.set(Some("Player not authenticated".to_string()));
                    loading.set(false);
                    return;
                };

                // If we have a fresh cache entry for this profile with real stats, hydrate and skip fetch.
                // Never use a cached bundle that has zero stats (would overwrite good data with zeros).
                if let Some(ref ctx) = ctx {
                    let now_ms = Date::new_0().get_time();
                    if let Some(entry) = (*ctx.cache).get(&profile_param) {
                        if entry.is_fresh(now_ms) && entry.bundle.player_stats.total_contests > 0 {
                            hydrate_from_bundle(
                                &entry.bundle,
                                viewing_other_player,
                                &display_label,
                                &core_stats,
                                &achievements,
                                &game_performance,
                                &performance_trends,
                                &opponents_who_beat_me,
                                &opponents_i_beat,
                            );
                            loading.set(false);
                            error.set(None);
                            glicko_loading.set(false);
                            rating_history_loading.set(false);
                            return;
                        }
                    }
                }

                let profile_param_for_cache = profile_param.clone();
                let cache_ctx_for_fetch = ctx.clone();

                spawn_local(async move {
                    loading.set(true);
                    error.set(None);
                    glicko_loading.set(true);
                    rating_history_loading.set(true);

                    let summary_url =
                        format!("/api/analytics/players/{}/profile/summary", profile_param);
                    let achievements_url = format!(
                        "/api/analytics/players/{}/achievements?refresh=true",
                        profile_param
                    );
                    let profile_url = format!("/api/analytics/players/{}/profile", profile_param);
                    let ratings_url = if player_id == "me" {
                        "/api/ratings/current".to_string()
                    } else {
                        format!("/api/ratings/player/{}", player_id)
                    };
                    let history_url = "/api/ratings/history?scope=global";

                    // Task 1: summary only — show page as soon as it returns (fast first paint)
                    {
                        let loading = loading.clone();
                        let error = error.clone();
                        let display_label = display_label.clone();
                        let core_stats = core_stats.clone();
                        spawn_local(async move {
                            match authenticated_get(&summary_url).send().await {
                                Ok(resp) if resp.ok() => {
                                    if let Ok(s) = resp.json::<ProfileSummaryDto>().await {
                                        display_label.set(s.display_label);
                                        // Never set core_stats to zeros; only apply when summary has real stats
                                        if s.player_stats.total_contests > 0 {
                                            core_stats.set(Some(CoreStats {
                                                total_contests: s.player_stats.total_contests,
                                                total_wins: s.player_stats.total_wins,
                                                total_losses: s.player_stats.total_losses,
                                                win_rate: s.player_stats.win_rate,
                                                average_placement: s.player_stats.average_placement,
                                                best_placement: s.player_stats.best_placement,
                                                worst_placement: 0,
                                                current_streak: s.player_stats.current_streak,
                                                longest_streak: s.player_stats.longest_streak,
                                                skill_rating: s.player_stats.skill_rating,
                                                total_points: s.player_stats.total_points,
                                            }));
                                        }
                                    }
                                    loading.set(false);
                                }
                                Ok(resp) => {
                                    error.set(Some(format!(
                                        "Profile summary failed (status {})",
                                        resp.status()
                                    )));
                                    loading.set(false);
                                }
                                Err(e) => {
                                    error.set(Some(format!("Failed to load profile: {}", e)));
                                    loading.set(false);
                                }
                            }
                        });
                    }

                    // Task 1b: achievements only — so Achievements tab can show data without waiting for full bundle
                    {
                        let achievements = achievements.clone();
                        spawn_local(async move {
                            if let Ok(resp) = authenticated_get(&achievements_url).send().await {
                                if resp.ok() {
                                    if let Ok(a) = resp.json::<PlayerAchievementsDto>().await {
                                        achievements.set(Some(a));
                                    }
                                }
                            }
                        });
                    }

                    // Task 1c: opponents only (nemesis) — so Nemesis tab can show data without waiting for full bundle.
                    let opponents_url =
                        format!("/api/analytics/players/{}/profile/opponents", profile_param);
                    {
                        let opponents_who_beat_me = opponents_who_beat_me.clone();
                        let opponents_i_beat = opponents_i_beat.clone();
                        spawn_local(async move {
                            match authenticated_get(&opponents_url).send().await {
                                Ok(resp) if resp.ok() => {
                                    if let Ok(o) = resp.json::<ProfileOpponentsDto>().await {
                                        opponents_who_beat_me.set(Some(
                                            o.opponents_who_beat_me
                                                .into_iter()
                                                .map(|x| HeadToHeadRecordDto {
                                                    opponent_id: x.player_id,
                                                    opponent_handle: x.player_handle,
                                                    opponent_name: x.player_name,
                                                    total_contests: x.contests_played,
                                                    my_wins: x.losses_to_me,
                                                    opponent_wins: x.wins_against_me,
                                                    my_win_rate: 100.0 - x.win_rate_against_me,
                                                    contest_history: vec![],
                                                })
                                                .collect(),
                                        ));
                                        opponents_i_beat.set(Some(
                                            o.opponents_i_beat
                                                .into_iter()
                                                .map(|x| HeadToHeadRecordDto {
                                                    opponent_id: x.player_id,
                                                    opponent_handle: x.player_handle,
                                                    opponent_name: x.player_name,
                                                    total_contests: x.contests_played,
                                                    my_wins: x.losses_to_me,
                                                    opponent_wins: x.wins_against_me,
                                                    my_win_rate: 100.0 - x.win_rate_against_me,
                                                    contest_history: vec![],
                                                })
                                                .collect(),
                                        ));
                                    } else {
                                        // Parse failed: show empty so Nemesis/Owned tabs don't spin forever
                                        opponents_who_beat_me.set(Some(vec![]));
                                        opponents_i_beat.set(Some(vec![]));
                                    }
                                }
                                _ => {
                                    // Non-ok (401, 404, 500): set empty so tabs show "No nemeses yet" / "No dominated opponents" instead of loading forever
                                    opponents_who_beat_me.set(Some(vec![]));
                                    opponents_i_beat.set(Some(vec![]));
                                }
                            }
                        });
                    }

                    // Task 2: full bundle + ratings + history (fills other tabs; clears loading if summary failed)
                    let profile_fut = async {
                        match authenticated_get(&profile_url).send().await {
                            Ok(resp) if resp.ok() => resp
                                .json::<ProfileBundleDto>()
                                .await
                                .map_err(|_| "Failed to parse profile".to_string()),
                            Ok(_) => Err("Failed to load profile".to_string()),
                            Err(e) => Err(format!("Failed to load profile: {}", e)),
                        }
                    };
                    let ratings_fut = async {
                        let response = authenticated_get(&ratings_url).send().await.ok()?;
                        if response.ok() {
                            response.json::<Vec<serde_json::Value>>().await.ok()
                        } else {
                            None
                        }
                    };
                    let history_fut = async {
                        let response = authenticated_get(history_url).send().await.ok()?;
                        if response.status() == 404 {
                            return Some(vec![]);
                        }
                        if response.ok() {
                            response.json::<Vec<serde_json::Value>>().await.ok()
                        } else {
                            None
                        }
                    };

                    let (profile_result, glicko_data, history_data) =
                        join3(profile_fut, ratings_fut, history_fut).await;

                    // Apply full bundle (tabs data) and ratings/history
                    match profile_result {
                        Ok(bundle) => {
                            let bundle_for_cache = bundle.clone();
                            if viewing_other_player && display_label.as_ref().is_none() {
                                display_label.set(bundle.display_label);
                            }
                            // Only overwrite core_stats from bundle when bundle has real stats;
                            // never replace good summary data with zeros (avoids flash-then-zero).
                            let new_core = CoreStats {
                                total_contests: bundle.player_stats.total_contests,
                                total_wins: bundle.player_stats.total_wins,
                                total_losses: bundle.player_stats.total_losses,
                                win_rate: bundle.player_stats.win_rate,
                                average_placement: bundle.player_stats.average_placement,
                                best_placement: bundle.player_stats.best_placement,
                                worst_placement: 0,
                                current_streak: bundle.player_stats.current_streak,
                                longest_streak: bundle.player_stats.longest_streak,
                                skill_rating: bundle.player_stats.skill_rating,
                                total_points: bundle.player_stats.total_points,
                            };
                            let use_bundle_stats = bundle.player_stats.total_contests > 0
                                && (core_stats.as_ref().is_none()
                                    || core_stats
                                        .as_ref()
                                        .map(|c| {
                                            bundle.player_stats.total_contests >= c.total_contests
                                        })
                                        .unwrap_or(true));
                            if use_bundle_stats {
                                core_stats.set(Some(new_core));
                            }
                            achievements.set(Some(bundle.achievements));
                            game_performance.set(Some(
                                bundle
                                    .game_performance
                                    .into_iter()
                                    .map(|dto| GamePerformance {
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
                                    })
                                    .collect(),
                            ));
                            performance_trends.set(Some(
                                bundle
                                    .performance_trends
                                    .into_iter()
                                    .map(|dto| PerformanceTrend {
                                        period: dto.month,
                                        contests_played: dto.contests_played,
                                        wins: dto.wins,
                                        win_rate: dto.win_rate,
                                        average_placement: dto.average_placement,
                                        skill_rating: dto.skill_rating,
                                    })
                                    .collect(),
                            ));
                            opponents_who_beat_me.set(Some(
                                bundle
                                    .opponents_who_beat_me
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
                                    .collect(),
                            ));
                            opponents_i_beat.set(Some(
                                bundle
                                    .opponents_i_beat
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
                                    .collect(),
                            ));

                            // Store in local cache only when bundle has real stats (never cache zeros).
                            if let Some(ref cache_ctx) = cache_ctx_for_fetch {
                                if bundle_for_cache.player_stats.total_contests > 0 {
                                    let now_ms = Date::new_0().get_time();
                                    let entry = CachedProfileEntry {
                                        bundle: bundle_for_cache,
                                        fetched_at_ms: now_ms,
                                    };
                                    cache_ctx.cache.set({
                                        let mut m = (*cache_ctx.cache).clone();
                                        m.insert(profile_param_for_cache.clone(), entry);
                                        m
                                    });
                                }
                            }

                            // Only merge rating into existing core_stats; never set zeros (unwrap_or_default would overwrite with 0)
                            if let Some(ref data) = glicko_data {
                                if let Some(global_rating) = data.iter().find(|r| {
                                    r.get("scope")
                                        .and_then(|s| s.get("type"))
                                        .and_then(|t| t.as_str())
                                        == Some("Global")
                                }) {
                                    if let Some(rating_value) = global_rating
                                        .get("rating")
                                        .and_then(|r: &serde_json::Value| r.as_f64())
                                    {
                                        if let Some(mut updated) = core_stats.as_ref().cloned() {
                                            updated.skill_rating = rating_value;
                                            core_stats.set(Some(updated));
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error.set(Some(e));
                        }
                    }

                    if glicko_data.is_some() {
                        glicko_ratings.set(glicko_data);
                    }
                    if let Some(hist) = history_data {
                        rating_history.set(Some(hist));
                    }

                    glicko_loading.set(false);
                    rating_history_loading.set(false);
                    loading.set(false);
                });
            },
        );
    }

    let data_handles = ProfileDataHandles {
        display_label,
        core_stats,
        achievements,
        game_performance,
        performance_trends,
        opponents_who_beat_me,
        opponents_i_beat,
        glicko_ratings,
        glicko_loading,
        glicko_error,
        rating_history,
        rating_history_loading,
        rating_history_error,
    };

    (loading, error, data_handles)
}
