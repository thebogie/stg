use yew::prelude::*;
use serde_json::Value;

#[derive(Properties, PartialEq, Clone)]
pub struct RatingsTabProps {
    pub glicko_loading: bool,
    pub glicko_error: Option<String>,
    pub glicko_ratings: Option<Vec<Value>>,
}

#[function_component(RatingsTab)]
pub fn ratings_tab(props: &RatingsTabProps) -> Html {
    html! {
        <div class="px-4 py-4">
            <h2 class="text-2xl font-bold text-gray-900 mb-4">{"Glicko2 Ratings"}</h2>
            <div class="mb-4">
                <p class="text-gray-600">
                    <strong>{"Skill Rating System:"}</strong> {"Your Glicko2 rating represents your skill level. Higher ratings indicate stronger players. Ratings change based on your performance in contests."}
                </p>
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
                                            <h4 class="text-sm font-medium text-gray-500 uppercase tracking-wide">{"Games Played"}</h4>
                                            <p class="text-2xl font-semibold text-gray-900">{games_played}</p>
                                            <p class="text-xs text-gray-600 mt-1">{"Contest experience"}</p>
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
                                    <div class="h-64 bg-gray-100 rounded-lg flex items-center justify-center">
                                        <p class="text-gray-500">{"Rating history chart coming soon..."}</p>
                                    </div>
                                </div>
                            </div>
                        }
                    } else {
                        html! { <p class="text-gray-600 text-center py-8">{"No rating data available"}</p> }
                    }
                } }
            }
        </div>
    }
}
