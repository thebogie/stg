//! Live contest tracker (ESP32 + BLE). Turn-order MVP in Tauri; web shows guidance.

use crate::pages::contest::TRACKER_SESSION_STORAGE_KEY;
use crate::tauri::is_tauri;
use crate::Route;
use gloo_storage::{SessionStorage, Storage};
use serde::{Deserialize, Serialize};
use shared::dto::contest::OutcomeDto;
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
struct TrackerHandoff {
    start: chrono::DateTime<chrono::FixedOffset>,
    stop: chrono::DateTime<chrono::FixedOffset>,
    timezone: String,
    venue: Option<VenueDto>,
    games: Vec<GameDto>,
    outcomes: Vec<OutcomeDto>,
}

#[function_component(TrackerStart)]
pub fn tracker_start() -> Html {
    let navigator = use_navigator().unwrap();
    let in_tauri = is_tauri();

    let players = use_state(Vec::<String>::new);
    let new_player = use_state(String::new);
    let current_turn = use_state(|| 0usize);

    let on_record = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&Route::Contest);
        })
    };

    let on_back = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&Route::Contests);
        })
    };

    let on_add_player = {
        let players = players.clone();
        let new_player = new_player.clone();
        Callback::from(move |_| {
            let name = new_player.trim().to_string();
            if name.is_empty() {
                return;
            }
            let mut list = (*players).clone();
            list.push(name);
            players.set(list);
            new_player.set(String::new());
        })
    };

    let on_remove_player = {
        let players = players.clone();
        let current_turn = current_turn.clone();
        Callback::from(move |idx: usize| {
            let mut list = (*players).clone();
            if idx < list.len() {
                list.remove(idx);
                players.set(list);
                current_turn.set(0);
            }
        })
    };

    let on_next_turn = {
        let players = players.clone();
        let current_turn = current_turn.clone();
        Callback::from(move |_| {
            if players.is_empty() {
                return;
            }
            current_turn.set((*current_turn + 1) % players.len());
        })
    };

    let on_end_contest = {
        let players = players.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if players.is_empty() {
                return;
            }
            let now = chrono::Utc::now().fixed_offset();
            let outcomes: Vec<OutcomeDto> = players
                .iter()
                .enumerate()
                .map(|(i, handle)| OutcomeDto {
                    player_id: String::new(),
                    handle: handle.clone(),
                    email: format!("{}@tracker.local", handle.to_lowercase().replace(' ', "_")),
                    place: (i + 1).to_string(),
                    result: String::new(),
                    score: String::new(),
                })
                .collect();
            let handoff = TrackerHandoff {
                start: now - chrono::Duration::hours(1),
                stop: now,
                timezone: "UTC".to_string(),
                venue: None,
                games: vec![],
                outcomes,
            };
            if let Ok(json) = serde_json::to_string(&handoff) {
                let _ = SessionStorage::set(TRACKER_SESSION_STORAGE_KEY, json);
            }
            navigator.push(&Route::Contest);
        })
    };

    html! {
        <div class="min-h-screen bg-gray-50">
            <header class="app-bar-material px-3 py-3 sm:p-4">
                <div class="mx-auto flex max-w-3xl flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <h1 class="text-lg sm:text-xl font-medium text-gray-900">{"Start Contest"}</h1>
                    <button type="button" onclick={on_back} class="btn-material-secondary w-full sm:w-auto min-h-[44px]">
                        {"Back to contests"}
                    </button>
                </div>
            </header>
            <main class="mx-auto w-full max-w-3xl px-3 py-6 sm:px-4 sm:py-10">
                <div class="card-material space-y-6 p-5 sm:p-8">
                    if in_tauri {
                        <div class="rounded-lg border border-indigo-200 bg-indigo-50 px-4 py-3 text-sm text-indigo-950">
                            <strong>{"Table tracker"}</strong>
                            {" — run turn order at the table, then end the session as a contest on "}
                            <span class="font-medium">{"smacktalkgaming.com"}</span>
                            {"."}
                        </div>

                        <div class="space-y-3">
                            <label class="block text-sm font-medium text-gray-700">{"Add player"}</label>
                            <div class="flex gap-2">
                                <input
                                    type="text"
                                    class="flex-1 rounded-lg border border-gray-300 px-3 py-2 min-h-[44px]"
                                    placeholder="Player name"
                                    value={(*new_player).clone()}
                                    oninput={{
                                        let new_player = new_player.clone();
                                        Callback::from(move |e: InputEvent| {
                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                            new_player.set(input.value());
                                        })
                                    }}
                                />
                                <button type="button" onclick={on_add_player} class="btn-material-primary min-h-[44px] px-4">
                                    {"Add"}
                                </button>
                            </div>
                        </div>

                        if !players.is_empty() {
                            <div class="space-y-2">
                                <h2 class="text-sm font-semibold text-gray-800">{"Turn order"}</h2>
                                <ul class="space-y-2">
                                    {players.iter().enumerate().map(|(i, name)| {
                                        let active = i == *current_turn;
                                        html! {
                                            <li class={classes!(
                                                "flex", "items-center", "justify-between", "rounded-lg", "border", "px-3", "py-2", "min-h-[44px]",
                                                if active { "border-indigo-400" } else { "border-gray-200" },
                                                if active { "bg-indigo-50" } else { "bg-white" },
                                                if active { "font-semibold" } else { "" }
                                            )}>
                                                <span>{name}{if active { " (current turn)" } else { "" }}</span>
                                                <button type="button" onclick={{
                                                    let on_remove_player = on_remove_player.clone();
                                                    Callback::from(move |_| on_remove_player.emit(i))
                                                }} class="text-xs text-red-600 hover:underline">{"Remove"}</button>
                                            </li>
                                        }
                                    }).collect::<Html>()}
                                </ul>
                                <button type="button" onclick={on_next_turn} class="btn-material-secondary w-full min-h-[44px]">
                                    {"Next turn"}
                                </button>
                            </div>
                        }

                        <button
                            type="button"
                            onclick={on_end_contest}
                            disabled={players.is_empty()}
                            class="btn-material-primary w-full min-h-[44px] disabled:opacity-50"
                        >
                            {"End Contest → Record on site"}
                        </button>
                    } else {
                        <div class="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-950">
                            <strong>{"Install the STG app"}</strong>
                            {" to run a live contest at the table. The website is for "}
                            <strong>{"Record Contest"}</strong>
                            {" after a game finishes."}
                        </div>
                        <p class="text-gray-700 leading-relaxed">
                            {"Start Contest uses the STG Tauri app to drive turn order at the table. "}
                            {"When you end the session it opens the contest form with players prefilled."}
                        </p>
                    }
                    <div class="flex flex-col sm:flex-row gap-3 pt-2">
                        <button type="button" onclick={on_record} class="btn-material-secondary min-h-[44px]">
                            {"Record Contest instead"}
                        </button>
                    </div>
                </div>
            </main>
        </div>
    }
}
