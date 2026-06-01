//! Live contest tracker (ESP32 + BLE). Full flow ships in the Tauri app; web shows guidance.

use crate::tauri::is_tauri;
use crate::Route;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(TrackerStart)]
pub fn tracker_start() -> Html {
    let navigator = use_navigator().unwrap();
    let in_tauri = is_tauri();

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
                            {" — pair ELECROW ESP32 displays, run turn order, and end the session as a contest on "}
                            <span class="font-medium">{"smacktalkgaming.com"}</span>
                            {"."}
                        </div>
                        <p class="text-gray-700 leading-relaxed">
                            {"The live tracker (Bluetooth pairing, turn alerts, End Contest) is being built. "}
                            {"This screen will host setup and play controls in the STG desktop/Android app."}
                        </p>
                        <ul class="list-disc pl-5 text-sm text-gray-600 space-y-2">
                            <li>{"Assign one ESP32 display per player at the table"}</li>
                            <li>{"Highlight whose turn it is; optional turn timer"}</li>
                            <li>{"End Contest saves a normal contest for moderation on the site"}</li>
                        </ul>
                    } else {
                        <div class="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-950">
                            <strong>{"Install the STG app"}</strong>
                            {" to run a live contest at the table. The website is for "}
                            <strong>{"Record Contest"}</strong>
                            {" after a game finishes."}
                        </div>
                        <p class="text-gray-700 leading-relaxed">
                            {"Start Contest uses Bluetooth from the organizer’s phone or laptop (STG Tauri app) "}
                            {"to drive player displays. It syncs to "}
                            <a href="https://smacktalkgaming.com" class="text-indigo-600 hover:underline">
                                {"smacktalkgaming.com"}
                            </a>
                            {" when you end the session."}
                        </p>
                        <p class="text-sm text-gray-600">
                            {"Download the production STG app from your release artifacts "}
                            {"(see deploy/WEB_AND_TAURI.md). Production builds use "}
                            <code class="text-xs bg-gray-100 px-1 rounded">{"https://smacktalkgaming.com"}</code>
                            {" as the API."}
                        </p>
                    }
                    <div class="flex flex-col sm:flex-row gap-3 pt-2">
                        <button type="button" onclick={on_record} class="btn-material-primary min-h-[44px]">
                            {"Record Contest instead"}
                        </button>
                    </div>
                </div>
            </main>
        </div>
    }
}
