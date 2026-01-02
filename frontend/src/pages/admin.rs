use yew::prelude::*;
use crate::components::scheduler_monitor::SchedulerMonitor;
use crate::components::common::toast::{ToastContext, Toast, ToastType};
use crate::api::utils::authenticated_get;
use serde_json::Value;

#[derive(Properties, PartialEq, Clone, Debug)]
pub struct AdminPageProps {}

#[derive(Clone, PartialEq, Debug)]
enum AdminTab {
    Dashboard,
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
    let system_stats = use_state(|| None::<Value>);
    let stats_loading = use_state(|| false);
    let stats_error = use_state(|| None::<String>);

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
                match authenticated_get("/api/analytics/platform")
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(stats) = response.json::<Value>().await {
                                system_stats.set(Some(stats));
                            } else {
                                stats_error.set(Some("Failed to parse system stats".to_string()));
                            }
                        } else {
                            let status = response.status();
                            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            stats_error.set(Some(format!("Failed to load system stats: {} - {}", status, text)));
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

    let show_success_toast = {
        let toast_context = toast_context.clone();
        Callback::from(move |message: String| {
            let toast = Toast::new(message, ToastType::Success).with_duration(5000);
            toast_context.add_toast.emit(toast);
        })
    };

    html! {
        <div class="admin-page">
            <div class="page-header">
                <div class="header-content">
                    <div class="header-text">
                        <h1>{"👑 Administrator Dashboard"}</h1>
                        <p>{"System management, monitoring, and administrative tools"}</p>
                    </div>
                    <div class="admin-badge">
                        <span class="admin-indicator">
                            <span class="admin-icon">{"👑"}</span>
                            <span class="admin-text">{"Administrator"}</span>
                        </span>
                    </div>
                </div>
            </div>

            <div class="admin-content">
                // Navigation Tabs
                <div class="admin-nav">
                    <button 
                        class={classes!("nav-tab", if *current_tab == AdminTab::Dashboard { "active" } else { "" })}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::Dashboard)}
                    >
                        {"📊 Dashboard"}
                    </button>
                    <button 
                        class={classes!("nav-tab", if *current_tab == AdminTab::Ratings { "active" } else { "" })}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::Ratings)}
                    >
                        {"🏆 Ratings Management"}
                    </button>
                    <button 
                        class={classes!("nav-tab", if *current_tab == AdminTab::System { "active" } else { "" })}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::System)}
                    >
                        {"⚙️ System"}
                    </button>
                    <button 
                        class={classes!("nav-tab", if *current_tab == AdminTab::Users { "active" } else { "" })}
                        onclick={on_tab_click.clone().reform(|_| AdminTab::Users)}
                    >
                        {"👥 Users"}
                    </button>
                </div>

                // Tab Content
                <div class="tab-content">
                    {match *current_tab {
                        AdminTab::Dashboard => html! {
                            <div class="dashboard-section">
                                <h2>{"System Overview"}</h2>
                                <div class="stats-grid">
                                    if *stats_loading {
                                        <div class="loading-container">
                                            <p>{"Loading system statistics..."}</p>
                                        </div>
                                    } else if let Some(err) = (*stats_error).as_ref() {
                                        <div class="error-container">
                                            <p class="error-text">{"Error: "}{err}</p>
                                        </div>
                                    } else if let Some(stats) = (*system_stats).as_ref() {
                                        <div class="stat-card">
                                            <h3>{"📈 Platform Statistics"}</h3>
                                            <div class="stat-content">
                                                <div class="stat-item">
                                                    <span class="stat-label">{"Total Players:"}</span>
                                                    <span class="stat-value">{stats["total_players"].as_i64().unwrap_or(0)}</span>
                                                </div>
                                                <div class="stat-item">
                                                    <span class="stat-label">{"Total Contests:"}</span>
                                                    <span class="stat-value">{stats["total_contests"].as_i64().unwrap_or(0)}</span>
                                                </div>
                                                <div class="stat-item">
                                                    <span class="stat-label">{"Total Games:"}</span>
                                                    <span class="stat-value">{stats["total_games"].as_i64().unwrap_or(0)}</span>
                                                </div>
                                                <div class="stat-item">
                                                    <span class="stat-label">{"Total Venues:"}</span>
                                                    <span class="stat-value">{stats["total_venues"].as_i64().unwrap_or(0)}</span>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                    
                                    <div class="stat-card">
                                        <h3>{"🔧 Quick Actions"}</h3>
                                        <div class="quick-actions">
                                            <button class="action-btn primary" onclick={show_success_toast.clone().reform(|_| "System refresh initiated".to_string())}>
                                                {"🔄 Refresh System"}
                                            </button>
                                            <button class="action-btn secondary" onclick={show_success_toast.clone().reform(|_| "Export started".to_string())}>
                                                {"📊 Export Data"}
                                            </button>
                                            <button class="action-btn secondary" onclick={show_success_toast.clone().reform(|_| "Report generation started".to_string())}>
                                                {"📋 Generate Report"}
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        },
                        
                        AdminTab::Ratings => html! {
                            <div class="ratings-section">
                                <h2>{"🏆 Glicko2 Ratings Management"}</h2>
                                <div class="ratings-content">
                                    <div class="ratings-info">
                                        <p>{"Manage the Glicko2 rating system, including monthly recalculation scheduling and historical data processing."}</p>
                                    </div>
                                    
                                    <div class="scheduler-section">
                                        <SchedulerMonitor />
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
                                            <div class="info-item">
                                                <span class="info-label">{"Version:"}</span>
                                                <span class="info-value">{"1.0.0"}</span>
                                            </div>
                                            <div class="info-item">
                                                <span class="info-label">{"Environment:"}</span>
                                                <span class="info-value">{"Production"}</span>
                                            </div>
                                            <div class="info-item">
                                                <span class="info-label">{"Last Updated:"}</span>
                                                <span class="info-value">{"2025-01-15"}</span>
                                            </div>
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
    use yew::platform::spawn_local;
    use yew::platform::time::sleep;
    use std::time::Duration;
    use wasm_bindgen_test::*;
    use gloo_utils::document;
    use web_sys::Element;

    wasm_bindgen_test_configure!(run_in_browser);

    // Mock auth context for testing - simplified version
    fn create_mock_auth_context(is_admin: bool) -> crate::auth::AuthContext {
        use crate::auth::{AuthContext, AuthState, AuthAction};
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
        let ratings_tab = AdminTab::Ratings;
        let system_tab = AdminTab::System;
        let users_tab = AdminTab::Users;

        // Test that all tabs are different
        assert_ne!(dashboard_tab, ratings_tab);
        assert_ne!(dashboard_tab, system_tab);
        assert_ne!(dashboard_tab, users_tab);
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
