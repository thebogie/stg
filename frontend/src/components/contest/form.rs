use crate::flatpickr::{fp_destroy_all, fp_init, fp_set_value};
use shared::dto::{contest::OutcomeDto, game::GameDto, venue::VenueDto};
use wasm_bindgen::prelude::*;
use yew::prelude::*;

use super::game_selector::GameSelector;
use super::outcome_selector::OutcomeSelector;
use super::venue_picker::VenuePicker;

#[wasm_bindgen(module = "/src/js/timezone.js")]
extern "C" {
    pub fn getBrowserIanaTimezone() -> String;
    pub fn getBrowserLocalTimezoneOffset() -> String;
    pub fn getTimezoneOffsetForDate(tz: &str, iso_date: &str) -> String;
    pub fn getLocalDateTimeString(date: &js_sys::Date) -> String;
    pub fn getTimezoneOffsetForInstant(tz: &str, iso_instant: &str) -> String;
    pub fn normalizeIanaTimezone(tz: &str) -> String;
}

// Timezone handling is now automatic based on venue selection

#[derive(Properties, PartialEq, Clone)]
pub struct ContestFormProps {
    pub start: chrono::DateTime<chrono::FixedOffset>,
    pub stop: chrono::DateTime<chrono::FixedOffset>,
    pub timezone: String,
    pub venue: Option<VenueDto>,
    pub games: Vec<GameDto>,
    pub outcomes: Vec<OutcomeDto>,
    pub on_start_change: Callback<chrono::DateTime<chrono::FixedOffset>>,
    pub on_stop_change: Callback<chrono::DateTime<chrono::FixedOffset>>,
    pub on_venue_select: Callback<VenueDto>,
    pub on_games_change: Callback<Vec<GameDto>>,
    pub on_outcomes_change: Callback<Vec<OutcomeDto>>,
    pub on_submit: Callback<()>,
    pub locked: bool,
}

// Local datetime state for stable flatpickr inputs
#[derive(Clone, PartialEq)]
struct LocalDateTimeState {
    start_local: String,
    stop_local: String,
}

// Helper: parse offset string to seconds. Supports "+HH:MM", "-HH:MM", or minute offset like "-300"
pub(crate) fn parse_offset_to_seconds(offset: &str) -> Option<i32> {
    // Case 1: pure minutes (e.g., "-300", "+60", "0")
    if let Ok(mins) = offset.parse::<i32>() {
        return Some(mins * 60);
    }
    // Case 2: "+HH:MM" or "-HH:MM"
    if offset.len() >= 6 {
        let sign = if &offset[0..1] == "+" { 1 } else { -1 };
        if let (Ok(hours), Ok(minutes)) = (offset[1..3].parse::<i32>(), offset[4..6].parse::<i32>())
        {
            return Some(sign * (hours * 3600 + minutes * 60));
        }
    }
    None
}

// Helper: format seconds offset to friendly string like "UTC+05:30" or "UTC-08:00"
fn format_utc_offset_label(offset_seconds: i32) -> String {
    let sign = if offset_seconds >= 0 { '+' } else { '-' };
    let abs = offset_seconds.abs();
    let hours = abs / 3600;
    let minutes = (abs % 3600) / 60;
    format!("UTC{}{:02}:{:02}", sign, hours, minutes)
}

