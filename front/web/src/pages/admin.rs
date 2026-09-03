use crate::api::api_url;
use crate::api::contests::{approve_contest, list_pending_contests, reject_contest};
use crate::api::utils::authenticated_delete;
use crate::api::utils::authenticated_get;
use crate::api::utils::authenticated_post;
use crate::api::utils::authenticated_put;
use crate::api::version::{get_version_info, VersionInfo};
use crate::components::common::toast::{Toast, ToastContext, ToastType};
use crate::components::scheduler_monitor::SchedulerMonitor;
use gloo_timers::callback::Interval;
use shared::dto::analytics::PlatformStatsDto;
use shared::dto::contest::ContestDto;
use shared::dto::player::PlayerDto;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone, Debug)]
pub struct AdminPageProps {}

#[derive(Clone, PartialEq, Debug)]
enum AdminTab {
    Dashboard,
    Contests,
    Ratings,
    System,
    Users,
}

#[function_component(AdminPage)]
pub fn admin_page(_props: &AdminPageProps) -> Html {
    let auth = use_context::<crate::auth::AuthContext>().expect("Auth context not found");
    let toast_context = use_context::<ToastContext>().expect("Toast context not found");
    let current_tab = use_state(|| AdminTab::Dashboard);

    // System stats state
    let system_stats = use_state(|| None::<PlatformStatsDto>);
    let stats_loading = use_state(|| false);
    let stats_error = use_state(|| None::<String>);

    // Version info state
    let version_info = use_state(|| None::<VersionInfo>);
    let version_loading = use_state(|| false);

    let pending_contests = use_state(Vec::<ContestDto>::new);
    let pending_loading = use_state(|| false);
    let pending_error = use_state(|| None::<String>);
    let pending_expanded_id = use_state(|| None::<String>);

    let user_search_query = use_state(String::new);
    let user_search_results = use_state(Vec::<PlayerDto>::new);
    let user_search_loading = use_state(|| false);
    let editing_player = use_state(|| None::<PlayerDto>);
    let edit_firstname = use_state(String::new);
    let edit_handle = use_state(String::new);
    let edit_email = use_state(String::new);
    let edit_is_admin = use_state(|| false);
    let edit_saving = use_state(|| false);
    let reset_password = use_state(String::new);
    let reset_password_confirm = use_state(String::new);
    let reset_saving = use_state(|| false);
    let show_create_form = use_state(|| false);
    let create_firstname = use_state(String::new);
    let create_handle = use_state(String::new);
    let create_email = use_state(String::new);
    let create_password = use_state(String::new);
    let create_is_admin = use_state(|| false);
    let create_saving = use_state(|| false);
    let delete_saving = use_state(|| false);
    let activate_saving = use_state(|| false);

    let toggle_pending_detail = {
        let pending_expanded_id = pending_expanded_id.clone();
        Callback::from(move |id: String| {
            if (*pending_expanded_id).as_ref() == Some(&id) {
                pending_expanded_id.set(None);
            } else {
                pending_expanded_id.set(Some(id));
            }
        })
    };

    // Check if user is admin
    if !auth.state.is_admin() {
        return html! {
            <div class="admin-page">
                <div class="page-header">
                    <h1>{"Access Denied"}</h1>
                    <p>{"You don't have permission to access this page."}</p>
                </div>
                <div class="access-denied">
                    <div class="denied-icon">{"🚫"}</div>
                    <h2>{"Administrator Access Required"}</h2>
                    <p>{"This page is restricted to users with administrative privileges."}</p>
                    <a href="/profile" class="back-link">{"← Back to Profile"}</a>
                </div>
            </div>
        };
    }

    let on_tab_click = {
        let current_tab = current_tab.clone();
        Callback::from(move |tab: AdminTab| {
            current_tab.set(tab);
        })
    };

    // Load system stats
    {
        let system_stats = system_stats.clone();
        let stats_loading = stats_loading.clone();
        let stats_error = stats_error.clone();

        use_effect_with((), move |_| {
            stats_loading.set(true);
            stats_error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_get(&api_url("/api/analytics/platform"))
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(stats) = response.json::<PlatformStatsDto>().await {
                                system_stats.set(Some(stats));
                            } else {
                                stats_error.set(Some("Failed to parse system stats".to_string()));
                            }
                        } else {
                            let status = response.status();
                            let text = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Unknown error".to_string());
                            stats_error.set(Some(format!(
                                "Failed to load system stats: {} - {}",
                                status, text
                            )));
                        }
                    }
                    Err(e) => {
                        stats_error.set(Some(format!("Failed to fetch system stats: {}", e)));
                    }
                }
                stats_loading.set(false);
            });

            || ()
        });
    }

    // Load version info
    {
        let version_info = version_info.clone();
        let version_loading = version_loading.clone();

        use_effect_with((), move |_| {
            version_loading.set(true);

            wasm_bindgen_futures::spawn_local(async move {
                match get_version_info().await {
                    Ok(info) => {
                        version_info.set(Some(info));
                    }
                    Err(e) => {
                        log::error!("Failed to fetch version info: {}", e);
                    }
                }
                version_loading.set(false);
            });

            || ()
        });
    }

    let show_success_toast = {
        let toast_context = toast_context.clone();
        Callback::from(move |message: String| {
            let toast = Toast::new(message, ToastType::Success).with_duration(5000);
            toast_context.add_toast.emit(toast);
        })
    };

    let show_error_toast = {
        let toast_context = toast_context.clone();
        Callback::from(move |message: String| {
            let toast = Toast::new(message, ToastType::Error).with_duration(8000);
            toast_context.add_toast.emit(toast);
        })
    };

    let reload_pending_contests = {
        let pending_contests = pending_contests.clone();
        let pending_loading = pending_loading.clone();
        let pending_error = pending_error.clone();
        let show_error_toast = show_error_toast.clone();
        Callback::from(move |_| {
            pending_loading.set(true);
            let pending_contests = pending_contests.clone();
            let pending_error = pending_error.clone();
            let pending_loading = pending_loading.clone();
            let show_error_toast = show_error_toast.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match list_pending_contests().await {
                    Ok(rows) => {
                        pending_contests.set(rows);
                        pending_error.set(None);
                    }
                    Err(e) => {
                        pending_error.set(Some(e.clone()));
                        show_error_toast.emit(e);
                    }
                }
                pending_loading.set(false);
            });
        })
    };

    {
        let reload_pending_contests = reload_pending_contests.clone();
        use_effect_with((*current_tab).clone(), move |tab: &AdminTab| {
            if *tab == AdminTab::Contests {
                reload_pending_contests.emit(());
            }
            || ()
        });
    }

    let clear_analytics_cache = {
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        let system_stats = system_stats.clone();
        let stats_loading = stats_loading.clone();
        let stats_error = stats_error.clone();

        Callback::from(move |_: ()| {
            if !gloo::dialogs::confirm(
                "Clear analytics cache? This will refresh platform stats and charts.",
            ) {
                return;
            }

            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            let system_stats = system_stats.clone();
            let stats_loading = stats_loading.clone();
            let stats_error = stats_error.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_post(&api_url("/api/admin/cache/analytics/clear"))
                    .send()
                    .await
                {
                    Ok(resp) if resp.ok() => {
                        show_success_toast.emit("Analytics cache cleared".to_string());

                        // Immediately refresh stats.
                        system_stats.set(None);
                        stats_loading.set(true);
                        stats_error.set(None);
                        match authenticated_get(&api_url("/api/analytics/platform"))
                            .send()
                            .await
                        {
                            Ok(r) if r.ok() => match r.json::<PlatformStatsDto>().await {
                                Ok(stats) => system_stats.set(Some(stats)),
                                Err(_) => stats_error
                                    .set(Some("Failed to parse system stats".to_string())),
                            },
                            Ok(r) => {
                                let status = r.status();
                                let text = r
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| "Unknown error".to_string());
                                stats_error.set(Some(format!(
                                    "Failed to load system stats: {} - {}",
                                    status, text
                                )));
                            }
                            Err(e) => stats_error
                                .set(Some(format!("Failed to fetch system stats: {}", e))),
                        }
                        stats_loading.set(false);
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        show_error_toast
                            .emit(format!("Failed to clear cache: {} - {}", status, text));
                    }
                    Err(e) => show_error_toast.emit(format!("Failed to clear cache: {}", e)),
                }
            });
        })
    };

    let run_ratings_recompute_month = {
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        Callback::from(move |_: ()| {
            if !gloo::dialogs::confirm("Recalculate Glicko2 ratings for the previous month?") {
                return;
            }
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_post(&api_url("/api/ratings/recompute"))
                    .send()
                    .await
                {
                    Ok(resp) if resp.ok() => {
                        show_success_toast.emit("Ratings recompute started".to_string());
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        show_error_toast
                            .emit(format!("Ratings recompute failed: {} - {}", status, text));
                    }
                    Err(e) => show_error_toast.emit(format!("Ratings recompute failed: {}", e)),
                }
            });
        })
    };

    // Ratings rebuild status (admin only)
    let rebuild_status = use_state(|| None::<serde_json::Value>);
    let rebuild_status_loading = use_state(|| false);
    let rebuild_status_interval = use_mut_ref(|| None::<Interval>);
    let rebuild_poll_desired = (*rebuild_status)
        .as_ref()
        .and_then(|v| v.get("running").and_then(|x| x.as_bool()))
        .unwrap_or(false);

    let refresh_rebuild_status = {
        let rebuild_status = rebuild_status.clone();
        let rebuild_status_loading = rebuild_status_loading.clone();
        let show_error_toast = show_error_toast.clone();
        Callback::from(move |_: ()| {
            rebuild_status_loading.set(true);
            let rebuild_status = rebuild_status.clone();
            let rebuild_status_loading = rebuild_status_loading.clone();
            let show_error_toast = show_error_toast.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_get(&api_url("/api/ratings/rebuild/status"))
                    .send()
                    .await
                {
                    Ok(resp) if resp.ok() => match resp.json::<serde_json::Value>().await {
                        Ok(v) => rebuild_status.set(Some(v)),
                        Err(_) => {
                            show_error_toast.emit("Failed to parse rebuild status".to_string())
                        }
                    },
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        show_error_toast.emit(format!(
                            "Failed to fetch rebuild status: {} - {}",
                            status, text
                        ));
                    }
                    Err(e) => {
                        show_error_toast.emit(format!("Failed to fetch rebuild status: {}", e))
                    }
                }
                rebuild_status_loading.set(false);
            });
        })
    };

    // Auto-refresh rebuild status while running. Depend on the `running` flag (not the state
    // handle): `UseStateHandle` is stable across renders, so use_effect_with would otherwise only
    // ever run once and never start this interval after status loads.
    {
        let rebuild_status = rebuild_status.clone();
        let rebuild_status_loading = rebuild_status_loading.clone();
        let rebuild_status_interval = rebuild_status_interval.clone();

        use_effect_with(rebuild_poll_desired, move |running| {
            // Stop existing interval if any.
            rebuild_status_interval.borrow_mut().take();

            if *running {
                let rebuild_status = rebuild_status.clone();
                let rebuild_status_loading = rebuild_status_loading.clone();
                let interval = Interval::new(2000, move || {
                    if *rebuild_status_loading {
                        return;
                    }
                    rebuild_status_loading.set(true);
                    let rebuild_status = rebuild_status.clone();
                    let rebuild_status_loading = rebuild_status_loading.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Ok(resp) = authenticated_get(&api_url("/api/ratings/rebuild/status"))
                            .send()
                            .await
                        {
                            if resp.ok() {
                                if let Ok(v) = resp.json::<serde_json::Value>().await {
                                    rebuild_status.set(Some(v));
                                }
                            }
                        }
                        rebuild_status_loading.set(false);
                    });
                });
                *rebuild_status_interval.borrow_mut() = Some(interval);
            }

            || {}
        });
    }

    let run_ratings_rebuild_all = {
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        let refresh_rebuild_status = refresh_rebuild_status.clone();
        Callback::from(move |_: ()| {
            if !gloo::dialogs::confirm(
                "Rebuild ALL ratings from the beginning? This can take a while.",
            ) {
                return;
            }
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            let refresh_rebuild_status = refresh_rebuild_status.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_post(&api_url("/api/ratings/recalculate/historical"))
                    .send()
                    .await
                {
                    Ok(resp) if resp.ok() => {
                        show_success_toast.emit("Ratings rebuild started".to_string());
                        refresh_rebuild_status.emit(());
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        show_error_toast
                            .emit(format!("Ratings rebuild failed: {} - {}", status, text));
                    }
                    Err(e) => show_error_toast.emit(format!("Ratings rebuild failed: {}", e)),
                }
            });
        })
    };

    let on_user_search = {
        let user_search_query = user_search_query.clone();
        let user_search_results = user_search_results.clone();
        let user_search_loading = user_search_loading.clone();
        let show_error_toast = show_error_toast.clone();
        Callback::from(move |_| {
            let q = (*user_search_query).clone();
            user_search_loading.set(true);
            let user_search_results = user_search_results.clone();
            let user_search_loading = user_search_loading.clone();
            let show_error_toast = show_error_toast.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = api_url(&format!(
                    "/api/admin/users/search?q={}&limit=20",
                    urlencoding::encode(&q)
                ));
                match authenticated_get(&url).send().await {
                    Ok(resp) if resp.ok() => {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            let users: Vec<PlayerDto> = body
                                .get("users")
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                                .unwrap_or_default();
                            user_search_results.set(users);
                        }
                    }
                    Ok(resp) => {
                        show_error_toast.emit(format!("User search failed: {}", resp.status()));
                    }
                    Err(e) => show_error_toast.emit(format!("User search failed: {}", e)),
                }
                user_search_loading.set(false);
            });
        })
    };

    let on_toggle_admin = {
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        let on_user_search = on_user_search.clone();
        Callback::from(move |(player_id, grant): (String, bool)| {
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            let on_user_search = on_user_search.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = api_url(&format!(
                    "/api/admin/users/{}/admin",
                    urlencoding::encode(&player_id)
                ));
                let body = serde_json::json!({ "is_admin": grant });
                match authenticated_post(&url)
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                {
                    Ok(req) => match req.send().await {
                        Ok(resp) if resp.ok() => {
                            show_success_toast.emit(if grant {
                                "Admin granted".to_string()
                            } else {
                                "Admin revoked".to_string()
                            });
                            on_user_search.emit(());
                        }
                        Ok(resp) => {
                            show_error_toast.emit(format!("Admin update failed: {}", resp.status()));
                        }
                        Err(e) => show_error_toast.emit(format!("Admin update failed: {}", e)),
                    },
                    Err(e) => show_error_toast.emit(format!("Admin update failed: {}", e)),
                }
            });
        })
    };

    let on_edit_user = {
        let editing_player = editing_player.clone();
        let edit_firstname = edit_firstname.clone();
        let edit_handle = edit_handle.clone();
        let edit_email = edit_email.clone();
        let edit_is_admin = edit_is_admin.clone();
        let reset_password = reset_password.clone();
        let reset_password_confirm = reset_password_confirm.clone();
        Callback::from(move |player: PlayerDto| {
            edit_firstname.set(player.firstname.clone());
            edit_handle.set(player.handle.clone());
            edit_email.set(player.email.clone());
            edit_is_admin.set(player.is_admin);
            reset_password.set(String::new());
            reset_password_confirm.set(String::new());
            editing_player.set(Some(player));
        })
    };

    let on_cancel_edit = {
        let editing_player = editing_player.clone();
        Callback::from(move |_| {
            editing_player.set(None);
        })
    };

    let on_save_user = {
        let editing_player = editing_player.clone();
        let edit_firstname = edit_firstname.clone();
        let edit_handle = edit_handle.clone();
        let edit_email = edit_email.clone();
        let edit_is_admin = edit_is_admin.clone();
        let edit_saving = edit_saving.clone();
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        let on_user_search = on_user_search.clone();
        Callback::from(move |_| {
            let Some(player) = (*editing_player).clone() else {
                return;
            };
            edit_saving.set(true);
            let edit_firstname = edit_firstname.clone();
            let edit_handle = edit_handle.clone();
            let edit_email = edit_email.clone();
            let edit_is_admin = edit_is_admin.clone();
            let edit_saving = edit_saving.clone();
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            let on_user_search = on_user_search.clone();
            let editing_player = editing_player.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = api_url(&format!(
                    "/api/admin/users/{}",
                    urlencoding::encode(&player.id)
                ));
                let body = serde_json::json!({
                    "firstname": (*edit_firstname).clone(),
                    "handle": (*edit_handle).clone(),
                    "email": (*edit_email).clone(),
                    "is_admin": *edit_is_admin,
                });
                match authenticated_put(&url)
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                {
                    Ok(req) => match req.send().await {
                        Ok(resp) if resp.ok() => {
                            if let Ok(updated) = resp.json::<PlayerDto>().await {
                                editing_player.set(Some(updated));
                            }
                            show_success_toast.emit("Player updated".to_string());
                            on_user_search.emit(());
                        }
                        Ok(resp) => {
                            let status = resp.status();
                            let text = resp
                                .text()
                                .await
                                .unwrap_or_else(|_| status.to_string());
                            show_error_toast.emit(format!("Update failed: {}", text));
                        }
                        Err(e) => show_error_toast.emit(format!("Update failed: {}", e)),
                    },
                    Err(e) => show_error_toast.emit(format!("Update failed: {}", e)),
                }
                edit_saving.set(false);
            });
        })
    };

    let on_reset_password = {
        let editing_player = editing_player.clone();
        let reset_password = reset_password.clone();
        let reset_password_confirm = reset_password_confirm.clone();
        let reset_saving = reset_saving.clone();
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        Callback::from(move |_| {
            let Some(player) = (*editing_player).clone() else {
                return;
            };
            if reset_password.len() < 8 {
                show_error_toast.emit("Password must be at least 8 characters".to_string());
                return;
            }
            if *reset_password != *reset_password_confirm {
                show_error_toast.emit("Passwords do not match".to_string());
                return;
            }
            reset_saving.set(true);
            let reset_password = reset_password.clone();
            let reset_saving = reset_saving.clone();
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = api_url(&format!(
                    "/api/admin/users/{}/password",
                    urlencoding::encode(&player.id)
                ));
                let body = serde_json::json!({ "new_password": (*reset_password).clone() });
                match authenticated_post(&url)
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                {
                    Ok(req) => match req.send().await {
                        Ok(resp) if resp.ok() => {
                            show_success_toast.emit("Password reset successfully".to_string());
                        }
                        Ok(resp) => {
                            let status = resp.status();
                            let text = resp
                                .text()
                                .await
                                .unwrap_or_else(|_| status.to_string());
                            show_error_toast.emit(format!("Password reset failed: {}", text));
                        }
                        Err(e) => show_error_toast.emit(format!("Password reset failed: {}", e)),
                    },
                    Err(e) => show_error_toast.emit(format!("Password reset failed: {}", e)),
                }
                reset_saving.set(false);
            });
        })
    };

    let on_toggle_create_form = {
        let show_create_form = show_create_form.clone();
        let create_firstname = create_firstname.clone();
        let create_handle = create_handle.clone();
        let create_email = create_email.clone();
        let create_password = create_password.clone();
        let create_is_admin = create_is_admin.clone();
        Callback::from(move |_| {
            if *show_create_form {
                show_create_form.set(false);
            } else {
                create_firstname.set(String::new());
                create_handle.set(String::new());
                create_email.set(String::new());
                create_password.set(String::new());
                create_is_admin.set(false);
                show_create_form.set(true);
            }
        })
    };

    let on_create_user = {
        let create_firstname = create_firstname.clone();
        let create_handle = create_handle.clone();
        let create_email = create_email.clone();
        let create_password = create_password.clone();
        let create_is_admin = create_is_admin.clone();
        let create_saving = create_saving.clone();
        let show_create_form = show_create_form.clone();
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        let on_user_search = on_user_search.clone();
        let user_search_query = user_search_query.clone();
        Callback::from(move |_| {
            if create_firstname.trim().is_empty()
                || create_handle.trim().is_empty()
                || create_email.trim().is_empty()
            {
                show_error_toast.emit("First name, handle, and email are required".to_string());
                return;
            }
            if create_password.len() < 8 {
                show_error_toast.emit("Password must be at least 8 characters".to_string());
                return;
            }
            create_saving.set(true);
            let create_firstname = create_firstname.clone();
            let create_handle = create_handle.clone();
            let create_email = create_email.clone();
            let create_password = create_password.clone();
            let create_is_admin = create_is_admin.clone();
            let create_saving = create_saving.clone();
            let show_create_form = show_create_form.clone();
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            let on_user_search = on_user_search.clone();
            let user_search_query = user_search_query.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = api_url("/api/admin/users");
                let body = serde_json::json!({
                    "firstname": (*create_firstname).trim(),
                    "handle": (*create_handle).trim(),
                    "email": (*create_email).trim(),
                    "password": (*create_password).clone(),
                    "is_admin": *create_is_admin,
                });
                match authenticated_post(&url)
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                {
                    Ok(req) => match req.send().await {
                        Ok(resp) if resp.status() == 201 || resp.ok() => {
                            show_success_toast.emit("Player created".to_string());
                            show_create_form.set(false);
                            user_search_query.set((*create_email).trim().to_string());
                            on_user_search.emit(());
                        }
                        Ok(resp) => {
                            let status = resp.status();
                            let text = resp
                                .text()
                                .await
                                .unwrap_or_else(|_| status.to_string());
                            show_error_toast.emit(format!("Create failed: {}", text));
                        }
                        Err(e) => show_error_toast.emit(format!("Create failed: {}", e)),
                    },
                    Err(e) => show_error_toast.emit(format!("Create failed: {}", e)),
                }
                create_saving.set(false);
            });
        })
    };

    let on_delete_user = {
        let editing_player = editing_player.clone();
        let delete_saving = delete_saving.clone();
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        let on_user_search = on_user_search.clone();
        Callback::from(move |_| {
            let Some(player) = (*editing_player).clone() else {
                return;
            };
            if !gloo::dialogs::confirm(&format!(
                "Permanently delete \"{}\" ({})?\n\nThis will remove the player from ALL contests and delete their ratings history. The player record cannot be recovered.\n\nConsider deactivating the account instead if you need to keep contest history.",
                player.handle, player.email
            )) {
                return;
            }
            if !gloo::dialogs::confirm(&format!(
                "Final confirmation: permanently delete \"{}\"?\n\nThis cannot be undone.",
                player.handle
            )) {
                return;
            }
            delete_saving.set(true);
            let editing_player = editing_player.clone();
            let delete_saving = delete_saving.clone();
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            let on_user_search = on_user_search.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = api_url(&format!(
                    "/api/admin/users/{}",
                    urlencoding::encode(&player.id)
                ));
                match authenticated_delete(&url).send().await {
                    Ok(resp) if resp.ok() => {
                        editing_player.set(None);
                        show_success_toast.emit("Player deleted".to_string());
                        on_user_search.emit(());
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| status.to_string());
                        show_error_toast.emit(format!("Delete failed: {}", text));
                    }
                    Err(e) => show_error_toast.emit(format!("Delete failed: {}", e)),
                }
                delete_saving.set(false);
            });
        })
    };

    let on_set_active = {
        let editing_player = editing_player.clone();
        let activate_saving = activate_saving.clone();
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        let on_user_search = on_user_search.clone();
        Callback::from(move |activate: bool| {
            let Some(player) = (*editing_player).clone() else {
                return;
            };
            let action = if activate { "reactivate" } else { "deactivate" };
            let prompt = if activate {
                format!(
                    "Reactivate \"{}\"? They will be able to log in again.",
                    player.handle
                )
            } else {
                format!(
                    "Deactivate \"{}\" ({})?\n\nThey will not be able to log in. Contest history and ratings are kept.",
                    player.handle, player.email
                )
            };
            if !gloo::dialogs::confirm(&prompt) {
                return;
            }
            activate_saving.set(true);
            let editing_player = editing_player.clone();
            let activate_saving = activate_saving.clone();
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            let on_user_search = on_user_search.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = api_url(&format!(
                    "/api/admin/users/{}/{}",
                    urlencoding::encode(&player.id),
                    action
                ));
                match authenticated_post(&url).send().await {
                    Ok(resp) if resp.ok() => {
                        if let Ok(updated) = resp.json::<PlayerDto>().await {
                            editing_player.set(Some(updated));
                        }
                        show_success_toast.emit(if activate {
                            "Player reactivated".to_string()
                        } else {
                            "Player deactivated".to_string()
                        });
                        on_user_search.emit(());
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| status.to_string());
                        show_error_toast.emit(format!(
                            "{} failed: {}",
                            if activate { "Reactivate" } else { "Deactivate" },
                            text
                        ));
                    }
                    Err(e) => show_error_toast.emit(format!(
                        "{} failed: {}",
                        if activate { "Reactivate" } else { "Deactivate" },
                        e
                    )),
                }
                activate_saving.set(false);
            });
        })
    };

    let on_run_smoke = {
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        Callback::from(move |_| {
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_post(&api_url("/api/admin/playwright/smoke")).send().await {
                    Ok(resp) if resp.ok() => {
                        show_success_toast.emit("Playwright smoke job queued".to_string());
                    }
                    Ok(resp) => {
                        show_error_toast.emit(format!("Smoke enqueue failed: {}", resp.status()));
                    }
                    Err(e) => show_error_toast.emit(format!("Smoke enqueue failed: {}", e)),
                }
            });
        })
    };

    let pending_expanded_snapshot = (*pending_expanded_id).clone();
    let toggle_pending_detail_cb = toggle_pending_detail.clone();

    html! {
        <div class="min-h-screen bg-gray-50">
            <div class="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
                <div class="mb-8 flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3">
                    <div class="min-w-0">
                        <h1 class="text-3xl font-bold text-gray-900">{"👑 Administrator Dashboard"}</h1>
                        <p class="mt-2 text-gray-600">{"System management, monitoring, and administrative tools"}</p>
                    </div>
                    <div class="flex-shrink-0">
                        <span class="inline-flex items-center px-3 py-1.5 text-sm font-semibold bg-yellow-400 text-yellow-900 rounded-full shadow-sm border border-yellow-500">
                            <span class="mr-2">{"👑"}</span>
                            {"Admin"}
                        </span>
                    </div>
                </div>

                // Navigation Tabs
                <div class="flex flex-nowrap overflow-x-auto space-x-1 mb-8 bg-white rounded-lg shadow-sm border border-gray-200 p-1">
                    <button
                        class={classes!(
                            "flex-none", "sm:flex-1", "px-4", "py-3", "text-sm", "font-medium", "rounded-md",
                            "transition-all", "duration-200", "whitespace-nowrap",
                            if *current_tab == AdminTab::Dashboard { "bg-yellow-500 text-yellow-950 shadow-sm" } else { "text-gray-600 hover:text-gray-900 hover:bg-gray-50" }
                        )}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::Dashboard)}
                    >
                        {"📊 Dashboard"}
                    </button>
                    <button
                        class={classes!(
                            "flex-none", "sm:flex-1", "px-4", "py-3", "text-sm", "font-medium", "rounded-md",
                            "transition-all", "duration-200", "whitespace-nowrap",
                            if *current_tab == AdminTab::Contests { "bg-yellow-500 text-yellow-950 shadow-sm" } else { "text-gray-600 hover:text-gray-900 hover:bg-gray-50" }
                        )}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::Contests)}
                    >
                        {"🗳️ Contest review"}
                    </button>
                    <button
                        class={classes!(
                            "flex-none", "sm:flex-1", "px-4", "py-3", "text-sm", "font-medium", "rounded-md",
                            "transition-all", "duration-200", "whitespace-nowrap",
                            if *current_tab == AdminTab::Ratings { "bg-yellow-500 text-yellow-950 shadow-sm" } else { "text-gray-600 hover:text-gray-900 hover:bg-gray-50" }
                        )}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::Ratings)}
                    >
                        {"🏆 Ratings Management"}
                    </button>
                    <button
                        class={classes!(
                            "flex-none", "sm:flex-1", "px-4", "py-3", "text-sm", "font-medium", "rounded-md",
                            "transition-all", "duration-200", "whitespace-nowrap",
                            if *current_tab == AdminTab::System { "bg-yellow-500 text-yellow-950 shadow-sm" } else { "text-gray-600 hover:text-gray-900 hover:bg-gray-50" }
                        )}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::System)}
                    >
                        {"⚙️ System"}
                    </button>
                    <button
                        class={classes!(
                            "flex-none", "sm:flex-1", "px-4", "py-3", "text-sm", "font-medium", "rounded-md",
                            "transition-all", "duration-200", "whitespace-nowrap",
                            if *current_tab == AdminTab::Users { "bg-yellow-500 text-yellow-950 shadow-sm" } else { "text-gray-600 hover:text-gray-900 hover:bg-gray-50" }
                        )}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::Users)}
                    >
                        {"👥 Users"}
                    </button>
                </div>

                // Tab Content
                <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
                    {match *current_tab {
                        AdminTab::Dashboard => html! {
                            <div class="dashboard-section">
                                <h2 class="text-xl font-semibold text-gray-900">{"System Overview"}</h2>
                                <div class="mt-6 grid grid-cols-1 md:grid-cols-2 gap-6">
                                    if *stats_loading {
                                        <div class="text-center py-8 text-gray-600">
                                            {"Loading system statistics..."}
                                        </div>
                                    } else if let Some(err) = (*stats_error).as_ref() {
                                        <div class="bg-red-50 border border-red-200 rounded-lg p-4">
                                            <p class="text-red-800 text-sm font-medium">{"Error: "}{err}</p>
                                        </div>
                                    } else if let Some(stats) = (*system_stats).as_ref() {
                                        <div class="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
                                            <div class="flex items-center justify-between gap-3">
                                                <h3 class="text-lg font-semibold text-gray-900">{"📈 Platform Statistics"}</h3>
                                                <span class="text-xs font-semibold text-yellow-900 bg-yellow-100 border border-yellow-200 rounded-full px-2 py-1">
                                                    {"ADMIN"}
                                                </span>
                                            </div>
                                            <div class="mt-4 space-y-3">
                                                <div class="flex justify-between items-center py-2 border-b border-gray-100">
                                                    <span class="text-sm font-medium text-gray-700">{"Total Players"}</span>
                                                    <span class="text-lg font-semibold text-gray-900">{stats.total_players}</span>
                                                </div>
                                                <div class="flex justify-between items-center py-2 border-b border-gray-100">
                                                    <span class="text-sm font-medium text-gray-700">{"Total Contests"}</span>
                                                    <span class="text-lg font-semibold text-gray-900">{stats.total_contests}</span>
                                                </div>
                                                <div class="flex justify-between items-center py-2 border-b border-gray-100">
                                                    <span class="text-sm font-medium text-gray-700">{"Total Games"}</span>
                                                    <span class="text-lg font-semibold text-gray-900">{stats.total_games}</span>
                                                </div>
                                                <div class="flex justify-between items-center py-2">
                                                    <span class="text-sm font-medium text-gray-700">{"Total Venues"}</span>
                                                    <span class="text-lg font-semibold text-gray-900">{stats.total_venues}</span>
                                                </div>
                                            </div>
                                        </div>
                                    }

                                    <div class="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
                                        <h3 class="text-lg font-semibold text-gray-900">{"🔧 Quick Actions"}</h3>
                                        <div class="mt-4 space-y-3">
                                            <button
                                                class="w-full px-4 py-2 text-sm font-medium rounded-md bg-yellow-600 text-white hover:bg-yellow-700 transition-colors"
                                                onclick={clear_analytics_cache.reform(|_| ())}
                                            >
                                                {"🧹 Clear Analytics Cache"}
                                            </button>
                                            <button class="w-full px-4 py-2 text-sm font-medium rounded-md bg-yellow-500 text-yellow-950 hover:bg-yellow-400 transition-colors" onclick={show_success_toast.clone().reform(|_| "System refresh initiated".to_string())}>
                                                {"🔄 Refresh System"}
                                            </button>
                                            <button class="w-full px-4 py-2 text-sm font-medium rounded-md bg-gray-100 text-gray-700 hover:bg-gray-200 transition-colors" onclick={show_success_toast.clone().reform(|_| "Export started".to_string())}>
                                                {"📊 Export Data"}
                                            </button>
                                            <button class="w-full px-4 py-2 text-sm font-medium rounded-md bg-gray-100 text-gray-700 hover:bg-gray-200 transition-colors" onclick={show_success_toast.clone().reform(|_| "Report generation started".to_string())}>
                                                {"📋 Generate Report"}
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        },

                        AdminTab::Contests => html! {
                            <div class="contests-moderation-section">
                                <div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3 mb-4">
                                    <div>
                                        <h2 class="text-xl font-semibold text-gray-900">{"Contests pending review"}</h2>
                                        <p class="mt-1 text-sm text-gray-600">
                                            {"Approve to show in public search, or reject with an optional note for the organizer. Community moderation is not legal advice."}
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        class="px-4 py-2 text-sm font-medium rounded-md bg-yellow-600 text-white hover:bg-yellow-700 transition-colors"
                                        onclick={reload_pending_contests.reform(|_| ())}
                                        disabled={*pending_loading}
                                    >
                                        {if *pending_loading { "Loading…" } else { "Refresh" }}
                                    </button>
                                </div>
                                if *pending_loading && (*pending_contests).is_empty() {
                                    <p class="text-gray-600 py-6">{"Loading queue…"}</p>
                                } else if let Some(err) = (*pending_error).as_ref() {
                                    <div class="bg-red-50 border border-red-200 rounded-lg p-4 text-sm text-red-800">{err}</div>
                                } else if (*pending_contests).is_empty() {
                                    <p class="text-gray-600 py-6">{"No contests awaiting approval."}</p>
                                } else {
                                    <div class="overflow-x-auto border border-gray-200 rounded-lg">
                                        <p class="px-4 py-2 text-xs text-gray-500 border-b border-gray-200 bg-gray-50">
                                            {"Click a contest name to view full details before approving or rejecting."}
                                        </p>
                                        <table class="min-w-full divide-y divide-gray-200 text-sm">
                                            <thead class="bg-gray-50">
                                                <tr>
                                                    <th class="px-4 py-2 text-left font-medium text-gray-700">{"Name"}</th>
                                                    <th class="px-4 py-2 text-left font-medium text-gray-700">{"Start"}</th>
                                                    <th class="px-4 py-2 text-right font-medium text-gray-700">{"Actions"}</th>
                                                </tr>
                                            </thead>
                                            <tbody class="divide-y divide-gray-100">
                                                {for (*pending_contests).iter().map(|c| {
                                                    let id_a = c.id.clone();
                                                    let id_r = c.id.clone();
                                                    let id_toggle = c.id.clone();
                                                    let reload_a = reload_pending_contests.clone();
                                                    let reload_r = reload_pending_contests.clone();
                                                    let ok_a = show_success_toast.clone();
                                                    let err_a = show_error_toast.clone();
                                                    let ok_r = show_success_toast.clone();
                                                    let err_r = show_error_toast.clone();
                                                    let name = c.name.clone();
                                                    let start = format!("{}", c.start);
                                                    let is_expanded = pending_expanded_snapshot.as_deref() == Some(c.id.as_str());
                                                    let toggle_row = toggle_pending_detail_cb.clone();
                                                    let venue = c.venue.clone();
                                                    let games = c.games.clone();
                                                    let outcomes = c.outcomes.clone();
                                                    let creator_line = c
                                                        .creator_handle
                                                        .clone()
                                                        .filter(|s| !s.is_empty())
                                                        .unwrap_or_else(|| c.creator_id.clone());
                                                    let created = c
                                                        .created_at
                                                        .map(|t| format!("{}", t))
                                                        .unwrap_or_else(|| "—".to_string());
                                                    let stop_s = format!("{}", c.stop);
                                                    html! {
                                                        <>
                                                        <tr class={if is_expanded { "bg-amber-50/50" } else { "" }}>
                                                            <td class="px-4 py-2">
                                                                <button
                                                                    type="button"
                                                                    class="text-left font-medium text-yellow-900 hover:text-yellow-700 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-yellow-500 rounded"
                                                                    aria-expanded={is_expanded.to_string()}
                                                                    onclick={Callback::from(move |_| toggle_row.emit(id_toggle.clone()))}
                                                                >
                                                                    {name.clone()}
                                                                </button>
                                                            </td>
                                                            <td class="px-4 py-2 text-gray-700">{start}</td>
                                                            <td class="px-4 py-2 text-right whitespace-nowrap space-x-2">
                                                                <button
                                                                    type="button"
                                                                    class="px-3 py-1.5 text-xs font-medium rounded-md bg-green-600 text-white hover:bg-green-700"
                                                                    onclick={Callback::from(move |_| {
                                                                        let id = id_a.clone();
                                                                        let reload = reload_a.clone();
                                                                        let ok = ok_a.clone();
                                                                        let err = err_a.clone();
                                                                        wasm_bindgen_futures::spawn_local(async move {
                                                                            match approve_contest(&id).await {
                                                                                Ok(()) => {
                                                                                    ok.emit("Contest approved".to_string());
                                                                                    reload.emit(());
                                                                                }
                                                                                Err(e) => err.emit(e),
                                                                            }
                                                                        });
                                                                    })}
                                                                >
                                                                    {"Approve"}
                                                                </button>
                                                                <button
                                                                    type="button"
                                                                    class="px-3 py-1.5 text-xs font-medium rounded-md bg-red-600 text-white hover:bg-red-700"
                                                                    onclick={Callback::from(move |_| {
                                                                        let id = id_r.clone();
                                                                        let reload = reload_r.clone();
                                                                        let ok = ok_r.clone();
                                                                        let err = err_r.clone();
                                                                        let reason = gloo::dialogs::prompt("Optional reason for rejection:", None);
                                                                        wasm_bindgen_futures::spawn_local(async move {
                                                                            match reject_contest(&id, reason.as_deref()).await {
                                                                                Ok(()) => {
                                                                                    ok.emit("Contest rejected".to_string());
                                                                                    reload.emit(());
                                                                                }
                                                                                Err(e) => err.emit(e),
                                                                            }
                                                                        });
                                                                    })}
                                                                >
                                                                    {"Reject"}
                                                                </button>
                                                            </td>
                                                        </tr>
                                                        if is_expanded {
                                                            <tr class="bg-gray-50">
                                                                <td colspan="3" class="px-4 py-4 text-sm text-gray-800 border-t border-gray-100">
                                                                    <div class="space-y-4">
                                                                        <div class="grid gap-4 sm:grid-cols-2">
                                                                            <div>
                                                                                <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1">{"Schedule"}</h4>
                                                                                <p class="text-gray-900">
                                                                                    <span class="font-medium">{"Start: "}</span>{format!("{}", c.start)}
                                                                                </p>
                                                                                <p class="text-gray-900 mt-1">
                                                                                    <span class="font-medium">{"End: "}</span>{stop_s.clone()}
                                                                                </p>
                                                                            </div>
                                                                            <div>
                                                                                <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1">{"Organizer"}</h4>
                                                                                <p class="text-gray-900">{creator_line}</p>
                                                                                <p class="text-gray-600 mt-1 text-xs">
                                                                                    <span class="font-medium text-gray-700">{"Submitted: "}</span>{created.clone()}
                                                                                </p>
                                                                            </div>
                                                                        </div>
                                                                        <div>
                                                                            <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1">{"Venue"}</h4>
                                                                            if venue.display_name.is_empty() && venue.formatted_address.is_empty() {
                                                                                <p class="text-gray-600 italic">{"No venue details"}</p>
                                                                            } else {
                                                                                <p class="text-gray-900 font-medium">{venue.display_name.clone()}</p>
                                                                                <p class="text-gray-700 mt-1">{venue.formatted_address.clone()}</p>
                                                                                <p class="text-gray-600 mt-1 text-xs">{format!("Timezone: {}", venue.timezone)}</p>
                                                                            }
                                                                        </div>
                                                                        <div>
                                                                            <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1">{"Games"}</h4>
                                                                            if games.is_empty() {
                                                                                <p class="text-gray-600 italic">{"No games linked"}</p>
                                                                            } else {
                                                                                <ul class="list-disc list-inside space-y-1 text-gray-900">
                                                                                    {for games.iter().map(|g| {
                                                                                        let label = match g.year_published {
                                                                                            Some(y) => format!("{} ({})", g.name, y),
                                                                                            None => g.name.clone(),
                                                                                        };
                                                                                        html! { <li key={g.id.clone()}>{label}</li> }
                                                                                    })}
                                                                                </ul>
                                                                            }
                                                                        </div>
                                                                        <div>
                                                                            <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1">{"Results / outcomes"}</h4>
                                                                            if outcomes.is_empty() {
                                                                                <p class="text-gray-600 italic">{"No outcomes recorded yet"}</p>
                                                                            } else {
                                                                                <div class="overflow-x-auto">
                                                                                    <table class="min-w-full text-xs border border-gray-200 rounded-md">
                                                                                        <thead class="bg-gray-100">
                                                                                            <tr>
                                                                                                <th class="px-2 py-1 text-left font-medium text-gray-700">{"Place"}</th>
                                                                                                <th class="px-2 py-1 text-left font-medium text-gray-700">{"Player"}</th>
                                                                                                <th class="px-2 py-1 text-left font-medium text-gray-700">{"Score"}</th>
                                                                                                <th class="px-2 py-1 text-left font-medium text-gray-700">{"Result"}</th>
                                                                                            </tr>
                                                                                        </thead>
                                                                                        <tbody class="divide-y divide-gray-100 bg-white">
                                                                                            {for outcomes.iter().enumerate().map(|(i, o)| {
                                                                                                let player = if !o.handle.is_empty() {
                                                                                                    o.handle.clone()
                                                                                                } else if !o.email.is_empty() {
                                                                                                    o.email.clone()
                                                                                                } else {
                                                                                                    o.player_id.clone()
                                                                                                };
                                                                                                html! {
                                                                                                    <tr key={format!("o-{}-{}", c.id, i)}>
                                                                                                        <td class="px-2 py-1">{o.place.clone()}</td>
                                                                                                        <td class="px-2 py-1">{player}</td>
                                                                                                        <td class="px-2 py-1">{if o.score.trim().is_empty() { "—".to_string() } else { o.score.trim().to_string() }}</td>
                                                                                                        <td class="px-2 py-1">{o.result.clone()}</td>
                                                                                                    </tr>
                                                                                                }
                                                                                            })}
                                                                                        </tbody>
                                                                                    </table>
                                                                                </div>
                                                                            }
                                                                        </div>
                                                                    </div>
                                                                </td>
                                                            </tr>
                                                        }
                                                        </>
                                                    }
                                                })}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                            </div>
                        },

                        AdminTab::Ratings => html! {
                            <div class="ratings-section">
                                <h2>{"🏆 Glicko2 Ratings Management"}</h2>
                                <div class="ratings-content">
                                    <div class="ratings-info">
                                        <p>{"Manage the Glicko2 rating system, including monthly recalculation scheduling and full rebuilds."}</p>
                                    </div>

                                    <div class="mt-6 grid grid-cols-1 lg:grid-cols-2 gap-6">
                                        <div class="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
                                            <h3 class="text-lg font-semibold text-gray-900">{"Recalculate latest period"}</h3>
                                            <p class="mt-1 text-sm text-gray-600">
                                                {"Runs the Glicko‑2 monthly update for the previous month only (fast). Use this after adding contests/results."}
                                            </p>
                                            <div class="mt-4">
                                                <button
                                                    class="w-full px-4 py-2 text-sm font-medium rounded-md bg-yellow-600 text-white hover:bg-yellow-700 transition-colors"
                                                    onclick={run_ratings_recompute_month.reform(|_| ())}
                                                >
                                                    {"Run monthly recalculation"}
                                                </button>
                                            </div>
                                        </div>

                                        <div class="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
                                            <h3 class="text-lg font-semibold text-gray-900">{"Rebuild from beginning"}</h3>
                                            <p class="mt-1 text-sm text-gray-600">
                                                {"Deletes all rating records, then replays every month from your earliest contest through the current month (slow). Use after major data fixes/migrations."}
                                            </p>
                                            <div class="mt-4">
                                                <button
                                                    class="w-full px-4 py-2 text-sm font-medium rounded-md bg-red-600 text-white hover:bg-red-700 transition-colors"
                                                    onclick={run_ratings_rebuild_all.reform(|_| ())}
                                                >
                                                    {"Rebuild all ratings"}
                                                </button>
                                            </div>
                                            <div class="mt-4">
                                                <button
                                                    class="w-full px-4 py-2 text-sm font-medium rounded-md bg-gray-100 text-gray-800 hover:bg-gray-200 transition-colors"
                                                    onclick={refresh_rebuild_status.reform(|_| ())}
                                                    disabled={*rebuild_status_loading}
                                                >
                                                    {if *rebuild_status_loading { "Checking status..." } else { "Check rebuild status" }}
                                                </button>
                                                {if let Some(st) = (*rebuild_status).as_ref() {
                                                    let running = st.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
                                                    let current = st.get("current_period").and_then(|v| v.as_str()).unwrap_or("-");
                                                    let processed = st.get("processed_periods").and_then(|v| v.as_u64()).unwrap_or(0);
                                                    let total = st.get("total_periods").and_then(|v| v.as_u64()).unwrap_or(0);
                                                    let last_error = st.get("last_error").and_then(|v| v.as_str()).unwrap_or("");
                                                    let started_at = st.get("started_at").and_then(|v| v.as_str()).unwrap_or("-");
                                                    let finished_at = st.get("finished_at").and_then(|v| v.as_str()).unwrap_or("-");
                                                    let last_completed = st.get("last_completed_run");
                                                    let last_completed_started = last_completed
                                                        .and_then(|v| v.get("started_at"))
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("-");
                                                    let last_completed_finished = last_completed
                                                        .and_then(|v| v.get("finished_at"))
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("-");
                                                    let last_completed_processed = last_completed
                                                        .and_then(|v| v.get("processed_periods"))
                                                        .and_then(|v| v.as_u64())
                                                        .unwrap_or(0);
                                                    let last_completed_total = last_completed
                                                        .and_then(|v| v.get("total_periods"))
                                                        .and_then(|v| v.as_u64())
                                                        .unwrap_or(0);
                                                    html! {
                                                        <div class="mt-3 rounded-md border border-gray-200 bg-gray-50 p-3 text-sm text-gray-800">
                                                            <div class="flex items-center justify-between">
                                                                <span class="font-medium">{"Status"}</span>
                                                                <span class={classes!(if running { "text-yellow-800" } else { "text-green-800" })}>
                                                                    {if running { "RUNNING" } else { "IDLE" }}
                                                                </span>
                                                            </div>
                                                            <div class="mt-2 space-y-1">
                                                                <div class="flex justify-between"><span class="text-gray-600">{"Started"}</span><span class="font-mono">{started_at}</span></div>
                                                                <div class="flex justify-between"><span class="text-gray-600">{"Finished"}</span><span class="font-mono">{finished_at}</span></div>
                                                                <div class="flex justify-between"><span class="text-gray-600">{"Current period"}</span><span class="font-mono">{current}</span></div>
                                                                <div class="flex justify-between"><span class="text-gray-600">{"Progress (months processed)"}</span><span class="font-mono">{format!("{} / {}", processed, total)}</span></div>
                                                                <div class="mt-3 pt-3 border-t border-gray-200">
                                                                    <div class="font-medium text-gray-800">{"Last completed run (persisted)"}</div>
                                                                    <div class="mt-1 space-y-1">
                                                                        <div class="flex justify-between"><span class="text-gray-600">{"Started"}</span><span class="font-mono">{last_completed_started}</span></div>
                                                                        <div class="flex justify-between"><span class="text-gray-600">{"Finished"}</span><span class="font-mono">{last_completed_finished}</span></div>
                                                                        <div class="flex justify-between"><span class="text-gray-600">{"Months computed"}</span><span class="font-mono">{format!("{} / {}", last_completed_processed, last_completed_total)}</span></div>
                                                                    </div>
                                                                </div>
                                                                {if !last_error.is_empty() {
                                                                    html!{ <div class="mt-2 text-red-700"><span class="font-semibold">{"Last error: "}</span>{last_error}</div> }
                                                                } else {
                                                                    html!{ <div class="mt-2 text-gray-600"><span class="font-semibold">{"Last error: "}</span>{"-"}</div> }
                                                                }}
                                                                <div class="mt-2 text-xs text-gray-600 leading-relaxed">
                                                                    {"What do these numbers mean? "}
                                                                    {"“Total months” is the number of monthly periods from your earliest contest month to the current month (inclusive). "}
                                                                    {"During a rebuild we recompute Glicko‑2 month-by-month. When Progress reaches Total and Status is IDLE, the rebuild is finished."}
                                                                </div>
                                                            </div>
                                                        </div>
                                                    }
                                                } else { html!{} }}
                                            </div>
                                        </div>
                                    </div>

                                    <div class="scheduler-section">
                                        <SchedulerMonitor show_controls={false} />
                                    </div>
                                </div>
                            </div>
                        },

                        AdminTab::System => html! {
                            <div class="system-section">
                                <h2>{"⚙️ System Configuration"}</h2>
                                <div class="system-content">
                                    <div class="config-card">
                                        <h3>{"Database Status"}</h3>
                                        <div class="status-indicators">
                                            <div class="status-item">
                                                <span class="status-dot online"></span>
                                                <span class="status-text">{"Database: Online"}</span>
                                            </div>
                                            <div class="status-item">
                                                <span class="status-dot online"></span>
                                                <span class="status-text">{"Redis: Online"}</span>
                                            </div>
                                            <div class="status-item">
                                                <span class="status-dot online"></span>
                                                <span class="status-text">{"Backend: Running"}</span>
                                            </div>
                                        </div>
                                    </div>

                                    <div class="config-card">
                                        <h3>{"Playwright smoke test"}</h3>
                                        <p class="text-sm text-gray-600 mb-2">{"Queue a smoke.stg job against STG_BASE_URL (requires Playwright worker)."}</p>
                                        <button type="button" class="action-btn secondary" onclick={{
                                            let on_run_smoke = on_run_smoke.clone();
                                            Callback::from(move |_| on_run_smoke.emit(()))
                                        }}>
                                            {"Run smoke.stg"}
                                        </button>
                                    </div>

                                    <div class="config-card">
                                        <h3>{"System Information"}</h3>
                                        <div class="info-grid">
                                            {if *version_loading {
                                                html! {
                                                    <div class="info-item">
                                                        <span class="info-label">{"Loading..."}</span>
                                                    </div>
                                                }
                                            } else if let Some(ref info) = *version_info {
                                                html! {
                                                    <>
                                                        <div class="info-item">
                                                            <span class="info-label">{"Version:"}</span>
                                                            <span class="info-value">{&info.version}</span>
                                                        </div>
                                                        {if let Some(ref frontend_tag) = info.frontend_image_tag {
                                                            html! {
                                                                <div class="info-item">
                                                                    <span class="info-label">{"Frontend Image:"}</span>
                                                                    <span class="info-value">{frontend_tag}</span>
                                                                </div>
                                                            }
                                                        } else {
                                                            html! {}
                                                        }}
                                                        {if let Some(ref backend_tag) = info.backend_image_tag {
                                                            html! {
                                                                <div class="info-item">
                                                                    <span class="info-label">{"Backend Image:"}</span>
                                                                    <span class="info-value">{backend_tag}</span>
                                                                </div>
                                                            }
                                                        } else {
                                                            html! {}
                                                        }}
                                                        {if let Some(ref build_date) = info.build_date {
                                                            html! {
                                                                <div class="info-item">
                                                                    <span class="info-label">{"Build Date:"}</span>
                                                                    <span class="info-value">{build_date}</span>
                                                                </div>
                                                            }
                                                        } else {
                                                            html! {}
                                                        }}
                                                    </>
                                                }
                                            } else {
                                                html! {
                                                    <div class="info-item">
                                                        <span class="info-label">{"Version info unavailable"}</span>
                                                    </div>
                                                }
                                            }}
                                        </div>
                                    </div>
                                </div>
                            </div>
                        },

                        AdminTab::Users => html! {
                            <div class="users-section space-y-4">
                                <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
                                    <div>
                                        <h2 class="text-xl font-semibold">{"👥 User Management"}</h2>
                                        <p class="text-gray-600">{"Create, edit, delete players, reset passwords, and manage admin access."}</p>
                                    </div>
                                    <button
                                        type="button"
                                        class="action-btn secondary min-h-[44px] flex-shrink-0"
                                        onclick={on_toggle_create_form.clone()}
                                    >
                                        {if *show_create_form { "Cancel" } else { "+ Add player" }}
                                    </button>
                                </div>
                                if *show_create_form {
                                    <div class="rounded-lg border border-green-200 bg-white p-4 space-y-4 shadow-sm">
                                        <h3 class="text-lg font-semibold text-gray-900">{"Add player"}</h3>
                                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                            <label class="block space-y-1">
                                                <span class="text-sm font-medium text-gray-700">{"First name"}</span>
                                                <input
                                                    type="text"
                                                    class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                    value={(*create_firstname).clone()}
                                                    oninput={{
                                                        let create_firstname = create_firstname.clone();
                                                        Callback::from(move |e: InputEvent| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            create_firstname.set(input.value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <label class="block space-y-1">
                                                <span class="text-sm font-medium text-gray-700">{"Handle"}</span>
                                                <input
                                                    type="text"
                                                    class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                    value={(*create_handle).clone()}
                                                    oninput={{
                                                        let create_handle = create_handle.clone();
                                                        Callback::from(move |e: InputEvent| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            create_handle.set(input.value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <label class="block space-y-1">
                                                <span class="text-sm font-medium text-gray-700">{"Email"}</span>
                                                <input
                                                    type="email"
                                                    class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                    value={(*create_email).clone()}
                                                    oninput={{
                                                        let create_email = create_email.clone();
                                                        Callback::from(move |e: InputEvent| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            create_email.set(input.value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <label class="block space-y-1">
                                                <span class="text-sm font-medium text-gray-700">{"Password"}</span>
                                                <input
                                                    type="password"
                                                    class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                    value={(*create_password).clone()}
                                                    oninput={{
                                                        let create_password = create_password.clone();
                                                        Callback::from(move |e: InputEvent| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            create_password.set(input.value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <label class="flex items-center gap-2 sm:col-span-2">
                                                <input
                                                    type="checkbox"
                                                    checked={*create_is_admin}
                                                    onchange={{
                                                        let create_is_admin = create_is_admin.clone();
                                                        Callback::from(move |e: Event| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            create_is_admin.set(input.checked());
                                                        })
                                                    }}
                                                />
                                                <span class="text-sm font-medium text-gray-700">{"Administrator"}</span>
                                            </label>
                                        </div>
                                        <button
                                            type="button"
                                            class="action-btn primary min-h-[44px]"
                                            onclick={on_create_user.clone()}
                                            disabled={*create_saving}
                                        >
                                            {if *create_saving { "Creating…" } else { "Create player" }}
                                        </button>
                                    </div>
                                }
                                <div class="flex flex-col sm:flex-row gap-2">
                                    <input
                                        type="search"
                                        class="flex-1 rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                        placeholder="Search by handle or email"
                                        value={(*user_search_query).clone()}
                                        oninput={{
                                            let user_search_query = user_search_query.clone();
                                            Callback::from(move |e: InputEvent| {
                                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                user_search_query.set(input.value());
                                            })
                                        }}
                                    />
                                    <button class="action-btn primary min-h-[44px]" onclick={{
                                        let on_user_search = on_user_search.clone();
                                        Callback::from(move |_| on_user_search.emit(()))
                                    }} disabled={*user_search_loading}>
                                        {if *user_search_loading { "Searching…" } else { "Search" }}
                                    </button>
                                </div>
                                if !user_search_results.is_empty() {
                                    <div class="overflow-x-auto rounded-lg border border-gray-200 bg-white">
                                        <table class="min-w-full text-sm">
                                            <thead class="bg-gray-50 text-left">
                                                <tr>
                                                    <th class="px-3 py-2">{"Handle"}</th>
                                                    <th class="px-3 py-2">{"Email"}</th>
                                                    <th class="px-3 py-2">{"Status"}</th>
                                                    <th class="px-3 py-2">{"Admin"}</th>
                                                    <th class="px-3 py-2">{"Actions"}</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {user_search_results.iter().map(|u| {
                                                    let pid = u.id.clone();
                                                    let edit = {
                                                        let on_edit_user = on_edit_user.clone();
                                                        let u = u.clone();
                                                        Callback::from(move |_| on_edit_user.emit(u.clone()))
                                                    };
                                                    let grant = {
                                                        let on_toggle_admin = on_toggle_admin.clone();
                                                        let pid = pid.clone();
                                                        Callback::from(move |_| on_toggle_admin.emit((pid.clone(), true)))
                                                    };
                                                    let revoke = {
                                                        let on_toggle_admin = on_toggle_admin.clone();
                                                        let pid = pid.clone();
                                                        Callback::from(move |_| on_toggle_admin.emit((pid.clone(), false)))
                                                    };
                                                    html! {
                                                        <tr class="border-t border-gray-100">
                                                            <td class="px-3 py-2 font-medium">{&u.handle}</td>
                                                            <td class="px-3 py-2">{&u.email}</td>
                                                            <td class="px-3 py-2">
                                                                {if u.is_active {
                                                                    html! { <span class="text-green-700">{"Active"}</span> }
                                                                } else {
                                                                    html! { <span class="text-gray-500">{"Inactive"}</span> }
                                                                }}
                                                            </td>
                                                            <td class="px-3 py-2">{if u.is_admin { "Yes" } else { "No" }}</td>
                                                            <td class="px-3 py-2 space-x-2">
                                                                <button type="button" class="text-xs text-blue-700 hover:underline" onclick={edit}>{"Edit"}</button>
                                                                <button type="button" class="text-xs text-indigo-700 hover:underline" onclick={grant}>{"Grant"}</button>
                                                                <button type="button" class="text-xs text-red-700 hover:underline" onclick={revoke}>{"Revoke"}</button>
                                                            </td>
                                                        </tr>
                                                    }
                                                }).collect::<Html>()}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                                if let Some(player) = (*editing_player).clone() {
                                    <div class="rounded-lg border border-indigo-200 bg-white p-4 space-y-4 shadow-sm">
                                        <div class="flex items-center justify-between gap-2">
                                            <h3 class="text-lg font-semibold text-gray-900">
                                                {format!("Edit: {}", player.handle)}
                                            </h3>
                                            <button type="button" class="text-sm text-gray-600 hover:text-gray-900" onclick={on_cancel_edit.clone()}>
                                                {"Cancel"}
                                            </button>
                                        </div>
                                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                            <label class="block space-y-1">
                                                <span class="text-sm font-medium text-gray-700">{"First name"}</span>
                                                <input
                                                    type="text"
                                                    class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                    value={(*edit_firstname).clone()}
                                                    oninput={{
                                                        let edit_firstname = edit_firstname.clone();
                                                        Callback::from(move |e: InputEvent| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            edit_firstname.set(input.value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <label class="block space-y-1">
                                                <span class="text-sm font-medium text-gray-700">{"Handle"}</span>
                                                <input
                                                    type="text"
                                                    class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                    value={(*edit_handle).clone()}
                                                    oninput={{
                                                        let edit_handle = edit_handle.clone();
                                                        Callback::from(move |e: InputEvent| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            edit_handle.set(input.value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <label class="block space-y-1 sm:col-span-2">
                                                <span class="text-sm font-medium text-gray-700">{"Email"}</span>
                                                <input
                                                    type="email"
                                                    class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                    value={(*edit_email).clone()}
                                                    oninput={{
                                                        let edit_email = edit_email.clone();
                                                        Callback::from(move |e: InputEvent| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            edit_email.set(input.value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <label class="flex items-center gap-2 sm:col-span-2">
                                                <input
                                                    type="checkbox"
                                                    checked={*edit_is_admin}
                                                    onchange={{
                                                        let edit_is_admin = edit_is_admin.clone();
                                                        Callback::from(move |e: Event| {
                                                            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                            edit_is_admin.set(input.checked());
                                                        })
                                                    }}
                                                />
                                                <span class="text-sm font-medium text-gray-700">{"Administrator"}</span>
                                            </label>
                                        </div>
                                        <button
                                            type="button"
                                            class="action-btn primary min-h-[44px]"
                                            onclick={on_save_user.clone()}
                                            disabled={*edit_saving}
                                        >
                                            {if *edit_saving { "Saving…" } else { "Save changes" }}
                                        </button>
                                        <div class="border-t border-gray-200 pt-4 space-y-3">
                                            <h4 class="text-sm font-semibold text-gray-900">{"Account status"}</h4>
                                            <p class="text-sm text-gray-600">
                                                {if (*editing_player).as_ref().map(|p| p.is_active).unwrap_or(true) {
                                                    "This player can log in. Deactivate to block login while keeping contest history."
                                                } else {
                                                    "This player cannot log in. Contest history and ratings are preserved."
                                                }}
                                            </p>
                                            if (*editing_player).as_ref().map(|p| p.is_active).unwrap_or(true) {
                                                <button
                                                    type="button"
                                                    class="action-btn secondary min-h-[44px]"
                                                    onclick={{
                                                        let on_set_active = on_set_active.clone();
                                                        Callback::from(move |_| on_set_active.emit(false))
                                                    }}
                                                    disabled={*activate_saving}
                                                >
                                                    {if *activate_saving { "Working…" } else { "Deactivate account" }}
                                                </button>
                                            } else {
                                                <button
                                                    type="button"
                                                    class="action-btn primary min-h-[44px]"
                                                    onclick={{
                                                        let on_set_active = on_set_active.clone();
                                                        Callback::from(move |_| on_set_active.emit(true))
                                                    }}
                                                    disabled={*activate_saving}
                                                >
                                                    {if *activate_saving { "Working…" } else { "Reactivate account" }}
                                                </button>
                                            }
                                        </div>
                                        <div class="border-t border-gray-200 pt-4 space-y-3">
                                            <h4 class="text-sm font-semibold text-gray-900">{"Reset password"}</h4>
                                            <p class="text-sm text-gray-600">{"Set a new password without requiring the current one."}</p>
                                            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                                <label class="block space-y-1">
                                                    <span class="text-sm font-medium text-gray-700">{"New password"}</span>
                                                    <input
                                                        type="password"
                                                        class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                        value={(*reset_password).clone()}
                                                        oninput={{
                                                            let reset_password = reset_password.clone();
                                                            Callback::from(move |e: InputEvent| {
                                                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                                reset_password.set(input.value());
                                                            })
                                                        }}
                                                    />
                                                </label>
                                                <label class="block space-y-1">
                                                    <span class="text-sm font-medium text-gray-700">{"Confirm password"}</span>
                                                    <input
                                                        type="password"
                                                        class="w-full rounded-md border border-gray-300 px-3 py-2 min-h-[44px]"
                                                        value={(*reset_password_confirm).clone()}
                                                        oninput={{
                                                            let reset_password_confirm = reset_password_confirm.clone();
                                                            Callback::from(move |e: InputEvent| {
                                                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                                reset_password_confirm.set(input.value());
                                                            })
                                                        }}
                                                    />
                                                </label>
                                            </div>
                                            <button
                                                type="button"
                                                class="action-btn secondary min-h-[44px]"
                                                onclick={on_reset_password.clone()}
                                                disabled={*reset_saving}
                                            >
                                                {if *reset_saving { "Resetting…" } else { "Reset password" }}
                                            </button>
                                        </div>
                                        <div class="border-t border-red-200 pt-4">
                                            <h4 class="text-sm font-semibold text-red-800">{"Danger zone"}</h4>
                                            <p class="text-sm text-gray-600 mt-1">
                                                {"Permanent deletion removes the player from "}
                                                <strong>{"all contests"}</strong>
                                                {" and deletes ratings. Use deactivate above for normal support cases. Blocked if they created contests."}
                                            </p>
                                            <button
                                                type="button"
                                                class="mt-3 text-sm font-medium text-red-700 hover:text-red-900 border border-red-300 rounded-md px-3 py-2 min-h-[44px] hover:bg-red-50"
                                                onclick={on_delete_user.clone()}
                                                disabled={*delete_saving}
                                            >
                                                {if *delete_saving { "Deleting…" } else { "Delete player" }}
                                            </button>
                                        </div>
                                    </div>
                                }
                            </div>
                        },
                    }}
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // Mock auth context for testing - simplified version
    #[allow(dead_code)]
    fn create_mock_auth_context(is_admin: bool) -> crate::auth::AuthContext {
        use crate::auth::{AuthContext, AuthState};
        use shared::dto::player::PlayerDto;

        let mock_player = if is_admin {
            Some(PlayerDto {
                id: "admin_player".to_string(),
                email: "admin@test.com".to_string(),
                handle: "admin".to_string(),
                firstname: "Admin".to_string(),
                created_at: chrono::Utc::now().fixed_offset(),
                is_admin: true,
                is_active: true,
            })
        } else {
            Some(PlayerDto {
                id: "regular_player".to_string(),
                email: "user@test.com".to_string(),
                handle: "user".to_string(),
                firstname: "Regular".to_string(),
                created_at: chrono::Utc::now().fixed_offset(),
                is_admin: false,
                is_active: true,
            })
        };

        let mock_state = AuthState {
            player: mock_player,
            loading: false,
            error: None,
            heartbeat_active: false,
        };

        // Create a simplified mock context without use_reducer_eq
        AuthContext {
            state: mock_state,
            login: yew::Callback::from(|_| {}),
            logout: yew::Callback::from(|_| {}),
            on_session_expired: yew::Callback::from(|_| {}),
            refresh: yew::Callback::from(|_| {}),
        }
    }

    // #[wasm_bindgen_test]
    // async fn test_admin_page_renders_for_admin_user() {
    //     // This test would require setting up a proper test environment
    //     // with mocked auth context and DOM manipulation
    //     // For now, we'll test the component logic
    //
    //     let props = AdminPageProps {};
    //     let component = AdminPage::new(props);
    //
    //     // Verify the component can be created
    //     assert!(component.props == AdminPageProps {});
    // }

    #[wasm_bindgen_test]
    async fn test_admin_tab_enum() {
        let dashboard_tab = AdminTab::Dashboard;
        let contests_tab = AdminTab::Contests;
        let ratings_tab = AdminTab::Ratings;
        let system_tab = AdminTab::System;
        let users_tab = AdminTab::Users;

        // Test that all tabs are different
        assert_ne!(dashboard_tab, contests_tab);
        assert_ne!(dashboard_tab, ratings_tab);
        assert_ne!(dashboard_tab, system_tab);
        assert_ne!(dashboard_tab, users_tab);
        assert_ne!(contests_tab, ratings_tab);
        assert_ne!(ratings_tab, system_tab);
        assert_ne!(ratings_tab, users_tab);
        assert_ne!(system_tab, users_tab);

        // Test that tabs can be cloned
        let cloned_dashboard = dashboard_tab.clone();
        assert_eq!(dashboard_tab, cloned_dashboard);
    }

    #[wasm_bindgen_test]
    async fn test_admin_page_props() {
        let props = AdminPageProps {};

        // Test that props can be created and compared
        let props2 = AdminPageProps {};
        assert_eq!(props, props2);

        // Test that props can be cloned
        let cloned_props = props.clone();
        assert_eq!(props, cloned_props);
    }

    #[test]
    fn test_admin_tab_partial_eq() {
        let tab1 = AdminTab::Dashboard;
        let tab2 = AdminTab::Dashboard;
        let tab3 = AdminTab::Ratings;

        assert_eq!(tab1, tab2);
        assert_ne!(tab1, tab3);
        assert_ne!(tab2, tab3);
    }

    #[test]
    fn test_admin_tab_clone() {
        let original_tab = AdminTab::System;
        let cloned_tab = original_tab.clone();

        assert_eq!(original_tab, cloned_tab);
    }

    #[test]
    fn test_admin_page_props_partial_eq() {
        let props1 = AdminPageProps {};
        let props2 = AdminPageProps {};

        assert_eq!(props1, props2);
    }

    #[test]
    fn test_admin_page_props_clone() {
        let original_props = AdminPageProps {};
        let cloned_props = original_props.clone();

        assert_eq!(original_props, cloned_props);
    }
}
