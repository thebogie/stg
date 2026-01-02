use yew::prelude::*;
use yew_router::prelude::*;
use crate::Route;
use crate::api::venues::{get_venue_by_id, update_venue};
use crate::api::contests::{search_contests, ContestSearchResponse};
use crate::auth::AuthContext;
use shared::VenueDto;
use chrono::DateTime;

#[derive(Properties, PartialEq)]
pub struct VenueDetailsProps {
    pub venue_id: String,
}

#[function_component(VenueDetails)]
pub fn venue_details(props: &VenueDetailsProps) -> Html {
    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let navigator = use_navigator().unwrap();
    let venue = use_state(|| None::<VenueDto>);
    let contests = use_state(|| None::<ContestSearchResponse>);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let editing = use_state(|| false);
    let edit_form = use_state(|| VenueDto {
        id: String::new(),
        display_name: String::new(),
        formatted_address: String::new(),
        place_id: String::new(),
        lat: 0.0,
        lng: 0.0,
        timezone: String::new(),
        source: shared::models::venue::VenueSource::Database,
    });
    let saving = use_state(|| false);

    // Navigation callbacks
    let on_back = {
        let nav = navigator.clone();
        Callback::from(move |_| nav.back())
    };
    let on_view_all_contests = {
        let nav = navigator.clone();
        Callback::from(move |_| nav.push(&Route::Contests))
    };
    let on_to_venues = {
        let nav = navigator.clone();
        Callback::from(move |_| nav.push(&Route::Venues))
    };

    // Load venue details
    {
        let venue_id = props.venue_id.clone();
        let venue = venue.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
            let venue_id = venue_id.clone();
            let venue = venue.clone();
            let loading = loading.clone();
            let error = error.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match get_venue_by_id(&venue_id).await {
                    Ok(venue_data) => {
                        venue.set(Some(venue_data));
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        loading.set(false);
                    }
                }
            });
        });
    }

    // Load contests for this venue
    {
        let venue_id = props.venue_id.clone();
        let contests = contests.clone();
        use_effect_with((), move |_| {
            let venue_id = venue_id.clone();
            let contests = contests.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let params = vec![("venue_id", venue_id), ("scope", "all".to_string())];
                match search_contests(&params).await {
                    Ok(results) => {
                        contests.set(Some(results));
                    }
                    Err(_) => {
                        contests.set(Some(ContestSearchResponse {
                            items: vec![],
                            total: 0,
                            page: 1,
                            page_size: 20,
                        }));
                    }
                }
            });
        });
    }

    let on_create_contest = {
        let nav_create = navigator.clone();
        Callback::from(move |_| {
            nav_create.push(&Route::Contest);
        })
    };

    // Edit functionality
    let on_start_edit = {
        let venue = venue.clone();
        let edit_form = edit_form.clone();
        let editing = editing.clone();
        Callback::from(move |_| {
            if let Some(venue_data) = &*venue {
                edit_form.set(venue_data.clone());
                editing.set(true);
            }
        })
    };

    let on_cancel_edit = {
        let editing = editing.clone();
        Callback::from(move |_| {
            editing.set(false);
        })
    };

    let on_save_edit = {
        let venue_id = props.venue_id.clone();
        let edit_form = edit_form.clone();
        let venue = venue.clone();
        let editing = editing.clone();
        let saving = saving.clone();
        let error = error.clone();
        Callback::from(move |_| {
            let venue_id = venue_id.clone();
            let edit_form = edit_form.clone();
            let venue = venue.clone();
            let editing = editing.clone();
            let saving = saving.clone();
            let error = error.clone();
            
            saving.set(true);
            error.set(None);
            
            wasm_bindgen_futures::spawn_local(async move {
                match update_venue(&venue_id, (*edit_form).clone()).await {
                    Ok(updated_venue) => {
                        venue.set(Some(updated_venue));
                        editing.set(false);
                        saving.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        saving.set(false);
                    }
                }
            });
        })
    };

    // Form input handlers
    let on_display_name_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut form = (*edit_form).clone();
            form.display_name = input.value();
            edit_form.set(form);
        })
    };

    let on_address_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut form = (*edit_form).clone();
            form.formatted_address = input.value();
            edit_form.set(form);
        })
    };

    let on_timezone_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut form = (*edit_form).clone();
            form.timezone = input.value();
            edit_form.set(form);
        })
    };

    let on_lat_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(lat) = input.value().parse::<f64>() {
                let mut form = (*edit_form).clone();
                form.lat = lat;
                edit_form.set(form);
            }
        })
    };

    let on_lng_change = {
        let edit_form = edit_form.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(lng) = input.value().parse::<f64>() {
                let mut form = (*edit_form).clone();
                form.lng = lng;
                edit_form.set(form);
            }
        })
    };

    let format_time = |time_str: &str| {
        if let Ok(utc_time) = DateTime::parse_from_rfc3339(time_str) {
            utc_time.format("%d/%m/%Y %H:%M").to_string()
        } else {
            time_str.to_string()
        }
    };

    if *loading {
        return html! {
            <div class="min-h-screen bg-gray-50 flex items-center justify-center">
                <div class="text-center">
                    <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
                    <p class="text-gray-600">{"Loading venue details..."}</p>
                </div>
            </div>
        };
    }

    if let Some(err) = &*error {
        return html! {
            <div class="min-h-screen bg-gray-50">
                <header class="app-bar-material p-4 sticky top-0 z-40 bg-white shadow-sm">
                    <div class="container mx-auto flex justify-between items-center">
                        <button
                            onclick={on_back.clone()}
                            class="flex items-center gap-2 text-gray-600 hover:text-gray-900"
                        >
                            <span>{"←"}</span>
                            <span>{"Back"}</span>
                        </button>
                        <h1 class="text-xl font-medium">{"Venue Details"}</h1>
                        <div></div>
                    </div>
                </header>
                <main class="container mx-auto px-4 py-6">
                    <div class="bg-red-50 border border-red-200 rounded-lg p-4">
                        <div class="flex">
                            <div class="text-red-400">{"⚠️"}</div>
                            <div class="ml-3">
                                <h3 class="text-sm font-medium text-red-800">{"Error"}</h3>
                                <div class="mt-1 text-sm text-red-700">{err}</div>
                            </div>
                        </div>
                    </div>
                </main>
            </div>
        };
    }

    if let Some(venue_data) = &*venue {
        let contests_data = contests.as_ref().map(|c| c.items.len()).unwrap_or(0);
        let total_contests = contests.as_ref().map(|c| c.total).unwrap_or(0);

        html! {
            <div class="min-h-screen bg-gray-50">
                <header class="app-bar-material p-4 sticky top-0 z-40 bg-white shadow-sm">
                    <div class="container mx-auto flex justify-between items-center">
                        <button
                            onclick={on_back.clone()}
                            class="flex items-center gap-2 text-gray-600 hover:text-gray-900"
                        >
                            <span>{"←"}</span>
                            <span>{"Back"}</span>
                        </button>
                        <h1 class="text-xl font-medium">{&venue_data.display_name}</h1>
                        <div class="flex gap-2">
                            if auth.state.player.as_ref().map_or(false, |p| p.is_admin) {
                                if *editing {
                                    <button
                                        onclick={on_save_edit.clone()}
                                        disabled={*saving}
                                        class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 text-sm"
                                    >
                                        {if *saving { "Saving..." } else { "Save" }}
                                    </button>
                                    <button
                                        onclick={on_cancel_edit.clone()}
                                        disabled={*saving}
                                        class="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 disabled:opacity-50 text-sm"
                                    >
                                        {"Cancel"}
                                    </button>
                                } else {
                                    <button
                                        onclick={on_start_edit.clone()}
                                        class="px-4 py-2 bg-yellow-600 text-white rounded-lg hover:bg-yellow-700 text-sm"
                                    >
                                        {"Edit Venue"}
                                    </button>
                                }
                            }
                            <button
                                onclick={on_create_contest.clone()}
                                class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm"
                            >
                                {"Create Contest"}
                            </button>
                        </div>
                    </div>
                </header>

                <main class="container mx-auto px-4 py-6">
                    // Overview Section
                    <div class="bg-white rounded-lg shadow-sm p-6 mb-6">
                        <h2 class="text-2xl font-bold text-gray-900 mb-4">{"Overview"}</h2>
                        if *editing {
                            <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-4 mb-4">
                                <div class="flex">
                                    <div class="text-yellow-400">{"✏️"}</div>
                                    <div class="ml-3">
                                        <h3 class="text-sm font-medium text-yellow-800">{"Editing Mode"}</h3>
                                        <div class="mt-1 text-sm text-yellow-700">{"You are editing this venue. Changes will be saved to the database."}</div>
                                    </div>
                                </div>
                            </div>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div>
                                    <h3 class="text-lg font-semibold text-gray-800 mb-2">{"Venue Information"}</h3>
                                    <div class="space-y-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">{"Display Name"}</label>
                                            <input
                                                type="text"
                                                value={edit_form.display_name.clone()}
                                                oninput={on_display_name_change.clone()}
                                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">{"Address"}</label>
                                            <input
                                                type="text"
                                                value={edit_form.formatted_address.clone()}
                                                oninput={on_address_change.clone()}
                                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">{"Timezone"}</label>
                                            <input
                                                type="text"
                                                value={edit_form.timezone.clone()}
                                                oninput={on_timezone_change.clone()}
                                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                                placeholder="e.g., America/New_York"
                                            />
                                        </div>
                                        <div class="flex">
                                            <span class="font-medium text-gray-600 w-24">{"Source:"}</span>
                                            <span class="text-gray-900">{format!("{:?}", edit_form.source)}</span>
                                        </div>
                                    </div>
                                </div>
                                <div>
                                    <h3 class="text-lg font-semibold text-gray-800 mb-2">{"Location"}</h3>
                                    <div class="space-y-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">{"Latitude"}</label>
                                            <input
                                                type="number"
                                                step="any"
                                                value={edit_form.lat.to_string()}
                                                oninput={on_lat_change.clone()}
                                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">{"Longitude"}</label>
                                            <input
                                                type="number"
                                                step="any"
                                                value={edit_form.lng.to_string()}
                                                oninput={on_lng_change.clone()}
                                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                            />
                                        </div>
                                        <div class="mt-4">
                                            <div class="bg-gray-100 rounded-lg p-4 text-center">
                                                <div class="text-sm text-gray-600 mb-2">{"Location Link"}</div>
                                                <a
                                                    href={format!("https://www.google.com/maps/search/?api=1&query={},{}", edit_form.lat, edit_form.lng)}
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                    class="inline-flex items-center px-3 py-2 text-sm font-medium text-white bg-blue-600 rounded hover:bg-blue-700"
                                                >
                                                    {"Open in Google Maps"}
                                                </a>
                                                <div class="text-xs text-gray-500 mt-2">
                                                    {format!("📍 {:.4}, {:.4}", edit_form.lat, edit_form.lng)}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        } else {
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div>
                                    <h3 class="text-lg font-semibold text-gray-800 mb-2">{"Venue Information"}</h3>
                                    <div class="space-y-2">
                                        <div class="flex">
                                            <span class="font-medium text-gray-600 w-24">{"Name:"}</span>
                                            <span class="text-gray-900">{&venue_data.display_name}</span>
                                        </div>
                                        <div class="flex">
                                            <span class="font-medium text-gray-600 w-24">{"Address:"}</span>
                                            <span class="text-gray-900">{&venue_data.formatted_address}</span>
                                        </div>
                                        <div class="flex">
                                            <span class="font-medium text-gray-600 w-24">{"Timezone:"}</span>
                                            <span class="text-gray-900">{&venue_data.timezone}</span>
                                        </div>
                                        <div class="flex">
                                            <span class="font-medium text-gray-600 w-24">{"Source:"}</span>
                                            <span class="text-gray-900">{format!("{:?}", venue_data.source)}</span>
                                        </div>
                                    </div>
                                </div>
                                <div>
                                    <h3 class="text-lg font-semibold text-gray-800 mb-2">{"Location"}</h3>
                                    <div class="space-y-2">
                                        <div class="flex">
                                            <span class="font-medium text-gray-600 w-16">{"Lat:"}</span>
                                            <span class="text-gray-900">{format!("{:.6}", venue_data.lat)}</span>
                                        </div>
                                        <div class="flex">
                                            <span class="font-medium text-gray-600 w-16">{"Lng:"}</span>
                                            <span class="text-gray-900">{format!("{:.6}", venue_data.lng)}</span>
                                        </div>
                                        <div class="mt-4">
                                            <div class="bg-gray-100 rounded-lg p-4 text-center">
                                                <div class="text-sm text-gray-600 mb-2">{"Location Link"}</div>
                                                <a
                                                    href={format!("https://www.google.com/maps/search/?api=1&query={},{}", venue_data.lat, venue_data.lng)}
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                    class="inline-flex items-center px-3 py-2 text-sm font-medium text-white bg-blue-600 rounded hover:bg-blue-700"
                                                >
                                                    {"Open in Google Maps"}
                                                </a>
                                                <div class="text-xs text-gray-500 mt-2">
                                                    {format!("📍 {:.4}, {:.4}", venue_data.lat, venue_data.lng)}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }
                    </div>

                    // Stats Section
                    <div class="bg-white rounded-lg shadow-sm p-6 mb-6">
                        <h2 class="text-2xl font-bold text-gray-900 mb-4">{"Statistics"}</h2>
                        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                            <div class="bg-blue-50 rounded-lg p-4 text-center">
                                <div class="text-3xl font-bold text-blue-600">{total_contests}</div>
                                <div class="text-sm text-blue-800">{"Total Contests"}</div>
                            </div>
                            <div class="bg-green-50 rounded-lg p-4 text-center">
                                <div class="text-3xl font-bold text-green-600">{contests_data}</div>
                                <div class="text-sm text-green-800">{"Recent Contests"}</div>
                            </div>
                            <div class="bg-purple-50 rounded-lg p-4 text-center">
                                <div class="text-3xl font-bold text-purple-600">{"—"}</div>
                                <div class="text-sm text-purple-800">{"Last Activity"}</div>
                            </div>
                        </div>
                    </div>

                    // Recent Contests Section
                    <div class="bg-white rounded-lg shadow-sm p-6">
                        <div class="flex justify-between items-center mb-4">
                            <h2 class="text-2xl font-bold text-gray-900">{"Recent Contests"}</h2>
                            <button
                                onclick={on_view_all_contests.clone()}
                                class="text-blue-600 hover:text-blue-800 text-sm font-medium"
                            >
                                {"View All Contests →"}
                            </button>
                        </div>
                        
                        if let Some(contests_data) = &*contests {
                            if contests_data.items.is_empty() {
                                <div class="text-center py-8">
                                    <div class="text-gray-400 mb-4">
                                        <svg class="mx-auto h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
                                        </svg>
                                    </div>
                                    <h3 class="text-lg font-medium text-gray-900 mb-2">{"No Contests Yet"}</h3>
                                    <p class="text-gray-500 mb-4">{"This venue hasn't hosted any contests yet."}</p>
                                    <button
                                        onclick={on_create_contest.clone()}
                                        class="inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                                    >
                                        <span class="mr-2">{"🏆"}</span>
                                        {"Create First Contest"}
                                    </button>
                                </div>
                            } else {
                                <div class="overflow-x-auto">
                                    <table class="min-w-full divide-y divide-gray-200">
                                        <thead class="bg-gray-50">
                                            <tr>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Name"}</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Start"}</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"End"}</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Games"}</th>
                                            </tr>
                                        </thead>
                                        <tbody class="bg-white divide-y divide-gray-200">
                                            {for contests_data.items.iter().take(10).map(|contest| {
                                                let contest_id = contest.id.clone();
                                                let nav_row = navigator.clone();
                                                html! {
                                                    <tr 
                                                        class="hover:bg-gray-50 cursor-pointer"
                                                        onclick={Callback::from(move |_| {
                                                            nav_row.push(&Route::ContestDetails { contest_id: contest_id.clone() });
                                                        })}
                                                    >
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">{&contest.name}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{format_time(&contest.start)}</td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{format_time(&contest.stop)}</td>
                                                        <td class="px-6 py-4 text-sm text-gray-500">
                                                            <div class="flex flex-wrap gap-1">
                                                                {for contest.games.iter().take(3).map(|game| {
                                                                    if let Some(name) = game.get("name").and_then(|v| v.as_str()) {
                                                                        html! {
                                                                            <span class="inline-flex px-2 py-1 text-xs font-medium bg-blue-100 text-blue-800 rounded">
                                                                                {name}
                                                                            </span>
                                                                        }
                                                                    } else {
                                                                        html! {}
                                                                    }
                                                                })}
                                                                {if contest.games.len() > 3 {
                                                                    html! {
                                                                        <span class="inline-flex px-2 py-1 text-xs font-medium bg-gray-100 text-gray-800 rounded">
                                                                            {format!("+{}", contest.games.len() - 3)}
                                                                        </span>
                                                                    }
                                                                } else {
                                                                    html! {}
                                                                }}
                                                            </div>
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
                                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-4"></div>
                                <p class="text-gray-600">{"Loading contests..."}</p>
                            </div>
                        }
                    </div>
                </main>
            </div>
        }
    } else {
        html! {
            <div class="min-h-screen bg-gray-50 flex items-center justify-center">
                <div class="text-center">
                    <div class="text-red-400 mb-4">{"❌"}</div>
                    <h3 class="text-lg font-medium text-gray-900 mb-2">{"Venue Not Found"}</h3>
                    <p class="text-gray-500 mb-4">{"The requested venue could not be found."}</p>
                    <button
                        onclick={on_to_venues.clone()}
                        class="inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                    >
                        {"← Back to Venues"}
                    </button>
                </div>
            </div>
        }
    }
}
