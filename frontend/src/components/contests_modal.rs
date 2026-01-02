use yew::prelude::*;
use serde_json::Value;
use yew_router::prelude::*;
use crate::Route;

fn format_friendly_date(date_str: &str) -> String {
    if date_str.is_empty() {
        return "Unknown".to_string();
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return dt.format("%b %e, %Y").to_string();
    }
    // Fallback: try plain yyyy-mm-dd
    if date_str.len() >= 10 {
        return date_str[0..10].to_string();
    }
    date_str.to_string()
}

#[derive(Properties, PartialEq, Clone)]
pub struct ContestsModalProps {
    pub is_open: bool,
    pub on_close: Callback<()>,
    pub title: String,
    pub subtitle: Option<String>,
    pub contests: Option<Vec<Value>>,
    pub loading: bool,
    pub error: Option<String>,
    pub show_bgg_link: Option<String>, // Optional BGG game ID
}

#[function_component(ContestsModal)]
pub fn contests_modal(props: &ContestsModalProps) -> Html {
    let navigator = use_navigator().unwrap();

    if !props.is_open {
        return html! {};
    }

    let on_contest_click = {
        let navigator = navigator.clone();
        let on_close = props.on_close.clone();
        Callback::from(move |contest_key: String| {
            on_close.emit(());
            navigator.push(&Route::ContestDetails { contest_id: contest_key });
        })
    };

    html! {
        <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
            <div class="relative top-20 mx-auto p-5 border w-11/12 md:w-3/4 lg:w-1/2 shadow-lg rounded-md bg-white">
                <div class="mt-3">
                    <div class="flex items-center justify-between mb-4">
                        <div>
                            <h3 class="text-lg font-medium text-gray-900">
                                {props.title.clone()}
                            </h3>
                            {if let Some(subtitle) = &props.subtitle {
                                html! { <p class="text-sm text-gray-600 mt-1">{subtitle.clone()}</p> }
                            } else { html! {} }}
                            {if let Some(bgg_id) = &props.show_bgg_link {
                                html! {
                                    <div class="mt-2">
                                        <a 
                                            href={format!("https://boardgamegeek.com/boardgame/{}", bgg_id)}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            class="inline-flex items-center text-sm text-blue-600 hover:text-blue-800 hover:underline"
                                        >
                                            <span class="mr-1">{"🔗"}</span>
                                            {"View on BoardGameGeek"}
                                            <span class="ml-1">{"↗"}</span>
                                        </a>
                                    </div>
                                }
                            } else { html! {} }}
                        </div>
                        <button
                            class="text-gray-400 hover:text-gray-600"
                            onclick={let on_close = props.on_close.clone(); yew::Callback::from(move |_| on_close.emit(()))}
                        >
                            <span class="sr-only">{"Close"}</span>
                            <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                    
                    {if props.loading {
                        html! {
                            <div class="text-center py-8">
                                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto"></div>
                                <p class="mt-2 text-gray-600">{"Loading contests..."}</p>
                            </div>
                        }
                    } else if let Some(ref error) = props.error {
                        html! {
                            <div class="text-center py-8 text-red-600">
                                <p>{"Error: "}{error}</p>
                            </div>
                        }
                    } else if let Some(ref contests) = props.contests {
                        if contests.is_empty() {
                            html! {
                                <div class="text-center py-8 text-gray-500">
                                    <p>{"No contests found."}</p>
                                </div>
                            }
                        } else {
                            html! {
                                <div class="space-y-4">
                                    <div class="flex items-center justify-between">
                                        <p class="text-sm text-gray-600">
                                            {"Click on any contest to view full details"}
                                        </p>
                                        <span class="text-sm text-gray-500">
                                            {format!("{} total", contests.len())}
                                        </span>
                                    </div>
                                    <div class="max-h-96 overflow-y-auto">
                                        <div class="space-y-2">
                                            {contests.iter().map(|contest| {
                                                let contest_date = contest.get("contest_date").and_then(|v| v.as_str()).unwrap_or("");
                                                let formatted_date = format_friendly_date(contest_date);
                                                let contest_name = contest.get("contest_name").and_then(|v| v.as_str()).unwrap_or("Contest");
                                                
                                                let game_id_full = contest.get("game_id").and_then(|v| v.as_str()).unwrap_or("");
                                                let game_id = if game_id_full.contains('/') { game_id_full.split('/').nth(1).unwrap_or(game_id_full) } else { game_id_full };
                                                let game_name = contest.get("game_name").and_then(|v| v.as_str()).unwrap_or("Unknown Game");
                                                let venue_id_full = contest.get("venue_id").and_then(|v| v.as_str()).unwrap_or("");
                                                let venue_id = if venue_id_full.contains('/') { venue_id_full.split('/').nth(1).unwrap_or(venue_id_full) } else { venue_id_full };
                                                
                                                // Debug logging
                                                if !game_id.is_empty() {
                                                    log::info!("Game ID: full={}, extracted={}", game_id_full, game_id);
                                                }
                                                if !venue_id.is_empty() {
                                                    log::info!("Venue ID: full={}, extracted={}", venue_id_full, venue_id);
                                                }
                                                let venue_name = contest.get("venue_name").and_then(|v| v.as_str()).unwrap_or("Unknown Venue");
                                                let contest_id = contest.get("contest_id").and_then(|v| v.as_str()).unwrap_or("");
                                                let contest_key = if contest_id.contains('/') { contest_id.split('/').nth(1).unwrap_or(contest_id) } else { contest_id };
                                                let my_place = contest.get("my_placement").and_then(|v| v.as_i64()).unwrap_or(0);
                                                let opponent_place = contest.get("opponent_placement").and_then(|v| v.as_i64()).unwrap_or(0);
                                                let i_won = contest.get("i_won").and_then(|b| b.as_bool()).unwrap_or(false);
                                                
                                                let result = if i_won { ("Won", "text-green-600", "🏆") } else if my_place > opponent_place { ("Lost", "text-red-600", "💔") } else { ("Tied", "text-yellow-600", "🤝") };
                                                
                                                html! {
                                                    <div 
                                                        class="p-3 bg-gray-50 rounded-lg hover:bg-gray-100 cursor-pointer border border-gray-200"
                                                        onclick={let contest_key = contest_key.to_string(); let on_contest_click = on_contest_click.clone(); yew::Callback::from(move |_| on_contest_click.emit(contest_key.clone()))}
                                                    >
                                                        <div class="flex items-center justify-between">
                                                            <div class="flex-1">
                                                                <div class="flex items-center space-x-3">
                                                                    <span class="text-sm font-medium text-gray-900">{contest_name}</span>
                                                                    <span class="text-sm text-gray-600">{"•"}</span>
                                                                    <a href={format!("/game/{}/history", game_id)}
                                                                       onclick={let nav = navigator.clone(); let g = game_id.to_string(); yew::Callback::from(move |e: yew::MouseEvent| { 
                                                                           log::info!("Navigating to game history with game_id: {}", g);
                                                                           e.prevent_default(); 
                                                                           e.stop_propagation(); 
                                                                           nav.push(&Route::GameHistory { game_id: g.clone() }); 
                                                                       })}
                                                                       class="text-sm font-medium text-blue-600 hover:underline">{game_name}</a>
                                                                    <span class="text-sm text-gray-600">{"•"}</span>
                                                                    <a href={format!("/venue/{}/history", venue_id)}
                                                                       onclick={let nav = navigator.clone(); let v = venue_id.to_string(); yew::Callback::from(move |e: yew::MouseEvent| { 
                                                                           log::info!("Navigating to venue history with venue_id: {}", v);
                                                                           e.prevent_default(); 
                                                                           e.stop_propagation(); 
                                                                           nav.push(&Route::VenueHistory { venue_id: v.clone() }); 
                                                                       })}
                                                                       class="text-sm font-medium text-gray-900 hover:underline">{venue_name}</a>
                                                                    <span class="text-sm text-gray-600">{"•"}</span>
                                                                    <span class="text-sm text-gray-600">{formatted_date}</span>
                                                                </div>
                                                                <div class="flex items-center space-x-3 mt-1">
                                                                    <span class="text-sm text-gray-600">{"Your Place:"}</span>
                                                                    <span class="text-sm font-medium text-gray-900">
                                                                        {if my_place > 0 { format!("#{}", my_place) } else { "N/A".to_string() }}
                                                                    </span>
                                                                    <span class="text-sm text-gray-600">{"Opponent Place:"}</span>
                                                                    <span class="text-sm font-medium text-gray-900">
                                                                        {if opponent_place > 0 { format!("#{}", opponent_place) } else { "N/A".to_string() }}
                                                                    </span>
                                                                </div>
                                                            </div>
                                                            <div class="flex items-center space-x-2">
                                                                <span class={classes!("text-sm", "font-medium", result.1)}>
                                                                    <span class="mr-1">{result.2}</span>
                                                                    {result.0}
                                                                </span>
                                                                <span class="text-blue-600 text-sm">{"→"}</span>
                                                            </div>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Html>()}
                                        </div>
                                    </div>
                                </div>
                            }
                        }
                    } else {
                        html! {}
                    }}
                </div>
            </div>
        </div>
    }
}
