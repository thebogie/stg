use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::JsCast;
use crate::api::auth::{update_handle, update_password};
use crate::auth::AuthContext;

#[derive(Properties, PartialEq)]
pub struct ProfileEditorProps {
    pub on_update: Callback<()>,
}

#[derive(PartialEq, Clone)]
pub enum EditMode {
    None,
    Handle,
    Password,
}

#[function_component(ProfileEditor)]
pub fn profile_editor(props: &ProfileEditorProps) -> Html {
    let auth_context = use_context::<AuthContext>().expect("AuthContext not found");
    
    let edit_mode = use_state(|| EditMode::None);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let success = use_state(|| None::<String>);
    
    let new_handle = use_state(|| String::new());
    let current_password = use_state(|| String::new());
    let new_password = use_state(|| String::new());
    let confirm_password = use_state(|| String::new());

    let on_edit_click = {
        let edit_mode = edit_mode.clone();
        let new_handle = new_handle.clone();
        let current_password = current_password.clone();
        let new_password = new_password.clone();
        let confirm_password = confirm_password.clone();
        let error = error.clone();
        let success = success.clone();
        
        Callback::from(move |mode: EditMode| {
            edit_mode.set(mode.clone());
            match mode {
                EditMode::Handle => {
                    new_handle.set(String::new());
                    current_password.set(String::new());
                }
                EditMode::Password => {
                    current_password.set(String::new());
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                }
                EditMode::None => {}
            }
            error.set(None);
            success.set(None);
        })
    };

    let on_cancel = {
        let edit_mode = edit_mode.clone();
        let error = error.clone();
        let success = success.clone();
        
        Callback::from(move |_| {
            edit_mode.set(EditMode::None);
            error.set(None);
            success.set(None);
        })
    };

    let on_handle_update = {
        let edit_mode = edit_mode.clone();
        let loading = loading.clone();
        let error = error.clone();
        let success = success.clone();
        let new_handle = new_handle.clone();
        let current_password = current_password.clone();
        let on_update = props.on_update.clone();
        let auth_context = auth_context.clone();

        Callback::from(move |_| {
            if new_handle.is_empty() || current_password.is_empty() {
                error.set(Some("Please fill in all fields".to_string()));
                return;
            }

            let new_handle_val = new_handle.to_string();
            let current_password_val = current_password.to_string();
            
            loading.set(true);
            error.set(None);
            success.set(None);

            let edit_mode_clone = edit_mode.clone();
            let loading_clone = loading.clone();
            let success_clone = success.clone();
            let error_clone = error.clone();
            let on_update_clone = on_update.clone();
            let auth_context_clone = auth_context.clone();
            spawn_local(async move {
                match update_handle(&new_handle_val, &current_password_val).await {
                    Ok(_) => {
                        success_clone.set(Some("Handle updated successfully".to_string()));
                        on_update_clone.emit(());
                        
                        // Update the auth context with the new player data
                        auth_context_clone.refresh.emit(());
                        
                        // Close the form after a delay
                        gloo_timers::callback::Timeout::new(2000, move || {
                            edit_mode_clone.set(EditMode::None);
                        }).forget();
                    }
                    Err(e) => {
                        error_clone.set(Some(format!("Failed to update handle: {}", e)));
                    }
                }
                loading_clone.set(false);
            });
        })
    };

    let on_password_update = {
        let edit_mode = edit_mode.clone();
        let loading = loading.clone();
        let error = error.clone();
        let success = success.clone();
        let current_password = current_password.clone();
        let new_password = new_password.clone();
        let confirm_password = confirm_password.clone();
        let on_update = props.on_update.clone();
        let auth_context = auth_context.clone();

        Callback::from(move |_| {
            if current_password.is_empty() || new_password.is_empty() || confirm_password.is_empty() {
                error.set(Some("Please fill in all fields".to_string()));
                return;
            }

            if new_password != confirm_password {
                error.set(Some("New passwords do not match".to_string()));
                return;
            }

            if new_password.len() < 8 {
                error.set(Some("New password must be at least 8 characters".to_string()));
                return;
            }

            let current_password_val = current_password.to_string();
            let new_password_val = new_password.to_string();
            
            loading.set(true);
            error.set(None);
            success.set(None);

            let edit_mode_clone = edit_mode.clone();
            let loading_clone = loading.clone();
            let success_clone = success.clone();
            let error_clone = error.clone();
            let on_update_clone = on_update.clone();
            let auth_context_clone = auth_context.clone();
            spawn_local(async move {
                match update_password(&current_password_val, &new_password_val).await {
                    Ok(_) => {
                        success_clone.set(Some("Password updated successfully".to_string()));
                        on_update_clone.emit(());
                        
                        // Update the auth context with the new player data
                        auth_context_clone.refresh.emit(());
                        
                        // Close the form after a delay
                        gloo_timers::callback::Timeout::new(2000, move || {
                            edit_mode_clone.set(EditMode::None);
                        }).forget();
                    }
                    Err(e) => {
                        error_clone.set(Some(format!("Failed to update password: {}", e)));
                    }
                }
                loading_clone.set(false);
            });
        })
    };

    // Force re-render when auth context changes
    let auth_state = auth_context.state.clone();
    use_effect_with(auth_state, |_| {
        // This effect will run whenever the auth context state changes
        // causing the component to re-render with updated player data
        || ()
    });

    let player = match &auth_context.state.player {
        Some(p) => p,
        None => return html! { <div class="text-red-500">{"Not authenticated"}</div> },
    };

    html! {
        <div class="bg-white shadow rounded-lg p-6">
            <h2 class="text-2xl font-bold text-gray-900 mb-6">{"Profile Settings"}</h2>
            
            // Current Profile Display
            <div class="mb-6 p-4 bg-gray-50 rounded-lg">
                <h3 class="text-lg font-semibold text-gray-700 mb-3">{"Current Profile"}</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-600">{"Email"}</label>
                        <p class="text-gray-900">{&player.email}</p>
                        <p class="text-xs text-gray-500 mt-1">{"Email cannot be changed (used as unique identifier)"}</p>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-600">{"Handle"}</label>
                        <p class="text-gray-900">{&player.handle}</p>
                    </div>
                </div>
            </div>

            // Error and Success Messages
            if let Some(error_msg) = (*error).as_ref() {
                <div class="mb-4 p-4 bg-red-100 border border-red-400 text-red-700 rounded">
                    {error_msg}
                </div>
            }
            
            if let Some(success_msg) = (*success).as_ref() {
                <div class="mb-4 p-4 bg-green-100 border border-green-400 text-green-700 rounded">
                    {success_msg}
                </div>
            }

            // Edit Forms
            {match *edit_mode {
                EditMode::None => {
                    html! {
                        <div class="space-y-4">
                            <button 
                                onclick={on_edit_click.reform(|_| EditMode::Handle)}
                                class="w-full md:w-auto px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                            >
                                {"Update Handle"}
                            </button>
                            <button 
                                onclick={on_edit_click.reform(|_| EditMode::Password)}
                                class="w-full md:w-auto px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                            >
                                {"Update Password"}
                            </button>
                        </div>
                    }
                }
                EditMode::Handle => {
                    html! {
                        <div class="space-y-4">
                            <h3 class="text-lg font-semibold text-gray-700">{"Update Handle"}</h3>
                            <div>
                                <label class="block text-sm font-medium text-gray-600 mb-2">{"New Handle"}</label>
                                <input
                                    type="text"
                                    value={(*new_handle).clone()}
                                    onchange={Callback::from(move |e: Event| {
                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
                                        new_handle.set(target.value());
                                    })}
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    placeholder="Enter new handle (3-50 characters, letters/numbers/underscores only)"
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-600 mb-2">{"Current Password"}</label>
                                <input
                                    type="password"
                                    value={(*current_password).clone()}
                                    onchange={Callback::from(move |e: Event| {
                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
                                        current_password.set(target.value());
                                    })}
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    placeholder="Enter current password"
                                />
                            </div>
                            <div class="flex space-x-3">
                                <button 
                                    onclick={on_handle_update}
                                    disabled={*loading}
                                    class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50 transition-colors"
                                >
                                    {if *loading { "Updating..." } else { "Update Handle" }}
                                </button>
                                <button 
                                    onclick={on_cancel}
                                    class="px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 transition-colors"
                                >
                                    {"Cancel"}
                                </button>
                            </div>
                        </div>
                    }
                }
                EditMode::Password => {
                    html! {
                        <div class="space-y-4">
                            <h3 class="text-lg font-semibold text-gray-700">{"Update Password"}</h3>
                            <div>
                                <label class="block text-sm font-medium text-gray-600 mb-2">{"Current Password"}</label>
                                <input
                                    type="password"
                                    value={(*current_password).clone()}
                                    onchange={Callback::from(move |e: Event| {
                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
                                        current_password.set(target.value());
                                    })}
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    placeholder="Enter current password"
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-600 mb-2">{"New Password"}</label>
                                <input
                                    type="password"
                                    value={(*new_password).clone()}
                                    onchange={Callback::from(move |e: Event| {
                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
                                        new_password.set(target.value());
                                    })}
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    placeholder="Enter new password (min 8 characters)"
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-600 mb-2">{"Confirm New Password"}</label>
                                <input
                                    type="password"
                                    value={(*confirm_password).clone()}
                                    onchange={Callback::from(move |e: Event| {
                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
                                        confirm_password.set(target.value());
                                    })}
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    placeholder="Confirm new password"
                                />
                            </div>
                            <div class="flex space-x-3">
                                <button 
                                    onclick={on_password_update}
                                    disabled={*loading}
                                    class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50 transition-colors"
                                >
                                    {if *loading { "Updating..." } else { "Update Password" }}
                                </button>
                                <button 
                                    onclick={on_cancel}
                                    class="px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 transition-colors"
                                >
                                    {"Cancel"}
                                </button>
                            </div>
                        </div>
                    }
                }
            }}
        </div>
    }
}
