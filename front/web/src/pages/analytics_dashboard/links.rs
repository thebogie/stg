use crate::api::contests::contest_key_from_any;
use crate::Route;
use serde_json::Value;
use urlencoding::encode;
use yew::prelude::*;
use yew_router::prelude::*;

pub(super) fn player_timezone_query(tz: &str) -> String {
    format!(
        "timezone={}",
        encode(&shared::timezone::normalize_iana_timezone(tz))
    )
}

pub(super) fn format_in_player_timezone(value: &str, tz: &str) -> String {
    shared::timezone::format_rfc3339_short(value, tz)
}

pub(super) fn timezone_label(tz: &str) -> String {
    shared::timezone::get_timezone_abbreviation(tz)
}

pub(super) fn section_guide(what: &str, player_value: &str) -> Html {
    html! {
        <div class="section-guide">
            <p class="section-guide-what">{what}</p>
            <p class="section-guide-value"><strong>{"How this helps you: "}</strong>{player_value}</p>
        </div>
    }
}

fn player_profile_key(player_id: &str) -> String {
    player_id.rsplit('/').next().unwrap_or(player_id).to_string()
}

fn artifact_label_or_link(id: Option<&str>, label: &str, route: impl FnOnce(String) -> Route) -> Html {
    let id = id.unwrap_or("").trim();
    if id.is_empty() {
        html! { <span>{label}</span> }
    } else {
        html! {
            <Link<Route> to={route(id.to_string())} classes="text-blue-600 hover:text-blue-800 hover:underline artifact-link">
                {label}
            </Link<Route>>
        }
    }
}

pub(super) fn game_link_from(v: &Value, id_key: &str, name_key: &str, fallback: &str) -> Html {
    let id = v.get(id_key).and_then(|x| x.as_str());
    let name = v.get(name_key).and_then(|x| x.as_str()).unwrap_or(fallback);
    artifact_label_or_link(id, name, |id| Route::GameDetails { game_id: id })
}

pub(super) fn venue_link_from(v: &Value, id_key: &str, name_key: &str, fallback: &str) -> Html {
    let id = v.get(id_key).and_then(|x| x.as_str());
    let name = v.get(name_key).and_then(|x| x.as_str()).unwrap_or(fallback);
    artifact_label_or_link(id, name, |id| Route::VenueDetails { venue_id: id })
}

pub(super) fn venue_link(id: &str, label: &str) -> Html {
    artifact_label_or_link(Some(id), label, |id| Route::VenueDetails { venue_id: id })
}

pub(super) fn game_link(id: &str, label: &str) -> Html {
    artifact_label_or_link(Some(id), label, |id| Route::GameDetails { game_id: id })
}

pub(super) fn contest_link(contest_id: &str, label: &str) -> Html {
    let key = contest_key_from_any(contest_id);
    if key.is_empty() {
        html! { <span>{label}</span> }
    } else {
        html! {
            <Link<Route> to={Route::ContestDetails { contest_id: key }} classes="text-blue-600 hover:text-blue-800 hover:underline artifact-link">
                {label}
            </Link<Route>>
        }
    }
}

pub(super) fn contest_label_from_json(c: &Value) -> String {
    c.get("contest_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| {
            c.get("most_popular_game")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from)
        })
        .unwrap_or_else(|| {
            c.get("contest_id")
                .and_then(|v| v.as_str())
                .unwrap_or("Contest")
                .to_string()
        })
}

pub(super) fn player_link_from_id(player_id: &str, label: &str) -> Html {
    let key = player_profile_key(player_id);
    if key.is_empty() {
        html! { <span>{label}</span> }
    } else {
        html! {
            <Link<Route> to={Route::PlayerProfile { player_id: key }} classes="text-blue-600 hover:text-blue-800 hover:underline artifact-link">
                {label}
            </Link<Route>>
        }
    }
}
