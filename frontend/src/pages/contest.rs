use yew::prelude::*;
use yew_router::prelude::*;
use shared::dto::contest::ContestDto;

use crate::Route;
use crate::components::contest::form::ContestForm;
use crate::components::contest::confirmation_modal::ContestConfirmationModal;
use crate::auth::AuthContext;
use crate::api::contests::submit_contest;
use crate::api::timezone::{resolve_timezone, resolve_timezone_by_place_id};
use shared::dto::venue::VenueDto;
use shared::dto::game::GameDto;
use shared::dto::contest::OutcomeDto;

use gloo_storage::{LocalStorage, Storage};
use serde::{Serialize, Deserialize};
use gloo::console::log;
use wasm_bindgen::prelude::*;
use crate::api::venues::get_venue_by_id;

#[wasm_bindgen(module = "/src/js/timezone.js")]
extern "C" {
    fn getBrowserIanaTimezone() -> String;
    fn getTimezoneOffsetForDate(tz: &str, iso_date: &str) -> String;
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
struct ContestFormState {
    start: chrono::DateTime<chrono::FixedOffset>,
    stop: chrono::DateTime<chrono::FixedOffset>,
    timezone: String,
    venue: Option<VenueDto>,
    games: Vec<GameDto>,
    outcomes: Vec<OutcomeDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum ContestFormAction {
    SetStart(chrono::DateTime<chrono::FixedOffset>),
    SetStop(chrono::DateTime<chrono::FixedOffset>),
    SetTimezone(String),
    SetVenue(Option<VenueDto>),
    SetGames(Vec<GameDto>),
    SetOutcomes(Vec<OutcomeDto>),
    Reset,
}

fn contest_form_reducer(state: &mut ContestFormState, action: ContestFormAction) {
    log!(format!("Reducer action: {:?}", &action));
    match action {

        ContestFormAction::SetStart(dt) => state.start = dt,
        ContestFormAction::SetStop(dt) => state.stop = dt,
        ContestFormAction::SetTimezone(tz) => state.timezone = tz,
        ContestFormAction::SetVenue(v) => state.venue = v,
        ContestFormAction::SetGames(g) => state.games = g,
        ContestFormAction::SetOutcomes(o) => state.outcomes = o,
        ContestFormAction::Reset => {
            log!("Reducer: RESET action");
            // Use UTC for storage of start/stop
            let now_utc = chrono::Utc::now().fixed_offset();
            let browser_timezone = match getBrowserIanaTimezone().as_str() {
                "" => {
                    log!("Warning: Could not detect browser timezone, using UTC as fallback");
                    "UTC".to_string()
                },
                tz => {
                    log!(format!("Reset - Detected browser timezone: {}", tz));
                    tz.to_string()
                }
            };
            *state = ContestFormState {
                start: now_utc - chrono::Duration::hours(1),
                stop: now_utc,
                timezone: browser_timezone,
                venue: None,
                games: vec![],
                outcomes: vec![],
            };
        }
    }
}

impl yew::Reducible for ContestFormState {
    type Action = ContestFormAction;
    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        let mut new = (*self).clone();
        contest_form_reducer(&mut new, action);
        std::rc::Rc::new(new)
    }
}



#[function_component(Contest)]
pub fn contest() -> Html {
    log!("ContestPage render");
    let navigator = use_navigator().unwrap();
    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let show_confirmation = use_state(|| false);
    let contest_data = use_state(|| None::<ContestDto>);
    let is_submitting = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    // Reducer for form state
    let reducer = {
        use_reducer_eq(|| {
            log!("Reducer INIT");
            let now_utc = chrono::Utc::now().fixed_offset();
            let browser_timezone = match getBrowserIanaTimezone().as_str() {
                "" => {
                    log!("Warning: Could not detect browser timezone, using UTC as fallback");
                    "UTC".to_string()
                },
                tz => {
                    log!(format!("Detected browser timezone: {}", tz));
                    tz.to_string()
                }
            };
            
            // Try to load from localStorage, but always override the timezone with detected browser timezone
            // AND clear any stale venue/games/outcomes data to ensure fresh lookups
            // Initialize from localStorage, but always override times to current UTC defaults
            let mut saved_state = LocalStorage::get::<ContestFormState>("contest_form_state").unwrap_or_else(|_| ContestFormState {
                start: (now_utc - chrono::Duration::hours(1)),
                stop: now_utc,
                timezone: browser_timezone.clone(),
                venue: None,
                games: vec![],
                outcomes: vec![],
            });

            // Always reset times to current UTC defaults on page load
            saved_state.stop = now_utc;
            saved_state.start = now_utc - chrono::Duration::hours(1);
            
            // Always update the timezone to the detected browser timezone
            if saved_state.timezone != browser_timezone {
                log!(format!("Updating timezone from '{}' to '{}'", saved_state.timezone, browser_timezone));
                saved_state.timezone = browser_timezone;
            }
            
            // Clear any stale venue/games/outcomes data to ensure fresh lookups
            // This prevents using stale IDs from different environments
            if saved_state.venue.is_some() || !saved_state.games.is_empty() || !saved_state.outcomes.is_empty() {
                log!("Clearing stale venue/games/outcomes data to ensure fresh lookups");
                saved_state.venue = None;
                saved_state.games = vec![];
                saved_state.outcomes = vec![];
            }
            
            saved_state
        })
    };

    // Persist to localStorage on every change, but only save time/timezone data
    // Don't persist venue/games/outcomes as they should always be fetched fresh
    {
        let reducer = reducer.clone();
        use_effect_with(reducer, move |reducer| {
            // Create a minimal state object with only time/timezone data
            let minimal_state = ContestFormState {
                start: reducer.start,
                stop: reducer.stop,
                timezone: reducer.timezone.clone(),
                venue: None, // Don't persist venue
                games: vec![], // Don't persist games
                outcomes: vec![], // Don't persist outcomes
            };
            let _ = LocalStorage::set("contest_form_state", &minimal_state);
            || ()
        })
    };

    // Cleanup session flag on unmount
    {
        use_effect_with((), move |_| {
            move || {
                // Clear session flag when component unmounts
                let _ = LocalStorage::set("user_selected_venue", false);
            }
        });
    }

    // Ensure timezone is always set to detected browser timezone (run only once on mount)
    {
        let reducer = reducer.clone();
        use_effect_with((), move |_| {
            // Only preload venue if no venue is currently selected AND user hasn't made a selection this session
            let user_already_selected = LocalStorage::get::<bool>("user_selected_venue").unwrap_or(false);
            let current_venue = (*reducer).venue.clone();
            
            if !user_already_selected && current_venue.is_none() {
                if let Ok(venue_id) = LocalStorage::get::<String>("last_selected_venue_id") {
                    let reducer = reducer.clone();
                    let id = venue_id.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        match get_venue_by_id(&id).await {
                            Ok(v) => {
                                // Double-check that no venue was selected while we were fetching
                                if (*reducer).venue.is_none() {
                                    log!(format!("Preloading last venue: {}", v.display_name));
                                    reducer.dispatch(ContestFormAction::SetVenue(Some(v.clone())));
                                    reducer.dispatch(ContestFormAction::SetTimezone(v.timezone));
                                } else {
                                    log!("Skipping preload - venue was selected while fetching");
                                }
                            },
                            Err(e) => log!(format!("Failed to preload last venue: {}", e)),
                        }
                    });
                }
            } else if user_already_selected {
                log!("Skipping venue preload - user has already made a selection this session");
            } else if current_venue.is_some() {
                log!("Skipping venue preload - venue is already selected");
            }
            
            let browser_timezone = getBrowserIanaTimezone();
            if !browser_timezone.is_empty() && browser_timezone != reducer.timezone {
                log!(format!("Effect: Updating timezone from '{}' to '{}'", reducer.timezone, browser_timezone));
                reducer.dispatch(ContestFormAction::SetTimezone(browser_timezone));
            }
            || ()
        });
    }

    let on_back = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            // Clear the session flag when navigating away
            let _ = LocalStorage::set("user_selected_venue", false);
            navigator.push(&Route::Home);
        })
    };

    // Timezone is now automatically set by venue selection
    let on_start_change = {
        let reducer = reducer.clone();
        Callback::from(move |dt: chrono::DateTime<chrono::FixedOffset>| {
            reducer.dispatch(ContestFormAction::SetStart(dt));
        })
    };
    let on_stop_change = {
        let reducer = reducer.clone();
        Callback::from(move |dt: chrono::DateTime<chrono::FixedOffset>| {
            reducer.dispatch(ContestFormAction::SetStop(dt));
        })
    };
    let on_venue_select = {
        let reducer = reducer.clone();
        Callback::from(move |v: VenueDto| {
            // Mark that the user actively selected a venue to suppress any late preloads
            let _ = gloo_storage::LocalStorage::set("user_selected_venue", true);
            log!(format!("on_venue_select called: {:?}", &v));
            log!(format!("Current state venue before update: {:?}", reducer.venue));
            
            // Set venue immediately and synchronously
            reducer.dispatch(ContestFormAction::SetVenue(Some(v.clone())));
            
            // Verify the venue was actually set
            let updated_venue = (*reducer).venue.clone();
            log!(format!("Current state venue after update: {:?}", updated_venue));
            
            if updated_venue.is_none() {
                log!("ERROR: Venue was not set in reducer state!");
                return;
            }
            
            // Database venues: use their stored timezone
            if v.source == shared::models::venue::VenueSource::Database {
                log!(format!("Database venue selected, using stored timezone: {}", v.timezone));
                reducer.dispatch(ContestFormAction::SetTimezone(v.timezone.clone()));
            }
            // Google venues: resolve timezone from place_id or coordinates
            else if v.source == shared::models::venue::VenueSource::Google {
                log!("Google venue selected, resolving timezone");
                log!(format!("Venue place_id: {}, coords: lat={}, lng={}", v.place_id, v.lat, v.lng));
                
                let reducer_for_resolve = reducer.clone();
                let place_id = v.place_id.clone();
                let lat = v.lat;
                let lng = v.lng;
                
                wasm_bindgen_futures::spawn_local(async move {
                    // Try place_id first
                    let mut tz_result = if !place_id.is_empty() {
                        log!(format!("Resolving timezone by place_id: {}", place_id));
                        resolve_timezone_by_place_id(&place_id).await
                    } else {
                        Err("Missing place_id".to_string())
                    };

                    // Fallback to coordinates if place_id lookup failed
                    if tz_result.is_err() && (lat != 0.0 || lng != 0.0) {
                        log!(format!("Place_id lookup failed; falling back to coords: lat={}, lng={}", lat, lng));
                        tz_result = resolve_timezone(lat, lng).await;
                    }

                    if let Ok(tz) = tz_result {
                        log!(format!("Frontend: Resolved timezone: {}", tz));
                        if let Some(mut updated_venue) = (*reducer_for_resolve).venue.clone() {
                            updated_venue.timezone = tz.clone();
                            reducer_for_resolve.dispatch(ContestFormAction::SetVenue(Some(updated_venue)));
                        }
                        reducer_for_resolve.dispatch(ContestFormAction::SetTimezone(tz));
                    } else {
                        log!("Failed to resolve timezone via place_id and coords; keeping existing timezone");
                    }
                });
            }
            // Fallback: keep browser timezone
            
            // Don't reset start/stop times - let flatpickr maintain stable values
            // Only change timezone, inputs will update their display automatically
            
            // Persist last selected venue id only for real DB venues with valid IDs
            if v.source == shared::models::venue::VenueSource::Database && !v.id.is_empty() && v.id.starts_with("venue/") {
                let _ = LocalStorage::set("last_selected_venue_id", v.id.clone());
            }
        })
    };
    let on_games_change = {
        let reducer = reducer.clone();
        Callback::from(move |g: Vec<GameDto>| {
            reducer.dispatch(ContestFormAction::SetGames(g));
        })
    };
    let on_outcomes_change = {
        let reducer = reducer.clone();
        Callback::from(move |o: Vec<OutcomeDto>| {
            reducer.dispatch(ContestFormAction::SetOutcomes(o));
        })
    };

    let on_contest_submit = {
        let show_confirmation = show_confirmation.clone();
        let contest_data = contest_data.clone();
        let state = reducer.clone();
        let error_message = error_message.clone();
        Callback::from(move |()| {
            // Clear any previous errors
            error_message.set(None);

            let show_confirmation = show_confirmation.clone();
            let contest_data = contest_data.clone();
            let state_for_submit = state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                log!(format!("Contest submit - current state venue: {:?}", state_for_submit.venue));
                let mut venue = state_for_submit.venue.clone().unwrap();
                // Ensure Google venues have a resolved timezone before proceeding
                if venue.source == shared::models::venue::VenueSource::Google && (venue.timezone.is_empty() || venue.timezone == "UTC") {
                    let place_id = venue.place_id.clone();
                    let lat = venue.lat;
                    let lng = venue.lng;
                    let mut tz_result = if !place_id.is_empty() {
                        resolve_timezone_by_place_id(&place_id).await
                    } else if lat != 0.0 || lng != 0.0 {
                        resolve_timezone(lat, lng).await
                    } else {
                        Err("No place_id or coordinates available".to_string())
                    };
                    if tz_result.is_err() && (lat != 0.0 || lng != 0.0) {
                        tz_result = resolve_timezone(lat, lng).await;
                    }
                    if let Ok(tz) = tz_result {
                        venue.timezone = tz;
                    }
                }

                let contest_dto = ContestDto {
                    id: format!("contest/{}", uuid::Uuid::new_v4()),
                    name: String::new(),
                    start: state_for_submit.start,
                    stop: state_for_submit.stop,
                    venue,
                    games: state_for_submit.games.clone(),
                    outcomes: state_for_submit.outcomes.clone(),
                    creator_id: String::new(),
                    created_at: None,
                };

                log!(format!("Submitting contest with venue: id='{}', name='{}', source='{:?}', tz='{}'",
                    contest_dto.venue.id, contest_dto.venue.display_name, contest_dto.venue.source, contest_dto.venue.timezone));

                contest_data.set(Some(contest_dto));
                show_confirmation.set(true);
            });
        })
    };

    let on_confirmation_cancel = {
        let show_confirmation = show_confirmation.clone();
        Callback::from(move |_| {
            show_confirmation.set(false);
        })
    };

    let on_confirmation_confirm = {
        let navigator = navigator.clone();
        let contest_data = contest_data.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();
        let dispatch = reducer.dispatcher();
        Callback::from(move |_| {
            if let Some(contest) = (*contest_data).clone() {
                is_submitting.set(true);
                error_message.set(None);
                let navigator = navigator.clone();
                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let dispatch = dispatch.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match submit_contest(contest).await {
                        Ok(_saved_contest) => {
                            is_submitting.set(false);
                            dispatch.dispatch(ContestFormAction::Reset);
                            let _ = LocalStorage::delete("contest_form_state");
                            navigator.push(&Route::Contests);
                        },
                        Err(err) => {
                            error_message.set(Some(format!("Failed to create contest: {}", err)));
                            is_submitting.set(false);
                        }
                    }
                });
            }
        })
    };

    html! {
        <div class="min-h-screen bg-gray-50">
            <header class="app-bar-material p-4">
                <div class="container mx-auto flex justify-between items-center">
                    <h1 class="text-xl font-medium">{"Create New Contest"}</h1>
                    <button
                        onclick={on_back}
                        class="btn-material-secondary"
                    >
                        {"Back to Home"}
                    </button>
                </div>
            </header>
            <main class="container mx-auto px-4 py-8">
                <div class="card-material p-6 max-w-4xl mx-auto">
                    if let Some(error) = (*error_message).clone() {
                        <div class="text-error-600 text-sm bg-error-50 p-3 rounded-material mb-4">
                            {error}
                        </div>
                    }
                    if auth.state.player.is_some() {
                        if *is_submitting {
                            <div class="text-center py-8">
                                <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
                                <p class="mt-2 text-gray-600">{"Creating contest..."}</p>
                            </div>
                        } else {
                            <ContestForm
                                start={(*reducer).start}
                                stop={(*reducer).stop}
                                timezone={(*reducer).timezone.clone()}
                                venue={(*reducer).venue.clone()}
                                games={(*reducer).games.clone()}
                                outcomes={(*reducer).outcomes.clone()}
                                on_start_change={on_start_change.clone()}
                                on_stop_change={on_stop_change.clone()}
                                on_venue_select={on_venue_select.clone()}
                                on_games_change={on_games_change.clone()}
                                on_outcomes_change={on_outcomes_change.clone()}
                                on_submit={on_contest_submit.clone()}
                                locked={false}
                            />
                            <ContestConfirmationModal
                                contest={(*contest_data).clone()}
                                is_open={*show_confirmation}
                                on_confirm={on_confirmation_confirm}
                                on_cancel={on_confirmation_cancel.clone()}
                                on_edit={on_confirmation_cancel}
                            />
                        }
                    } else {
                        <div class="text-center text-gray-600 py-8">
                            {"You must be logged in to create a contest."}
                        </div>
                    }
                </div>
            </main>
        </div>
    }
} 
