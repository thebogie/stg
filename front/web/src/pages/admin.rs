use crate::api::api_url;
use crate::api::contests::{approve_contest, list_pending_contests, reject_contest};
use crate::api::utils::authenticated_get;
use crate::api::utils::authenticated_post;
use crate::api::version::{get_version_info, VersionInfo};
use crate::components::common::toast::{Toast, ToastContext, ToastType};
use crate::components::scheduler_monitor::SchedulerMonitor;
use gloo_timers::callback::Interval;
use shared::dto::analytics::PlatformStatsDto;
use shared::dto::contest::ContestDto;
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

    let run_ratings_rebuild_all = {
        let show_success_toast = show_success_toast.clone();
        let show_error_toast = show_error_toast.clone();
        Callback::from(move |_: ()| {
            if !gloo::dialogs::confirm(
                "Rebuild ALL ratings from the beginning? This can take a while.",
            ) {
                return;
            }
            let show_success_toast = show_success_toast.clone();
            let show_error_toast = show_error_toast.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match authenticated_post(&api_url("/api/ratings/recalculate/historical"))
                    .send()
                    .await
                {
                    Ok(resp) if resp.ok() => {
                        show_success_toast.emit("Ratings rebuild started".to_string());
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

    // Ratings rebuild status (admin only)
    let rebuild_status = use_state(|| None::<serde_json::Value>);
    let rebuild_status_loading = use_state(|| false);
    let rebuild_status_interval = use_mut_ref(|| None::<Interval>);

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

    // Auto-refresh rebuild status while running.
    {
        let rebuild_status = rebuild_status.clone();
        let rebuild_status_loading = rebuild_status_loading.clone();
        let rebuild_status_interval = rebuild_status_interval.clone();

        use_effect_with(rebuild_status.clone(), move |st| {
            let running = st
                .as_ref()
                .and_then(|v| v.get("running").and_then(|x| x.as_bool()))
                .unwrap_or(false);

            // Stop existing interval if any.
            rebuild_status_interval.borrow_mut().take();

            if running {
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
                                                    let reload_a = reload_pending_contests.clone();
                                                    let reload_r = reload_pending_contests.clone();
                                                    let ok_a = show_success_toast.clone();
                                                    let err_a = show_error_toast.clone();
                                                    let ok_r = show_success_toast.clone();
                                                    let err_r = show_error_toast.clone();
                                                    let name = c.name.clone();
                                                    let start = format!("{}", c.start);
                                                    html! {
                                                        <tr>
                                                            <td class="px-4 py-2 font-medium text-gray-900">{name}</td>
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
                            <div class="users-section">
                                <h2>{"👥 User Management"}</h2>
                                <div class="users-content">
                                    <div class="users-info">
                                        <p>{"Manage user accounts, permissions, and administrative access."}</p>
                                    </div>

                                    <div class="users-actions">
                                        <button class="action-btn primary" onclick={show_success_toast.clone().reform(|_| "User management features coming soon!".to_string())}>
                                            {"👤 Manage Users"}
                                        </button>
                                        <button class="action-btn secondary" onclick={show_success_toast.clone().reform(|_| "Permission management features coming soon!".to_string())}>
                                            {"🔐 Manage Permissions"}
                                        </button>
                                    </div>

                                    <div class="coming-soon">
                                        <h3>{"🚧 Coming Soon"}</h3>
                                        <p>{"Advanced user management features are under development and will include:"}</p>
                                        <ul>
                                            <li>{"User role assignment and management"}</li>
                                            <li>{"Permission level configuration"}</li>
                                            <li>{"User activity monitoring"}</li>
                                            <li>{"Bulk user operations"}</li>
                                        </ul>
                                    </div>
                                </div>
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
            })
        } else {
            Some(PlayerDto {
                id: "regular_player".to_string(),
                email: "user@test.com".to_string(),
                handle: "user".to_string(),
                firstname: "Regular".to_string(),
                created_at: chrono::Utc::now().fixed_offset(),
                is_admin: false,
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
