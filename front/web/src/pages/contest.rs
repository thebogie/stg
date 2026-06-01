use shared::dto::contest::ContestDto;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::contests::{submit_contest, upload_contest_image};
use crate::api::timezone::{resolve_timezone, resolve_timezone_by_place_id};
use crate::auth::AuthContext;
use crate::components::contest::confirmation_modal::ContestConfirmationModal;
use crate::components::contest::form::ContestForm;
use crate::components::contest::venue_picker::VENUE_SEARCH_TOUCHED_STORAGE_KEY;
use crate::Route;
use shared::dto::contest::OutcomeDto;
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;

use crate::api::venues::get_venue_by_id;
use gloo::console::log;
use gloo_storage::{LocalStorage, SessionStorage, Storage};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Set after successful submit; contests list reads once and shows a one-time notice.
pub const CONTEST_SUBMITTED_MODERATION_FLASH: &str = "stg_contest_submitted_moderation";

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
    /// Applies venue and contest timezone together (avoids stale reducer reads after async resolve).
    SetVenueWithTimezone {
        venue: VenueDto,
        timezone: String,
    },
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
        ContestFormAction::SetVenueWithTimezone { venue, timezone } => {
            state.timezone = timezone;
            state.venue = Some(venue);
        }
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
                }
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

/// Timezone for flatpickr and the Time & Date chip. Prefer the venue once it has a real IANA id
/// (Google search results use `UTC` as a placeholder until `/api/timezone/resolve` completes).
fn contest_timezone_for_form(venue: &Option<VenueDto>, reducer_tz: &str) -> String {
    match venue {
        Some(v) => {
            let tz = v.timezone.trim();
            let is_google_placeholder = v.source == shared::models::venue::VenueSource::Google
                && (tz.is_empty() || tz.eq_ignore_ascii_case("utc"));
            if is_google_placeholder {
                reducer_tz.to_string()
            } else if !tz.is_empty() {
                v.timezone.clone()
            } else {
                reducer_tz.to_string()
            }
        }
        None => reducer_tz.to_string(),
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
    let pending_image = use_state(|| None::<(Vec<u8>, String)>);
    let image_preview_url = use_state(|| None::<String>);
    let image_error = use_state(|| None::<String>);
    let error_message = use_state(|| None::<String>);

    // Reducer for form state — use `use_reducer` (not `use_reducer_eq`): the contest form must
    // re-render on every dispatch so async venue timezone resolution updates the Time & Date chip.
    let reducer = {
        use_reducer(|| {
            log!("Reducer INIT");
            let now_utc = chrono::Utc::now().fixed_offset();
            let browser_timezone = match getBrowserIanaTimezone().as_str() {
                "" => {
                    log!("Warning: Could not detect browser timezone, using UTC as fallback");
                    "UTC".to_string()
                }
                tz => {
                    log!(format!("Detected browser timezone: {}", tz));
                    tz.to_string()
                }
            };

            // Try to load from localStorage, but always override the timezone with detected browser timezone
            // AND clear any stale venue/games/outcomes data to ensure fresh lookups
            // Initialize from localStorage, but always override times to current UTC defaults
            let mut saved_state = LocalStorage::get::<ContestFormState>("contest_form_state")
                .unwrap_or_else(|_| ContestFormState {
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
                log!(format!(
                    "Updating timezone from '{}' to '{}'",
                    saved_state.timezone, browser_timezone
                ));
                saved_state.timezone = browser_timezone;
            }

            // Clear any stale venue/games/outcomes data to ensure fresh lookups
            // This prevents using stale IDs from different environments
            if saved_state.venue.is_some()
                || !saved_state.games.is_empty()
                || !saved_state.outcomes.is_empty()
            {
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
                venue: None,      // Don't persist venue
                games: vec![],    // Don't persist games
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

    // Venue preload from last DB pick (once on mount). Do not force browser timezone here; the
    // contest timezone must follow the selected venue after `/api/timezone/resolve`.
    {
        let reducer = reducer.clone();
        use_effect_with((), move |_| {
            let _ = LocalStorage::set(VENUE_SEARCH_TOUCHED_STORAGE_KEY, false);
            // Only preload venue if no venue is currently selected AND user hasn't made a selection this session
            let user_already_selected =
                LocalStorage::get::<bool>("user_selected_venue").unwrap_or(false);
            let current_venue = (*reducer).venue.clone();

            if !user_already_selected && current_venue.is_none() {
                if let Ok(venue_id) = LocalStorage::get::<String>("last_selected_venue_id") {
                    let reducer = reducer.clone();
                    let id = venue_id.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        match get_venue_by_id(&id).await {
                            Ok(v) => {
                                let search_touched =
                                    LocalStorage::get::<bool>(VENUE_SEARCH_TOUCHED_STORAGE_KEY)
                                        .unwrap_or(false);
                                // Double-check that no venue was selected while we were fetching
                                if (*reducer).venue.is_none() && !search_touched {
                                    log!(format!("Preloading last venue: {}", v.display_name));
                                    reducer.dispatch(ContestFormAction::SetVenue(Some(v.clone())));
                                    reducer.dispatch(ContestFormAction::SetTimezone(v.timezone));
                                } else {
                                    log!("Skipping preload - venue was selected while fetching or user edited search");
                                }
                            }
                            Err(e) => log!(format!("Failed to preload last venue: {}", e)),
                        }
                    });
                }
            } else if user_already_selected {
                log!("Skipping venue preload - user has already made a selection this session");
            } else if current_venue.is_some() {
                log!("Skipping venue preload - venue is already selected");
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
    let on_venue_clear = {
        let reducer = reducer.clone();
        Callback::from(move |_| {
            reducer.dispatch(ContestFormAction::SetVenue(None));
        })
    };

    let on_venue_select = {
        let reducer = reducer.clone();
        Callback::from(move |v: VenueDto| {
            // Mark that the user actively selected a venue to suppress any late preloads
            let _ = gloo_storage::LocalStorage::set("user_selected_venue", true);
            log!(format!("on_venue_select called: {:?}", &v));
            log!(format!(
                "Current state venue before update: {:?}",
                reducer.venue
            ));

            // Set venue immediately so the UI reflects the selection while timezone resolves.
            reducer.dispatch(ContestFormAction::SetVenue(Some(v.clone())));

            // Database venues: use their stored timezone
            if v.source == shared::models::venue::VenueSource::Database {
                log!(format!(
                    "Database venue selected, using stored timezone: {}",
                    v.timezone
                ));
                reducer.dispatch(ContestFormAction::SetTimezone(v.timezone.clone()));
            }
            // Google venues: resolve timezone from place_id or coordinates
            else if v.source == shared::models::venue::VenueSource::Google {
                log!("Google venue selected, resolving timezone");
                log!(format!(
                    "Venue place_id: {}, coords: lat={}, lng={}",
                    v.place_id, v.lat, v.lng
                ));

                let reducer_for_resolve = reducer.clone();
                let venue_for_tz = v.clone();
                let place_id = venue_for_tz.place_id.clone();
                let lat = venue_for_tz.lat;
                let lng = venue_for_tz.lng;

                wasm_bindgen_futures::spawn_local(async move {
                    let mut venue = venue_for_tz;
                    // Try place_id first
                    let mut tz_result = if !place_id.is_empty() {
                        log!(format!("Resolving timezone by place_id: {}", place_id));
                        resolve_timezone_by_place_id(&place_id).await
                    } else {
                        Err("Missing place_id".to_string())
                    };

                    // Fallback to coordinates if place_id lookup failed
                    if tz_result.is_err() && (lat != 0.0 || lng != 0.0) {
                        log!(format!(
                            "Place_id lookup failed; falling back to coords: lat={}, lng={}",
                            lat, lng
                        ));
                        tz_result = resolve_timezone(lat, lng).await;
                    }

                    if let Ok(tz) = tz_result {
                        log!(format!("Frontend: Resolved timezone: {}", tz));
                        // Do not read `(*reducer_for_resolve).venue` after `await`: `UseReducerHandle`
                        // derefs to a cached `Rc` from the last render, not live hook state, so a
                        // "still the same venue?" check always sees stale data and skipped dispatch.
                        venue.timezone = tz.clone();
                        reducer_for_resolve.dispatch(ContestFormAction::SetVenueWithTimezone {
                            venue,
                            timezone: tz,
                        });
                    } else {
                        log!("Failed to resolve timezone via place_id and coords; keeping existing timezone");
                    }
                });
            }
            // Fallback: keep browser timezone

            // Don't reset start/stop times - let flatpickr maintain stable values
            // Only change timezone, inputs will update their display automatically

            // Persist last selected venue id only for real DB venues with valid IDs
            if v.source == shared::models::venue::VenueSource::Database
                && !v.id.is_empty()
                && v.id.starts_with("venue/")
            {
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

    let on_image_pick = {
        let pending_image = pending_image.clone();
        let image_preview_url = image_preview_url.clone();
        let image_error = image_error.clone();
        Callback::from(move |pick: Option<Result<(Vec<u8>, String), String>>| {
            if let Some(prev) = (*image_preview_url).clone() {
                let _ = web_sys::Url::revoke_object_url(&prev);
            }
            match pick {
                None => {
                    pending_image.set(None);
                    image_preview_url.set(None);
                    image_error.set(None);
                }
                Some(Ok((bytes, mime))) => {
                    image_error.set(None);
                    pending_image.set(Some((bytes.clone(), mime.clone())));
                    let parts = js_sys::Array::new();
                    parts.push(&js_sys::Uint8Array::from(bytes.as_slice()).into());
                    let type_ = mime;
                    let bag = web_sys::BlobPropertyBag::new();
                    bag.set_type(&type_);
                    if let Ok(blob) =
                        web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &bag)
                    {
                        if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                            image_preview_url.set(Some(url));
                        }
                    } else if let Ok(blob) =
                        web_sys::Blob::new_with_u8_array_sequence(&parts)
                    {
                        if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                            image_preview_url.set(Some(url));
                        }
                    }
                }
                Some(Err(e)) => {
                    pending_image.set(None);
                    image_preview_url.set(None);
                    image_error.set(Some(e));
                }
            }
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
                log!(format!(
                    "Contest submit - current state venue: {:?}",
                    state_for_submit.venue
                ));
                let mut venue = state_for_submit.venue.clone().unwrap();
                // Ensure Google venues have a resolved timezone before proceeding
                if venue.source == shared::models::venue::VenueSource::Google
                    && (venue.timezone.is_empty() || venue.timezone == "UTC")
                {
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
                    creator_handle: None,
                    created_at: None,
                    moderation_status: String::new(),
                    moderated_at: None,
                    moderated_by: None,
                    moderation_note: None,
                    has_image: false,
                    image_url: None,
                    image_detail_url: None,
                };

                log!(format!(
                    "Submitting contest with venue: id='{}', name='{}', source='{:?}', tz='{}'",
                    contest_dto.venue.id,
                    contest_dto.venue.display_name,
                    contest_dto.venue.source,
                    contest_dto.venue.timezone
                ));

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
        let reducer = reducer.clone();
        let is_submitting = is_submitting.clone();
        let error_message = error_message.clone();
        let dispatch = reducer.dispatcher();
        let pending_image = pending_image.clone();
        let image_preview_url = image_preview_url.clone();
        let image_error = image_error.clone();
        Callback::from(move |_| {
            if let Some(mut contest) = (*contest_data).clone() {
                // Use latest form state (scores/outcomes may change after opening review).
                let state = (*reducer).clone();
                contest.start = state.start;
                contest.stop = state.stop;
                contest.games = state.games.clone();
                contest.outcomes = state.outcomes.clone();
                if let Some(venue) = state.venue.clone() {
                    contest.venue = venue;
                }
                is_submitting.set(true);
                error_message.set(None);
                let navigator = navigator.clone();
                let is_submitting = is_submitting.clone();
                let error_message = error_message.clone();
                let dispatch = dispatch.clone();
                let pending_image = pending_image.clone();
                let image_preview_url = image_preview_url.clone();
                let image_error = image_error.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let image_upload = (*pending_image).clone();
                    match submit_contest(contest).await {
                        Ok(saved) => {
                            if let Some((bytes, mime)) = image_upload {
                                if let Err(e) =
                                    upload_contest_image(&saved.id, bytes, &mime).await
                                {
                                    error_message.set(Some(format!(
                                        "Contest created, but thumbnail upload failed: {}",
                                        e
                                    )));
                                    is_submitting.set(false);
                                    return;
                                }
                            }
                            if let Some(prev) = (*image_preview_url).clone() {
                                let _ = web_sys::Url::revoke_object_url(&prev);
                            }
                            pending_image.set(None);
                            image_preview_url.set(None);
                            image_error.set(None);
                            is_submitting.set(false);
                            dispatch.dispatch(ContestFormAction::Reset);
                            let _ = LocalStorage::delete("contest_form_state");
                            let _ = SessionStorage::set(CONTEST_SUBMITTED_MODERATION_FLASH, "1");
                            navigator.push(&Route::Contests);
                            if let Some(w) = web_sys::window() {
                                let _ = w.scroll_to_with_x_and_y(0.0, 0.0);
                            }
                        }
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
            <header class="app-bar-material px-3 py-3 sm:p-4">
                <div class="mx-auto flex max-w-4xl flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <h1 class="text-lg sm:text-xl font-medium text-gray-900">{"Create New Contest"}</h1>
                    <button
                        onclick={on_back}
                        type="button"
                        class="btn-material-secondary w-full sm:w-auto min-h-[44px] shrink-0"
                    >
                        {"Back to Home"}
                    </button>
                </div>
            </header>
            <main class="mx-auto w-full max-w-4xl px-3 py-4 sm:px-4 sm:py-8">
                <div class="card-material w-full min-w-0 p-4 sm:p-6">
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
                                timezone={contest_timezone_for_form(
                                    &(*reducer).venue,
                                    &(*reducer).timezone,
                                )}
                                venue={(*reducer).venue.clone()}
                                games={(*reducer).games.clone()}
                                outcomes={(*reducer).outcomes.clone()}
                                on_start_change={on_start_change.clone()}
                                on_stop_change={on_stop_change.clone()}
                                on_venue_select={on_venue_select.clone()}
                                on_venue_clear={on_venue_clear.clone()}
                                on_games_change={on_games_change.clone()}
                                on_outcomes_change={on_outcomes_change.clone()}
                                on_submit={on_contest_submit.clone()}
                                on_image_pick={on_image_pick.clone()}
                                image_preview_url={(*image_preview_url).clone()}
                                image_error={(*image_error).clone()}
                                locked={*show_confirmation}
                            />
                            <ContestConfirmationModal
                                contest={(*contest_data).clone()}
                                image_preview_url={(*image_preview_url).clone()}
                                creator_display={auth
                                    .state
                                    .player
                                    .as_ref()
                                    .map(|p| p.handle.clone())
                                    .unwrap_or_default()}
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
