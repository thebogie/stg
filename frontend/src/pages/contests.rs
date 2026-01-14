use crate::api::contests::{search_contests, ContestSearchResponse};
use crate::api::games::{get_all_games, search_games};
use crate::api::players::search_players;
use crate::api::venues::get_all_venues;
use crate::auth::AuthContext;
use crate::Route;
use chrono::DateTime;
use shared::dto::player::PlayerDto;
use shared::{GameDto, VenueDto};
use wasm_bindgen::JsCast;
use web_sys::js_sys;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ContestsProps {}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchState {
    pub query: String,
    pub start_from: String,
    pub start_to: String,
    pub stop_from: String,
    pub stop_to: String,
    pub venue_id: String,
    pub game_ids: Vec<String>,
    pub player_ids: Vec<String>,
    pub scope: String,
    pub page: u32,
    pub page_size: u32,
    pub sort_by: String,
    pub sort_dir: String,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            start_from: String::new(),
            start_to: String::new(),
            stop_from: String::new(),
            stop_to: String::new(),
            venue_id: String::new(),
            game_ids: Vec::new(),
            player_ids: Vec::new(),
            scope: "all".to_string(),
            page: 1,
            page_size: 20,
            sort_by: "start".to_string(),
            sort_dir: "desc".to_string(),
        }
    }
}

