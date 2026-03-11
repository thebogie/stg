//! Ensures app config is loaded before rendering the app. In Tauri, fetches config via invoke.
//! In the browser, initializes from origin and renders immediately.

use crate::config::{self, is_tauri};
use crate::tauri::get_app_config_from_tauri;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ConfigLoaderProps {
    pub children: Children,
}

#[function_component(ConfigLoader)]
pub fn config_loader(props: &ConfigLoaderProps) -> Html {
    // In browser we're ready immediately and set config synchronously so api_url() works from first paint
    let ready = use_state(|| !is_tauri());

    if !is_tauri() {
        config::init_browser_config();
    }

    {
        let ready = ready.clone();
        use_effect_with((), move |_| {
            if is_tauri() {
                spawn_local(async move {
                    if let Some(config) = get_app_config_from_tauri().await {
                        config::set_app_config(config);
                        ready.set(true);
                    } else {
                        config::init_browser_config();
                        ready.set(true);
                    }
                });
            }
        });
    }

    if *ready {
        html! {
            <>
                {props.children.clone()}
            </>
        }
    } else {
        html! {
            <div class="min-h-screen flex items-center justify-center bg-gray-50">
                <div class="text-center">
                    <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
                    <p class="mt-4 text-gray-600">{"Loading..."}</p>
                </div>
            </div>
        }
    }
}
