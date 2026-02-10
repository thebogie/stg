use crate::api::utils::authenticated_get;
use crate::auth::AuthContext;
use crate::components::chart_renderer::ChartRenderer;
use js_sys::Date;
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::use_effect_with;

#[function_component(ComparisonTab)]
pub fn comparison_tab() -> Html {
    let auth_context = use_context::<AuthContext>().expect("AuthContext not found");
    let chart_data = use_state(|| None::<String>);
    let chart_loading = use_state(|| false);
    let chart_error = use_state(|| None::<String>);
    let last_updated = use_state(|| None::<String>);
    let leaderboard = use_state(|| None::<Vec<Value>>);
    let leaderboard_loading = use_state(|| false);
    let leaderboard_error = use_state(|| None::<String>);

    {
        let leaderboard = leaderboard.clone();
        let leaderboard_loading = leaderboard_loading.clone();
        let leaderboard_error = leaderboard_error.clone();

        use_effect_with((), move |_| {
            leaderboard_loading.set(true);
            leaderboard_error.set(None);

            spawn_local(async move {
                match authenticated_get(
                    "/api/ratings/leaderboard?scope=global&min_games=3&limit=10",
                )
                .send()
                .await
                {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Vec<Value>>().await {
                                Ok(rows) => leaderboard.set(Some(rows)),
                                Err(e) => leaderboard_error
                                    .set(Some(format!("Failed to parse leaderboard: {}", e))),
                            }
                        } else {
                            leaderboard_error.set(Some(format!(
                                "Leaderboard request failed: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => {
                        leaderboard_error.set(Some(format!("Failed to fetch leaderboard: {}", e)))
                    }
                }

                leaderboard_loading.set(false);
            });

            || ()
        });
    }

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
    if let Some(leaderboard) = &*leaderboard {
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
        let last_updated = last_updated.clone();
        let ids = ids.clone();

        use_effect_with(ids, move |ids| -> Box<dyn FnOnce()> {
            let ids = ids.clone();

            if ids.len() < 2 {
                chart_data.set(None);
                chart_error.set(None);
                chart_loading.set(false);
                last_updated.set(None);
                return Box::new(|| ());
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
                                Ok(chart_json) => {
                                    chart_data.set(Some(chart_json));
                                    last_updated.set(Some(
                                        Date::new_0()
                                            .to_iso_string()
                                            .as_string()
                                            .unwrap_or_default(),
                                    ));
                                }
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
                            last_updated.set(None);
                        }
                    }
                    Err(e) => {
                        chart_data.set(None);
                        chart_error.set(Some(format!("Failed to fetch comparison chart: {}", e)));
                        last_updated.set(None);
                    }
                }

                chart_loading.set(false);
            });

            Box::new(|| ())
        });
    }

    let status = if let Some(error) = chart_error.as_ref().or_else(|| leaderboard_error.as_ref()) {
        html! {
            <div class="mb-4 p-3 bg-red-50 rounded text-sm text-red-800">
                {error}
            </div>
        }
    } else if *leaderboard_loading || *chart_loading {
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
        let updated_text = if let Some(updated_at) = last_updated.as_ref() {
            format!("Last updated: {}", updated_at)
        } else {
            "Last updated: just now".to_string()
        };
        html! {
            <div class="mb-4 p-3 bg-blue-50 rounded text-sm text-blue-800">
                <div>{format!("Comparing you to {} top players from the global leaderboard.", compare_count)}</div>
                <div class="text-xs text-blue-700 mt-1">{updated_text}</div>
            </div>
        }
    };

    html! {
        <div class="space-y-6">
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="flex items-center justify-between mb-4">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-900">{"Player Comparison"}</h2>
                        <p class="mt-1 text-gray-600">
                            {"Compare your key metrics to top peers from the leaderboard."}
                        </p>
                    </div>
                    <div class="text-4xl">{"🧭"}</div>
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
