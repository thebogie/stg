use yew::prelude::*;
use yew::events::SubmitEvent;
use yew_router::prelude::*;
use web_sys::HtmlInputElement;
use log::debug;

use crate::auth::AuthContext;
use crate::Route;

#[function_component(Login)]
pub fn login() -> Html {
    let email = use_state(String::new);
    let password = use_state(String::new);
    let error = use_state(String::new);
    let loading = use_state(|| false);

    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let navigator = use_navigator().unwrap();

    // Redirect to profile if already authenticated
    {
        let navigator = navigator.clone();
        let auth_state = auth.state.clone();
        use_effect_with((), move |_| {
            if auth_state.player.is_some() {
                debug!("User already authenticated, redirecting to profile");
                navigator.push(&Route::Profile);
            }
            || ()
        });
    }

    let onsubmit = {
        let email = email.clone();
        let password = password.clone();
        let error = error.clone();
        let loading = loading.clone();
        let auth = auth.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let email = email.to_string();
            let password = password.to_string();

            if email.is_empty() || password.is_empty() {
                error.set("Please enter both email and password".to_string());
                return;
            }

            loading.set(true);
            error.set(String::new());
            auth.login.emit((email, password));
        })
    };

    let onemailchange = {
        let email = email.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            email.set(input.value());
        })
    };

    let onpasswordchange = {
        let password = password.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            password.set(input.value());
        })
    };

    // Update loading and error states based on auth state
    {
        let loading = loading.clone();
        let error = error.clone();
        let auth_state = auth.state.clone();
        use_effect_with(auth_state, move |state| {
            loading.set(state.loading);
            if let Some(err) = &state.error {
                error.set(err.clone());
            }
            || ()
        });
    }

    // Redirect to profile page after successful login
    {
        let auth_state = auth.state.clone();
        let navigator = navigator.clone();
        use_effect_with(auth_state.player.clone(), move |player| {
            if player.is_some() {
                debug!("Login successful, redirecting to profile");
                navigator.push(&Route::Profile);
            }
            || ()
        });
    }

    html! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
                        {"Sign in to your account"}
                    </h2>
                </div>
                <form class="mt-8 space-y-6" onsubmit={onsubmit}>
                    <div class="rounded-md shadow-sm -space-y-px">
                        <div>
                            <label for="email" class="sr-only">{"Email address"}</label>
                            <input
                                id="email"
                                name="email"
                                type="email"
                                required=true
                                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 focus:z-10 sm:text-sm"
                                placeholder="Email address"
                                onchange={onemailchange}
                            />
                        </div>
                        <div>
                            <label for="password" class="sr-only">{"Password"}</label>
                            <input
                                id="password"
                                name="password"
                                type="password"
                                required=true
                                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 focus:z-10 sm:text-sm"
                                placeholder="Password"
                                onchange={onpasswordchange}
                            />
                        </div>
                    </div>

                    if !error.is_empty() {
                        <div class="text-red-500 text-sm text-center">
                            {error.to_string()}
                        </div>
                    }

                    <div>
                        <button
                            type="submit"
                            disabled={*loading}
                            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50"
                        >
                            if *loading {
                                <span class="absolute left-0 inset-y-0 flex items-center pl-3">
                                    <svg class="animate-spin h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                </span>
                                {"Signing in..."}
                            } else {
                                {"Sign in"}
                            }
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
} 