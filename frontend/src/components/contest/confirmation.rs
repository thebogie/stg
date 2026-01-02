use yew::prelude::*;
use shared::dto::contest::ContestDto;

use crate::components::contest::form::parse_offset_to_seconds;

#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/js/timezone.js")]
extern "C" {
    fn getTimezoneOffsetForInstant(tz: &str, iso_instant: &str) -> String;
    fn normalizeIanaTimezone(tz: &str) -> String;
}

#[derive(Properties, Clone)]
pub struct ContestConfirmationProps {
    pub contest: ContestDto,
    pub on_confirm: Callback<()>,
    pub on_cancel: Callback<()>,
    pub on_edit: Callback<()>,
}

impl PartialEq for ContestConfirmationProps {
    fn eq(&self, other: &Self) -> bool {
        // Compare the fields we care about for equality
        self.on_confirm == other.on_confirm && 
        self.on_cancel == other.on_cancel &&
        self.on_edit == other.on_edit
        // Note: We're intentionally not comparing contest field
        // since ContestDto doesn't implement PartialEq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_offset_to_seconds_basic() {
        assert_eq!(parse_offset_to_seconds("+02:00"), Some(7200));
        assert_eq!(parse_offset_to_seconds("-05:30"), Some(-19800));
        assert_eq!(parse_offset_to_seconds("60"), Some(3600));
        assert_eq!(parse_offset_to_seconds("0"), Some(0));
        assert_eq!(parse_offset_to_seconds("bad"), None);
    }
}

#[function_component(ContestConfirmation)]
pub fn contest_confirmation(props: &ContestConfirmationProps) -> Html {
    let props = props.clone();

    let on_confirm = {
        let on_confirm = props.on_confirm.clone();
        Callback::from(move |_| {
            gloo::console::log!("🔘 ContestConfirmation: Confirm button clicked");
            gloo::console::log!("🔘 ContestConfirmation: About to emit on_confirm callback");
            on_confirm.emit(());
            gloo::console::log!("🔘 ContestConfirmation: on_confirm callback emitted");
        })
    };

    let _on_cancel = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_: MouseEvent| {
            on_cancel.emit(());
        })
    };

    let on_edit = {
        let on_edit = props.on_edit.clone();
        Callback::from(move |_: MouseEvent| {
            on_edit.emit(());
        })
    };

    // Helpers to format offset label similar to the form component
    let format_utc_offset_label = |offset_seconds: i32| {
        let sign = if offset_seconds >= 0 { '+' } else { '-' };
        let abs = offset_seconds.abs();
        let hours = abs / 3600;
        let minutes = (abs % 3600) / 60;
        format!("UTC{}{:02}:{:02}", sign, hours, minutes)
    };

    // Compute timezone-aware display strings (use venue timezone)
    let tz_name = {
        let raw = &props.contest.venue.timezone;
        let normalized = normalizeIanaTimezone(raw);
        if normalized.is_empty() { raw.clone() } else { normalized }
    };

    let start_display = {
        let offset_str = getTimezoneOffsetForInstant(&tz_name, &props.contest.start.to_rfc3339());
        let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
        let tz = chrono::FixedOffset::east_opt(tz_seconds).unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        props.contest.start.with_timezone(&tz).format("%B %d, %Y at %I:%M %p").to_string()
    };
    let stop_display = {
        let offset_str = getTimezoneOffsetForInstant(&tz_name, &props.contest.stop.to_rfc3339());
        let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
        let tz = chrono::FixedOffset::east_opt(tz_seconds).unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        props.contest.stop.with_timezone(&tz).format("%B %d, %Y at %I:%M %p").to_string()
    };
    let tz_label = {
        let offset_str = getTimezoneOffsetForInstant(&tz_name, &props.contest.start.to_rfc3339());
        let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
        format!("{} ({})", tz_name, format_utc_offset_label(tz_seconds))
    };

    html! {
        <div class="space-y-6">
            <h2 class="text-xl font-semibold text-gray-800">{"Confirm Contest Details"}</h2>
            <p class="text-gray-600">{"Please review the contest details before creating."}</p>

            <div class="space-y-4 bg-gray-50 p-4 rounded-material">
                <div>
                    <h3 class="text-sm font-medium text-gray-500">{"Timezone"}</h3>
                    <p class="mt-1 text-gray-900">{tz_label.clone()}</p>
                </div>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <h3 class="text-sm font-medium text-gray-500">{"Start Time"}</h3>
                        <p class="mt-1 text-gray-900">{start_display}</p>
                    </div>

                    <div>
                        <h3 class="text-sm font-medium text-gray-500">{"End Time"}</h3>
                        <p class="mt-1 text-gray-900">{stop_display}</p>
                    </div>
                </div>

                <div>
                    <h3 class="text-sm font-medium text-gray-500">{"Venue"}</h3>
                    <p class="mt-1 text-gray-900">{&props.contest.venue.display_name}</p>
                    <p class="text-sm text-gray-500">{&props.contest.venue.formatted_address}</p>
                </div>

                <div>
                    <h3 class="text-sm font-medium text-gray-500">{"Games"}</h3>
                    <ul class="mt-1 space-y-1">
                        {props.contest.games.iter().map(|game| {
                            html! {
                                <li class="text-gray-900">{&game.name}</li>
                            }
                        }).collect::<Html>()}
                    </ul>
                </div>

                <div>
                    <h3 class="text-sm font-medium text-gray-500">{"Outcomes"}</h3>
                    <ul class="mt-1 space-y-1">
                        {props.contest.outcomes.iter().map(|outcome| {
                            // Use email and handle if available, otherwise fallback to player_id
                            let player_display = if !outcome.email.is_empty() && !outcome.handle.is_empty() {
                                // If we have both email and handle, use the requested format
                                format!("{}({})", outcome.email, outcome.handle)
                            } else if outcome.player_id.contains('@') {
                                // If player_id contains @, it's likely an email
                                outcome.player_id.clone()
                            } else if outcome.player_id.starts_with("player/") {
                                // If player_id starts with "player/", extract the UUID part
                                outcome.player_id.split('/').last().unwrap_or(&outcome.player_id).to_string()
                            } else {
                                // For any other format, just display as is
                                outcome.player_id.clone()
                            };

                            html! {
                                <li class="text-gray-900">{format!("Player: {}, Place: {}, Result: {}", 
                                    player_display, outcome.place, outcome.result)}</li>
                            }
                        }).collect::<Html>()}
                    </ul>
                </div>
            </div>

            <div class="flex justify-end space-x-4">
                <button 
                    onclick={on_edit}
                    class="btn-material-secondary flex items-center"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 mr-1" viewBox="0 0 20 20" fill="currentColor">
                        <path fill-rule="evenodd" d="M9.707 14.707a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414l4-4a1 1 0 011.414 1.414L7.414 9H15a1 1 0 110 2H7.414l2.293 2.293a1 1 0 010 1.414z" clip-rule="evenodd" />
                    </svg>
                    {"Back to Edit"}
                </button>
                <button 
                    onclick={on_confirm}
                    class="btn-material-primary"
                >
                    {"Confirm & Add Contest"}
                </button>
            </div>
        </div>
    }
}
