use crate::api::utils::authenticated_get;
use crate::Route;
use shared::dto::analytics::{LeaderboardCategory, LeaderboardResponse, TimePeriod};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::Link;

#[function_component(Leaderboards)]
pub fn leaderboards() -> Html {
    let category = use_state(|| LeaderboardCategory::WinRate);
    let time_period = use_state(|| TimePeriod::AllTime);
    let limit = use_state(|| 25i32);
    let offset = use_state(|| 0i32);
    let leaderboard = use_state(|| None::<LeaderboardResponse>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);

    {
        let category_handle = category.clone();
        let time_period_handle = time_period.clone();
        let limit_handle = limit.clone();
        let offset_handle = offset.clone();
        let leaderboard = leaderboard.clone();
        let loading = loading.clone();
        let error = error.clone();

        use_effect_with(
            (
                (*category_handle).clone(),
                (*time_period_handle).clone(),
                *limit_handle,
                *offset_handle,
            ),
            move |_| {
                let limit: i32 = *limit_handle;
                let offset: i32 = *offset_handle;

                loading.set(true);
                error.set(None);

                spawn_local(async move {
                    let category_param = match (*category_handle).clone() {
                        LeaderboardCategory::WinRate => "win_rate",
                        LeaderboardCategory::TotalWins => "total_wins",
                        LeaderboardCategory::SkillRating => "skill_rating",
                        LeaderboardCategory::TotalContests => "total_contests",
                        LeaderboardCategory::LongestStreak => "longest_streak",
                        LeaderboardCategory::BestPlacement => "best_placement",
                    };
                    let time_param = match (*time_period_handle).clone() {
                        TimePeriod::AllTime => "all_time",
                        TimePeriod::Last30Days => "last_30_days",
                        TimePeriod::Last7Days => "last_7_days",
                        TimePeriod::Last90Days => "last_90_days",
                        TimePeriod::ThisYear => "this_year",
                    };

                    let url = format!(
                        "/api/analytics/leaderboard?category={}&time_period={}&limit={}&offset={}",
                        category_param, time_param, limit, offset
                    );

                    match authenticated_get(&url).send().await {
                        Ok(response) => {
                            if response.ok() {
                                match response.json::<LeaderboardResponse>().await {
                                    Ok(data) => leaderboard.set(Some(data)),
                                    Err(e) => error
                                        .set(Some(format!("Failed to parse leaderboard: {}", e))),
                                }
                            } else {
                                error.set(Some(format!(
                                    "Failed to fetch leaderboard: {}",
                                    response.status()
                                )));
                            }
                        }
                        Err(e) => error.set(Some(format!("Failed to fetch leaderboard: {}", e))),
                    }

                    loading.set(false);
                });

                || ()
            },
        );
    }

    let format_value = |category: &LeaderboardCategory, value: f64| -> String {
        match category {
            LeaderboardCategory::WinRate => format!("{:.1}%", value),
            LeaderboardCategory::BestPlacement => {
                if value > 0.0 {
                    format!("#{}", value as i32)
                } else {
                    "-".to_string()
                }
            }
            LeaderboardCategory::SkillRating => format!("{:.0}", value),
            _ => format!("{:.0}", value),
        }
    };

    let (current_page, total_pages) = if let Some(data) = &*leaderboard {
        let total = data.total_entries.max(1);
        let size = (*limit).max(1);
        let page = ((*offset) / size) + 1;
        let pages = (total + size - 1) / size;
        (page, pages)
    } else {
        (1, 1)
    };

    html! {
        <div class="min-h-screen bg-gray-50">
            <div class="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                    <div class="flex items-center justify-between mb-4">
                        <div>
                            <h1 class="text-3xl font-bold text-gray-900">{"Leaderboards"}</h1>
                            <p class="mt-1 text-gray-600">{"See top performers across the platform."}</p>
                        </div>
                        <div class="text-4xl">{"🏆"}</div>
                    </div>

                    <div class="bg-gray-50 rounded-lg p-4 mb-6">
                        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                            <div>
                                <label class="block text-xs font-medium text-gray-500 mb-1">{"Category"}</label>
                                <select
                                    class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm"
                                    value={format!("{:?}", (*category).clone())}
                                    onchange={{
                                        let category = category.clone();
                                        let offset = offset.clone();
                                        Callback::from(move |event: Event| {
                                            let value = event
                                                .target_unchecked_into::<web_sys::HtmlSelectElement>()
                                                .value();
                                            let parsed = match value.as_str() {
                                                "WinRate" => LeaderboardCategory::WinRate,
                                                "TotalWins" => LeaderboardCategory::TotalWins,
                                                "SkillRating" => LeaderboardCategory::SkillRating,
                                                "TotalContests" => LeaderboardCategory::TotalContests,
                                                "LongestStreak" => LeaderboardCategory::LongestStreak,
                                                "BestPlacement" => LeaderboardCategory::BestPlacement,
                                                _ => LeaderboardCategory::WinRate,
                                            };
                                            category.set(parsed);
                                            offset.set(0);
                                        })
                                    }}
                                >
                                    <option value="WinRate">{"Win Rate"}</option>
                                    <option value="TotalWins">{"Total Wins"}</option>
                                    <option value="TotalContests">{"Total Contests"}</option>
                                    <option value="SkillRating">{"Skill Rating"}</option>
                                    <option value="LongestStreak">{"Longest Streak"}</option>
                                    <option value="BestPlacement">{"Best Placement"}</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-xs font-medium text-gray-500 mb-1">{"Time Period"}</label>
                                <select
                                    class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm"
                                    value={format!("{:?}", (*time_period).clone())}
                                    onchange={{
                                        let time_period = time_period.clone();
                                        let offset = offset.clone();
                                        Callback::from(move |event: Event| {
                                            let value = event
                                                .target_unchecked_into::<web_sys::HtmlSelectElement>()
                                                .value();
                                            let parsed = match value.as_str() {
                                                "AllTime" => TimePeriod::AllTime,
                                                "Last30Days" => TimePeriod::Last30Days,
                                                "Last7Days" => TimePeriod::Last7Days,
                                                "Last90Days" => TimePeriod::Last90Days,
                                                "ThisYear" => TimePeriod::ThisYear,
                                                _ => TimePeriod::AllTime,
                                            };
                                            time_period.set(parsed);
                                            offset.set(0);
                                        })
                                    }}
                                >
                                    <option value="AllTime">{"All time"}</option>
                                    <option value="Last30Days">{"Last 30 days"}</option>
                                    <option value="Last7Days">{"Last 7 days"}</option>
                                    <option value="Last90Days">{"Last 90 days"}</option>
                                    <option value="ThisYear">{"This year"}</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-xs font-medium text-gray-500 mb-1">{"Rows"}</label>
                                <select
                                    class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm"
                                    value={(*limit).to_string()}
                                    onchange={{
                                        let limit = limit.clone();
                                        let offset = offset.clone();
                                        Callback::from(move |event: Event| {
                                            let value = event
                                                .target_unchecked_into::<web_sys::HtmlSelectElement>()
                                                .value();
                                            let parsed = value.parse::<i32>().unwrap_or(25);
                                            limit.set(parsed);
                                            offset.set(0);
                                        })
                                    }}
                                >
                                    <option value="10">{"Top 10"}</option>
                                    <option value="25">{"Top 25"}</option>
                                    <option value="50">{"Top 50"}</option>
                                </select>
                            </div>
                        </div>
                    </div>

                    if *loading {
                        <div class="text-center py-8 text-gray-500">{"Loading leaderboard..."}</div>
                    } else if let Some(error) = &*error {
                        <div class="text-center py-8 text-red-600">{error}</div>
                    } else if let Some(data) = &*leaderboard {
                        <div class="space-y-4">
                            <div class="flex items-center justify-between text-xs text-gray-500">
                                <span>{format!("Last updated: {}", data.last_updated)}</span>
                                <span>{format!("Page {} of {}", current_page, total_pages)}</span>
                            </div>
                            <div class="overflow-x-auto rounded-lg border border-gray-200">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Rank"}</th>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Player"}</th>
                                            <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{data.category.to_string()}</th>
                                            <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"Profile"}</th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white divide-y divide-gray-200">
                                        {data.entries.iter().map(|entry| {
                                            let display_name = if !entry.player_handle.is_empty() {
                                                entry.player_handle.clone()
                                            } else {
                                                entry.player_name.clone()
                                            };
                                            let player_key = entry
                                                .player_id
                                                .split('/')
                                                .last()
                                                .unwrap_or(entry.player_id.as_str())
                                                .to_string();
                                            html! {
                                                <tr class="hover:bg-gray-50">
                                                    <td class="px-3 py-2 text-sm text-gray-900">{entry.rank}</td>
                                                    <td class="px-3 py-2 text-sm text-gray-900">
                                                        <div class="font-medium">{display_name}</div>
                                                        <div class="text-xs text-gray-500">{entry.player_id.clone()}</div>
                                                    </td>
                                                    <td class="px-3 py-2 text-sm text-right font-medium text-gray-900">
                                                        {format_value(&data.category, entry.value)}
                                                    </td>
                                                    <td class="px-3 py-2 text-sm text-right">
                                                        <Link<Route>
                                                            to={Route::PlayerProfile { player_id: player_key }}
                                                            classes="text-blue-600 hover:text-blue-800 hover:underline"
                                                        >
                                                            {"View profile"}
                                                        </Link<Route>>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect::<Html>()}
                                    </tbody>
                                </table>
                            </div>
                            <div class="flex items-center justify-between">
                                <button
                                    class="inline-flex items-center rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-600 hover:bg-gray-50 disabled:opacity-50"
                                    disabled={*offset == 0}
                                    onclick={{
                                        let offset = offset.clone();
                                        let limit = limit.clone();
                                        Callback::from(move |_| {
                                            let next = (*offset - *limit).max(0);
                                            offset.set(next);
                                        })
                                    }}
                                    type="button"
                                >
                                    {"Previous"}
                                </button>
                                <button
                                    class="inline-flex items-center rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-600 hover:bg-gray-50 disabled:opacity-50"
                                    disabled={current_page >= total_pages}
                                    onclick={{
                                        let offset = offset.clone();
                                        let limit = limit.clone();
                                        Callback::from(move |_| {
                                            offset.set(*offset + *limit);
                                        })
                                    }}
                                    type="button"
                                >
                                    {"Next"}
                                </button>
                            </div>
                        </div>
                    } else {
                        <div class="text-center py-8 text-gray-500">{"No leaderboard data yet."}</div>
                    }
                </div>
            </div>
        </div>
    }
}