#[function_component(Contests)]
pub fn contests(_props: &ContestsProps) -> Html {
    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let navigator = use_navigator().unwrap();

    // Active filters used for querying
    let search_state = use_state(|| SearchState::default());
    // Draft filters edited in the UI before applying
    let draft_state = use_state(|| SearchState::default());
    let search_results = use_state(|| None::<ContestSearchResponse>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let venues = use_state(|| Vec::<VenueDto>::new());
    let games = use_state(|| Vec::<GameDto>::new());
    let draft_players = use_state(|| Vec::<PlayerDto>::new());
    let selected_players = use_state(|| Vec::<PlayerDto>::new());
    let game_search_query = use_state(|| String::new());
    let game_search_results = use_state(|| Vec::<GameDto>::new());
    let player_search_query = use_state(|| String::new());
    let player_search_results = use_state(|| Vec::<PlayerDto>::new());
    let show_filters = use_state(|| false);

    let on_create_contest = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&Route::Contest);
        })
    };

    // Load venues and games on mount
    {
        let venues = venues.clone();
        let games = games.clone();
        use_effect_with((), move |_| {
            let venues = venues.clone();
            let games = games.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(venue_list) = get_all_venues().await {
                    venues.set(venue_list);
                }
                if let Ok(game_list) = get_all_games().await {
                    games.set(game_list);
                }
            });
        });
    }

    // Search function (uses active search_state)
    let perform_search = {
        let search_state = search_state.clone();
        let search_results = search_results.clone();
        let loading = loading.clone();
        let error = error.clone();
        Callback::from(move |_| {
            let search_state = search_state.clone();
            let search_results = search_results.clone();
            let loading = loading.clone();
            let error = error.clone();

            loading.set(true);
            error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let mut params = Vec::new();
                if !search_state.query.is_empty() {
                    params.push(("q", search_state.query.clone()));
                }
                if !search_state.start_from.is_empty() {
                    params.push(("start_from", search_state.start_from.clone()));
                }
                if !search_state.start_to.is_empty() {
                    params.push(("start_to", search_state.start_to.clone()));
                }
                if !search_state.stop_from.is_empty() {
                    params.push(("stop_from", search_state.stop_from.clone()));
                }
                if !search_state.stop_to.is_empty() {
                    params.push(("stop_to", search_state.stop_to.clone()));
                }
                if !search_state.venue_id.is_empty() {
                    params.push(("venue_id", search_state.venue_id.clone()));
                }
                if !search_state.game_ids.is_empty() {
                    let csv = search_state.game_ids.join(",");
                    params.push(("game_ids", csv));
                }
                if search_state.player_ids.len() == 1 {
                    params.push(("player_id", search_state.player_ids[0].clone()));
                }
                // Scope is already set appropriately in state (defaults to 'all' when unauthenticated)
                params.push(("scope", search_state.scope.clone()));
                params.push(("page", search_state.page.to_string()));
                params.push(("page_size", search_state.page_size.to_string()));
                params.push(("sort_by", search_state.sort_by.clone()));
                params.push(("sort_dir", search_state.sort_dir.clone()));

                match search_contests(&params).await {
                    Ok(results) => {
                        gloo::console::log!("Search returned", results.items.len(), "contests");
                        if let Some(first_contest) = results.items.first() {
                            gloo::console::log!("First contest ID:", &first_contest.id);
                        }
                        search_results.set(Some(results));
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        loading.set(false);
                    }
                }
            });
        })
    };

    // Initial search on mount
    {
        let perform_search = perform_search.clone();
        use_effect_with((), move |_| {
            perform_search.emit(());
        });
    }

    // If not authenticated, default scope to 'all' so search doesn't request 'mine'
    {
        let auth = auth.clone();
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        use_effect_with((), move |_| {
            if auth.state.player.is_none() {
                let mut state = (*search_state).clone();
                if state.scope != "all" {
                    state.scope = "all".to_string();
                    search_state.set(state);
                    perform_search.emit(());
                }
            }
            || ()
        });
    }

    // Input handlers (write to draft_state only)
    let on_query_change = {
        let draft_state = draft_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut state = (*draft_state).clone();
            state.query = input.value();
            draft_state.set(state);
        })
    };

    let on_scope_change = {
        let draft_state = draft_state.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let mut state = (*draft_state).clone();
            state.scope = input.value();
            draft_state.set(state);
        })
    };

    let on_venue_filter_change = {
        let draft_state = draft_state.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let mut state = (*draft_state).clone();
            state.venue_id = input.value();
            draft_state.set(state);
        })
    };

    let _on_games_filter_change = {
        let draft_state = draft_state.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let selected_options = input.selected_options();
            let mut game_ids = Vec::new();
            for i in 0..selected_options.length() {
                if let Some(option) = selected_options.item(i) {
                    // Option elements are just Element types in web_sys
                    // Get the value from the value attribute
                    if let Some(value) = option.get_attribute("value") {
                        game_ids.push(value);
                    }
                }
            }
            let mut state = (*draft_state).clone();
            state.game_ids = game_ids;
            draft_state.set(state);
        })
    };

    // Typeahead search handlers
    let on_game_search_input = {
        let query_state = game_search_query.clone();
        let results_state = game_search_results.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let q = input.value();
            query_state.set(q.clone());
            let results_state = results_state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if q.len() >= 2 {
                    match search_games(&q).await {
                        Ok(list) => results_state.set(list),
                        Err(_) => results_state.set(Vec::new()),
                    }
                } else {
                    results_state.set(Vec::new());
                }
            });
        })
    };

    let on_pick_game = {
        let draft_state = draft_state.clone();
        let query_state = game_search_query.clone();
        let results_state = game_search_results.clone();
        Callback::from(move |game: GameDto| {
            let mut state = (*draft_state).clone();
            if !state.game_ids.iter().any(|id| id == &game.id) {
                state.game_ids.push(game.id.clone());
            }
            draft_state.set(state);
            query_state.set(String::new());
            results_state.set(Vec::new());
        })
    };

    let on_player_search_input = {
        let query_state = player_search_query.clone();
        let results_state = player_search_results.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let q = input.value();
            query_state.set(q.clone());
            let results_state = results_state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if q.len() >= 2 {
                    match search_players(&q).await {
                        Ok(list) => results_state.set(list),
                        Err(_) => results_state.set(Vec::new()),
                    }
                } else {
                    results_state.set(Vec::new());
                }
            });
        })
    };

    let on_pick_player = {
        let draft_state = draft_state.clone();
        let draft_players = draft_players.clone();
        let query_state = player_search_query.clone();
        let results_state = player_search_results.clone();
        Callback::from(move |player: PlayerDto| {
            let mut ids_state = (*draft_state).clone();
            if !ids_state.player_ids.iter().any(|id| id == &player.id) {
                ids_state.player_ids.push(player.id.clone());
            }
            draft_state.set(ids_state);
            let mut players = (*draft_players).clone();
            if !players.iter().any(|p| p.id == player.id) {
                players.push(player);
                draft_players.set(players);
            }
            query_state.set(String::new());
            results_state.set(Vec::new());
        })
    };

    let on_start_from_change = {
        let draft_state = draft_state.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut state = (*draft_state).clone();
            state.start_from = input.value();
            draft_state.set(state);
        })
    };

    let on_start_to_change = {
        let draft_state = draft_state.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut state = (*draft_state).clone();
            state.start_to = input.value();
            draft_state.set(state);
        })
    };

    // Apply and Clear actions
    let apply_filters = {
        let draft_state = draft_state.clone();
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        let draft_players = draft_players.clone();
        let selected_players = selected_players.clone();
        Callback::from(move |_| {
            let mut next = (*draft_state).clone();
            next.page = 1; // reset to first page on apply
            search_state.set(next);
            selected_players.set((*draft_players).clone());
            perform_search.emit(());
        })
    };

    let clear_filters = {
        let draft_state = draft_state.clone();
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        let draft_players = draft_players.clone();
        let selected_players = selected_players.clone();
        Callback::from(move |_| {
            let mut cleared = SearchState::default();
            // Preserve current scope if user has no admin access guard below
            cleared.scope = (*draft_state).scope.clone();
            draft_state.set(cleared.clone());
            search_state.set(cleared);
            draft_players.set(Vec::new());
            selected_players.set(Vec::new());
            perform_search.emit(());
        })
    };

    let on_page_change = {
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |page: u32| {
            let mut state = (*search_state).clone();
            state.page = page;
            search_state.set(state);
            perform_search.emit(());
        })
    };

    let on_sort_change = {
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |(sort_by, sort_dir): (String, String)| {
            let mut state = (*search_state).clone();
            state.sort_by = sort_by;
            state.sort_dir = sort_dir;
            state.page = 1; // Reset to first page
            search_state.set(state);
            perform_search.emit(());
        })
    };

    let toggle_filters = {
        let show_filters = show_filters.clone();
        Callback::from(move |_| {
            show_filters.set(!*show_filters);
        })
    };

    // Helper function to format time in venue timezone
    let format_time = |time_str: &str, venue: &Option<serde_json::Value>| {
        if let Ok(utc_time) = DateTime::parse_from_rfc3339(time_str) {
            // Get venue timezone, fallback to UTC
            let timezone = if let Some(venue) = venue {
                venue
                    .get("timezone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UTC")
            } else {
                "UTC"
            };

            // Convert to venue timezone using JavaScript
            let js_code = format!(
                r#"
                (function() {{
                    const utcDate = new Date('{}');
                    const options = {{
                        timeZone: '{}',
                        year: 'numeric',
                        day: '2-digit',
                        month: '2-digit',
                        hour: '2-digit',
                        minute: '2-digit',
                        hour12: false
                    }};
                    const formatted = utcDate.toLocaleString('en-US', options);
                    const offset = utcDate.toLocaleString('en-US', {{
                        timeZone: '{}',
                        timeZoneName: 'short'
                    }}).split(' ').pop();
                    return formatted + ' ' + offset;
                }})()
                "#,
                utc_time.to_rfc3339(),
                timezone,
                timezone
            );

            // Execute JavaScript to get properly formatted time
            if let Ok(result) = js_sys::eval(&js_code) {
                if let Some(formatted) = result.as_string() {
                    return formatted;
                }
            }

            // Fallback: just format with timezone name
            format!("{} {}", utc_time.format("%d/%m/%Y %H:%M"), timezone)
        } else {
            time_str.to_string()
        }
    };

    // Extract values for use in HTML
    let current_page = search_state.page;
    let current_page_size = search_state.page_size;
    // Active filter chips and count (based on applied search_state)
    let active_filter_count = {
        let mut c = 0u32;
        if !search_state.query.is_empty() {
            c += 1;
        }
        if !search_state.start_from.is_empty() {
            c += 1;
        }
        if !search_state.start_to.is_empty() {
            c += 1;
        }
        if !search_state.venue_id.is_empty() {
            c += 1;
        }
        c + (search_state.game_ids.len() as u32) + (selected_players.len() as u32)
    };

    // Removal handlers update both draft and active states and trigger a search
    let remove_query = {
        let draft_state = draft_state.clone();
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |_| {
            let mut next_draft = (*draft_state).clone();
            next_draft.query.clear();
            draft_state.set(next_draft.clone());
            let mut next_active = (*search_state).clone();
            next_active.query.clear();
            next_active.page = 1;
            search_state.set(next_active);
            perform_search.emit(());
        })
    };

    let remove_start_from = {
        let draft_state = draft_state.clone();
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |_| {
            let mut d = (*draft_state).clone();
            d.start_from.clear();
            draft_state.set(d.clone());
            let mut s = (*search_state).clone();
            s.start_from.clear();
            s.page = 1;
            search_state.set(s);
            perform_search.emit(());
        })
    };

    let remove_start_to = {
        let draft_state = draft_state.clone();
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |_| {
            let mut d = (*draft_state).clone();
            d.start_to.clear();
            draft_state.set(d.clone());
            let mut s = (*search_state).clone();
            s.start_to.clear();
            s.page = 1;
            search_state.set(s);
            perform_search.emit(());
        })
    };

    let remove_venue = {
        let draft_state = draft_state.clone();
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |_| {
            let mut d = (*draft_state).clone();
            d.venue_id.clear();
            draft_state.set(d.clone());
            let mut s = (*search_state).clone();
            s.venue_id.clear();
            s.page = 1;
            search_state.set(s);
            perform_search.emit(());
        })
    };

    let remove_game = {
        let draft_state = draft_state.clone();
        let search_state = search_state.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |game_id: String| {
            let mut d = (*draft_state).clone();
            d.game_ids.retain(|g| g != &game_id);
            draft_state.set(d.clone());
            let mut s = (*search_state).clone();
            s.game_ids.retain(|g| g != &game_id);
            s.page = 1;
            search_state.set(s);
            perform_search.emit(());
        })
    };

    let remove_player = {
        let draft_state = draft_state.clone();
        let search_state = search_state.clone();
        let draft_players = draft_players.clone();
        let selected_players = selected_players.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |player_id: String| {
            let mut d = (*draft_state).clone();
            d.player_ids.retain(|g| g != &player_id);
            draft_state.set(d.clone());
            let mut s = (*search_state).clone();
            s.player_ids.retain(|g| g != &player_id);
            s.page = 1;
            search_state.set(s);
            let mut dp = (*draft_players).clone();
            dp.retain(|p| p.id != player_id);
            draft_players.set(dp.clone());
            let mut sp = (*selected_players).clone();
            sp.retain(|p| p.id != player_id);
            selected_players.set(sp);
            perform_search.emit(());
        })
    };

    // Precompute all filter chips to avoid complex logic inside html! arms
    let filter_chips: Html = {
        let mut chips: Vec<Html> = Vec::new();
        if !search_state.query.is_empty() {
            chips.push(html!{
                <span class="inline-flex items-center gap-1 px-2 py-1 bg-gray-100 text-gray-800 text-xs rounded-full">
                    {format!("Query: {}", search_state.query)}
                    <button onclick={remove_query.reform(|_| ())} class="ml-1 text-gray-500 hover:text-gray-700">{"✕"}</button>
                </span>
            });
        }
        if !search_state.start_from.is_empty() {
            chips.push(html!{
                <span class="inline-flex items-center gap-1 px-2 py-1 bg-gray-100 text-gray-800 text-xs rounded-full">
                    {format!("Start ≥ {}", search_state.start_from)}
                    <button onclick={remove_start_from.reform(|_| ())} class="ml-1 text-gray-500 hover:text-gray-700">{"✕"}</button>
                </span>
            });
        }
        if !search_state.start_to.is_empty() {
            chips.push(html!{
                <span class="inline-flex items-center gap-1 px-2 py-1 bg-gray-100 text-gray-800 text-xs rounded-full">
                    {format!("Start ≤ {}", search_state.start_to)}
                    <button onclick={remove_start_to.reform(|_| ())} class="ml-1 text-gray-500 hover:text-gray-700">{"✕"}</button>
                </span>
            });
        }
        if !search_state.venue_id.is_empty() {
            let venue_name = venues
                .iter()
                .find(|v| v.id == search_state.venue_id)
                .map(|v| v.display_name.clone())
                .unwrap_or_else(|| "Venue".to_string());
            chips.push(html!{
                <span class="inline-flex items-center gap-1 px-2 py-1 bg-gray-100 text-gray-800 text-xs rounded-full">
                    {format!("Venue: {}", venue_name)}
                    <button onclick={remove_venue.reform(|_| ())} class="ml-1 text-gray-500 hover:text-gray-700">{"✕"}</button>
                </span>
            });
        }
        for gid in &search_state.game_ids {
            let name = games
                .iter()
                .find(|g| g.id == *gid)
                .map(|g| g.name.clone())
                .unwrap_or_else(|| "Game".to_string());
            let remove_game_cb = {
                let remove_game = remove_game.clone();
                let gid_clone = gid.clone();
                Callback::from(move |_| remove_game.emit(gid_clone.clone()))
            };
            chips.push(html!{
                <span class="inline-flex items-center gap-1 px-2 py-1 bg-blue-50 text-blue-800 text-xs rounded-full">
                    {format!("Game: {}", name)}
                    <button onclick={remove_game_cb} class="ml-1">{"✕"}</button>
                </span>
            });
        }
        for p in &*selected_players {
            let pid = p.id.clone();
            let remove_player_cb = {
                let remove_player = remove_player.clone();
                Callback::from(move |_| remove_player.emit(pid.clone()))
            };
            chips.push(html!{
                <span class="inline-flex items-center gap-1 px-2 py-1 bg-purple-100 text-purple-800 text-xs rounded-full">
                    {format!("Person: {}", p.handle)}
                    <button onclick={remove_player_cb} class="ml-1 text-purple-500 hover:text-purple-700">{"✕"}</button>
                </span>
            });
        }
        html! { <>{ for chips }</> }
    };

    html! {
        <div class="min-h-screen bg-gray-50">
            <header class="app-bar-material p-4 sticky top-0 z-40 bg-white shadow-sm">
                <div class="container mx-auto flex justify-between items-center flex-wrap gap-3">
                    <h1 class="text-xl font-medium">{"Contests"}</h1>
                    if auth.state.player.is_some() {
                        <button
                            onclick={on_create_contest.clone()}
                            class="inline-flex items-center justify-center px-6 py-3 text-base font-semibold btn-material-primary shadow-md"
                        >
                            <span class="mr-2">{"➕"}</span>
                            {"Create Contest"}
                        </button>
                    }
                </div>
            </header>

            <main class="container mx-auto px-4 py-6">
                // Search Bar
                <div class="bg-white rounded-lg shadow-sm p-4 mb-6">
                    <div class="flex flex-col md:flex-row gap-4">
                        <div class="flex-1">
                            <input
                                type="text"
                                placeholder="Search contests..."
                                value={search_state.query.clone()}
                                oninput={on_query_change}
                                class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            />
                        </div>
                        <div class="flex gap-2">
                            <select
                                value={draft_state.scope.clone()}
                                onchange={on_scope_change}
                                class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                            >
                                <option value="mine">{"My Contests"}</option>
                                if auth.state.player.as_ref().map_or(false, |p| p.is_admin) {
                                    <option value="all">{"All Contests"}</option>
                                }
                            </select>
                            <button
                                onclick={toggle_filters}
                                class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50 flex items-center gap-2 relative"
                            >
                                <span>{"Filters"}</span>
                                <span class="text-sm">{"▼"}</span>
                                {if active_filter_count > 0 {
                                    html! { <span class="absolute -top-2 -right-2 inline-flex items-center justify-center rounded-full bg-blue-600 text-white text-xs w-5 h-5">{active_filter_count}</span> }
                                } else { html! {} }}
                            </button>
                            <button
                                onclick={apply_filters.reform(|_| ())}
                                disabled={*loading}
                                class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                            >
                                {if *loading { "Searching..." } else { "Search" }}
                            </button>
                            <button
                                onclick={clear_filters.reform(|_| ())}
                                class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
                            >
                                {"Clear"}
                            </button>
                        </div>
                    </div>

                    // Active filter chips (applied)
                    if active_filter_count > 0 {
                        <div class="mt-3 flex flex-wrap gap-2">{filter_chips.clone()}</div>
                    }

                    // Advanced Filters (collapsible)
                    if *show_filters {
                        <div class="mt-4 pt-4 border-t border-gray-200">
                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">{"Start Date From"}</label>
                                    <input
                                        type="date"
                                        value={draft_state.start_from.clone()}
                                        onchange={on_start_from_change}
                                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                        placeholder="2022-01-01"
                                    />
                                    <p class="text-xs text-gray-500 mt-1">{"e.g., 2022-01-01 for all contests from 2022"}</p>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">{"Start Date To"}</label>
                                    <input
                                        type="date"
                                        value={draft_state.start_to.clone()}
                                        onchange={on_start_to_change}
                                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                        placeholder="2023-12-31"
                                    />
                                    <p class="text-xs text-gray-500 mt-1">{"e.g., 2023-12-31 for all contests until end of 2023"}</p>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">{"Venue"}</label>
                                    <select
                                        value={draft_state.venue_id.clone()}
                                        onchange={on_venue_filter_change}
                                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                    >
                                        <option value="">{"All Venues"}</option>
                                        {for venues.iter().map(|venue| html! {
                                            <option value={venue.id.clone()}>{&venue.display_name}</option>
                                        })}
                                    </select>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">{"Games"}</label>
                                    <input
                                        type="text"
                                        placeholder="Search games..."
                                        value={(*game_search_query).clone()}
                                        oninput={on_game_search_input}
                                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 mb-2"
                                    />
                                    {if !game_search_results.is_empty() { html! {
                                        <div class="max-h-40 overflow-auto border border-gray-200 rounded-md">
                                            {for game_search_results.iter().map(|g| {
                                                let pick = on_pick_game.clone();
                                                let g_clone = g.clone();
                                                html!{
                                                    <button onclick={Callback::from(move |_| pick.emit(g_clone.clone()))} class="w-full text-left px-3 py-2 hover:bg-gray-50">
                                                        {&g.name}
                                                    </button>
                                                }
                                            })}
                                        </div>
                                    }} else { html!{} }}
                                    <div class="mt-2 flex flex-wrap gap-2">
                                        {for draft_state.game_ids.iter().map(|gid| {
                                            let name = games.iter().find(|g| g.id == *gid).map(|g| g.name.clone()).unwrap_or_else(|| "Game".to_string());
                                            let remove = remove_game.clone();
                                            let gid_clone = gid.clone();
                                            html!{
                                                <span class="inline-flex items-center gap-1 px-2 py-1 bg-blue-50 text-blue-800 text-xs rounded-full">
                                                    {name}
                                                    <button onclick={Callback::from(move |_| remove.emit(gid_clone.clone()))} class="ml-1">{"✕"}</button>
                                                </span>
                                            }
                                        })}
                                    </div>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">{"People"}</label>
                                    <input
                                        type="text"
                                        placeholder="Search people..."
                                        value={(*player_search_query).clone()}
                                        oninput={on_player_search_input}
                                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 mb-2"
                                    />
                                    {if !player_search_results.is_empty() { html! {
                                        <div class="max-h-40 overflow-auto border border-gray-200 rounded-md">
                                            {for player_search_results.iter().map(|p| {
                                                let pick = on_pick_player.clone();
                                                let p_clone = p.clone();
                                                html!{
                                                    <button onclick={Callback::from(move |_| pick.emit(p_clone.clone()))} class="w-full text-left px-3 py-2 hover:bg-gray-50">
                                                        {format!("{} ({})", &p.handle, &p.email)}
                                                    </button>
                                                }
                                            })}
                                        </div>
                                    }} else { html!{} }}
                                    <div class="mt-2 flex flex-wrap gap-2">
                                        {for draft_players.iter().map(|p| {
                                            let remove_player = remove_player.clone();
                                            let pid = p.id.clone();
                                            html!{
                                                <span class="inline-flex items-center gap-1 px-2 py-1 bg-purple-50 text-purple-800 text-xs rounded-full">
                                                    {&p.handle}
                                                    <button onclick={Callback::from(move |_| remove_player.emit(pid.clone()))} class="ml-1">{"✕"}</button>
                                                </span>
                                            }
                                        })}
                                    </div>
                                </div>
                            </div>
                        </div>
                    }
                </div>

                // Results
                if let Some(error) = &*error {
                    <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
                        <div class="flex">
                            <div class="text-red-400">{"⚠️"}</div>
                            <div class="ml-3">
                                <h3 class="text-sm font-medium text-red-800">{"Error"}</h3>
                                <div class="mt-1 text-sm text-red-700">{error}</div>
                            </div>
                        </div>
                    </div>
                } else if let Some(results) = &*search_results {
                    if results.items.is_empty() {
                        <div class="bg-white rounded-lg shadow-sm p-12 text-center">
                            <div class="text-6xl mb-4">{"🏆"}</div>
                            <h2 class="text-2xl font-bold text-gray-900 mb-4">{"No Contests Found"}</h2>
                            <p class="text-gray-600 mb-6">
                                {"No contests match your search criteria. Try adjusting your filters or create a new contest."}
                            </p>
                            if auth.state.player.is_some() {
                                <button
                                    onclick={on_create_contest.clone()}
                                    class="inline-flex items-center justify-center px-6 py-3 text-lg font-semibold text-white bg-gradient-to-r from-blue-600 to-indigo-600 rounded-xl shadow-lg hover:shadow-xl transform hover:-translate-y-1 transition-all duration-200"
                                >
                                    <span class="mr-2 text-xl">{"🚀"}</span>
                                    {"Create Contest"}
                                </button>
                            }
                        </div>
                    } else {
                        // Results Table
                        <div class="bg-white rounded-lg shadow-sm overflow-hidden">
                            <div class="overflow-x-auto">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                <button
                                                    onclick={on_sort_change.reform({
                                                        let sort_by = search_state.sort_by.clone();
                                                        let sort_dir = search_state.sort_dir.clone();
                                                        move |_| ("name".to_string(), if sort_by == "name" && sort_dir == "asc" { "desc".to_string() } else { "asc".to_string() })
                                                    })}
                                                    class="flex items-center gap-1 hover:text-gray-700"
                                                >
                                                    {"Name"}
                                                    {if search_state.sort_by == "name" {
                                                        if search_state.sort_dir == "asc" { "↑" } else { "↓" }
                                                    } else { "" }}
                                                </button>
                                            </th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                <button
                                                    onclick={on_sort_change.reform({
                                                        let sort_by = search_state.sort_by.clone();
                                                        let sort_dir = search_state.sort_dir.clone();
                                                        move |_| ("start".to_string(), if sort_by == "start" && sort_dir == "asc" { "desc".to_string() } else { "asc".to_string() })
                                                    })}
                                                    class="flex items-center gap-1 hover:text-gray-700"
                                                >
                                                    {"Start"}
                                                    {if search_state.sort_by == "start" {
                                                        if search_state.sort_dir == "asc" { "↑" } else { "↓" }
                                                    } else { "" }}
                                                </button>
                                            </th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                <button
                                                    onclick={on_sort_change.reform({
                                                        let sort_by = search_state.sort_by.clone();
                                                        let sort_dir = search_state.sort_dir.clone();
                                                        move |_| ("stop".to_string(), if sort_by == "stop" && sort_dir == "asc" { "desc".to_string() } else { "asc".to_string() })
                                                    })}
                                                    class="flex items-center gap-1 hover:text-gray-700"
                                                >
                                                    {"End"}
                                                    {if search_state.sort_by == "stop" {
                                                        if search_state.sort_dir == "asc" { "↑" } else { "↓" }
                                                    } else { "" }}
                                                </button>
                                            </th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Venue"}</th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider min-w-0">{"Games"}</th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white divide-y divide-gray-200">
                                        {for results.items.iter().map(|contest| {
                                            let contest_id = contest.id.clone();
                                            let navigator = navigator.clone();
                                            html! {
                                                <tr
                                                    class="hover:bg-gray-50 cursor-pointer"
                                                    onclick={Callback::from(move |_| {
                                                        navigator.push(&Route::ContestDetails { contest_id: contest_id.clone() });
                                                    })}
                                                >
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <div class="text-sm font-medium text-gray-900">{&contest.name}</div>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                    <div class="text-xs text-gray-600">
                                                        {format_time(&contest.start, &contest.venue)}
                                                    </div>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                    <div class="text-xs text-gray-600">
                                                        {format_time(&contest.stop, &contest.venue)}
                                                    </div>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                    {if let Some(venue) = &contest.venue {
                                                        if let Some(name) = venue.get("displayName").and_then(|v| v.as_str()) {
                                                            name
                                                        } else {
                                                            "Unknown"
                                                        }
                                                    } else {
                                                        "Online"
                                                    }}
                                                </td>
                                                <td class="px-6 py-4 text-sm text-gray-500 min-w-0">
                                                    <div class="flex flex-wrap gap-1">
                                                        {for contest.games.iter().filter_map(|game| {
                                                            if let Some(name) = game.get("name").and_then(|v| v.as_str()) {
                                                                // Accept either `_id` or `id` from backend payloads
                                                                if let Some(game_id) = game.get("_id").or_else(|| game.get("id")).and_then(|v| v.as_str()) {
                                                                    // Only show games that match the current filter, or all games if no filter is applied
                                                                    let should_show = search_state.game_ids.is_empty() ||
                                                                        search_state.game_ids.iter().any(|filter_id| filter_id == game_id);
                                                                    if should_show {
                                                                        Some(html! {
                                                                            <span class="inline-flex px-2 py-1 text-xs font-medium bg-blue-100 text-blue-800 rounded">
                                                                                {name}
                                                                            </span>
                                                                        })
                                                                    } else {
                                                                        None
                                                                    }
                                                                } else {
                                                                    // No _id present for this game entry
                                                                    if search_state.game_ids.is_empty() {
                                                                        // When no filter is active, still show named games even without _id
                                                                        Some(html! {
                                                                            <span class="inline-flex px-2 py-1 text-xs font-medium bg-blue-100 text-blue-800 rounded">
                                                                                {name}
                                                                            </span>
                                                                        })
                                                                    } else {
                                                                        // When filtered by game_ids, skip items without an _id match
                                                                        None
                                                                    }
                                                                }
                                                            } else {
                                                                None
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                </td>
                                                </tr>
                                            }
                                        })}
                                    </tbody>
                                </table>
                            </div>

                            // Pagination
                            if results.total > results.page_size as u64 {
                                <div class="bg-white px-4 py-3 flex items-center justify-between border-t border-gray-200 sm:px-6">
                                    <div class="flex-1 flex justify-between sm:hidden">
                                        <button
                                            onclick={on_page_change.reform(move |_| if current_page > 1 { current_page - 1 } else { 1 })}
                                            disabled={current_page <= 1}
                                            class="relative inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 disabled:opacity-50"
                                        >
                                            {"Previous"}
                                        </button>
                                        <button
                                            onclick={on_page_change.reform(move |_| current_page + 1)}
                                            disabled={current_page * current_page_size >= results.total as u32}
                                            class="ml-3 relative inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 disabled:opacity-50"
                                        >
                                            {"Next"}
                                        </button>
                                    </div>
                                    <div class="hidden sm:flex-1 sm:flex sm:items-center sm:justify-between">
                                        <div>
                                            <p class="text-sm text-gray-700">
                                                {"Showing "}
                                                <span class="font-medium">{(current_page - 1) * current_page_size + 1}</span>
                                                {" to "}
                                                <span class="font-medium">{(current_page * current_page_size).min(results.total as u32)}</span>
                                                {" of "}
                                                <span class="font-medium">{results.total}</span>
                                                {" results"}
                                            </p>
                                        </div>
                                        <div>
                                            <nav class="relative z-0 inline-flex rounded-md shadow-sm -space-x-px">
                                                <button
                                                    onclick={on_page_change.reform(move |_| if current_page > 1 { current_page - 1 } else { 1 })}
                                                    disabled={current_page <= 1}
                                                    class="relative inline-flex items-center px-2 py-2 rounded-l-md border border-gray-300 bg-white text-sm font-medium text-gray-500 hover:bg-gray-50 disabled:opacity-50"
                                                >
                                                    {"Previous"}
                                                </button>

                                                // Page numbers
                                                {{
                                                    let total_pages = ((results.total as f64) / (current_page_size as f64)).ceil() as u32;
                                                    let start_page = if current_page <= 3 { 1 } else { current_page - 2 };
                                                    let end_page = if current_page + 2 >= total_pages { total_pages } else { current_page + 2 };

                                                    (start_page..=end_page).map(|page_num| {
                                                        let is_current = page_num == current_page;
                                                        html! {
                                                            <button
                                                                onclick={on_page_change.reform(move |_| page_num)}
                                                                class={classes!(
                                                                    "relative", "inline-flex", "items-center", "px-4", "py-2", "border", "text-sm", "font-medium",
                                                                    if is_current {
                                                                        classes!("z-10", "bg-indigo-50", "border-indigo-500", "text-indigo-600")
                                                                    } else {
                                                                        classes!("bg-white", "border-gray-300", "text-gray-500", "hover:bg-gray-50")
                                                                    }
                                                                )}
                                                            >
                                                                {page_num}
                                                            </button>
                                                        }
                                                    }).collect::<Vec<_>>()
                                                }}

                                                <button
                                                    onclick={on_page_change.reform(move |_| current_page + 1)}
                                                    disabled={current_page * current_page_size >= results.total as u32}
                                                    class="relative inline-flex items-center px-2 py-2 rounded-r-md border border-gray-300 bg-white text-sm font-medium text-gray-500 hover:bg-gray-50 disabled:opacity-50"
                                                >
                                                    {"Next"}
                                                </button>
                                            </nav>
                                        </div>
                                    </div>
                                </div>
                            }
                        </div>
                    }
                }
            </main>

            // Floating Create Contest CTA (mobile)
            if auth.state.player.is_some() {
                <div class="fixed bottom-6 right-6 z-50 md:hidden">
                    <button
                        onclick={on_create_contest}
                        class="w-16 h-16 bg-blue-600 text-white rounded-full shadow-lg hover:bg-blue-700 transform hover:scale-110 transition-all duration-200 flex items-center justify-center"
                    >
                        <span class="text-3xl">{"+"}</span>
                    </button>
                </div>
            }
        </div>
    }
}
