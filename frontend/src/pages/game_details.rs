use crate::api::contests::search_contests;
use crate::api::contests::ContestSearchResponse;
use crate::api::games::{find_similar_games, get_game_by_id, merge_games, update_game};
use crate::auth::AuthContext;
use crate::Route;
use shared::dto::game::GameDto;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct GameDetailsProps {
    pub game_id: String,
}

#[function_component(GameDetails)]
pub fn game_details(props: &GameDetailsProps) -> Html {
    let auth_context = use_context::<AuthContext>().expect("AuthContext not found");
    let navigator = use_navigator().unwrap();

    // State
    let game = use_state(|| None::<GameDto>);
    let contests = use_state(|| None::<ContestSearchResponse>);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let editing = use_state(|| false);
    let edit_form = use_state(|| None::<GameDto>);
    let saving = use_state(|| false);

    // Admin state
    let similar_games = use_state(|| None::<Vec<GameDto>>);
    let similar_loading = use_state(|| false);
    let show_merge = use_state(|| false);
    let merge_target = use_state(|| None::<String>);
    let merging = use_state(|| false);

    // Navigation callbacks
    let on_back = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.back();
        })
    };

    let on_view_all_contests = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&Route::Contests);
        })
    };

    let on_to_games = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&Route::Games);
        })
    };

    let on_create_contest = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&Route::Contest);
        })
    };

    // Edit callbacks
    let on_start_edit = {
        let game = game.clone();
        let edit_form = edit_form.clone();
        let editing = editing.clone();
        Callback::from(move |_| {
            if let Some(game_data) = &*game {
                edit_form.set(Some(game_data.clone()));
                editing.set(true);
            }
        })
    };

    let on_cancel_edit = {
        let editing = editing.clone();
        let edit_form = edit_form.clone();
        Callback::from(move |_| {
            editing.set(false);
            edit_form.set(None);
        })
    };

    let on_save_edit = {
        let game_id = props.game_id.clone();
        let edit_form = edit_form.clone();
        let game = game.clone();
        let editing = editing.clone();
        let saving = saving.clone();
        let error = error.clone();
        Callback::from(move |_| {
            if let Some(form_data) = (*edit_form).clone() {
                saving.set(true);
                error.set(None);

                let game_id = game_id.clone();
                let game = game.clone();
                let editing = editing.clone();
                let edit_form = edit_form.clone();
                let saving = saving.clone();
                let error = error.clone();

                spawn_local(async move {
                    match update_game(&game_id, form_data).await {
                        Ok(updated_game) => {
                            game.set(Some(updated_game));
                            editing.set(false);
                            edit_form.set(None);
                            error.set(None);
                        }
                        Err(e) => {
                            error.set(Some(e));
                        }
                    }
                    saving.set(false);
                });
            }
        })
    };

    // Admin callbacks

    let on_toggle_merge = {
        let show_merge = show_merge.clone();
        let similar_games = similar_games.clone();
        let similar_loading = similar_loading.clone();
        let game_id = props.game_id.clone();
        Callback::from(move |_| {
            let show = !*show_merge;
            show_merge.set(show);

            if show && similar_games.is_none() {
                similar_loading.set(true);
                let similar_games = similar_games.clone();
                let similar_loading = similar_loading.clone();
                let game_id = game_id.clone();

                spawn_local(async move {
                    match find_similar_games(&game_id).await {
                        Ok(games) => {
                            similar_games.set(Some(games));
                        }
                        Err(e) => {
                            log::error!("Failed to find similar games: {}", e);
                        }
                    }
                    similar_loading.set(false);
                });
            }
        })
    };

    let on_select_merge_target = {
        let merge_target = merge_target.clone();
        Callback::from(move |game_id: String| {
            merge_target.set(Some(game_id));
        })
    };

    let on_confirm_merge = {
        let game_id = props.game_id.clone();
        let merge_target = merge_target.clone();
        let merging = merging.clone();
        let navigator = navigator.clone();
        Callback::from(move |_: yew::MouseEvent| {
            if let Some(target_id) = &*merge_target {
                merging.set(true);
                let game_id = game_id.clone();
                let target_id = target_id.clone();
                let merging = merging.clone();
                let navigator = navigator.clone();

                spawn_local(async move {
                    match merge_games(&game_id, &target_id).await {
                        Ok(_merged_game) => {
                            // Redirect to the target game after successful merge
                            navigator.push(&Route::GameDetails { game_id: target_id });
                        }
                        Err(e) => {
                            log::error!("Failed to merge games: {}", e);
                        }
                    }
                    merging.set(false);
                });
            }
        })
    };

    let on_cancel_merge = {
        let show_merge = show_merge.clone();
        let merge_target = merge_target.clone();
        Callback::from(move |_| {
            show_merge.set(false);
            merge_target.set(None);
        })
    };

    // Form input handlers
    let on_name_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(mut form) = (*edit_form).clone() {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                form.name = input.value();
                edit_form.set(Some(form));
            }
        })
    };

    let on_year_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(mut form) = (*edit_form).clone() {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                form.year_published = input.value().parse().ok();
                edit_form.set(Some(form));
            }
        })
    };

    let on_bgg_id_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(mut form) = (*edit_form).clone() {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                form.bgg_id = input.value().parse().ok();
                edit_form.set(Some(form));
            }
        })
    };

    let on_description_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(mut form) = (*edit_form).clone() {
                let input: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                form.description = Some(input.value());
                edit_form.set(Some(form));
            }
        })
    };

    // Load game data
    {
        let game_id = props.game_id.clone();
        let game = game.clone();
        let loading = loading.clone();
        let error = error.clone();

        use_effect_with((), move |_| {
            let game_id = game_id.clone();
            let game = game.clone();
            let loading = loading.clone();
            let error = error.clone();

            spawn_local(async move {
                match get_game_by_id(&game_id).await {
                    Ok(game_data) => {
                        game.set(Some(game_data));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        game.set(None);
                    }
                }
                loading.set(false);
            });
        });
    }

    // Load contests for this game
    {
        let game_id = props.game_id.clone();
        let contests = contests.clone();

        use_effect_with((), move |_| {
            let game_id = game_id.clone();
            let contests = contests.clone();

            spawn_local(async move {
                let params = vec![("game_ids", game_id.clone()), ("scope", "all".to_string())];

                match search_contests(&params).await {
                    Ok(contest_data) => {
                        contests.set(Some(contest_data));
                    }
                    Err(e) => {
                        log::error!("Failed to load contests for game {}: {}", game_id, e);
                    }
                }
            });
        });
    }

    // Check if user is admin
    let is_admin = auth_context
        .state
        .player
        .as_ref()
        .map(|p| p.is_admin)
        .unwrap_or(false);

    html! {
        <div class="min-h-screen bg-gray-50">
            <div class="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
                // Header
                <div class="mb-8">
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-4">
                            <button
                                onclick={on_back}
                                class="inline-flex items-center px-3 py-2 border border-gray-300 shadow-sm text-sm leading-4 font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                            >
                                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                                </svg>
                                {"Back"}
                            </button>
                            <div>
                                <h1 class="text-3xl font-bold text-gray-900">
                                    {if let Some(game_data) = &*game { &game_data.name } else { "Loading..." }}
                                </h1>
                                <p class="mt-2 text-gray-600">{"Game Details"}</p>
                            </div>
                        </div>
                        <div class="flex space-x-3">
                            <button
                                onclick={on_create_contest.clone()}
                                class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                            >
                                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                                </svg>
                                {"Create Contest"}
                            </button>
                            if is_admin && !*editing {
                                <button
                                    onclick={on_start_edit}
                                    class="inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md shadow-sm text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                                >
                                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                                    </svg>
                                    {"Edit Game"}
                                </button>
                                <button
                                    onclick={on_toggle_merge}
                                    class="inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md shadow-sm text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                                >
                                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                                    </svg>
                                    {"Merge Games"}
                                </button>
                            }
                        </div>
                    </div>
                </div>

                if *loading {
                    <div class="text-center py-12">
                        <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                        <p class="mt-2 text-gray-600">{"Loading game details..."}</p>
                    </div>
                } else if let Some(error_msg) = &*error {
                    <div class="bg-red-50 border border-red-200 rounded-md p-4">
                        <div class="flex">
                            <div class="flex-shrink-0">
                                <svg class="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
                                </svg>
                            </div>
                            <div class="ml-3">
                                <h3 class="text-sm font-medium text-red-800">{"Error"}</h3>
                                <div class="mt-2 text-sm text-red-700">
                                    <p>{error_msg}</p>
                                </div>
                            </div>
                        </div>
                    </div>
                } else if let Some(game_data) = &*game {
                    <div class="space-y-6">
                        // Editing banner
                        if *editing {
                            <div class="bg-yellow-50 border border-yellow-200 rounded-md p-4">
                                <div class="flex">
                                    <div class="flex-shrink-0">
                                        <svg class="h-5 w-5 text-yellow-400" viewBox="0 0 20 20" fill="currentColor">
                                            <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
                                        </svg>
                                    </div>
                                    <div class="ml-3">
                                        <h3 class="text-sm font-medium text-yellow-800">{"Editing Mode"}</h3>
                                        <div class="mt-2 text-sm text-yellow-700">
                                            <p>{"You are editing this game. Changes will be saved to the database."}</p>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }

                        // Game Overview
                        <div class="bg-white shadow rounded-lg p-6">
                            <h2 class="text-lg font-medium text-gray-900 mb-4">{"Game Overview"}</h2>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-2">{"Name"}</label>
                                    if *editing {
                                        <input
                                            type="text"
                                            value={if let Some(form) = &*edit_form { form.name.clone() } else { game_data.name.clone() }}
                                            oninput={on_name_change}
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                                        />
                                    } else {
                                        <p class="text-sm text-gray-900">{&game_data.name}</p>
                                    }
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-2">{"Year Published"}</label>
                                    if *editing {
                                        <input
                                            type="number"
                                            value={if let Some(form) = &*edit_form { form.year_published.map(|y| y.to_string()).unwrap_or_default() } else { game_data.year_published.map(|y| y.to_string()).unwrap_or_default() }}
                                            oninput={on_year_change}
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                                        />
                                    } else {
                                        <p class="text-sm text-gray-900">
                                            {game_data.year_published.map(|y| y.to_string()).unwrap_or_else(|| "Unknown".to_string())}
                                        </p>
                                    }
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-2">{"BGG ID"}</label>
                                    if *editing {
                                        <input
                                            type="number"
                                            value={if let Some(form) = &*edit_form { form.bgg_id.map(|id| id.to_string()).unwrap_or_default() } else { game_data.bgg_id.map(|id| id.to_string()).unwrap_or_default() }}
                                            oninput={on_bgg_id_change}
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                                        />
                                    } else {
                                        <p class="text-sm text-gray-900">
                                            {game_data.bgg_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string())}
                                        </p>
                                    }
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-2">{"Source"}</label>
                                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                                        {format!("{:?}", game_data.source)}
                                    </span>
                                </div>
                            </div>

                            <div class="mt-6">
                                <label class="block text-sm font-medium text-gray-700 mb-2">{"Description"}</label>
                                if *editing {
                                    <textarea
                                        rows="4"
                                        value={if let Some(form) = &*edit_form { form.description.clone().unwrap_or_default() } else { game_data.description.clone().unwrap_or_default() }}
                                        oninput={on_description_change}
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                                    />
                                } else {
                                    <p class="text-sm text-gray-900">
                                        {game_data.description.as_ref().unwrap_or(&"No description available".to_string())}
                                    </p>
                                }
                            </div>

                            // Edit buttons
                            if *editing {
                                <div class="mt-6 flex justify-end space-x-3">
                                    <button
                                        onclick={on_cancel_edit}
                                        class="px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                                    >
                                        {"Cancel"}
                                    </button>
                                    <button
                                        onclick={on_save_edit}
                                        disabled={*saving}
                                        class="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
                                    >
                                        {if *saving { "Saving..." } else { "Save Changes" }}
                                    </button>
                                </div>
                            }
                        </div>

                        // Statistics
                        <div class="bg-white shadow rounded-lg p-6">
                            <h2 class="text-lg font-medium text-gray-900 mb-4">{"Statistics"}</h2>
                            <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                                <div class="text-center">
                                    <div class="text-2xl font-bold text-blue-600">
                                        {contests.as_ref().map(|c| c.total).unwrap_or(0)}
                                    </div>
                                    <div class="text-sm text-gray-500">{"Total Contests"}</div>
                                </div>
                                <div class="text-center">
                                    <div class="text-2xl font-bold text-green-600">
                                        {contests.as_ref().map(|c| c.items.len()).unwrap_or(0)}
                                    </div>
                                    <div class="text-sm text-gray-500">{"Recent Contests"}</div>
                                </div>
                                <div class="text-center">
                                    <div class="text-2xl font-bold text-purple-600">
                                        {game_data.year_published.map(|y| (2024 - y).to_string()).unwrap_or_else(|| "?".to_string())}
                                    </div>
                                    <div class="text-sm text-gray-500">{"Years Old"}</div>
                                </div>
                            </div>
                        </div>

                        // Recent Contests
                        <div class="bg-white shadow rounded-lg p-6">
                            <div class="flex items-center justify-between mb-4">
                                <h2 class="text-lg font-medium text-gray-900">{"Recent Contests"}</h2>
                                <button
                                    onclick={on_view_all_contests}
                                    class="text-sm text-blue-600 hover:text-blue-500"
                                >
                                    {"View All Contests"}
                                </button>
                            </div>

                            if let Some(contest_data) = &*contests {
                                if contest_data.items.is_empty() {
                                    <div class="text-center py-8">
                                        <div class="text-gray-400 mb-4">
                                            <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                            </svg>
                                        </div>
                                        <h3 class="text-lg font-medium text-gray-900 mb-2">{"No Contests Found"}</h3>
                                        <p class="text-gray-500 mb-4">{"This game hasn't been played in any contests yet."}</p>
                                        <button
                                            onclick={on_create_contest}
                                            class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                                        >
                                            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                                            </svg>
                                            {"Create First Contest"}
                                        </button>
                                    </div>
                                } else {
                                    <div class="overflow-x-auto">
                                        <table class="min-w-full divide-y divide-gray-200">
                                            <thead class="bg-gray-50">
                                                <tr>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                        {"Contest"}
                                                    </th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                        {"Start Time"}
                                                    </th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                        {"End Time"}
                                                    </th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                                        {"Venue"}
                                                    </th>
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white divide-y divide-gray-200">
                                                {for contest_data.items.iter().take(10).map(|contest| {
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
                                                                <div class="text-sm font-medium text-gray-900">
                                                                    {&contest.name}
                                                                </div>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                                {&contest.start}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                                {&contest.stop}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                                                                {contest.venue.as_ref().and_then(|v| v.get("display_name")).and_then(|v| v.as_str()).unwrap_or("Unknown Venue")}
                                                            </td>
                                                        </tr>
                                                    }
                                                })}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                            } else {
                                <div class="text-center py-8">
                                    <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                                    <p class="mt-2 text-gray-600">{"Loading contests..."}</p>
                                </div>
                            }
                        </div>



                        // Admin Merge Games Section
                        if is_admin && *show_merge {
                            <div class="bg-white shadow rounded-lg p-6">
                                <div class="flex items-center justify-between mb-4">
                                    <h2 class="text-lg font-medium text-gray-900">{"🔄 Merge Duplicate Games"}</h2>
                                    <button
                                        onclick={&on_cancel_merge}
                                        class="text-sm text-gray-500 hover:text-gray-700"
                                    >
                                        {"Cancel"}
                                    </button>
                                </div>

                                <div class="bg-yellow-50 border border-yellow-200 rounded-md p-4 mb-6">
                                    <div class="flex">
                                        <div class="flex-shrink-0">
                                            <svg class="h-5 w-5 text-yellow-400" viewBox="0 0 20 20" fill="currentColor">
                                                <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
                                            </svg>
                                        </div>
                                        <div class="ml-3">
                                            <h3 class="text-sm font-medium text-yellow-800">{"Warning"}</h3>
                                            <div class="mt-2 text-sm text-yellow-700">
                                                <p>{"Merging games will move all contests from the source game to the target game and delete the source game. This action cannot be undone."}</p>
                                            </div>
                                        </div>
                                    </div>
                                </div>

                                if *similar_loading {
                                    <div class="text-center py-8">
                                        <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                                        <p class="mt-2 text-gray-600">{"Finding similar games..."}</p>
                                    </div>
                                } else if let Some(similar_games_list) = &*similar_games {
                                    if similar_games_list.is_empty() {
                                        <div class="text-center py-8">
                                            <div class="text-gray-400 mb-4">
                                                <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 12h6m-6-4h6m2 5.291A7.962 7.962 0 0112 15c-2.34 0-4.29-1.009-5.824-2.571" />
                                                </svg>
                                            </div>
                                            <h3 class="text-lg font-medium text-gray-900 mb-2">{"No Similar Games Found"}</h3>
                                            <p class="text-gray-500">{"No potential duplicate games were found for this game."}</p>
                                        </div>
                                    } else {
                                        <div class="space-y-4">
                                            <h3 class="text-sm font-medium text-gray-900">{"Select a game to merge into:"}</h3>
                                            <div class="space-y-2">
                                                {for similar_games_list.iter().map(|similar_game| {
                                                    let game_id = similar_game.id.clone();
                                                    let is_selected = merge_target.as_ref().map(|target| target == &game_id).unwrap_or(false);
                                                    let value = on_select_merge_target.clone();
                                                    html! {
                                                        <div
                                                            class={if is_selected {
                                                                "border-2 border-blue-500 bg-blue-50 rounded-lg p-4 cursor-pointer"
                                                            } else {
                                                                "border border-gray-200 rounded-lg p-4 cursor-pointer hover:bg-gray-50"
                                                            }}
                                                            onclick={Callback::from(move |_| value.emit(game_id.clone()))}
                                                        >
                                                            <div class="flex items-center justify-between">
                                                                <div>
                                                                    <div class="text-sm font-medium text-gray-900">
                                                                        {&similar_game.name}
                                                                    </div>
                                                                    <div class="text-sm text-gray-500">
                                                                        {if let Some(year) = similar_game.year_published {
                                                                            format!("Published: {}", year)
                                                                        } else {
                                                                            "Year unknown".to_string()
                                                                        }}
                                                                        {if let Some(bgg_id) = similar_game.bgg_id {
                                                                            format!(" • BGG ID: {}", bgg_id)
                                                                        } else {
                                                                            "".to_string()
                                                                        }}
                                                                    </div>
                                                                </div>
                                                                if is_selected {
                                                                    <svg class="w-5 h-5 text-blue-500" fill="currentColor" viewBox="0 0 20 20">
                                                                        <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
                                                                    </svg>
                                                                }
                                                            </div>
                                                        </div>
                                                    }
                                                })}
                                            </div>

                                            if merge_target.is_some() {
                                                <div class="flex justify-end space-x-3 pt-4 border-t">
                                                    <button
                                                        onclick={&on_cancel_merge}
                                                        class="px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                                                    >
                                                        {"Cancel"}
                                                    </button>
                                                    <button
                                                        onclick={on_confirm_merge}
                                                        disabled={*merging}
                                                        class="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500 disabled:opacity-50"
                                                    >
                                                        { if *merging { "Merging..." } else { "Confirm Merge" } }
                                                    </button>
                                                </div>
                                            }
                                        </div>
                                    }
                                } else {
                                    <div class="text-center py-8">
                                        <div class="text-gray-400 mb-4">
                                            <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                                            </svg>
                                        </div>
                                        <h3 class="text-lg font-medium text-gray-900 mb-2">{"Ready to Find Duplicates"}</h3>
                                        <p class="text-gray-500">{"Click the button above to search for similar games."}</p>
                                    </div>
                                }
                            </div>
                        }
                    </div>
                } else {
                    <div class="text-center py-12">
                        <div class="text-gray-400 mb-4">
                            <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 12h6m-6-4h6m2 5.291A7.962 7.962 0 0112 15c-2.34 0-4.29-1.009-5.824-2.571" />
                            </svg>
                        </div>
                        <h3 class="text-lg font-medium text-gray-900 mb-2">{"Game Not Found"}</h3>
                        <p class="text-gray-500 mb-4">{"The requested game could not be found."}</p>
                        <button
                            onclick={on_to_games}
                            class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                        >
                            {"Back to Games"}
                        </button>
                    </div>
                }
            </div>
        </div>
    }
}
