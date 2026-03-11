use crate::api::venues::{get_all_venues, search_venues};
use crate::Route;
use shared::VenueDto;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(Venues)]
pub fn venues() -> Html {
    let navigator = use_navigator().unwrap();
    let draft_query = use_state(|| String::new());
    let query = use_state(|| String::new());
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let results = use_state(|| Vec::<VenueDto>::new());

    let perform_search = {
        let query = query.clone();
        let loading = loading.clone();
        let error = error.clone();
        let results = results.clone();
        Callback::from(move |_| {
            let q = (*query).clone();
            let loading = loading.clone();
            let error = error.clone();
            let results = results.clone();
            loading.set(true);
            error.set(None);
            wasm_bindgen_futures::spawn_local(async move {
                let resp = if q.trim().is_empty() {
                    get_all_venues().await
                } else {
                    search_venues(&q).await
                };
                match resp {
                    Ok(list) => {
                        results.set(list);
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

    // Initial load
    {
        let perform_search = perform_search.clone();
        use_effect_with((), move |_| {
            perform_search.emit(());
        });
    }

    let on_query_input = {
        let draft_query = draft_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            draft_query.set(input.value());
        })
    };

    let apply_filters = {
        let draft_query = draft_query.clone();
        let query = query.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |_| {
            query.set((*draft_query).clone());
            perform_search.emit(());
        })
    };

    let clear_filters = {
        let draft_query = draft_query.clone();
        let query = query.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |_| {
            draft_query.set(String::new());
            query.set(String::new());
            perform_search.emit(());
        })
    };

    let remove_query_chip = {
        let draft_query = draft_query.clone();
        let query = query.clone();
        let perform_search = perform_search.clone();
        Callback::from(move |_| {
            draft_query.set(String::new());
            query.set(String::new());
            perform_search.emit(());
        })
    };

    let active_filter_count = if query.trim().is_empty() { 0 } else { 1 };

    html! {
        <div class="min-h-screen bg-gray-50">
            <header class="app-bar-material p-4 sticky top-0 z-40 bg-white shadow-sm">
                <div class="container mx-auto flex justify-between items-center flex-wrap gap-3">
                    <h1 class="text-xl font-medium">{"Venues"}</h1>
                </div>
            </header>

            <main class="container mx-auto px-4 py-6">
                <div class="bg-white rounded-lg shadow-sm p-4 mb-6">
                    <div class="flex flex-col md:flex-row gap-4">
                        <div class="flex-1">
                            <input
                                type="text"
                                placeholder="Search venues..."
                                value={(*draft_query).clone()}
                                oninput={on_query_input}
                                class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            />
                        </div>
                        <div class="flex gap-2">
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

                    if active_filter_count > 0 {
                        <div class="mt-3 flex flex-wrap gap-2">
                            <span class="inline-flex items-center gap-1 px-2 py-1 bg-gray-100 text-gray-800 text-xs rounded-full">
                                {format!("Query: {}", (*query).clone())}
                                <button onclick={remove_query_chip.reform(|_| ())} class="ml-1 text-gray-500 hover:text-gray-700">{"✕"}</button>
                            </span>
                        </div>
                    }
                </div>

                if let Some(err) = &*error {
                    <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
                        <div class="flex">
                            <div class="text-red-400">{"⚠️"}</div>
                            <div class="ml-3">
                                <h3 class="text-sm font-medium text-red-800">{"Error"}</h3>
                                <div class="mt-1 text-sm text-red-700">{err}</div>
                            </div>
                        </div>
                    </div>
                } else {
                    <div class="bg-white rounded-lg shadow-sm overflow-hidden">
                        <div class="overflow-x-auto">
                            <table class="min-w-full divide-y divide-gray-200">
                                <thead class="bg-gray-50">
                                    <tr>
                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Name"}</th>
                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Address"}</th>
                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Timezone"}</th>
                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Source"}</th>
                                    </tr>
                                </thead>
                                <tbody class="bg-white divide-y divide-gray-200">
                                    {for results.iter().map(|v| {
                                        let venue_id = v.id.clone();
                                        let navigator = navigator.clone();
                                        html!{
                                            <tr
                                                class="hover:bg-gray-50 cursor-pointer"
                                                onclick={Callback::from(move |_| {
                                                    navigator.push(&Route::VenueDetails { venue_id: venue_id.clone() });
                                                })}
                                            >
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{&v.display_name}</td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{v.formatted_address.clone()}</td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{v.timezone.clone()}</td>
                                                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{format!("{:?}", v.source)}</td>
                                            </tr>
                                        }
                                    })}
                                </tbody>
                            </table>
                        </div>
                        if results.is_empty() && !*loading {
                            <div class="p-8 text-center text-gray-500">{"No venues found"}</div>
                        }
                    </div>
                }
            </main>
        </div>
    }
}
