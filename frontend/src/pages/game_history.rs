use crate::api::utils::authenticated_get;
use crate::auth::AuthContext;
use crate::Route;
use serde_json::Value;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct GameHistoryProps {
    pub game_id: String,
}

#[function_component(GameHistory)]
pub fn game_history(props: &GameHistoryProps) -> Html {
    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let navigator = use_navigator().unwrap();

    let contests = use_state(|| None::<Vec<Value>>);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);

    // Load contests for this game for the current player
    {
        let contests = contests.clone();
        let loading = loading.clone();
        let error = error.clone();
        let game_id = props.game_id.clone();
        let player_id = auth
            .state
            .player
            .as_ref()
            .map(|p| p.id.clone())
            .unwrap_or_default();

        use_effect_with((game_id.clone(), player_id.clone()), move |_| {
            let contests = contests.clone();
            let loading = loading.clone();
            let error = error.clone();
            let game_id = game_id.clone();
            let player_id = player_id.clone();

            wasm_bindgen_futures::spawn_local(async move {
                loading.set(true);
                error.set(None);

                if player_id.is_empty() {
                    error.set(Some("Player not authenticated".to_string()));
                    loading.set(false);
                    return;
                }

                // Clone player_id and game_id early to avoid moves
                let player_id_str = player_id.as_str();
                let game_id_str = game_id.as_str();

                // Extract just the key part for player_id and game_id
                let pid = if player_id_str.starts_with("player/") {
                    player_id
                        .split('/')
                        .nth(1)
                        .unwrap_or(&player_id)
                        .to_string()
                } else {
                    player_id.clone()
                };
                let gid = if game_id_str.starts_with("game/") {
                    game_id.split('/').nth(1).unwrap_or(&game_id).to_string()
                } else {
                    game_id.clone()
                };
                log::info!(
                    "Game history: player_id={}, game_id={}, key={}",
                    player_id,
                    game_id,
                    gid
                );
                let url = format!("/api/contests/player/{}/game/{}", pid, gid);
                log::info!("Game history API call: {}", url);
                match authenticated_get(&url).send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Value>().await {
                                Ok(data) => {
                                    log::info!("Game history API response: {:?}", data);
                                    if let Some(arr) = data.as_array() {
                                        log::info!("Found {} contests in response", arr.len());
                                        contests.set(Some(arr.clone()));
                                    } else {
                                        log::info!(
                                            "Response is not an array, setting empty contests"
                                        );
                                        contests.set(Some(vec![]));
                                    }
                                }
                                Err(e) => {
                                    error.set(Some(format!("Failed to parse contests: {}", e)))
                                }
                            }
                        } else {
                            error.set(Some(format!(
                                "Failed to fetch contests: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => error.set(Some(format!("Failed to fetch contests: {}", e))),
                }

                loading.set(false);
            });
            || ()
        });
    }

    let on_back = {
        let navigator = navigator.clone();
        Callback::from(move |_| navigator.push(&Route::Profile))
    };

    // Extract game name from first contest if available
    let game_name = if let Some(cs) = &*contests {
        cs.first()
            .and_then(|c| c.get("game_name").and_then(|v| v.as_str()))
            .unwrap_or("Game")
    } else {
        "Game"
    };

    html! {
        <div class="min-h-screen bg-gray-50">
            <header class="bg-white shadow-sm border-b border-gray-200">
                <div class="max-w-6xl mx-auto px-4 py-3 flex items-center space-x-3">
                    <button onclick={on_back} class="px-3 py-1.5 bg-blue-600 text-white rounded-md hover:bg-blue-700">{"Back"}</button>
                    <h1 class="text-xl font-semibold text-gray-900">{format!("Game History: {}", game_name)}</h1>
                </div>
            </header>

            <main class="max-w-6xl mx-auto px-4 py-4">
                if *loading {
                    <div class="text-center text-gray-600 py-10">{"Loading game contests..."}</div>
                } else if let Some(err) = &*error {
                    <div class="text-center text-red-600 py-10">{err}</div>
                } else if let Some(cs) = &*contests {
                    if cs.is_empty() {
                        <div class="text-center text-gray-600 py-10">{"No contests found for this game."}</div>
                    } else {
                        <div class="mb-4 p-3 bg-blue-50 border border-blue-200 rounded-md">
                            <p class="text-sm text-blue-800">{"💡 Click on any contest row to view detailed information about that contest."}</p>
                        </div>
                        <div class="space-y-2">
                            {cs.iter().map(|contest| {
                                let contest_id = contest.get("contest_id").and_then(|v| v.as_str()).unwrap_or("");
                                let contest_key = if contest_id.contains('/') { contest_id.split('/').nth(1).unwrap_or(contest_id) } else { contest_id };
                                let contest_name = contest.get("contest_name").and_then(|v| v.as_str()).unwrap_or("Contest");
                                let date_raw = contest.get("contest_date").and_then(|v| v.as_str()).unwrap_or("");
                                let date = if !date_raw.is_empty() {
                                    // Try to parse and format the date for human reading
                                    if let Ok(parsed_date) = chrono::DateTime::parse_from_rfc3339(date_raw) {
                                        parsed_date.format("%B %d, %Y at %I:%M %p").to_string()
                                    } else {
                                        date_raw.to_string()
                                    }
                                } else {
                                    "Unknown Date".to_string()
                                };
                                let venue_name = contest.get("venue_name").and_then(|v| v.as_str()).unwrap_or("Unknown Venue");
                                let venue_display_name = contest.get("venue_display_name").and_then(|v| v.as_str()).unwrap_or(venue_name);
                                let my_place = contest.get("my_placement").and_then(|v| v.as_i64()).unwrap_or(0);
                                let my_result = contest.get("my_result").and_then(|v| v.as_str()).unwrap_or("");
                                let total_players = contest.get("total_players").and_then(|v| v.as_i64()).unwrap_or(0);

                                html! {
                                    <div class="p-4 bg-white rounded-md border border-gray-200 hover:shadow cursor-pointer"
                                        onclick={let key = contest_key.to_string(); let nav = navigator.clone(); yew::Callback::from(move |_| nav.push(&Route::ContestDetails { contest_id: key.clone() }))}>
                                        <div class="flex items-center justify-between mb-2">
                                            <div class="text-lg font-semibold text-gray-900">{date}</div>
                                            <div class="text-sm text-gray-600">{format!("{} players", total_players)}</div>
                                        </div>
                                        <div class="flex items-center justify-between">
                                            <div class="text-sm text-gray-800">
                                                <span class="font-medium">{contest_name}</span>
                                                <span class="text-gray-500 mx-2">{"•"}</span>
                                                <span>{venue_display_name}</span>
                                            </div>
                                            <div class="text-sm text-gray-600">
                                                {format!("Your place: {}", if my_place > 0 { format!("#{}", my_place) } else { "N/A".to_string() })}
                                                {if !my_result.is_empty() { format!(" ({})", my_result) } else { "".to_string() }}
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()}
                        </div>
                    }
                }
            </main>
        </div>
    }
}
