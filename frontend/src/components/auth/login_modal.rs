use web_sys::HtmlInputElement;
use yew::events::SubmitEvent;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::auth::AuthContext;
use crate::Route;

#[derive(Properties, Clone, PartialEq)]
pub struct LoginModalProps {
    pub on_close: Callback<MouseEvent>,
    #[prop_or_default]
    pub show: bool,
}

#[function_component(LoginModal)]
pub fn login_modal(props: &LoginModalProps) -> Html {
    let email = use_state(String::new);
    let password = use_state(String::new);
    let error = use_state(String::new);
    let loading = use_state(|| false);
    let navigator = use_navigator();

    let auth = use_context::<AuthContext>().expect("Auth context not found");

    let onsubmit = {
        let email = email.clone();
        let password = password.clone();
        let error = error.clone();
        let loading = loading.clone();
        let auth = auth.clone();
        let _on_close = props.on_close.clone();

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
            // Don't close modal immediately - wait for login result
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

    // Redirect to profile page after successful login and close modal
    {
        let show = props.show;
        let auth_state = auth.state.clone();
        let navigator = navigator.clone();
        let _on_close = props.on_close.clone();
        use_effect_with((auth_state.player.clone(), show), move |(player, show)| {
            if *show && player.is_some() {
                // Close modal on successful login
                _on_close.emit(MouseEvent::new("click").unwrap());
                // Navigate to profile page
                if let Some(navigator) = &navigator {
                    navigator.push(&Route::Profile);
                }
            }
            || ()
        });
    }

    let on_overlay_click = {
        let _on_close = props.on_close.clone();
        Callback::from(move |e: MouseEvent| {
            _on_close.emit(e);
        })
    };

    let on_close_click = {
        let _on_close = props.on_close.clone();
        Callback::from(move |e: MouseEvent| {
            _on_close.emit(e);
        })
    };

    if !props.show {
        return html! {};
    }

    html! {
        <div class="fixed inset-0 bg-black bg-opacity-50 backdrop-blur-sm flex items-center justify-center z-50" onclick={on_overlay_click}>
            <div class="bg-white rounded-xl shadow-xl w-full max-w-md mx-4 transform transition-all" onclick={|e: MouseEvent| e.stop_propagation()}>
                <div class="flex justify-between items-center p-6 border-b border-gray-200">
                    <h2 class="text-2xl font-semibold text-gray-800">{"Login"}</h2>
                    <button
                        class="text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-full p-2 transition-colors duration-200"
                        onclick={on_close_click}
                    >
                        {"×"}
                    </button>
                </div>
                <div class="p-6">
                    if !error.is_empty() {
                        <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
                            {&*error}
                        </div>
                    }
                    <form onsubmit={onsubmit} class="space-y-4">
                        <div>
                            <label for="email" class="block text-sm font-medium text-gray-700 mb-1">
                                {"Email"}
                            </label>
                            <input
                                type="email"
                                id="email"
                                value={(*email).clone()}
                                onchange={onemailchange}
                                disabled={*loading}
                                class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-gray-50 focus:bg-white transition-colors duration-200 disabled:bg-gray-100 disabled:cursor-not-allowed"
                            />
                        </div>
                        <div>
                            <label for="password" class="block text-sm font-medium text-gray-700 mb-1">
                                {"Password"}
                            </label>
                            <input
                                type="password"
                                id="password"
                                value={(*password).clone()}
                                onchange={onpasswordchange}
                                disabled={*loading}
                                class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-gray-50 focus:bg-white transition-colors duration-200 disabled:bg-gray-100 disabled:cursor-not-allowed"
                            />
                        </div>
                        <button
                            type="submit"
                            disabled={*loading}
                            class="w-full mt-6 px-4 py-2 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transform hover:-translate-y-0.5 transition-all duration-200 disabled:bg-gray-400 disabled:cursor-not-allowed disabled:transform-none"
                        >
                            if *loading {
                                <span class="flex items-center justify-center">
                                    <svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    {"Loading..."}
                                </span>
                            } else {
                                {"Login"}
                            }
                        </button>
                    </form>
                </div>
            </div>
        </div>
    }
}
