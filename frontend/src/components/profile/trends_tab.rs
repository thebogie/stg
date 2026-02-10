use shared::models::client_analytics::PerformanceTrend;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TrendsTabProps {
    pub performance_trends: Option<Vec<PerformanceTrend>>,
    pub current_rating: Option<f64>,
}

#[function_component(TrendsTab)]
pub fn trends_tab(props: &TrendsTabProps) -> Html {
    let time_range = use_state(|| 6usize);

    let filtered_trends: Option<Vec<PerformanceTrend>> = props.performance_trends.as_ref().map(|trends| {
        if *time_range == 0 || trends.len() <= *time_range {
            trends.clone()
        } else {
            trends.iter().rev().take(*time_range).cloned().collect::<Vec<_>>().into_iter().rev().collect()
        }
    });

    html! {
        <div class="space-y-6">
            <div class="bg-white shadow rounded-lg p-6">
                <h2 class="text-2xl font-bold text-gray-900 mb-4">{"📈 Performance Trends"}</h2>
                <div class="mb-4">
                    <p class="text-gray-600">
                        <strong>{"Performance Analysis:"}</strong> {"Track your gaming performance trends over time to identify patterns and areas for improvement."}
                    </p>
                </div>

                <div class="bg-gray-50 rounded-lg p-4 mb-6">
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                        <div>
                            <label class="block text-xs font-medium text-gray-500 mb-1">{"Time Range"}</label>
                            <select
                                class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm"
                                value={time_range.to_string()}
                                onchange={{
                                    let time_range = time_range.clone();
                                    Callback::from(move |event: Event| {
                                        let value = event
                                            .target_unchecked_into::<web_sys::HtmlSelectElement>()
                                            .value();
                                        let parsed = value.parse::<usize>().unwrap_or(6);
                                        time_range.set(parsed);
                                    })
                                }}
                            >
                                <option value="3">{"Last 3 months"}</option>
                                <option value="6">{"Last 6 months"}</option>
                                <option value="12">{"Last 12 months"}</option>
                                <option value="0">{"All time"}</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-xs font-medium text-gray-500 mb-1">{"Game"}</label>
                            <select class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm" disabled={true}>
                                <option>{"All games (coming soon)"}</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-xs font-medium text-gray-500 mb-1">{"Venue"}</label>
                            <select class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm" disabled={true}>
                                <option>{"All venues (coming soon)"}</option>
                            </select>
                        </div>
                    </div>
                    <p class="text-xs text-gray-500 mt-2">
                        {"Game and venue filters will be enabled once per-game and per-venue trend data is available."}
                    </p>
                </div>

                {if let Some(trends) = &filtered_trends {
                    if trends.is_empty() {
                        html! {
                            <div class="text-center py-8">
                                <div class="text-gray-400 mb-4">
                                    <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2zm0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h2a2 2 0 01-2-2z" />
                                    </svg>
                                </div>
                                <h3 class="text-lg font-medium text-gray-900 mb-2">{"No Trends Data Yet"}</h3>
                                <p class="text-gray-500">{"Play more contests to see your performance trends!"}</p>
                            </div>
                        }
                    } else {
                        html! {
                            <div class="space-y-6">
                                // Performance Overview Cards
                                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                                    {if let Some(latest_trend) = trends.iter().rev().find(|t| t.contests_played > 0) {
                                        html! {
                                            <>
                                <div class="bg-blue-50 rounded-lg p-4">
                                                    <h4 class="text-sm font-medium text-blue-900">{"Current Win Rate"}</h4>
                                                    <p class="text-2xl font-bold text-blue-700">{format!("{:.1}%", latest_trend.win_rate)}</p>
                                                    <p class="text-xs text-blue-600">{format!("{} wins / {} contests", latest_trend.wins, latest_trend.contests_played)}</p>
                                                </div>
                                                <div class="bg-green-50 rounded-lg p-4">
                                                    <h4 class="text-sm font-medium text-green-900">{"Average Placement"}</h4>
                                                    <p class="text-2xl font-bold text-green-700">{format!("{:.1}", latest_trend.average_placement)}</p>
                                                    <p class="text-xs text-green-600">{"Lower is better"}</p>
                                                </div>
                                                <div class="bg-purple-50 rounded-lg p-4">
                                                    <h4 class="text-sm font-medium text-purple-900">{"Current Rating"}</h4>
                                                    <p class="text-2xl font-bold text-purple-700">
                                                        {if let Some(rating) = props.current_rating {
                                                            format!("{:.0}", rating)
                                                        } else {
                                                            format!("{:.0}", latest_trend.skill_rating)
                                                        }}
                                                    </p>
                                                    <p class="text-xs text-purple-600">{"Glicko2 rating"}</p>
                                                </div>
                                            </>
                                        }
                                    } else { html! {} }}
                                </div>

                                // Compare to previous period
                                {if *time_range > 0 && trends.len() >= *time_range * 2 {
                                    let recent_slice: Vec<_> = trends.iter().rev().take(*time_range).collect();
                                    let previous_slice: Vec<_> = trends.iter().rev().skip(*time_range).take(*time_range).collect();
                                    let recent_avg_win = recent_slice.iter().map(|t| t.win_rate).sum::<f64>() / recent_slice.len() as f64;
                                    let prev_avg_win = previous_slice.iter().map(|t| t.win_rate).sum::<f64>() / previous_slice.len() as f64;
                                    let recent_avg_place = recent_slice.iter().map(|t| t.average_placement).sum::<f64>() / recent_slice.len() as f64;
                                    let prev_avg_place = previous_slice.iter().map(|t| t.average_placement).sum::<f64>() / previous_slice.len() as f64;
                                    let recent_contests = recent_slice.iter().map(|t| t.contests_played).sum::<i32>();
                                    let prev_contests = previous_slice.iter().map(|t| t.contests_played).sum::<i32>();
                                    html! {
                                        <div class="bg-white border rounded-lg p-4">
                                            <h3 class="text-sm font-semibold text-gray-900 mb-2">{"Compare to Previous Period"}</h3>
                                            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
                                                <div>
                                                    <div class="text-gray-500">{"Win Rate"}</div>
                                                    <div class="font-medium text-gray-900">
                                                        {format!("{:.1}% ({:+.1}%)", recent_avg_win, recent_avg_win - prev_avg_win)}
                                                    </div>
                                                </div>
                                                <div>
                                                    <div class="text-gray-500">{"Avg Placement"}</div>
                                                    <div class="font-medium text-gray-900">
                                                        {format!("{:.1} ({:+.1})", recent_avg_place, recent_avg_place - prev_avg_place)}
                                                    </div>
                                                </div>
                                                <div>
                                                    <div class="text-gray-500">{"Contests"}</div>
                                                    <div class="font-medium text-gray-900">
                                                        {format!("{} ({:+})", recent_contests, recent_contests - prev_contests)}
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                } else { html! {} }}

                                // Win Rate Trend Chart
                                <div class="bg-white border rounded-lg p-6">
                                    <h3 class="text-lg font-semibold text-gray-900 mb-4">{"Win Rate Trend"}</h3>
                                    <div class="h-64 flex items-end justify-between space-x-2">
                                        {trends.iter().map(|trend| {
                                            // Use a better scaling: minimum 8px for any non-zero value, then scale from there
                                            let height_percent = if trend.win_rate > 0.0 {
                                                // Start with 8px minimum, then scale the remaining 56px (64-8) by win rate
                                                let min_height_px = 8;
                                                let remaining_height_px = 64 - min_height_px;
                                                let scaled_height = (trend.win_rate / 100.0 * remaining_height_px as f64) as i32;
                                                min_height_px + scaled_height
                                            } else { 0 };
                                            html! {
                                                <div class="flex flex-col items-center flex-1">
                                                    <div
                                                        class="w-full bg-blue-500 rounded-t border border-blue-600"
                                                        style={format!("height: {}px", height_percent)}
                                                    ></div>
                                                    <div class="text-xs text-gray-500 mt-2 text-center">
                                                        <div class="font-medium">{format!("{:.0}%", trend.win_rate)}</div>
                                                        <div class="text-xs">{trend.period.clone()}</div>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Html>()}
                                    </div>
                                </div>

                                // Contest Frequency Chart
                                <div class="bg-white border rounded-lg p-6">
                                    <h3 class="text-lg font-semibold text-gray-900 mb-4">{"Contest Activity"}</h3>
                                    <div class="h-64 flex items-end justify-between space-x-2">
                                        {trends.iter().map(|trend| {
                                            let max_contests = trends.iter().map(|t| t.contests_played).max().unwrap_or(1);
                                            // Use a better scaling: minimum 8px for any non-zero value, then scale from there
                                            let height_px = if trend.contests_played > 0 {
                                                // Start with 8px minimum, then scale the remaining 56px (64-8) by contest ratio
                                                let min_height_px = 8;
                                                let remaining_height_px = 64 - min_height_px;
                                                let contest_ratio = trend.contests_played as f64 / max_contests as f64;
                                                let scaled_height = (contest_ratio * remaining_height_px as f64) as i32;
                                                min_height_px + scaled_height
                                            } else { 0 };
                                            html! {
                                                <div class="flex flex-col items-center flex-1">
                                                    <div
                                                        class="w-full bg-green-500 rounded-t border border-green-600"
                                                        style={format!("height: {}px", height_px)}
                                                    ></div>
                                                    <div class="text-xs text-gray-500 mt-2 text-center">
                                                        <div class="font-medium">{trend.contests_played}</div>
                                                        <div class="text-xs">{trend.period.clone()}</div>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Html>()}
                                    </div>
                                </div>

                                // Performance Insights
                                <div class="bg-gray-50 rounded-lg p-6">
                                    <h3 class="text-lg font-semibold text-gray-900 mb-4">{"Performance Insights"}</h3>
                                    <div class="space-y-4">
                                        {(|| {
                                            let mut insights = Vec::new();

                                            // Find best and worst periods
                                            if let (Some(best), Some(worst)) = (
                                                trends.iter().max_by(|a, b| a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal)),
                                                trends.iter().min_by(|a, b| a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal))
                                            ) {
                                                if best.win_rate > worst.win_rate {
                                                    insights.push(format!("🎯 Your best performance was in {} with a {:.1}% win rate", best.period, best.win_rate));
                                                    insights.push(format!("📉 Your lowest performance was in {} with a {:.1}% win rate", worst.period, worst.win_rate));
                                                }
                                            }

                                            // Activity patterns
                                            if let (Some(most_active), Some(least_active)) = (
                                                trends.iter().max_by_key(|t| t.contests_played),
                                                trends.iter().min_by_key(|t| t.contests_played)
                                            ) {
                                                if most_active.contests_played > least_active.contests_played {
                                                    insights.push(format!("🔥 Most active month: {} with {} contests", most_active.period, most_active.contests_played));
                                                    insights.push(format!("😴 Least active month: {} with {} contests", least_active.period, least_active.contests_played));
                                                }
                                            }

                                            // Trend analysis
                                            if trends.len() >= 2 {
                                                let recent_trends: Vec<_> = trends.iter().rev().take(3).collect();
                                                if recent_trends.len() >= 2 {
                                                    let first = recent_trends[0];
                                                    let last = recent_trends[recent_trends.len() - 1];
                                                    let win_rate_change = last.win_rate - first.win_rate;

                                                    if win_rate_change > 5.0 {
                                                        insights.push("📈 You're showing strong improvement in recent months!".to_string());
                                                    } else if win_rate_change < -5.0 {
                                                        insights.push("📉 Consider reviewing your strategy - win rate has declined recently".to_string());
                                                    } else {
                                                        insights.push("➡️ Your performance has been relatively stable recently".to_string());
                                                    }
                                                }
                                            }

                                            insights
                                        })().iter().map(|insight| {
                                            html! {
                                                <div class="flex items-start space-x-3">
                                                    <div class="flex-shrink-0 w-2 h-2 bg-blue-500 rounded-full mt-2"></div>
                                                    <p class="text-sm text-gray-700">{insight}</p>
                                                </div>
                                            }
                                        }).collect::<Html>()}
                                    </div>
                                </div>

                                // Detailed Trends Table
                                <div class="bg-white border rounded-lg p-6">
                                    <h3 class="text-lg font-semibold text-gray-900 mb-4">{"Detailed Monthly Breakdown"}</h3>
                                    <div class="overflow-x-auto">
                                        <table class="min-w-full divide-y divide-gray-200">
                                            <thead class="bg-gray-50">
                                                <tr>
                                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Period"}</th>
                                                    <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Contests"}</th>
                                                    <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Wins"}</th>
                                                    <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Win Rate"}</th>
                                                    <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Avg Placement"}</th>
                                                    <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Rating"}</th>
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white divide-y divide-gray-200">
                                                {trends.iter().rev().map(|trend| {
                                                    html! {
                                                        <tr class="hover:bg-gray-50">
                                                            <td class="px-3 py-2 text-sm font-medium text-gray-900">{trend.period.clone()}</td>
                                                            <td class="px-3 py-2 text-sm text-center text-gray-700">{trend.contests_played}</td>
                                                            <td class="px-3 py-2 text-sm text-center text-gray-700">{trend.wins}</td>
                                                            <td class="px-3 py-2 text-sm text-center font-medium text-gray-700">
                                                                {format!("{:.1}%", trend.win_rate)}
                                                            </td>
                                                            <td class="px-3 py-2 text-sm text-center text-gray-700">
                                                                {format!("{:.1}", trend.average_placement)}
                                                            </td>
                                                            <td class="px-3 py-2 text-sm text-center text-gray-700">
                                                                {if let Some(rating) = props.current_rating {
                                                                    format!("{:.0}", rating)
                                                                } else {
                                                                    format!("{:.0}", trend.skill_rating)
                                                                }}
                                                            </td>
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
                    html! {
                        <div class="text-center py-8 text-gray-500">
                            <p>{"Loading trends data..."}</p>
                        </div>
                    }
                }}
            </div>
        </div>
    }
}
