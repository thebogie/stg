use shared::dto::contest::ContestDto;
use yew::prelude::*;

use crate::components::contest::form::parse_offset_to_seconds;

#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/js/timezone.js")]
extern "C" {
    fn getTimezoneOffsetForInstant(tz: &str, iso_instant: &str) -> String;
    fn normalizeIanaTimezone(tz: &str) -> String;
}

#[derive(Properties, Clone)]
pub struct ContestConfirmationProps {
    pub contest: ContestDto,
    pub creator_display: String,
}

impl PartialEq for ContestConfirmationProps {
    fn eq(&self, other: &Self) -> bool {
        self.creator_display == other.creator_display
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

    let format_utc_offset_label = |offset_seconds: i32| {
        let sign = if offset_seconds >= 0 { '+' } else { '-' };
        let abs = offset_seconds.abs();
        let hours = abs / 3600;
        let minutes = (abs % 3600) / 60;
        format!("UTC{}{:02}:{:02}", sign, hours, minutes)
    };

    let tz_name = {
        let raw = &props.contest.venue.timezone;
        let normalized = normalizeIanaTimezone(raw);
        if normalized.is_empty() {
            raw.clone()
        } else {
            normalized
        }
    };

    let start_display = {
        let offset_str = getTimezoneOffsetForInstant(&tz_name, &props.contest.start.to_rfc3339());
        let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
        let tz = chrono::FixedOffset::east_opt(tz_seconds)
            .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        props
            .contest
            .start
            .with_timezone(&tz)
            .format("%b %d, %Y · %I:%M %p")
            .to_string()
    };
    let stop_display = {
        let offset_str = getTimezoneOffsetForInstant(&tz_name, &props.contest.stop.to_rfc3339());
        let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
        let tz = chrono::FixedOffset::east_opt(tz_seconds)
            .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        props
            .contest
            .stop
            .with_timezone(&tz)
            .format("%b %d, %Y · %I:%M %p")
            .to_string()
    };
    let tz_label = {
        let offset_str = getTimezoneOffsetForInstant(&tz_name, &props.contest.start.to_rfc3339());
        let tz_seconds = parse_offset_to_seconds(&offset_str).unwrap_or(0);
        format!("{} ({})", tz_name, format_utc_offset_label(tz_seconds))
    };

    html! {
        <div class="space-y-3 text-sm">
            if !props.creator_display.is_empty() {
                <div class="rounded-md border border-gray-200 bg-gray-50/80 px-3 py-2 text-gray-700">
                    <span class="text-xs font-semibold uppercase tracking-wide text-gray-500">{"Created by "}</span>
                    <span class="font-medium text-gray-900">{"@"}{props.creator_display.clone()}</span>
                </div>
            }

            <div class="rounded-md border border-gray-100 bg-gray-50 p-3 space-y-2.5">
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-2">
                    <div class="sm:col-span-2">
                        <h3 class="text-xs font-semibold uppercase tracking-wide text-gray-500">{"Venue"}</h3>
                        <p class="font-medium text-gray-900 leading-snug">{&props.contest.venue.display_name}</p>
                        <p class="text-xs text-gray-500 leading-snug">{&props.contest.venue.formatted_address}</p>
                    </div>

                    <div>
                        <h3 class="text-xs font-semibold uppercase tracking-wide text-gray-500">{"Timezone"}</h3>
                        <p class="text-gray-900">{tz_label.clone()}</p>
                    </div>

                    <div>
                        <h3 class="text-xs font-semibold uppercase tracking-wide text-gray-500">{"Schedule"}</h3>
                        <p class="text-gray-900"><span class="text-gray-500">{"Start · "}</span>{start_display}</p>
                        <p class="text-gray-900"><span class="text-gray-500">{"End · "}</span>{stop_display}</p>
                    </div>
                </div>

                <div>
                    <h3 class="text-xs font-semibold uppercase tracking-wide text-gray-500">{"Games"}</h3>
                    <p class="mt-0.5 text-gray-900 leading-snug">
                        {props.contest.games.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(", ")}
                    </p>
                </div>

                <div>
                    <h3 class="text-xs font-semibold uppercase tracking-wide text-gray-500">{"Outcomes"}</h3>
                    <ul class="mt-0.5 space-y-0.5 text-gray-900">
                        {props.contest.outcomes.iter().map(|outcome| {
                            let player_display = if !outcome.email.is_empty() && !outcome.handle.is_empty() {
                                format!("{}({})", outcome.email, outcome.handle)
                            } else if outcome.player_id.contains('@') {
                                outcome.player_id.clone()
                            } else if outcome.player_id.starts_with("player/") {
                                outcome.player_id.split('/').last().unwrap_or(&outcome.player_id).to_string()
                            } else {
                                outcome.player_id.clone()
                            };

                            html! {
                                <li class="text-xs sm:text-sm leading-snug">
                                    {format!(
                                        "{} — place {}, {}{}",
                                        player_display,
                                        outcome.place,
                                        outcome.result,
                                        if outcome.score.trim().is_empty() {
                                            String::new()
                                        } else {
                                            format!(", score {}", outcome.score.trim())
                                        }
                                    )}
                                </li>
                            }
                        }).collect::<Html>()}
                    </ul>
                </div>
            </div>
        </div>
    }
}
