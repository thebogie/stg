use yew::prelude::*;
use crate::components::chart_renderer::ChartRenderer;
use crate::auth::AuthContext;
use serde_json::Value;

#[derive(Properties, PartialEq)]
pub struct ComparisonTabProps {
    pub glicko_ratings: Option<Vec<Value>>,
}

#[function_component(ComparisonTab)]
pub fn comparison_tab(props: &ComparisonTabProps) -> Html {
    let auth_context = use_context::<AuthContext>().expect("AuthContext not found");
    
    // Build player IDs for comparison
    let me_id = auth_context.state.player.as_ref()
        .map(|p| p.id.clone())
        .unwrap_or_default();
    let mut ids: Vec<String> = Vec::new();
    if !me_id.is_empty() { 
        ids.push(me_id.clone()); 
    }
    
    // Add other players from Glicko2 ratings for comparison
    if let Some(ratings) = &props.glicko_ratings {
        for rating in ratings.iter().filter_map(|r| r.get("player_id").and_then(|v| v.as_str())) {
            let pid_str = rating.to_string();
            // Only add if it's not already in the list and not the same as me
            if !ids.contains(&pid_str) && pid_str != me_id {
                ids.push(pid_str);
                if ids.len() >= 6 { break; } // Limit to 6 players total
            }
        }
    }
    
    let chart_url = if ids.len() >= 2 {
        Some(format!(
            "/api/analytics/charts/player-comparison?player_ids={}&title=Player%20Performance%20Comparison",
            ids.join(",")
        ))
    } else { 
        None 
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
                
                <div class="mb-4 p-3 bg-blue-50 rounded text-sm">
                    <p class="font-bold text-blue-900">{"Chart Status:"}</p>
                    <p class="text-blue-800">{"API endpoint is working, but players need more contest data for comparison."}</p>
                    <p class="text-blue-800">{"Try playing more contests to generate comparison metrics."}</p>
                </div>
                
                {
                    if let Some(url) = chart_url {
                        html! { 
                            <ChartRenderer 
                                chart_data={url} 
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
