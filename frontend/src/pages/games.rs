use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen_futures::spawn_local;
use shared::dto::game::GameDto;
use crate::api::games::{get_all_games, search_games};
use crate::Route;

#[function_component(Games)]
pub fn games() -> Html {
    let navigator = use_navigator().unwrap();
    
    // Search state
    let draft_query = use_state(|| String::new());
    let query = use_state(|| String::new());
    let games = use_state(|| None::<Vec<GameDto>>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);

    // Callbacks
    let on_query_change = {
        let draft_query = draft_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            draft_query.set(input.value());
        })
    };

    let apply_filters = {
        let query = query.clone();
        let draft_query = draft_query.clone();
        let loading = loading.clone();
        let error = error.clone();
        let games = games.clone();
        let navigator = navigator.clone();
        
        Callback::from(move |_| {
            let search_query = (*draft_query).clone();
            query.set(search_query.clone());
            
            loading.set(true);
            error.set(None);
            
            let loading = loading.clone();
            let error = error.clone();
            let games = games.clone();
            let _navigator = navigator.clone();
            
            spawn_local(async move {
                let result = if search_query.is_empty() {
                    get_all_games().await
                } else {
                    search_games(&search_query).await
                };
                
                loading.set(false);
                match result {
                    Ok(game_list) => {
                        games.set(Some(game_list));
                        error.set(None);
                    },
                    Err(e) => {
                        error.set(Some(e));
                        games.set(None);
                    }
                }
            });
        })
    };

    let clear_filters = {
        let draft_query = draft_query.clone();
        let query = query.clone();
        let games = games.clone();
        let error = error.clone();
        
        Callback::from(move |_| {
            draft_query.set(String::new());
            query.set(String::new());
            games.set(None);
            error.set(None);
        })
    };

    // Load all games on mount
    {
        let loading = loading.clone();
        let games = games.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            error.set(None);
            
            let loading = loading.clone();
            let games = games.clone();
            let error = error.clone();
            
            spawn_local(async move {
                match get_all_games().await {
                    Ok(game_list) => {
                        games.set(Some(game_list));
                        error.set(None);
                    },
                    Err(e) => {
                        error.set(Some(e));
                        games.set(None);
                    }
                }
                loading.set(false);
            });
        });
    }

    // Filter chips
    let filter_chips = if !query.is_empty() {
        html! {
            <div class="flex items-center gap-2 mb-4">
                <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                    {"Search: "}{(*query).clone()}
                </span>
            </div>
        }
    } else {
        html! {}
    };

    html! {
        <div class="min-h-screen bg-gray-50">
            <div class="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
                <div class="mb-8">
                    <h1 class="text-3xl font-bold text-gray-900">{"🎮 Games"}</h1>
                    <p class="mt-2 text-gray-600">{"Browse and search games in your collection"}</p>
                </div>
                
                // Search and filters
                <div class="bg-white shadow rounded-lg p-6 mb-6">
                    <div class="flex flex-col sm:flex-row gap-4">
                        <div class="flex-1">
                            <label for="game-search" class="block text-sm font-medium text-gray-700 mb-2">
                                {"Search Games"}
                            </label>
                            <input
                                id="game-search"
                                type="text"
                                placeholder="Search by name, year, or description..."
                                value={(*draft_query).clone()}
                                oninput={on_query_change}
                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                            />
                        </div>
                        <div class="flex items-end gap-2">
                            <button
                                onclick={apply_filters}
                                class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                            >
                                {"Search"}
                            </button>
                            <button
                                onclick={clear_filters}
                                class="px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                            >
                                {"Clear"}
                            </button>
                        </div>
                    </div>
                    
                    {filter_chips}
                </div>

                // Results
                <div class="bg-white shadow rounded-lg">
                    if *loading {
                        <div class="p-8 text-center">
                            <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                            <p class="mt-2 text-gray-600">{"Loading games..."}</p>
                        </div>
                    } else if let Some(error_msg) = &*error {
                        <div class="p-8 text-center">
                            <div class="text-red-600 mb-2">
                                <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
                                </svg>
                            </div>
                            <h3 class="text-lg font-medium text-gray-900 mb-2">{"Error Loading Games"}</h3>
                            <p class="text-gray-500">{error_msg}</p>
                        </div>
                    } else if let Some(game_list) = &*games {
                        if game_list.is_empty() {
                            <div class="p-8 text-center">
                                <div class="text-gray-400 mb-4">
                                    <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 12h6m-6-4h6m2 5.291A7.962 7.962 0 0112 15c-2.34 0-4.29-1.009-5.824-2.571" />
                                    </svg>
                                </div>
                                <h3 class="text-lg font-medium text-gray-900 mb-2">{"No Games Found"}</h3>
                                <p class="text-gray-500">{"Try adjusting your search criteria"}</p>
                            </div>
                        } else {
                            <div class="overflow-x-auto">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {"Name"}
                                            </th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {"Year Published"}
                                            </th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {"BGG ID"}
                                            </th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {"Source"}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white divide-y divide-gray-200">
                                        {for game_list.iter().map(|game| {
                                            let game_id = game.id.clone();
                                            let navigator = navigator.clone();
                                            html! {
                                                <tr 
                                                    class="hover:bg-gray-50 cursor-pointer"
                                                    onclick={Callback::from(move |_| {
                                                        navigator.push(&Route::GameDetails { game_id: game_id.clone() });
                                                    })}
                                                >
                                                    <td class="px-6 py-4 whitespace-nowrap">
                                                        <div class="text-sm font-medium text-gray-900">
                                                            {&game.name}
                                                        </div>
                                                        if let Some(description) = &game.description {
                                                            <div class="text-sm text-gray-500 truncate max-w-xs">
                                                                {description}
                                                            </div>
                                                        }
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                        {game.year_published.map(|y| y.to_string()).unwrap_or_else(|| "Unknown".to_string())}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                        {game.bgg_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string())}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap">
                                                        <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                                                            {format!("{:?}", game.source)}
                                                        </span>
                                                    </td>
                                                </tr>
                                            }
                                        })}
                                    </tbody>
                                </table>
                            </div>
                        }
                    } else {
                        <div class="p-8 text-center">
                            <div class="text-gray-400 mb-4">
                                <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 12h6m-6-4h6m2 5.291A7.962 7.962 0 0112 15c-2.34 0-4.29-1.009-5.824-2.571" />
                                </svg>
                            </div>
                            <h3 class="text-lg font-medium text-gray-900 mb-2">{"No Games Loaded"}</h3>
                            <p class="text-gray-500">{"Click Search to load games"}</p>
                        </div>
                    }
                </div>
            </div>
        </div>
    }
} 