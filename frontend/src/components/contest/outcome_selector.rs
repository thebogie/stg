use crate::api::players;
use crate::components::common::modal::Modal;
use regex;
use shared::dto::contest::OutcomeDto;
use shared::dto::player::PlayerDto;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct OutcomeSelectorProps {
    pub on_outcomes_change: Callback<Vec<OutcomeDto>>,
}

#[derive(Clone, PartialEq, Debug)]
struct PlayerSearchResult {
    player: PlayerDto,
    exists: bool,
}

#[derive(Clone, PartialEq)]
struct PlayerOutcome {
    player_id: String,
    handle: String,
    email: String,
    place: String,
    result: String,
}

#[function_component(OutcomeSelector)]
pub fn outcome_selector(props: &OutcomeSelectorProps) -> Html {
    let props = props.clone();
    let outcomes = use_state(Vec::<PlayerOutcome>::new);
    let search_query = use_state(String::new);
    let search_results = use_state(Vec::<PlayerSearchResult>::new);
    let is_searching = use_state(|| false);
    let show_suggestions = use_state(|| false);
    let show_modal = use_state(|| false);
    let modal_message = use_state(String::new);
    let show_new_player_confirm = use_state(|| false);
    let pending_new_player = use_state(|| None::<PlayerDto>);
    let search_error = use_state(|| None::<String>);

    // Email validation pattern
    let email_pattern =
        regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    let is_valid_email = email_pattern.is_match(&*search_query);
    let is_valid_handle = !search_query.is_empty() && search_query.len() >= 3;
    let show_invalid_email = !search_query.is_empty() && !is_valid_email && !is_valid_handle;

    // Function to generate handle from email
    let generate_handle_from_email = |email: &str| -> String {
        // Extract the part before @ and clean it up
        let before_at = email.split('@').next().unwrap_or("user");
        before_at
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase()
    };

    // Search for players
    let on_search_change = {
        let search_query = search_query.clone();
        let search_results = search_results.clone();
        let is_searching = is_searching.clone();
        let show_suggestions = show_suggestions.clone();
        let search_error = search_error.clone();

        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let query = input.value();
            search_query.set(query.clone());
            show_suggestions.set(true);
            search_error.set(None); // Clear error on new input

            if query.is_empty() {
                search_results.set(Vec::new());
                return;
            }

            is_searching.set(true);
            let search_results = search_results.clone();
            let is_searching = is_searching.clone();
            let search_error = search_error.clone();

            spawn_local(async move {
                match players::search_players(&query).await {
                    Ok(players) => {
                        gloo::console::log!(format!(
                            "DEBUG: search_players response for query '{}': {:?}",
                            query, players
                        ));
                        let lc_query = query.to_lowercase();
                        let results: Vec<PlayerSearchResult> = players
                            .into_iter()
                            .map(|player| {
                                // Check if this result matches the query (by handle or email)
                                let exists = player.handle.to_lowercase() == lc_query
                                    || player.email.to_lowercase() == lc_query;
                                PlayerSearchResult { player, exists }
                            })
                            .collect();
                        gloo::console::log!(format!(
                            "DEBUG: computed search_results for query '{}': {:?}",
                            query, results
                        ));
                        search_results.set(results);
                        search_error.set(None);
                    }
                    Err(e) => {
                        search_results.set(Vec::new());
                        search_error.set(Some(format!("Failed to search players: {}", e)));
                    }
                }
                is_searching.set(false);
            });
        })
    };

    let reset_place_numbers = {
        let outcomes = outcomes.clone();
        let props = props.clone();

        Callback::from(move |mut current_outcomes: Vec<PlayerOutcome>| {
            // Reset place numbers based on row order
            for (index, outcome) in current_outcomes.iter_mut().enumerate() {
                outcome.place = (index + 1).to_string();
                // Set first row to "won", others to "lost" as default
                outcome.result = if index == 0 {
                    "won".to_string()
                } else {
                    "lost".to_string()
                };
            }
            outcomes.set(current_outcomes.clone());
            // Convert to OutcomeDto for the parent component
            let outcome_dtos: Vec<OutcomeDto> = current_outcomes
                .iter()
                .map(|o| OutcomeDto {
                    player_id: o.player_id.clone(),
                    place: o.place.clone(),
                    result: o.result.clone(),
                    email: o.email.clone(),
                    handle: o.handle.clone(),
                })
                .collect();
            props.on_outcomes_change.emit(outcome_dtos);
        })
    };

    let on_player_select = {
        let _props = props.clone();
        let outcomes = outcomes.clone();
        let show_suggestions = show_suggestions.clone();
        let search_query = search_query.clone();
        let reset_place_numbers = reset_place_numbers.clone();
        let show_modal = show_modal.clone();
        let modal_message = modal_message.clone();
        let show_new_player_confirm = show_new_player_confirm.clone();
        let pending_new_player = pending_new_player.clone();

        Callback::from(move |player: PlayerDto| {
            let current_outcomes = (*outcomes).clone();

            // Check if player is already added (by ID or email)
            if current_outcomes.iter().any(|o| {
                o.player_id == player.id || o.email.to_lowercase() == player.email.to_lowercase()
            }) {
                modal_message.set(format!(
                    "Player with email {} is already in the contest",
                    player.email
                ));
                show_modal.set(true);
                return;
            }

            // If this is a new player (not in database), show confirmation
            if player.id.starts_with("player/") && player.id.contains("Uuid") {
                pending_new_player.set(Some(player));
                show_new_player_confirm.set(true);
                search_query.set(String::new());
                show_suggestions.set(false);
                return;
            }

            // Existing player - add directly
            let outcome = PlayerOutcome {
                player_id: player.id.clone(),
                handle: player.handle.clone(),
                email: player.email.clone(),
                place: "1".to_string(), // Will be reset by reset_place_numbers
                result: "won".to_string(), // Will be reset by reset_place_numbers
            };

            let mut new_outcomes = current_outcomes;
            new_outcomes.push(outcome);
            reset_place_numbers.emit(new_outcomes);

            search_query.set(String::new());
            show_suggestions.set(false);
        })
    };

    let on_confirm_new_player = {
        let _props = props.clone();
        let outcomes = outcomes.clone();
        let reset_place_numbers = reset_place_numbers.clone();
        let show_new_player_confirm = show_new_player_confirm.clone();
        let pending_new_player = pending_new_player.clone();

        Callback::from(move |_: ()| {
            if let Some(player) = (*pending_new_player).clone() {
                let current_outcomes = (*outcomes).clone();
                let outcome = PlayerOutcome {
                    player_id: String::new(), // Set to empty for new player
                    handle: generate_handle_from_email(&player.email),
                    email: player.email.clone(),
                    place: "1".to_string(), // Will be reset by reset_place_numbers
                    result: "won".to_string(), // Will be reset by reset_place_numbers
                };

                let mut new_outcomes = current_outcomes;
                new_outcomes.push(outcome);
                reset_place_numbers.emit(new_outcomes);
            }

            show_new_player_confirm.set(false);
            pending_new_player.set(None);
        })
    };

    let _on_cancel_new_player = {
        let show_new_player_confirm = show_new_player_confirm.clone();
        let pending_new_player = pending_new_player.clone();

        Callback::from(move |_: ()| {
            show_new_player_confirm.set(false);
            pending_new_player.set(None);
        })
    };

    let on_add_player = {
        let _props = props.clone();
        let outcomes = outcomes.clone();
        let search_query = search_query.clone();
        let _reset_place_numbers = reset_place_numbers.clone();
        let show_modal = show_modal.clone();
        let modal_message = modal_message.clone();
        let show_new_player_confirm = show_new_player_confirm.clone();
        let pending_new_player = pending_new_player.clone();

        Callback::from(move |_| {
            let query = (*search_query).clone();
            if !query.is_empty() && is_valid_email {
                let current_outcomes = (*outcomes).clone();

                // Check if a player with this email already exists in the contest
                if current_outcomes
                    .iter()
                    .any(|o| o.email.to_lowercase() == query.to_lowercase())
                {
                    modal_message.set(format!(
                        "A player with email '{}' is already in the contest",
                        query
                    ));
                    show_modal.set(true);
                    return;
                }

                // Generate handle from email
                let generated_handle = generate_handle_from_email(&query);

                let new_player = PlayerDto {
                    id: format!("player/{}", uuid::Uuid::new_v4()),
                    firstname: "New".to_string(),
                    handle: generated_handle,
                    email: query.clone(),
                    created_at: chrono::Utc::now().fixed_offset(),
                    is_admin: false,
                };

                // Show confirmation for new player
                pending_new_player.set(Some(new_player));
                show_new_player_confirm.set(true);
                search_query.set(String::new());
            }
        })
    };

    let on_remove_outcome = {
        let _props = props.clone();
        let outcomes = outcomes.clone();
        let reset_place_numbers = reset_place_numbers.clone();

        Callback::from(move |player_id: String| {
            let mut new_outcomes = (*outcomes).clone();
            new_outcomes.retain(|o| o.player_id != player_id);
            reset_place_numbers.emit(new_outcomes);
        })
    };

    let on_place_change = {
        let props = props.clone();
        let outcomes = outcomes.clone();

        Callback::from(move |(player_id, place): (String, String)| {
            let mut new_outcomes = (*outcomes).clone();
            if let Some(outcome) = new_outcomes.iter_mut().find(|o| o.player_id == player_id) {
                outcome.place = place;
                outcomes.set(new_outcomes.clone());
                // Convert to OutcomeDto for the parent component
                let outcome_dtos: Vec<OutcomeDto> = new_outcomes
                    .iter()
                    .map(|o| OutcomeDto {
                        player_id: o.player_id.clone(),
                        place: o.place.clone(),
                        result: o.result.clone(),
                        email: o.email.clone(),
                        handle: o.handle.clone(),
                    })
                    .collect();
                props.on_outcomes_change.emit(outcome_dtos);
            }
        })
    };

    let on_result_change: Callback<Event> = {
        let props = props.clone();
        let outcomes = outcomes.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            let value = select.value();
            let player_id = select.get_attribute("data-player-id").unwrap_or_default();
            let mut new_outcomes = (*outcomes).clone();
            if let Some(outcome) = new_outcomes.iter_mut().find(|o| o.player_id == player_id) {
                outcome.result = value;
                outcomes.set(new_outcomes.clone());
                // Convert to OutcomeDto for the parent component
                let outcome_dtos: Vec<OutcomeDto> = new_outcomes
                    .iter()
                    .map(|o| OutcomeDto {
                        player_id: o.player_id.clone(),
                        place: o.place.clone(),
                        result: o.result.clone(),
                        email: o.email.clone(),
                        handle: o.handle.clone(),
                    })
                    .collect();
                props.on_outcomes_change.emit(outcome_dtos);
            }
        })
    };

    let on_modal_close = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| {
            show_modal.set(false);
        })
    };

    let result_options = vec!["won", "lost", "drop"];

    // Helper function to check if a player_id is a real DB id (not a new UUID)
    fn is_real_player_id(player_id: &str) -> bool {
        // Real DB ids are 'player/1234' (numbers or short ids), new ones have 'Uuid' in them
        player_id.starts_with("player/") && !player_id.contains("Uuid") && player_id.len() < 40
    }

    html! {
        <div class="space-y-4">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">
                    {"Player Outcomes"}
                </label>

                <div class="space-y-2">
                    <label class="block text-sm font-medium text-gray-700">
                        {"Add Players"}
                    </label>
                    <div class="relative">
                        <input
                            type="text"
                            placeholder="Search by handle or email..."
                            value={(*search_query).clone()}
                            oninput={on_search_change}
                            class={classes!(
                                "w-full", "px-3", "py-2", "border", "rounded-md",
                                "focus:outline-none", "focus:ring-2", "focus:ring-blue-500", "focus:border-blue-500",
                                if show_invalid_email { "border-red-500" } else { "border-gray-300" }
                            )}
                        />
                        if *is_searching {
                            <div class="absolute right-3 top-2">
                                <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600"></div>
                            </div>
                        }
                    </div>
                    if show_invalid_email {
                        <div class="text-xs text-red-600 mt-1">
                            {"Please enter a valid email address or handle (at least 3 characters)."}
                        </div>
                    }
                    if let Some(error_msg) = &*search_error {
                        <div class="text-xs text-red-600 mt-1">
                            {error_msg}
                        </div>
                    }
                    if *show_suggestions && !search_results.is_empty() {
                        <div class="absolute z-10 w-full bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-y-auto">
                            {search_results.iter().map(|result| {
                                let player = result.player.clone();
                                let on_click = {
                                    let on_player_select = on_player_select.clone();
                                    let player = player.clone();
                                    Callback::from(move |_| on_player_select.emit(player.clone()))
                                };
                                html! {
                                    <div
                                        class={classes!(
                                            "px-3", "py-2", "cursor-pointer", "hover:bg-gray-100",
                                            if !result.exists {
                                                classes!("bg-yellow-50", "border-l-4", "border-yellow-400", "border-r", "border-yellow-200")
                                            } else {
                                                classes!("bg-green-50", "border-l-4", "border-green-400", "border-r", "border-green-200")
                                            }
                                        )}
                                        onclick={on_click}
                                    >
                                        <div class="flex items-center justify-between">
                                            <div class="flex-1">
                                                <div class={classes!(
                                                    "font-medium",
                                                    if !result.exists { "text-yellow-800" } else { "text-green-800" }
                                                )}>
                                                    {&player.handle}
                                                </div>
                                                <div class={classes!(
                                                    "text-sm",
                                                    if !result.exists { "text-yellow-600" } else { "text-green-600" }
                                                )}>
                                                    {&player.email}
                                                </div>
                                            </div>
                                            <div class={classes!(
                                                "text-xs", "px-2", "py-1", "rounded", "font-medium",
                                                if !result.exists {
                                                    classes!("text-yellow-800", "bg-yellow-200")
                                                } else {
                                                    classes!("text-green-800", "bg-green-200")
                                                }
                                            )}>
                                                if !result.exists {
                                                    {"⚠️ New Player"}
                                                } else {
                                                    {"✓ Existing"}
                                                }
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()}
                        </div>
                    }

                    // Only allow adding a new player if there is a valid email, no error, and no results
                    if !search_query.is_empty() && is_valid_email && search_results.is_empty() && !*is_searching && search_error.is_none() {
                        <div class="mt-2">
                            <button
                                type="button"
                                onclick={on_add_player}
                                class="w-full px-3 py-2 bg-yellow-100 border border-yellow-300 rounded-md text-yellow-800 hover:bg-yellow-200 focus:outline-none focus:ring-2 focus:ring-yellow-500 font-medium"
                                disabled={!is_valid_email}
                            >
                                {"⚠️ Create new player with email '"} {&*search_query} {"' (handle: '"} {generate_handle_from_email(&*search_query)} {"')"}
                            </button>
                        </div>
                    }
                </div>
            </div>

            if !outcomes.is_empty() {
                <div class="mt-6 space-y-4">
                    <h3 class="text-sm font-medium text-gray-700">{"Contest Participants"}</h3>
                    <div class="space-y-3">
                        {outcomes.iter().map(|outcome| {
                            let player_id = outcome.player_id.clone();
                            let is_new_player = !is_real_player_id(&player_id);
                            let on_remove = {
                                let on_remove_outcome = on_remove_outcome.clone();
                                let player_id = player_id.clone();
                                Callback::from(move |_| on_remove_outcome.emit(player_id.clone()))
                            };
                            let on_place_change = {
                                let on_place_change = on_place_change.clone();
                                let player_id = player_id.clone();
                                Callback::from(move |e: InputEvent| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    on_place_change.emit((player_id.clone(), input.value()));
                                })
                            };
                            html! {
                                <div class={classes!(
                                    "flex", "items-center", "space-x-4", "p-3", "rounded-md",
                                    if is_new_player {
                                        classes!("bg-yellow-50", "border", "border-yellow-200")
                                    } else {
                                        classes!("bg-green-50", "border", "border-green-200")
                                    }
                                )}>
                                    <div class="flex-1">
                                        <div class="flex items-center space-x-2">
                                            <div class={classes!(
                                                "text-sm", "font-medium",
                                                if is_new_player { "text-yellow-800" } else { "text-green-800" }
                                            )}>
                                                {format!("{}({})", &outcome.handle, &outcome.email)}
                                            </div>
                                            if is_new_player {
                                                <span class="text-xs text-yellow-600 bg-yellow-200 px-2 py-1 rounded">
                                                    {"NEW"}
                                                </span>
                                            } else {
                                                <span class="text-xs text-green-600 bg-green-200 px-2 py-1 rounded">
                                                    {"EXISTING"}
                                                </span>
                                            }
                                        </div>
                                    </div>
                                    <div class="w-20">
                                        <input
                                            type="number"
                                            min="1"
                                            value={outcome.place.clone()}
                                            oninput={on_place_change}
                                            class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
                                            placeholder="Place"
                                        />
                                    </div>
                                    <div class="w-32">
                                        <select
                                            value={outcome.result.clone()}
                                            data-player-id={player_id.clone()}
                                            onchange={on_result_change.clone()}
                                            class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
                                        >
                                            {result_options.iter().map(|opt| html! {
                                                <option value={opt.to_string()} selected={*opt == outcome.result}>{opt}</option>
                                            }).collect::<Html>()}
                                        </select>
                                    </div>
                                    <button
                                        type="button"
                                        onclick={on_remove}
                                        class="text-red-500 hover:text-red-700"
                                    >
                                        {"×"}
                                    </button>
                                </div>
                            }
                        }).collect::<Html>()}
                    </div>
                </div>
            }

            <Modal
                is_open={*show_modal}
                title={"Duplicate Player".to_string()}
                message={(*modal_message).clone()}
                on_close={on_modal_close}
                button_class={"bg-red-600 hover:bg-red-700".to_string()}
                button_text={"OK".to_string()}
            />

            <Modal
                is_open={*show_new_player_confirm}
                title={"Confirm New Player".to_string()}
                message={if let Some(player) = (*pending_new_player).as_ref() {
                    format!("Are you sure you want to add '{}' as a new player? This will create a new player record in the database.", player.handle)
                } else {
                    "Are you sure you want to add this new player?".to_string()
                }}
                on_close={on_confirm_new_player}
                button_class={"bg-yellow-600 hover:bg-yellow-700".to_string()}
                button_text={"Yes, Add Player".to_string()}
            />
        </div>
    }
}
