use yew::prelude::*;
use gloo_storage::Storage;
use gloo_storage::LocalStorage;
use wasm_bindgen_futures::spawn_local;
use shared::dto::game::GameDto;
use gloo::timers::callback::Timeout;

#[derive(Properties, PartialEq, Clone)]
pub struct GameSelectorProps {
    pub games: Vec<GameDto>,
    pub on_games_change: Callback<Vec<GameDto>>,
    pub preload_last: bool,
}

#[function_component(GameSelector)]
pub fn game_selector(props: &GameSelectorProps) -> Html {
    let props = props.clone();
    let search_query = use_state(String::new);
    let game_suggestions = use_state(Vec::<GameDto>::new);
    let show_suggestions = use_state(|| false);
    let is_searching = use_state(|| false);
    let debounce_handle = use_mut_ref(|| None::<Timeout>);
    let is_interacting_suggestions = use_state(|| false);

    let on_search = {
        let search_query = search_query.clone();
        let game_suggestions = game_suggestions.clone();
        let show_suggestions = show_suggestions.clone();
        let _is_searching = is_searching.clone();
        let debounce_handle = debounce_handle.clone();

        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let query = input.value();
            search_query.set(query.clone());
            show_suggestions.set(true);

            if query.is_empty() {
                game_suggestions.set(Vec::new());
                return;
            }

            // Debounce: clear previous timeout
            if let Some(handle) = debounce_handle.borrow_mut().take() {
                handle.cancel();
            }
            let game_suggestions = game_suggestions.clone();
            let is_searching = _is_searching.clone();
            let query_clone = query.clone();
            debounce_handle.borrow_mut().replace(Timeout::new(700, move || {
                is_searching.set(true);
                spawn_local(async move {
                    let url = format!("/api/games/search?query={}", query_clone);
                    match crate::api::utils::authenticated_get(&url).send().await {
                        Ok(resp) => {
                            if let Ok(games) = resp.json::<Vec<GameDto>>().await {
                                // Filter: if a DB game exists for a given bgg_id, hide the BGG version(s)
                                let db_bgg_ids: std::collections::HashSet<i32> = games
                                    .iter()
                                    .filter(|g| g.id.starts_with("game/"))
                                    .filter_map(|g| g.bgg_id)
                                    .collect();

                                let filtered: Vec<GameDto> = games
                                    .into_iter()
                                    .filter(|g| {
                                        let is_bgg = g.id.starts_with("bgg_");
                                        if !is_bgg {
                                            return true; // always keep DB games
                                        }
                                        // For BGG items, keep only if we do NOT have a DB game with the same bgg_id
                                        match g.bgg_id {
                                            Some(id) => !db_bgg_ids.contains(&id),
                                            None => true, // no bgg_id to match on; keep
                                        }
                                    })
                                    .collect();

                                game_suggestions.set(filtered);
                            }
                        }
                        Err(e) => {
                            gloo::console::error!("Failed to fetch games:", e.to_string());
                        }
                    }
                    is_searching.set(false);
                });
            }));
        })
    };

    // Preload last selected games from localStorage if requested
    {
        let props = props.clone();
        use_effect_with(props.preload_last, move |preload| {
            if *preload {
                if let Ok(stored) = LocalStorage::get::<Vec<String>>("last_selected_game_ids") {
                    if !stored.is_empty() {
                        // Fetch each game by id via /api/games/{id}
                        let on_games_change = props.on_games_change.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let mut games = Vec::new();
                            for id in stored {
                                let url = format!("/api/games/{}", id);
                                if let Ok(resp) = crate::api::utils::authenticated_get(&url).send().await {
                                    if resp.ok() {
                                        if let Ok(game) = resp.json::<GameDto>().await {
                                            games.push(game);
                                        }
                                    }
                                }
                            }
                            if !games.is_empty() {
                                on_games_change.emit(games);
                            }
                        });
                    }
                }
            }
            || ()
        });
    }

    let on_game_select = {
        let props = props.clone();
        let search_query = search_query.clone();
        let _game_suggestions = game_suggestions.clone();
        let show_suggestions = show_suggestions.clone();
        Callback::from(move |game: GameDto| {
            let mut games = props.games.clone();
            if games.iter().any(|g| g.id == game.id) {
                search_query.set(String::new());
                return;
            }
            // If the game is from BGG, just add it; backend will upsert on contest create
            let is_bgg = game.id.starts_with("bgg_");
            if is_bgg {
                games.push(game.clone());
                props.on_games_change.emit(games.clone());
                search_query.set(String::new());
                show_suggestions.set(false);
            } else {
                // If already in DB, just add
                games.push(game.clone());
                props.on_games_change.emit(games.clone());
                search_query.set(String::new());
                show_suggestions.set(false);
            }
            // Persist last selected game ids
            let _ = LocalStorage::set(
                "last_selected_game_ids",
                games.iter().map(|g| g.id.clone()).collect::<Vec<_>>()
            );
        })
    };

    let on_game_remove = {
        let props = props.clone();
        Callback::from(move |game_id: String| {
            let mut games = props.games.clone();
            games.retain(|g| g.id != game_id);
            let updated = games.clone();
            props.on_games_change.emit(updated.clone());
            let _ = LocalStorage::set(
                "last_selected_game_ids",
                updated.iter().map(|g| g.id.clone()).collect::<Vec<_>>()
            );
        })
    };

    let on_input_focus = {
        let show_suggestions = show_suggestions.clone();
        let game_suggestions = game_suggestions.clone();

        Callback::from(move |_| {
            if !game_suggestions.is_empty() {
                show_suggestions.set(true);
            }
        })
    };

    let on_input_blur = {
        let show_suggestions = show_suggestions.clone();
        let is_interacting_suggestions = is_interacting_suggestions.clone();

        Callback::from(move |_| {
            // Delay hiding to allow for click events/scroll within suggestions
            let show_suggestions = show_suggestions.clone();
            let is_interacting_suggestions = is_interacting_suggestions.clone();
            wasm_bindgen_futures::spawn_local(async move {
                gloo::timers::callback::Timeout::new(150, move || {
                    if !*is_interacting_suggestions {
                        show_suggestions.set(false);
                    }
                })
                .forget();
            });
        })
    };

    let on_keydown = {
        let show_suggestions = show_suggestions.clone();

        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Escape" {
                show_suggestions.set(false);
            }
        })
    };

    html! {
        <div class="space-y-4">
            <div class="text-field-material">
                <label>{"Search Games"}</label>
                <input
                    type="text"
                    placeholder="Search for games... (select multiple games)"
                    value={(*search_query).clone()}
                    oninput={on_search}
                    onfocus={on_input_focus}
                    onblur={on_input_blur}
                    onkeydown={on_keydown}
                />
            </div>

            if *show_suggestions {
                <div
                    class="paper-material mt-1 max-h-60 overflow-auto mobile-scroll"
                    onpointerdown={
                        let is_interacting_suggestions = is_interacting_suggestions.clone();
                        Callback::from(move |_| is_interacting_suggestions.set(true))
                    }
                    onpointerup={
                        let is_interacting_suggestions = is_interacting_suggestions.clone();
                        Callback::from(move |_| is_interacting_suggestions.set(false))
                    }
                    onpointercancel={
                        let is_interacting_suggestions = is_interacting_suggestions.clone();
                        Callback::from(move |_| is_interacting_suggestions.set(false))
                    }
                    ontouchend={
                        let is_interacting_suggestions = is_interacting_suggestions.clone();
                        Callback::from(move |_| is_interacting_suggestions.set(false))
                    }
                >
                    if *is_searching {
                        <div class="p-4 text-center text-gray-500">
                            <div class="flex items-center justify-center space-x-2">
                                <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
                                <span>{"Searching games..."}</span>
                            </div>
                        </div>
                    } else if game_suggestions.is_empty() && !search_query.is_empty() {
                        <div class="p-4 text-center text-gray-500">
                            <div class="flex flex-col items-center space-y-2">
                                <span class="text-lg" role="img" aria-label="dice">{"\u{1F3B2}"}</span>
                                <span>{"No games found"}</span>
                                <span class="text-xs">{"Try a different search term"}</span>
                            </div>
                        </div>
                    } else if !game_suggestions.is_empty() {
                        <div class="p-2 border-b border-gray-200">
                            <div class="flex items-center justify-between">
                                <div class="flex items-center space-x-4 text-xs text-gray-600">
                                    <div class="flex items-center space-x-1">
                                        <span class="w-3 h-3 bg-blue-100 text-blue-800 rounded-full text-xs font-medium flex items-center justify-center">{"DB"}</span>
                                        <span>{"Database games"}</span>
                                    </div>
                                    <div class="flex items-center space-x-1">
                                        <span class="w-3 h-3 bg-orange-100 text-orange-800 rounded-full text-xs font-medium flex items-center justify-center">{"BGG"}</span>
                                        <span>{"BoardGameGeek"}</span>
                                    </div>
                                </div>
                                <div class="text-xs text-gray-500">
                                    {"Click to select multiple games"}
                                </div>
                            </div>
                        </div>
                        <ul class="list-material">
                            {game_suggestions.iter().map(|game| {
                                let game = game.clone();
                                let on_click = {
                                    let on_game_select = on_game_select.clone();
                                    let game = game.clone();
                                    Callback::from(move |_| on_game_select.emit(game.clone()))
                                };

                                // Determine source and styling with enhanced visual treatment
                                let is_db = game.id.starts_with("game/");
                                let (source_text, source_class, source_icon, source_description) = if is_db {
                                    ("Database", "bg-blue-100 text-blue-800 border-blue-200", "\u{1F5C4}\u{FE0F}", "Already in our database")
                                } else {
                                    ("BGG", "bg-orange-100 text-orange-800 border-orange-200", "\u{1F3B2}", "From BoardGameGeek")
                                };

                                html! {
                                    <li
                                        class="list-item-material hover:bg-gray-50 transition-colors duration-150 cursor-pointer"
                                        onclick={on_click}
                                    >
                                        <div class="flex items-start justify-between w-full">
                                            <div class="flex-1 min-w-0">
                                                <div class="flex items-center space-x-2">
                                                    <span class="text-lg">{source_icon}</span>
                                                    <div class="flex-1 min-w-0">
                                                        <div class="flex items-center space-x-2">
                                                            <span class="font-medium text-gray-900 truncate">
                                                                {&game.name}
                                                            </span>
                                                            if let Some(year) = game.year_published {
                                                                <span class="text-gray-500 text-xs bg-gray-100 px-1.5 py-0.5 rounded">
                                                                    {year}
                                                                </span>
                                                            }
                                                        </div>
                                                        if let Some(description) = &game.description {
                                                            <p class="text-xs text-gray-600 mt-1 line-clamp-2">
                                                                {description}
                                                            </p>
                                                        }
                                                        if let Some(bgg_id) = game.bgg_id {
                                                            <p class="text-xs text-gray-500 mt-1">
                                                                {"BGG ID: "}{bgg_id}
                                                            </p>
                                                        }
                                                    </div>
                                                </div>
                                            </div>
                                            <div class="flex flex-col items-end space-y-1 ml-3">
                                                <span class={format!("text-xs px-2 py-1 rounded-full font-medium border {}", source_class)}>
                                                    {source_text}
                                                </span>
                                                <span class="text-xs text-gray-500 text-right max-w-24">
                                                    {source_description}
                                                </span>
                                            </div>
                                        </div>
                                    </li>
                                }
                            }).collect::<Html>()}
                        </ul>
                    }
                </div>
            }

            if !props.games.is_empty() {
                <div class="space-y-2">
                    <div class="flex items-center justify-between">
                        <h3 class="text-sm font-medium text-gray-700">{"Selected Games"}</h3>
                        <div class="flex items-center space-x-2 text-xs text-gray-500">
                            <span class="flex items-center space-x-1">
                                <span class="w-2 h-2 bg-blue-100 rounded-full"></span>
                                <span>{"DB: "}{props.games.iter().filter(|g| g.id.starts_with("game/")).count()}</span>
                            </span>
                            <span class="flex items-center space-x-1">
                                <span class="w-2 h-2 bg-orange-100 rounded-full"></span>
                                <span>{"BGG: "}{props.games.iter().filter(|g| g.id.starts_with("bgg_")).count()}</span>
                            </span>
                        </div>
                    </div>
                    <div class="space-y-2">
                        {props.games.iter().map(|game| {
                            let game_id = game.id.clone();
                            let on_remove = {
                                let on_game_remove = on_game_remove.clone();
                                let game_id = game_id.clone();
                                Callback::from(move |_| on_game_remove.emit(game_id.clone()))
                            };

                            // Determine source and styling with enhanced visual treatment
                            let is_db = game.id.starts_with("game/");
                            let (source_text, source_class, source_icon, _source_description) = if is_db {
                                ("Database", "bg-blue-100 text-blue-800 border-blue-200", "\u{1F5C4}\u{FE0F}", "Database")
                            } else {
                                ("BGG", "bg-orange-100 text-orange-800 border-orange-200", "\u{1F3B2}", "BoardGameGeek")
                            };

                            html! {
                                <div class="paper-material p-3 hover:bg-gray-50 transition-colors duration-150">
                                    <div class="flex items-start justify-between">
                                        <div class="flex items-start space-x-3 flex-1">
                                            <span class="text-lg">{source_icon}</span>
                                            <div class="flex-1 min-w-0">
                                                <div class="flex items-center space-x-2">
                                                    <span class="font-medium text-gray-900">{&game.name}</span>
                                                    if let Some(year) = game.year_published {
                                                        <span class="text-gray-500 text-xs bg-gray-100 px-1.5 py-0.5 rounded">
                                                            {year}
                                                        </span>
                                                    }
                                                </div>
                                                if let Some(description) = &game.description {
                                                    <p class="text-xs text-gray-600 mt-1 line-clamp-1">
                                                        {description}
                                                    </p>
                                                }
                                                if let Some(bgg_id) = game.bgg_id {
                                                    <p class="text-xs text-gray-500 mt-1">
                                                        {"BGG ID: "}{bgg_id}
                                                    </p>
                                                }
                                            </div>
                                        </div>
                                        <div class="flex flex-col items-end space-y-1 ml-3">
                                            <span class={format!("text-xs px-2 py-1 rounded-full font-medium border {}", source_class)}>
                                                {source_text}
                                            </span>
                                            <button
                                                type="button"
                                                class="text-gray-400 hover:text-red-500 transition-colors duration-150 p-1 rounded hover:bg-red-50"
                                                onclick={on_remove}
                                                title="Remove game"
                                            >
                                                <span class="sr-only">{"Remove"}</span>
                                                {"×"}
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect::<Html>()}
                    </div>
                </div>
            }
        </div>
    }
} 
