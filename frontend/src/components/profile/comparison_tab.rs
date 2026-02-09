use crate::api::utils::authenticated_get;
use crate::auth::AuthContext;
use crate::components::chart_renderer::ChartRenderer;
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

#[derive(Properties, PartialEq)]
pub struct ComparisonTabProps {
    pub leaderboard: Option<Vec<Value>>,
    pub leaderboard_loading: bool,
    pub leaderboard_error: Option<String>,
}

#[function_component(ComparisonTab)]
pub fn comparison_tab(props: &ComparisonTabProps) -> Html {
    let auth_context = use_context::<AuthContext>().expect("AuthContext not found");
    let chart_data = use_state(|| None::<String>);
    let chart_loading = use_state(|| false);
    let chart_error = use_state(|| None::<String>);

    // Build player IDs for comparison
    let me_id = auth_context
        .state
        .player
        .as_ref()
        .map(|p| p.id.clone())
        .unwrap_or_default();
    let mut ids: Vec<String> = Vec::new();
    if !me_id.is_empty() {
        ids.push(me_id.clone());
    }

    // Add other players from leaderboard for comparison
    if let Some(leaderboard) = &props.leaderboard {
        for rating in leaderboard
            .iter()
            .filter_map(|r| r.get("player_id").and_then(|v| v.as_str()))
        {
            let pid_str = rating.to_string();
            // Only add if it's not already in the list and not the same as me
            if !ids.contains(&pid_str) && pid_str != me_id {
                ids.push(pid_str);
                if ids.len() >= 6 {
                    break;
                } // Limit to 6 players total
            }
        }
    }

    {
        let chart_data = chart_data.clone();
        let chart_loading = chart_loading.clone();
        let chart_error = chart_error.clone();
        let ids = ids.clone();

        use_effect_with(ids, move |ids| {
            let ids = ids.clone();

            if ids.len() < 2 {
                chart_data.set(None);
                chart_error.set(None);
                chart_loading.set(false);
                return || ();
            }

            chart_loading.set(true);
            chart_error.set(None);

            spawn_local(async move {
                let url = format!(
                    "/api/analytics/charts/player-comparison?player_ids={}&title=Player%20Performance%20Comparison",
                    ids.join(",")
                );

                match authenticated_get(&url).send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.text().await {
                                Ok(chart_json) => chart_data.set(Some(chart_json)),
                                Err(e) => {
                                    chart_data.set(None);
                                    chart_error
                                        .set(Some(format!("Failed to parse chart data: {}", e)));
                                }
                            }
                        } else {
                            chart_data.set(None);
                            chart_error.set(Some(format!(
                                "Comparison chart request failed: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => {
                        chart_data.set(None);
                        chart_error.set(Some(format!(
                            "Failed to fetch comparison chart: {}",
                            e
                        )));
                    }
                }

                chart_loading.set(false);
            });

            || ()
        });
    }

    let status = if let Some(error) = chart_error
        .as_ref()
        .or_else(|| props.leaderboard_error.as_ref())
    {
        html! {
            <div class="mb-4 p-3 bg-red-50 rounded text-sm text-red-800">
                {error}
            </div>
        }
    } else if props.leaderboard_loading || *chart_loading {
        html! {
            <div class="mb-4 p-3 bg-blue-50 rounded text-sm text-blue-800">
                {"Loading comparison data..."}
            </div>
        }
    } else if ids.len() < 2 {
        html! {
            <div class="mb-4 p-3 bg-gray-50 rounded text-sm text-gray-600">
                {"Not enough player data to build a comparison yet."}
            </div>
        }
    } else {
        let compare_count = ids.len().saturating_sub(1);
        html! {
            <div class="mb-4 p-3 bg-blue-50 rounded text-sm text-blue-800">
                {format!("Comparing you to {} top players from the global leaderboard.", compare_count)}
            </div>
        }
    };

    html! {
        <div class="space-y-6">
            <div class="bg-white shadow rounded-lg p-6">
                <h2 class="text-2xl font-bold text-gray-900 mb-4">{"🧭 Player Comparison"}</h2>
                <div class="mb-4">
                    <p class="text-gray-600">
                        <strong>{"Compare Your Performance:"}</strong> {"Radar chart compares your key metrics to top peers from the leaderboard."}
                    </p>
                </div>

                {status}

                {
                    if let Some(chart_json) = (*chart_data).clone() {
                        html! {
                            <ChartRenderer
                                chart_data={chart_json}
                                chart_id={"profile-comparison-chart".to_string()}
                                width={Some(800)}
                                height={Some(400)}
                            />
                        }
                    } else {
                        html! {
                            <div class="h-64 bg-gray-50 rounded-lg flex items-center justify-center text-gray-500">
                                {"Insufficient data to build comparison."}
                            </div>
                        }
                    }
                }
            </div>
        </div>
    }
}
