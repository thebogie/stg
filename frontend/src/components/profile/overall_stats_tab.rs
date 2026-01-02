use yew::prelude::*;
use shared::models::client_analytics::CoreStats;

#[derive(Properties, PartialEq)]
pub struct OverallStatsTabProps {
    pub core_stats: Option<CoreStats>,
    pub game_performance: Option<Vec<shared::models::client_analytics::GamePerformance>>,
}

#[function_component(OverallStatsTab)]
pub fn overall_stats_tab(props: &OverallStatsTabProps) -> Html {
    let core_stats = props.core_stats.clone();
    let game_performance = props.game_performance.clone();
    
    // Debug logging
    if let Some(stats) = &core_stats {
        web_sys::console::log_1(&format!("📊 OverallStatsTab received core_stats: current_streak={}, longest_streak={}", stats.current_streak, stats.longest_streak).into());
    } else {
        web_sys::console::log_1(&"📊 OverallStatsTab received no core_stats".into());
    }

    // Calculate additional stats from available data
    let unique_games = if let Some(games) = &game_performance {
        games.len()
    } else {
        0
    };


    html! {
        <div class="space-y-6">
            // Header
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="flex items-center justify-between">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-900">{"Overall Statistics"}</h2>
                        <p class="mt-1 text-gray-600">{"Your comprehensive gaming performance overview"}</p>
                    </div>
                    <div class="text-4xl">{"📊"}</div>
                </div>
            </div>

            // Core Performance Metrics
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                // Total Contests
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center">
                        <div class="flex-shrink-0">
                            <div class="w-8 h-8 bg-blue-100 rounded-lg flex items-center justify-center">
                                <span class="text-blue-600 text-lg">{"🎯"}</span>
                            </div>
                        </div>
                        <div class="ml-4">
                            <p class="text-sm font-medium text-gray-500">{"Total Contests"}</p>
                            <p class="text-2xl font-bold text-gray-900">
                                {if let Some(stats) = &core_stats {
                                    stats.total_contests.to_string()
                                } else {
                                    "0".to_string()
                                }}
                            </p>
                        </div>
                    </div>
                </div>

                // Total Wins
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center">
                        <div class="flex-shrink-0">
                            <div class="w-8 h-8 bg-green-100 rounded-lg flex items-center justify-center">
                                <span class="text-green-600 text-lg">{"🏆"}</span>
                            </div>
                        </div>
                        <div class="ml-4">
                            <p class="text-sm font-medium text-gray-500">{"Total Wins"}</p>
                            <p class="text-2xl font-bold text-gray-900">
                                {if let Some(stats) = &core_stats {
                                    stats.total_wins.to_string()
                                } else {
                                    "0".to_string()
                                }}
                            </p>
                        </div>
                    </div>
                </div>

                // Total Losses
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center">
                        <div class="flex-shrink-0">
                            <div class="w-8 h-8 bg-red-100 rounded-lg flex items-center justify-center">
                                <span class="text-red-600 text-lg">{"💔"}</span>
                            </div>
                        </div>
                        <div class="ml-4">
                            <p class="text-sm font-medium text-gray-500">{"Total Losses"}</p>
                            <p class="text-2xl font-bold text-gray-900">
                                {if let Some(stats) = &core_stats {
                                    stats.total_losses.to_string()
                                } else {
                                    "0".to_string()
                                }}
                            </p>
                        </div>
                    </div>
                </div>

                // Win Rate
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center">
                        <div class="flex-shrink-0">
                            <div class="w-8 h-8 bg-purple-100 rounded-lg flex items-center justify-center">
                                <span class="text-purple-600 text-lg">{"📈"}</span>
                            </div>
                        </div>
                        <div class="ml-4">
                            <p class="text-sm font-medium text-gray-500">{"Win Rate"}</p>
                            <p class="text-2xl font-bold text-gray-900">
                                {if let Some(stats) = &core_stats {
                                    format!("{:.1}%", stats.win_rate)
                                } else {
                                    "0.0%".to_string()
                                }}
                            </p>
                        </div>
                    </div>
                </div>

                // Average Placement
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center">
                        <div class="flex-shrink-0">
                            <div class="w-8 h-8 bg-yellow-100 rounded-lg flex items-center justify-center">
                                <span class="text-yellow-600 text-lg">{"🎯"}</span>
                            </div>
                        </div>
                        <div class="ml-4">
                            <p class="text-sm font-medium text-gray-500">{"Average Placement"}</p>
                            <p class="text-2xl font-bold text-gray-900">
                                {if let Some(stats) = &core_stats {
                                    format!("{:.1}", stats.average_placement)
                                } else {
                                    "0.0".to_string()
                                }}
                            </p>
                        </div>
                    </div>
                </div>

                // Best Placement
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center">
                        <div class="flex-shrink-0">
                            <div class="w-8 h-8 bg-indigo-100 rounded-lg flex items-center justify-center">
                                <span class="text-indigo-600 text-lg">{"⭐"}</span>
                            </div>
                        </div>
                        <div class="ml-4">
                            <p class="text-sm font-medium text-gray-500">{"Best Placement"}</p>
                            <p class="text-2xl font-bold text-gray-900">
                                {if let Some(stats) = &core_stats {
                                    stats.best_placement.to_string()
                                } else {
                                    "0".to_string()
                                }}
                            </p>
                        </div>
                    </div>
                </div>
            </div>

            // Activity Overview Section
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="flex items-center justify-between mb-4">
                    <div>
                        <h3 class="text-lg font-semibold text-gray-900">{"Activity Overview"}</h3>
                        <p class="text-sm text-gray-600">{"Your gaming diversity and engagement"}</p>
                    </div>
                    <div class="text-3xl">{"🎮"}</div>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div class="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
                        <span class="text-sm text-gray-600">{"Unique Games Played"}</span>
                        <span class="font-semibold text-gray-900">{unique_games}</span>
                    </div>
                    <div class="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
                        <span class="text-sm text-gray-600">{"Total Points Earned"}</span>
                        <span class="font-semibold text-gray-900">
                            {if let Some(stats) = &core_stats {
                                stats.total_points.to_string()
                            } else {
                                "0".to_string()
                            }}
                        </span>
                    </div>
                </div>
            </div>

            // Streaks Section
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                // Current Streak
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center justify-between">
                        <div>
                            <h3 class="text-lg font-semibold text-gray-900">{"Current Streak"}</h3>
                            <p class="text-sm text-gray-600">{"Your active winning streak"}</p>
                        </div>
                        <div class="text-3xl">{"🔥"}</div>
                    </div>
                    <div class="mt-4">
                        <p class="text-3xl font-bold text-orange-600">
                            {if let Some(stats) = &core_stats {
                                stats.current_streak.to_string()
                            } else {
                                "0".to_string()
                            }}
                        </p>
                        <p class="text-sm text-gray-500 mt-1">
                            {if let Some(stats) = &core_stats {
                                if stats.current_streak > 0 {
                                    "consecutive wins"
                                } else {
                                    "No active streak"
                                }
                            } else {
                                "Loading..."
                            }}
                        </p>
                    </div>
                </div>

                // Longest Streak
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center justify-between">
                        <div>
                            <h3 class="text-lg font-semibold text-gray-900">{"Longest Streak"}</h3>
                            <p class="text-sm text-gray-600">{"Your best winning streak ever"}</p>
                        </div>
                        <div class="text-3xl">{"💎"}</div>
                    </div>
                    <div class="mt-4">
                        <p class="text-3xl font-bold text-purple-600">
                            {if let Some(stats) = &core_stats {
                                stats.longest_streak.to_string()
                            } else {
                                "0".to_string()
                            }}
                        </p>
                        <p class="text-sm text-gray-500 mt-1">
                            {if let Some(stats) = &core_stats {
                                if stats.longest_streak > 0 {
                                    "consecutive wins"
                                } else {
                                    "No streak recorded"
                                }
                            } else {
                                "Loading..."
                            }}
                        </p>
                    </div>
                </div>
            </div>

            // Performance Summary
            <div class="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-xl p-6 border border-blue-100">
                <div class="flex items-center justify-between">
                    <div>
                        <h3 class="text-lg font-semibold text-gray-900">{"Performance Summary"}</h3>
                        <p class="text-sm text-gray-600">{"Your overall gaming performance"}</p>
                    </div>
                    <div class="text-3xl">{"📋"}</div>
                </div>
                <div class="mt-4 grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div class="text-center">
                        <p class="text-2xl font-bold text-blue-600">
                            {if let Some(stats) = &core_stats {
                                format!("{:.1}%", stats.win_rate)
                            } else {
                                "0.0%".to_string()
                            }}
                        </p>
                        <p class="text-sm text-gray-600">{"Win Rate"}</p>
                    </div>
                    <div class="text-center">
                        <p class="text-2xl font-bold text-green-600">
                            {if let Some(stats) = &core_stats {
                                stats.total_wins.to_string()
                            } else {
                                "0".to_string()
                            }}
                        </p>
                        <p class="text-sm text-gray-600">{"Total Wins"}</p>
                    </div>
                    <div class="text-center">
                        <p class="text-2xl font-bold text-purple-600">
                            {if let Some(stats) = &core_stats {
                                format!("{:.1}", stats.average_placement)
                            } else {
                                "0.0".to_string()
                            }}
                        </p>
                        <p class="text-sm text-gray-600">{"Avg Placement"}</p>
                    </div>
                </div>
            </div>
        </div>
    }
}
