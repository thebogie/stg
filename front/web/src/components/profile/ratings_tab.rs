use crate::components::chart_renderer::ChartRenderer;
use serde_json::Value;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct RatingsTabProps {
    pub glicko_ratings: Option<Vec<Value>>,
    pub glicko_loading: bool,
    pub glicko_error: Option<String>,
    pub rating_history: Option<Vec<Value>>,
    pub rating_history_loading: bool,
    pub rating_history_error: Option<String>,
}

#[function_component(RatingsTab)]
pub fn ratings_tab(props: &RatingsTabProps) -> Html {
    html! {
        <div class="space-y-6">
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="flex items-center justify-between mb-4">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-900">{"Glicko2 Ratings"}</h2>
                        <p class="mt-1 text-gray-600">
                            {"Your skill rating adjusts based on contest performance."}
                        </p>
                    </div>
                    <div class="text-4xl">{"🏅"}</div>
                </div>

                if props.glicko_loading {
                    <div class="flex items-center justify-center py-8">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                        <span class="ml-2 text-gray-600">{"Loading your ratings..."}</span>
                    </div>
                } else if let Some(error) = &props.glicko_error {
                    <div class="bg-red-50 border border-red-200 rounded-md p-4">
                        <div class="flex">
                            <div class="flex-shrink-0">
                                <svg class="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
                                </svg>
                            </div>
                            <div class="ml-3">
                                <h3 class="text-sm font-medium text-red-800">{"Error loading ratings"}</h3>
                                <div class="mt-2 text-sm text-red-700">{error}</div>
                            </div>
                        </div>
                    </div>
                } else if let Some(ratings) = &props.glicko_ratings {
                    { if ratings.is_empty() {
                        html! {
                            <div class="text-center py-8">
                                <div class="text-gray-400 mb-4">
                                    <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2zm0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h2a2 2 0 01-2-2z" />
                                    </svg>
                                </div>
                                <h3 class="text-lg font-medium text-gray-900 mb-2">{"No Ratings Yet"}</h3>
                                <p class="text-gray-500">{"Start playing contests to earn your first Glicko2 rating!"}</p>
                            </div>
                        }
                    } else {
                        if let Some(current_rating) = ratings.first() {
                            let rating_value = current_rating["rating"].as_f64().unwrap_or(1500.0);
                            let rd_value = current_rating["rd"].as_f64().unwrap_or(350.0);
                            let volatility = current_rating["volatility"].as_f64().unwrap_or(0.06);
                            let games_played = current_rating["games_played"].as_i64().unwrap_or(0);

                            let (tier_label, tier_class) = if rating_value >= 1900.0 {
                                ("Elite", "bg-green-100 text-green-800")
                            } else if rating_value >= 1700.0 {
                                ("Strong", "bg-emerald-100 text-emerald-800")
                            } else if rating_value >= 1600.0 {
                                ("Competitive", "bg-blue-100 text-blue-800")
                            } else if rating_value >= 1500.0 {
                                ("Intermediate", "bg-yellow-100 text-yellow-800")
                            } else {
                                ("New", "bg-gray-100 text-gray-800")
                            };

                            html! {
                                <div class="space-y-6">
                                    <div class="bg-white rounded-lg shadow p-6">
                                        <div class="flex items-center justify-between mb-4">
                                            <div>
                                                <h3 class="text-2xl font-bold text-gray-900">{"Your Glicko2 Rating"}</h3>
                                                <p class="text-sm text-gray-600">{"Current skill assessment"}</p>
                                            </div>
                                            <div class="text-right">
                                                <div class="text-4xl font-bold text-blue-600">{format!("{:.0}", rating_value)}</div>
                                                <span class={classes!("inline-flex", "items-center", "px-3", "py-1", "rounded-full", "text-sm", "font-medium", tier_class)}>{tier_label}</span>
                                            </div>
                                        </div>

                                        <div class="bg-blue-50 rounded-lg p-3 mb-4">
                                            <p class="text-sm text-blue-800">
                                                <strong>{"Note:"}</strong> {"This rating and all statistics below are from your latest month of play. The table below shows your monthly progression over time."}
                                            </p>
                                        </div>

                                        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-6">
                                            <div class="bg-gray-50 rounded-lg p-4">
                                                <h4 class="text-sm font-medium text-gray-500 uppercase tracking-wide">{"Rating Deviation (RD)"}</h4>
                                                <p class="text-2xl font-semibold text-gray-900">{format!("{:.0}", rd_value)}</p>
                                                <p class="text-xs text-gray-600 mt-1">{"Lower = more certain"}</p>
                                            </div>
                                            <div class="bg-gray-50 rounded-lg p-4">
                                                <h4 class="text-sm font-medium text-gray-500 uppercase tracking-wide">{"Volatility"}</h4>
                                                <p class="text-2xl font-semibold text-gray-900">{format!("{:.3}", volatility)}</p>
                                                <p class="text-xs text-gray-600 mt-1">{"Rating change speed"}</p>
                                            </div>
                                            <div class="bg-gray-50 rounded-lg p-4">
                                                <h4 class="text-sm font-medium text-gray-500 uppercase tracking-wide">{"Games Played (last month)"}</h4>
                                                <p class="text-2xl font-semibold text-gray-900">{games_played}</p>
                                                <p class="text-xs text-gray-600 mt-1">{"Most recent month of play"}</p>
                                            </div>
                                        </div>
                                    </div>

                                    <div class="bg-blue-50 rounded-lg p-6">
                                        <h4 class="text-lg font-semibold text-blue-900 mb-3">{"Understanding Glicko2"}</h4>
                                        <div class="space-y-3 text-sm text-blue-800">
                                            <div>
                                                <strong>{"Rating:"}</strong> {"Your skill level. Higher numbers = stronger players. Average is 1500."}
                                            </div>
                                            <div>
                                                <strong>{"Rating Deviation (RD):"}</strong> {"How certain we are about your rating. Lower RD = more confidence. New players start at 350."}
                                            </div>
                                            <div>
                                                <strong>{"Volatility:"}</strong> {"How quickly your rating can change. Higher volatility = more dramatic rating swings."}
                                            </div>
                                            <div>
                                                <strong>{"Rating Changes:"}</strong> {"Beat stronger players = bigger rating gains. Lose to weaker players = bigger rating losses."}
                                            </div>
                                        </div>
                                    </div>

                                    <div class="bg-white rounded-lg shadow p-6">
                                        <h4 class="text-lg font-semibold text-gray-900 mb-4">{"Rating Trends Over Time"}</h4>
                                        <div class="mb-4">
                                            <p class="text-gray-600 text-sm">
                                                <strong>{"Chart Explanation:"}</strong> {"This line chart shows your Glicko2 rating progression over time. Each point represents your rating at the end of a month. "}
                                                <strong>{"Trends:"}</strong> {"Upward lines show skill improvement, downward lines indicate challenging periods. "}
                                                <strong>{"Confidence:"}</strong> {"Steeper lines suggest rapid rating changes, while flatter lines show stability."}
                                            </p>
                                        </div>
                                        {
                                            if props.rating_history_loading {
                                                html! {<div class="h-64 bg-gray-50 rounded-lg flex items-center justify-center text-gray-500">{"Loading rating history..."}</div>}
                                            } else if let Some(err) = &props.rating_history_error {
                                                html! {<div class="bg-yellow-50 border border-yellow-200 rounded p-3 text-yellow-800">{err}</div>}
                                            } else if let Some(hist) = &props.rating_history {
                                                if hist.is_empty() {
                                                    html! {<div class="h-64 bg-gray-50 rounded-lg flex items-center justify-center text-gray-500">{"No history yet"}</div>}
                                                } else {
                                                    // Build multi-series chart data for Rating and RD
                                                    let mut rating_points: Vec<Value> = hist.iter()
                                                        .map(|h| {
                                                            let label = h["period_end"].as_str().unwrap_or("").to_string();
                                                            let value = h["rating"].as_f64().unwrap_or(1500.0);
                                                            serde_json::json!({
                                                                "label": label,
                                                                "value": value,
                                                                "color": null,
                                                                "metadata": null
                                                            })
                                                        }).collect();
                                                    let mut rd_points: Vec<Value> = hist.iter()
                                                        .map(|h| {
                                                            let label = h["period_end"].as_str().unwrap_or("").to_string();
                                                            let value = h["rd"].as_f64().unwrap_or(350.0);
                                                            serde_json::json!({
                                                                "label": label,
                                                                "value": value,
                                                                "color": null,
                                                                "metadata": null
                                                            })
                                                        }).collect();

                                                    // Limit to last 36 points for readability
                                                    if rating_points.len() > 36 { let _ = rating_points.drain(0..(rating_points.len()-36)); }
                                                    if rd_points.len() > 36 { let _ = rd_points.drain(0..(rd_points.len()-36)); }

                                                    let chart_json = serde_json::json!({
                                                        "chart_type": "Line",
                                                        "config": {
                                                            "title": "Monthly Rating and RD",
                                                            "width": 900,
                                                            "height": 300,
                                                            "colors": ["#2563EB", "#6B7280"],
                                                            "show_legend": true,
                                                            "show_grid": true,
                                                            "animation": true
                                                        },
                                                        "data": {
                                                            "MultiSeries": [
                                                                {
                                                                    "name": "Rating",
                                                                    "data": rating_points,
                                                                    "color": "#2563EB"
                                                                },
                                                                {
                                                                    "name": "RD",
                                                                    "data": rd_points,
                                                                    "color": "#6B7280"
                                                                }
                                                            ]
                                                        },
                                                        "metadata": {
                                                            "x_axis": "Month",
                                                            "y_axis_left": "Rating (skill estimate)",
                                                            "y_axis_right": "RD (uncertainty/confidence)",
                                                            "description": "Monthly Glicko2 rating (blue) and RD (gray). RD rises during inactivity.",
                                                            "insight": "High RD means the rating is less certain; consistent play reduces RD."
                                                        }
                                                    }).to_string();

                                                    html! {
                                                        <div>
                                                            <ChartRenderer chart_data={chart_json} chart_id={"rating-history".to_string()} width={Some(900)} height={Some(320)} />

                                                            <div class="mt-6">
                                                                <div class="overflow-x-auto">
                                                                    <table class="min-w-full divide-y divide-gray-200 text-sm">
                                                                        <thead class="bg-gray-50">
                                                                            <tr>
                                                                                <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Month"}</th>
                                                                                <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"Rating"}</th>
                                                                                <th class="px-2 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"RD (uncertainty)"}</th>
                                                                                <th class="px-2 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"Games"}</th>
                                                                                <th class="px-2 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"Wins"}</th>
                                                                                <th class="px-2 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"Losses"}</th>
                                                                            </tr>
                                                                        </thead>
                                                                        <tbody class="bg-white divide-y divide-gray-200">
                                                                             {hist.iter().take(12).map(|h| {
                                                                                 let period = h["period_end"].as_str().unwrap_or("").to_string();
                                                                                 // Format date to YYYY/MM/DD
                                                                                 let formatted_period = if !period.is_empty() {
                                                                                     // Handle ISO timestamp format like "2014-06-01T00:00:00Z"
                                                                                     if period.contains('T') {
                                                                                         let date_part = period.split('T').next().unwrap_or("");
                                                                                         let parts: Vec<&str> = date_part.split('-').collect();
                                                                                         if parts.len() >= 2 {
                                                                                             format!("{}/{}", parts[0], parts[1])
                                                                                         } else {
                                                                                             period
                                                                                         }
                                                                                     } else {
                                                                                         // Fallback for other formats
                                                                                         period
                                                                                     }
                                                                                 } else {
                                                                                     "Unknown".to_string()
                                                                                 };
                                                                                 let rating = h["rating"].as_f64().unwrap_or(1500.0);
                                                                                 let rd = h["rd"].as_f64().unwrap_or(350.0);
                                                                                 let games = h["period_games"].as_i64().unwrap_or(0);
                                                                                 let wins = h["wins"].as_i64().unwrap_or(0);
                                                                                 let losses = h["losses"].as_i64().unwrap_or(0);

                                                                                 html! {
                                                                                     <tr class="hover:bg-gray-50">
                                                                                         <td class="px-3 py-2 text-sm text-gray-900">{formatted_period}</td>
                                                                                         <td class="px-3 py-2 text-right text-sm font-medium text-gray-700">{format!("{:.0}", rating)}</td>
                                                                                         <td class="px-2 py-2 text-right text-sm text-gray-600">{format!("{:.0}", rd)}</td>
                                                                                         <td class="px-2 py-2 text-right text-sm text-gray-600">{games}</td>
                                                                                         <td class="px-2 py-2 text-right text-sm text-gray-600">{wins}</td>
                                                                                         <td class="px-2 py-2 text-right text-sm text-gray-600">{losses}</td>
                                                                                     </tr>
                                                                                 }
                                                                             }).collect::<Html>()}
                                                                         </tbody>
                                                                     </table>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    }
                                                }
                                            } else {
                                                html! {<div class="h-64 bg-gray-50 rounded-lg flex items-center justify-center text-gray-500">{"No history loaded"}</div>}
                                            }
                                        }
                                    </div>
                                </div>
                            }
                        } else {
                            html! { <p class="text-gray-600 text-center py-8">{"No rating data available"}</p> }
                        }
                    } }
                }
            </div>
        </div>
    }
}