#[function_component(ContestForm)]
pub fn contest_form(props: &ContestFormProps) -> Html {
    let props = props.clone();
    let is_submitting = use_state(|| false);
    let show_validation_modal = use_state(|| false);

    // Reset is_submitting state when component mounts or becomes visible again
    // This fixes the bug where the button stays spinning after returning from confirmation modal
    use_effect_with((), {
        let is_submitting = is_submitting.clone();
        move |_| {
            is_submitting.set(false);
            || ()
        }
    });

    // Local datetime state for stable flatpickr inputs
    let local_state = use_state(|| {
        // Convert UTC times to local strings for display
        let timezone = normalizeIanaTimezone(&props.timezone);
        let start_local = if !timezone.is_empty() {
            let offset_str = getTimezoneOffsetForInstant(&timezone, &props.start.to_rfc3339());
            let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
            let tz = chrono::FixedOffset::east_opt(tz_seconds)
                .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
            props
                .start
                .with_timezone(&tz)
                .format("%m/%d/%Y %H:%M")
                .to_string()
        } else {
            props.start.format("%m/%d/%Y %H:%M").to_string()
        };

        let stop_local = if !timezone.is_empty() {
            let offset_str = getTimezoneOffsetForInstant(&timezone, &props.stop.to_rfc3339());
            let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
            let tz = chrono::FixedOffset::east_opt(tz_seconds)
                .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
            props
                .stop
                .with_timezone(&tz)
                .format("%m/%d/%Y %H:%M")
                .to_string()
        } else {
            props.stop.format("%m/%d/%Y %H:%M").to_string()
        };

        LocalDateTimeState {
            start_local,
            stop_local,
        }
    });

    // Convert local datetime string to UTC and emit change
    let convert_and_emit_start = {
        let props = props.clone();
        move |datetime_str: String| {
            web_sys::console::log_1(
                &format!("convert_and_emit_start: input='{}'", datetime_str).into(),
            );

            // Try multiple date formats that flatpickr might return
            // Prioritize the new user-friendly format first
            let naive_dt = chrono::NaiveDateTime::parse_from_str(&datetime_str, "%m/%d/%Y %H:%M")
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&datetime_str, "%m/%d/%Y %I:%M %p")
                })
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%dT%H:%M"))
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M")
                });

            if let Ok(naive_dt) = naive_dt {
                let timezone = normalizeIanaTimezone(&props.timezone);
                web_sys::console::log_1(
                    &format!(
                        "convert_and_emit_start: naive_dt='{}', timezone='{}'",
                        naive_dt, timezone
                    )
                    .into(),
                );

                if !timezone.is_empty() {
                    // The naive_dt from flatpickr represents the venue's local time
                    // Convert it directly to UTC using the venue's timezone
                    let iso_for_offset = naive_dt.format("%Y-%m-%dT%H:%M:%S").to_string();
                    let offset_str = getTimezoneOffsetForDate(&timezone, &iso_for_offset);
                    web_sys::console::log_1(
                        &format!(
                            "convert_and_emit_start: iso_for_offset='{}', offset_str='{}'",
                            iso_for_offset, offset_str
                        )
                        .into(),
                    );
                    let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
                    web_sys::console::log_1(
                        &format!("convert_and_emit_start: tz_seconds={}", tz_seconds).into(),
                    );

                    // Create timezone offset and convert to UTC
                    if let Some(tz) = chrono::FixedOffset::east_opt(tz_seconds) {
                        // Treat naive_dt as being in the venue's timezone
                        if let chrono::LocalResult::Single(venue_dt) =
                            naive_dt.and_local_timezone(tz)
                        {
                            let utc_dt = venue_dt.with_timezone(&chrono::Utc);
                            let fixed_offset_dt =
                                utc_dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
                            web_sys::console::log_1(&format!("convert_and_emit_start: venue_dt='{}', utc_dt='{}', fixed_offset_dt='{}'", venue_dt, utc_dt, fixed_offset_dt).into());
                            props.on_start_change.emit(fixed_offset_dt);
                        }
                    }
                } else {
                    // Fallback to UTC if no timezone
                    if let Some(utc) = chrono::FixedOffset::east_opt(0) {
                        if let chrono::LocalResult::Single(utc_dt) =
                            naive_dt.and_local_timezone(utc)
                        {
                            web_sys::console::log_1(
                                &format!("convert_and_emit_start: fallback utc_dt='{}'", utc_dt)
                                    .into(),
                            );
                            props.on_start_change.emit(utc_dt);
                        }
                    }
                }
            } else {
                web_sys::console::log_1(
                    &format!("convert_and_emit_start: failed to parse '{}'", datetime_str).into(),
                );
            }
        }
    };

    let convert_and_emit_stop = {
        let props = props.clone();
        move |datetime_str: String| {
            web_sys::console::log_1(
                &format!("convert_and_emit_stop: input='{}'", datetime_str).into(),
            );

            // Try multiple date formats that flatpickr might return
            // Prioritize the new user-friendly format first
            let naive_dt = chrono::NaiveDateTime::parse_from_str(&datetime_str, "%m/%d/%Y %H:%M")
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&datetime_str, "%m/%d/%Y %I:%M %p")
                })
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%dT%H:%M"))
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M")
                });

            if let Ok(naive_dt) = naive_dt {
                let timezone = normalizeIanaTimezone(&props.timezone);
                web_sys::console::log_1(
                    &format!(
                        "convert_and_emit_stop: naive_dt='{}', timezone='{}'",
                        naive_dt, timezone
                    )
                    .into(),
                );

                if !timezone.is_empty() {
                    // The naive_dt from flatpickr is in the user's local timezone
                    // We need to treat it as being in the venue's timezone and convert to UTC
                    let iso_for_offset = naive_dt.format("%Y-%m-%dT%H:%M:%S").to_string();
                    let offset_str = getTimezoneOffsetForDate(&timezone, &iso_for_offset);
                    web_sys::console::log_1(
                        &format!(
                            "convert_and_emit_stop: iso_for_offset='{}', offset_str='{}'",
                            iso_for_offset, offset_str
                        )
                        .into(),
                    );
                    let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
                    web_sys::console::log_1(
                        &format!("convert_and_emit_stop: tz_seconds={}", tz_seconds).into(),
                    );

                    // Create timezone offset and convert to UTC
                    if let Some(tz) = chrono::FixedOffset::east_opt(tz_seconds) {
                        // Treat naive_dt as being in the venue's timezone
                        if let chrono::LocalResult::Single(venue_dt) =
                            naive_dt.and_local_timezone(tz)
                        {
                            let utc_dt = venue_dt.with_timezone(&chrono::Utc);
                            let fixed_offset_dt =
                                utc_dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
                            web_sys::console::log_1(&format!("convert_and_emit_stop: venue_dt='{}', utc_dt='{}', fixed_offset_dt='{}'", venue_dt, utc_dt, fixed_offset_dt).into());
                            props.on_stop_change.emit(fixed_offset_dt);
                        }
                    }
                } else {
                    // Fallback to UTC if no timezone
                    if let Some(utc) = chrono::FixedOffset::east_opt(0) {
                        if let chrono::LocalResult::Single(utc_dt) =
                            naive_dt.and_local_timezone(utc)
                        {
                            web_sys::console::log_1(
                                &format!("convert_and_emit_stop: fallback utc_dt='{}'", utc_dt)
                                    .into(),
                            );
                            props.on_stop_change.emit(utc_dt);
                        }
                    }
                }
            } else {
                web_sys::console::log_1(
                    &format!("convert_and_emit_stop: failed to parse '{}'", datetime_str).into(),
                );
            }
        }
    };

    // Initialize flatpickr when component mounts or timezone changes
    use_effect_with((props.timezone.clone(), props.venue.clone()), {
        let local_state = local_state.clone();
        let convert_start = convert_and_emit_start.clone();
        let convert_stop = convert_and_emit_stop.clone();
        move |_| {
            web_sys::console::log_1(&"Initializing flatpickr...".into());

            // Create JavaScript callbacks - use onClose instead of onChange to avoid repeated conversions
            let start_callback = Closure::wrap(Box::new(
                move |selected_dates: JsValue, date_str: JsValue, _instance: JsValue| {
                    web_sys::console::log_1(&"Start onClose callback triggered!".into());
                    web_sys::console::log_1(
                        &format!(
                            "Start onClose - selected_dates: {:?}, date_str: {:?}",
                            selected_dates, date_str
                        )
                        .into(),
                    );
                    if let Some(s) = date_str.as_string() {
                        web_sys::console::log_1(
                            &format!("Start onClose - calling convert_start with: {}", s).into(),
                        );
                        convert_start(s);
                    } else {
                        web_sys::console::log_1(&"Start onClose - date_str is not a string".into());
                    }
                },
            )
                as Box<dyn Fn(JsValue, JsValue, JsValue)>);

            let stop_callback = Closure::wrap(Box::new(
                move |selected_dates: JsValue, date_str: JsValue, _instance: JsValue| {
                    web_sys::console::log_1(&"Stop onClose callback triggered!".into());
                    web_sys::console::log_1(
                        &format!(
                            "Stop onClose - selected_dates: {:?}, date_str: {:?}",
                            selected_dates, date_str
                        )
                        .into(),
                    );
                    if let Some(s) = date_str.as_string() {
                        web_sys::console::log_1(
                            &format!("Stop onClose - calling convert_stop with: {}", s).into(),
                        );
                        convert_stop(s);
                    } else {
                        web_sys::console::log_1(&"Stop onClose - date_str is not a string".into());
                    }
                },
            )
                as Box<dyn Fn(JsValue, JsValue, JsValue)>);

            web_sys::console::log_1(
                &format!(
                    "Initializing start flatpickr with value: {}",
                    local_state.start_local
                )
                .into(),
            );
            // Initialize flatpickr instances
            if let Err(e) = fp_init(
                "start-datetime-input",
                Some(&local_state.start_local),
                Some(JsValue::from(start_callback.as_ref())),
            ) {
                web_sys::console::error_1(
                    &format!("Failed to initialize start flatpickr: {:?}", e).into(),
                );
            } else {
                web_sys::console::log_1(&"Start flatpickr initialized successfully".into());
            }

            web_sys::console::log_1(
                &format!(
                    "Initializing stop flatpickr with value: {}",
                    local_state.stop_local
                )
                .into(),
            );
            if let Err(e) = fp_init(
                "stop-datetime-input",
                Some(&local_state.stop_local),
                Some(JsValue::from(stop_callback.as_ref())),
            ) {
                web_sys::console::error_1(
                    &format!("Failed to initialize stop flatpickr: {:?}", e).into(),
                );
            } else {
                web_sys::console::log_1(&"Stop flatpickr initialized successfully".into());
            }

            // Keep callbacks alive
            start_callback.forget();
            stop_callback.forget();

            || {
                // Cleanup on unmount
                let _ = fp_destroy_all();
            }
        }
    });

    // Update local state when props change (but don't reset on venue change)
    use_effect_with((props.start, props.stop, props.timezone.clone()), {
        let local_state = local_state.clone();
        move |deps| {
            let (start, stop, timezone) = deps.clone();
            let timezone = normalizeIanaTimezone(&timezone);
            let start_local = if !timezone.is_empty() {
                let offset_str = getTimezoneOffsetForInstant(&timezone, &start.to_rfc3339());
                let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
                let tz = chrono::FixedOffset::east_opt(tz_seconds)
                    .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
                start
                    .with_timezone(&tz)
                    .format("%m/%d/%Y %H:%M")
                    .to_string()
            } else {
                start.format("%m/%d/%Y %H:%M").to_string()
            };

            let stop_local = if !timezone.is_empty() {
                let offset_str = getTimezoneOffsetForInstant(&timezone, &stop.to_rfc3339());
                let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
                let tz = chrono::FixedOffset::east_opt(tz_seconds)
                    .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
                stop.with_timezone(&tz).format("%m/%d/%Y %H:%M").to_string()
            } else {
                stop.format("%m/%d/%Y %H:%M").to_string()
            };

            local_state.set(LocalDateTimeState {
                start_local: start_local.clone(),
                stop_local: stop_local.clone(),
            });

            // Update flatpickr values
            let _ = fp_set_value("start-datetime-input", &start_local);
            let _ = fp_set_value("stop-datetime-input", &stop_local);
        }
    });

    let on_close_validation_modal = {
        let show_validation_modal = show_validation_modal.clone();
        Callback::from(move |_| {
            show_validation_modal.set(false);
        })
    };

    let on_submit = {
        let props = props.clone();
        let is_submitting = is_submitting.clone();
        let show_validation_modal = show_validation_modal.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if !*is_submitting {
                // Check for validation errors and show modal if needed
                let mut has_errors = false;

                // Basic validation
                if props.venue.is_none()
                    || props.games.is_empty()
                    || props.outcomes.is_empty()
                    || props.stop <= props.start
                {
                    has_errors = true;
                }

                // Detailed ID validation
                if let Some(venue) = &props.venue {
                    if venue.source == shared::models::venue::VenueSource::Database {
                        if venue.id.is_empty() || !venue.id.starts_with("venue/") {
                            has_errors = true;
                        }
                    }
                }

                for game in &props.games {
                    if game.id.is_empty()
                        || (!game.id.starts_with("game/") && !game.id.starts_with("bgg_"))
                    {
                        has_errors = true;
                    }
                }

                for outcome in &props.outcomes {
                    // Allow empty player_id for new players, but check format for existing players
                    if !outcome.player_id.is_empty() && !outcome.player_id.starts_with("player/") {
                        has_errors = true;
                    }
                }

                if has_errors {
                    show_validation_modal.set(true);
                    return;
                }

                is_submitting.set(true);
                props.on_submit.emit(());
            }
        })
    };

    // Build a human-friendly timezone label such as: "America/New_York (UTC-05:00)"
    let timezone_label = {
        log::info!("Form component: props.timezone = '{}'", props.timezone);
        let tz_name = normalizeIanaTimezone(&props.timezone);
        log::info!("Form component: normalized timezone = '{}'", tz_name);
        if tz_name.is_empty() {
            "Timezone: —".to_string()
        } else {
            let offset_str = getTimezoneOffsetForInstant(&tz_name, &props.start.to_rfc3339());
            let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
            let pretty = format_utc_offset_label(tz_seconds);
            format!("Timezone: {} ({})", tz_name, pretty)
        }
    };

    html! {
        <div class="contest-form-container">
            <form onsubmit={on_submit} class="space-y-6 sm:space-y-8">

                // Venue Section (first)
                <div class="bg-white rounded-xl shadow-mobile-soft p-4 sm:p-6 border border-gray-100">
                    <h3 class="text-lg sm:text-xl font-semibold text-gray-900 mb-4 sm:mb-6 flex items-center">
                        <span class="mr-2 text-xl">{"📍"}</span>
                        {"Venue"}
                    </h3>
                    <VenuePicker
                        on_venue_select={props.on_venue_select.clone()}
                        initial_venue={props.venue.clone()}
                    />
                </div>

                // Time and Date Section (after venue)
                <div class="bg-white rounded-xl shadow-mobile-soft p-4 sm:p-6 border border-gray-100">
                    <h3 class="text-lg sm:text-xl font-semibold text-gray-900 mb-4 sm:mb-6 flex items-center">
                        <span class="mr-2 text-xl">{"🕒"}</span>
                        {"Time & Date"}
                        <span class="ml-3 inline-flex items-center rounded-md border border-gray-200 bg-gray-50 px-2.5 py-1 text-xs font-medium text-gray-700">
                            {timezone_label.clone()}
                        </span>
                    </h3>
                    if props.venue.is_none() {
                        <div class="mb-4 sm:mb-6 rounded-lg border border-amber-200 bg-amber-50 p-3 text-amber-800 text-sm">
                            {"Pick a venue first to set the contest timezone. Date and time fields will unlock once a venue is selected. Times shown are in the venue's timezone; submissions are stored in UTC."}
                        </div>
                    }

                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 sm:gap-6">
                        // Start Date/Time
                        <div class="space-y-2">
                            <label class="block text-sm font-medium text-gray-700">
                                {"Start Date & Time"}
                            </label>
                            <input
                                id="start-datetime-input"
                                type="text"
                                placeholder="YYYY-MM-DD HH:MM"
                                disabled={props.locked || props.venue.is_none()}
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-colors duration-200 min-h-[40px] text-sm"
                                required=true
                                readonly=true
                            />
                        </div>

                        // End Date/Time
                        <div class="space-y-2">
                            <label class="block text-sm font-medium text-gray-700">
                                {"End Date & Time"}
                            </label>
                            <input
                                id="stop-datetime-input"
                                type="text"
                                placeholder="YYYY-MM-DD HH:MM"
                                disabled={props.locked || props.venue.is_none()}
                                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-colors duration-200 min-h-[40px] text-sm"
                                required=true
                                readonly=true
                            />
                        </div>
                    </div>

                    // Timezone display removed per requirements; venue timezone is used implicitly
                </div>

                // Games Section
                <div class="bg-white rounded-xl shadow-mobile-soft p-4 sm:p-6 border border-gray-100">
                    <h3 class="text-lg sm:text-xl font-semibold text-gray-900 mb-4 sm:mb-6 flex items-center">
                        <span class="mr-2 text-xl">{"🎮"}</span>
                        {"Games"}
                    </h3>
                    <GameSelector
                        games={props.games.clone()}
                        on_games_change={props.on_games_change.clone()}
                        preload_last={true}
                    />
                </div>

                // Outcomes Section
                <div class="bg-white rounded-xl shadow-mobile-soft p-4 sm:p-6 border border-gray-100">
                    <h3 class="text-lg sm:text-xl font-semibold text-gray-900 mb-4 sm:mb-6 flex items-center">
                        <span class="mr-2 text-xl">{"🏆"}</span>
                        {"Outcomes"}
                    </h3>
                    <OutcomeSelector
                        on_outcomes_change={props.on_outcomes_change.clone()}
                    />
                </div>

                // Submit Button
                <div class="flex justify-center pt-4 sm:pt-6">
                    <button
                        type="submit"
                        disabled={props.locked || *is_submitting}
                        class="w-full sm:w-auto inline-flex items-center justify-center px-8 py-4 text-lg font-semibold text-white bg-gradient-to-r from-blue-600 to-indigo-600 rounded-xl shadow-mobile-medium hover:shadow-mobile-strong transform hover:-translate-y-1 transition-all duration-200 active:scale-95 min-h-[56px] disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none"
                    >
                        if *is_submitting {
                            <span class="animate-spin mr-2">{"⏳"}</span>
                            {"Creating Contest..."}
                        } else {
                            <span class="mr-2 text-xl">{"🚀"}</span>
                            {"Add Contest"}
                        }
                    </button>
                </div>
            </form>

            // Validation Error Modal
            if *show_validation_modal {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
                    <div class="bg-white rounded-xl shadow-xl max-w-md w-full p-6">
                        <div class="flex items-center mb-4">
                            <span class="text-red-500 text-2xl mr-3">{"⚠️"}</span>
                            <h3 class="text-lg font-semibold text-gray-900">{"Validation Errors"}</h3>
                        </div>
                        <div class="text-gray-700 mb-6">
                            <p class="mb-3">{"Please fix the following issues:"}</p>
                            <ul class="space-y-2 text-sm">
                                if props.venue.is_none() {
                                    <li class="flex items-start">
                                        <span class="text-red-500 mr-2">{"•"}</span>
                                        {"Select a venue for the contest"}
                                    </li>
                                }
                                if let Some(venue) = &props.venue {
                                    if venue.source == shared::models::venue::VenueSource::Database {
                                        if venue.id.is_empty() || !venue.id.starts_with("venue/") {
                                            <li class="flex items-start">
                                                <span class="text-red-500 mr-2">{"•"}</span>
                                                {"Selected venue has no ID - please search and select a venue again"}
                                            </li>
                                        }
                                    }
                                }
                                if props.games.is_empty() {
                                    <li class="flex items-start">
                                        <span class="text-red-500 mr-2">{"•"}</span>
                                        {"Select at least one game for the contest"}
                                    </li>
                                }
                                {props.games.iter().enumerate().filter_map(|(i, game)| {
                                    if game.id.is_empty() || (!game.id.starts_with("game/") && !game.id.starts_with("bgg_")) {
                                        Some(html! {
                                            <li class="flex items-start">
                                                <span class="text-red-500 mr-2">{"•"}</span>
                                                {format!("Game {} has no ID - please search and select games again", i + 1)}
                                            </li>
                                        })
                                    } else {
                                        None
                                    }
                                }).collect::<Html>()}
                                if props.outcomes.is_empty() {
                                    <li class="flex items-start">
                                        <span class="text-red-500 mr-2">{"•"}</span>
                                        {"Add at least one player outcome"}
                                    </li>
                                }
                                {props.outcomes.iter().enumerate().filter_map(|(i, outcome)| {
                                    // Allow empty player_id for new players, but check format for existing players
                                    if !outcome.player_id.is_empty() && !outcome.player_id.starts_with("player/") {
                                        Some(html! {
                                            <li class="flex items-start">
                                                <span class="text-red-500 mr-2">{"•"}</span>
                                                {format!("Player {} has invalid ID format - please search and select players again", i + 1)}
                                            </li>
                                        })
                                    } else {
                                        None
                                    }
                                }).collect::<Html>()}
                                if props.stop <= props.start {
                                    <li class="flex items-start">
                                        <span class="text-red-500 mr-2">{"•"}</span>
                                        {"End date/time must be after start date/time"}
                                    </li>
                                }
                            </ul>
                        </div>
                        <div class="flex justify-end">
                            <button
                                onclick={on_close_validation_modal}
                                class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors duration-200"
                            >
                                {"OK"}
                            </button>
                        </div>
                    </div>
                </div>
            }
        </div>
    }
}

