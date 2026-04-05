use crate::api::players::search_players;
use crate::Route;
use shared::dto::player::PlayerDto;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::use_navigator;

const DIRECTORY_LIMIT: u32 = 50;
const SEARCH_LIMIT: u32 = 50;

#[function_component(Players)]
pub fn players() -> Html {
    let navigator = use_navigator().unwrap();

    let draft_query = use_state(|| String::new());
    let query = use_state(|| String::new());
    let players_state = use_state(|| None::<Vec<PlayerDto>>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);

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
        let players_state = players_state.clone();

        Callback::from(move |_| {
            let search_query = (*draft_query).clone();
            query.set(search_query.clone());

            loading.set(true);
            error.set(None);

            let loading = loading.clone();
            let error = error.clone();
            let players_state = players_state.clone();

            spawn_local(async move {
                let q = search_query.trim();
                let limit = if q.is_empty() {
                    DIRECTORY_LIMIT
                } else {
                    SEARCH_LIMIT
                };
                let result = search_players(q, limit).await;

                loading.set(false);
                match result {
                    Ok(list) => {
                        players_state.set(Some(list));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        players_state.set(None);
                    }
                }
            });
        })
    };

    let clear_filters = {
        let draft_query = draft_query.clone();
        let query = query.clone();
        let players_state = players_state.clone();
        let error = error.clone();
        let loading = loading.clone();

        Callback::from(move |_| {
            draft_query.set(String::new());
            query.set(String::new());
            error.set(None);

            loading.set(true);
            let loading = loading.clone();
            let players_state = players_state.clone();
            let error = error.clone();

            spawn_local(async move {
                match search_players("", DIRECTORY_LIMIT).await {
                    Ok(list) => {
                        players_state.set(Some(list));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        players_state.set(None);
                    }
                }
                loading.set(false);
            });
        })
    };

    {
        let loading = loading.clone();
        let players_state = players_state.clone();
        let error = error.clone();

        use_effect_with((), move |_| {
            loading.set(true);
            error.set(None);

            let loading = loading.clone();
            let players_state = players_state.clone();
            let error = error.clone();

            spawn_local(async move {
                match search_players("", DIRECTORY_LIMIT).await {
                    Ok(list) => {
                        players_state.set(Some(list));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        players_state.set(None);
                    }
                }
                loading.set(false);
            });

            || ()
        });
    }

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
                    <h1 class="text-3xl font-bold text-gray-900">{"👥 Players"}</h1>
                    <p class="mt-2 text-gray-600">
                        {"Browse community members or search by handle, name, or email. Open a row to view their public profile."}
                    </p>
                </div>

                <div class="bg-white shadow rounded-lg p-6 mb-6">
                    <div class="flex flex-col sm:flex-row gap-4">
                        <div class="flex-1">
                            <label for="player-search" class="block text-sm font-medium text-gray-700 mb-2">
                                {"Search players"}
                            </label>
                            <input
                                id="player-search"
                                type="text"
                                placeholder="Leave empty to browse from A–Z, or type to filter..."
                                value={(*draft_query).clone()}
                                oninput={on_query_change}
                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                            />
                        </div>
                        <div class="flex items-end gap-2">
                            <button
                                type="button"
                                onclick={apply_filters}
                                class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                            >
                                {"Search"}
                            </button>
                            <button
                                type="button"
                                onclick={clear_filters}
                                class="px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                            >
                                {"Clear"}
                            </button>
                        </div>
                    </div>

                    {filter_chips}
                </div>

                <div class="bg-white shadow rounded-lg">
                    if *loading {
                        <div class="p-8 text-center">
                            <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                            <p class="mt-2 text-gray-600">{"Loading players..."}</p>
                        </div>
                    } else if let Some(error_msg) = &*error {
                        <div class="p-8 text-center">
                            <div class="text-red-600 mb-2">
                                <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
                                </svg>
                            </div>
                            <h3 class="text-lg font-medium text-gray-900 mb-2">{"Could not load players"}</h3>
                            <p class="text-gray-500">{error_msg}</p>
                        </div>
                    } else if let Some(player_list) = &*players_state {
                        if player_list.is_empty() {
                            <div class="p-8 text-center">
                                <h3 class="text-lg font-medium text-gray-900 mb-2">{"No players found"}</h3>
                                <p class="text-gray-500">{"Try a different search term."}</p>
                            </div>
                        } else {
                            <div class="overflow-x-auto">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {"Handle"}
                                            </th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {"Display name"}
                                            </th>
                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                {"Profile"}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white divide-y divide-gray-200">
                                        {for player_list.iter().map(|player| {
                                            let player_id = player.id.clone();
                                            let navigator = navigator.clone();
                                            html! {
                                                <tr
                                                    class="hover:bg-gray-50 cursor-pointer"
                                                    onclick={Callback::from(move |_| {
                                                        navigator.push(&Route::PlayerProfile { player_id: player_id.clone() });
                                                    })}
                                                >
                                                    <td class="px-6 py-4 whitespace-nowrap">
                                                        <div class="text-sm font-medium text-gray-900">{&player.handle}</div>
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                        {&player.firstname}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-blue-600 font-medium">
                                                        {"View →"}
                                                    </td>
                                                </tr>
                                            }
                                        })}
                                    </tbody>
                                </table>
                            </div>
                        }
                    } else {
                        <div class="p-8 text-center text-gray-500">{"No data"}</div>
                    }
                </div>
            </div>
        </div>
    }
}
