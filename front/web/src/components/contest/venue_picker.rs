use crate::api::venues::{get_all_venues, search_venues_for_create};
use gloo::timers::callback::Timeout;
use shared::dto::venue::VenueDto;
use shared::models::venue::VenueSource;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct VenuePickerProps {
    pub on_venue_select: Callback<VenueDto>,
    pub initial_venue: Option<VenueDto>,
}

#[function_component(VenuePicker)]
pub fn venue_picker(props: &VenuePickerProps) -> Html {
    let props = props.clone();
    let search_query = use_state(String::new);
    let venue_suggestions = use_state(Vec::<VenueDto>::new);
    let show_suggestions = use_state(|| false);
    let is_searching = use_state(|| false);
    let selected_venue = use_state(|| None::<VenueDto>);
    let search_error = use_state(|| None::<String>);
    let debounce_handle = use_mut_ref(|| None::<Timeout>);
    let all_venues = use_state(Vec::<VenueDto>::new);
    let is_interacting_suggestions = use_state(|| false);

    // Load all venues on component mount
    {
        let all_venues = all_venues.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match get_all_venues().await {
                    Ok(venues) => {
                        gloo::console::log!("Loaded {} total venues", venues.len());
                        all_venues.set(venues);
                    }
                    Err(e) => {
                        gloo::console::error!("Failed to load all venues:", e);
                    }
                }
            });
            || ()
        });
    }

    // Keep local selected_venue in sync with parent-provided initial_venue
    use_effect_with(props.initial_venue.clone(), {
        let selected_venue = selected_venue.clone();
        let search_query = search_query.clone();
        move |initial: &Option<VenueDto>| {
            if let Some(v) = initial.clone() {
                gloo::console::log!(format!(
                    "VenuePicker: Updating from initial_venue: {}",
                    v.display_name
                ));
                selected_venue.set(Some(v.clone()));
                search_query.set(v.display_name.clone());
            } else {
                gloo::console::log!("VenuePicker: Clearing venue from initial_venue");
                selected_venue.set(None);
                search_query.set(String::new());
            }
            || ()
        }
    });

    let on_search = {
        let search_query = search_query.clone();
        let venue_suggestions = venue_suggestions.clone();
        let show_suggestions = show_suggestions.clone();
        let is_searching = is_searching.clone();
        let selected_venue = selected_venue.clone();
        let search_error = search_error.clone();
        let debounce_handle = debounce_handle.clone();

        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let query = input.value();
            search_query.set(query.clone());

            // Clear selected venue when user starts typing
            selected_venue.set(None);
            search_error.set(None);

            if query.is_empty() {
                venue_suggestions.set(Vec::new());
                show_suggestions.set(false);
                return;
            }

            show_suggestions.set(true);
            // Debounce: clear previous timeout
            if let Some(handle) = debounce_handle.borrow_mut().take() {
                handle.cancel();
            }
            let venue_suggestions = venue_suggestions.clone();
            let is_searching = is_searching.clone();
            let search_error = search_error.clone();
            let query_clone = query.clone();
            debounce_handle
                .borrow_mut()
                .replace(Timeout::new(700, move || {
                    is_searching.set(true);
                    let query_for_log = query_clone.clone();
                    let query_for_search = query_clone.clone();
                    gloo::console::log!("Searching for venues with query: '{}'", query_for_log);
                    spawn_local(async move {
                        match search_venues_for_create(&query_for_search).await {
                            Ok(venues) => {
                                gloo::console::log!("Search returned {} venues", venues.len());
                                venue_suggestions.set(venues);
                            }
                            Err(e) => {
                                let error_msg = e.clone();
                                gloo::console::error!("Failed to fetch venues:", error_msg);
                                venue_suggestions.set(Vec::new());
                                search_error.set(Some(e));
                            }
                        }
                        is_searching.set(false);
                    });
                }));
        })
    };

    let on_venue_click = {
        let props = props.clone();
        let show_suggestions = show_suggestions.clone();
        let search_query = search_query.clone();
        let selected_venue = selected_venue.clone();
        let search_error = search_error.clone();

        Callback::from(move |venue: VenueDto| {
            gloo::console::log!(format!(
                "VenuePicker: User selected venue: {}",
                venue.display_name
            ));
            props.on_venue_select.emit(venue.clone());
            search_query.set(venue.display_name.clone());
            selected_venue.set(Some(venue));
            show_suggestions.set(false);
            search_error.set(None);
        })
    };

    let on_input_focus = {
        let show_suggestions = show_suggestions.clone();
        let venue_suggestions = venue_suggestions.clone();

        Callback::from(move |_| {
            if !venue_suggestions.is_empty() {
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

    html! {
        <div class="relative">
            <div class="space-y-2">
                <label class="block text-sm font-medium text-gray-700">
                    {"Search Venue"}
                </label>
                <div class="relative">
                    <input
                        type="text"
                        placeholder="Search for a venue..."
                        value={(*search_query).clone()}
                        oninput={on_search}
                        onfocus={on_input_focus}
                        onblur={on_input_blur}
                        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    />
                    if *is_searching {
                        <div class="absolute inset-y-0 right-0 flex items-center pr-3">
                            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
                        </div>
                    }
                </div>
                <div class="flex items-center justify-between">
                    <span class="text-xs text-gray-500">
                        {"Total venues in DB: "}{all_venues.len()}
                    </span>
                </div>
            </div>

            if *show_suggestions {
                <div
                    class="absolute z-50 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-auto mobile-scroll"
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
                        <div class="px-3 py-4 text-center text-gray-500">
                            <div class="flex items-center justify-center space-x-2">
                                <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
                                <span>{"Searching venues..."}</span>
                            </div>
                        </div>
                    } else if let Some(error) = &*search_error {
                        <div class="px-3 py-4 text-center text-red-600">
                            <span>{"Error: "}{error}</span>
                        </div>
                    } else if venue_suggestions.is_empty() && !search_query.is_empty() {
                        <div class="px-3 py-4 text-center text-gray-500">
                            <span>{"No venues found"}</span>
                        </div>
                    } else {
                        <ul class="py-1">
                            {venue_suggestions.iter().map(|venue| {
                                let venue = venue.clone();
                                let on_click = {
                                    let on_venue_click = on_venue_click.clone();
                                    let venue = venue.clone();
                                    Callback::from(move |_| on_venue_click.emit(venue.clone()))
                                };

                                // Determine styling based on source
                                let (bg_class, text_class, border_class) = match venue.source {
                                    VenueSource::Database => (
                                        "hover:bg-blue-50",
                                        "text-gray-900",
                                        "border-l-4 border-l-blue-500"
                                    ),
                                    VenueSource::Google => (
                                        "hover:bg-yellow-50",
                                        "text-gray-700",
                                        "border-l-4 border-l-yellow-500"
                                    ),
                                };

                                html! {
                                    <li
                                        class={format!("px-3 py-2 cursor-pointer transition-colors duration-150 {} {} {}", bg_class, text_class, border_class)}
                                        onclick={on_click}
                                    >
                                        <div class="flex flex-col">
                                            <div class="flex items-center justify-between">
                                                <span class="font-medium truncate">
                                                    {&venue.display_name}
                                                </span>
                                                <span class={format!("text-xs px-2 py-1 rounded-full {}",
                                                    match venue.source {
                                                        VenueSource::Database => "bg-blue-100 text-blue-800",
                                                        VenueSource::Google => "bg-yellow-100 text-yellow-800",
                                                    }
                                                )}>
                                                    {match venue.source {
                                                        VenueSource::Database => "Database",
                                                        VenueSource::Google => "Google",
                                                    }}
                                                </span>
                                            </div>
                                            <span class="text-sm text-gray-500 truncate mt-1">
                                                {&venue.formatted_address}
                                            </span>
                                        </div>
                                    </li>
                                }
                            }).collect::<Html>()}
                        </ul>
                    }
                </div>
            }

            if let Some(selected) = &*selected_venue {
                <div class="mt-3 p-3 bg-blue-50 border border-blue-200 rounded-md">
                    <div class="flex items-start justify-between">
                        <div class="flex-1">
                            <div class="flex items-center justify-between">
                                <h4 class="text-sm font-medium text-blue-900">
                                    {"Selected Venue"}
                                </h4>
                                <span class={format!("text-xs px-2 py-1 rounded-full {}",
                                    match selected.source {
                                        VenueSource::Database => "bg-blue-100 text-blue-800",
                                        VenueSource::Google => "bg-yellow-100 text-yellow-800",
                                    }
                                )}>
                                    {match selected.source {
                                        VenueSource::Database => "Database",
                                        VenueSource::Google => "Google",
                                    }}
                                </span>
                            </div>
                            <p class="text-sm text-blue-800 mt-1">
                                {&selected.display_name}
                            </p>
                            <p class="text-xs text-blue-600 mt-1">
                                {&selected.formatted_address}
                            </p>
                        </div>
                    </div>
                </div>
            }
        </div>
    }
}