// ---------- Tests ----------
#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    #[test]
    fn parse_offset_supports_minutes_and_hhmm() {
        assert_eq!(
            crate::components::contest::form::parse_offset_to_seconds("0"),
            Some(0)
        );
        assert_eq!(
            crate::components::contest::form::parse_offset_to_seconds("-300"),
            Some(-300 * 60)
        );
        assert_eq!(
            crate::components::contest::form::parse_offset_to_seconds("+60"),
            Some(60 * 60)
        );
        assert_eq!(
            crate::components::contest::form::parse_offset_to_seconds("+05:30"),
            Some((5 * 3600) + 1800)
        );
        assert_eq!(
            crate::components::contest::form::parse_offset_to_seconds("-08:00"),
            Some(-(8 * 3600))
        );
        assert_eq!(
            crate::components::contest::form::parse_offset_to_seconds("bad"),
            None
        );
    }

    #[test]
    fn utc_conversion_roundtrip() {
        // Local 2024-03-10 01:30 in -08:00 should be 09:30 UTC
        let naive =
            chrono::NaiveDateTime::parse_from_str("2024-03-10T01:30", "%Y-%m-%dT%H:%M").unwrap();
        let tz = chrono::FixedOffset::east_opt(-8 * 3600).unwrap();
        let local = tz.from_local_datetime(&naive).single().unwrap();
        let utc = local.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
        assert_eq!(utc.format("%Y-%m-%dT%H:%M").to_string(), "2024-03-10T09:30");
    }
}
