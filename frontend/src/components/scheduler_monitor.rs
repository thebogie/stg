use yew::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use web_sys::console;
use crate::api::utils::{authenticated_get, authenticated_post};

#[derive(Properties, PartialEq)]
pub struct SchedulerMonitorProps {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchedulerStatus {
    is_running: bool,
    last_run: Option<DateTime<Utc>>,
    next_scheduled_run: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TriggerResponse {
    status: String,
    period: Option<String>,
}

#[function_component(SchedulerMonitor)]
pub fn scheduler_monitor(_props: &SchedulerMonitorProps) -> Html {
    let status = use_state(|| None::<SchedulerStatus>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let trigger_loading = use_state(|| false);
    let trigger_message = use_state(|| None::<String>);
    let period_input = use_state(|| String::new());
    let historical_loading = use_state(|| false);
    let historical_message = use_state(|| None::<String>);

    // Load scheduler status on component mount
    {
        let status = status.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            error.set(None);
            
            console::log_1(&"SchedulerMonitor: Starting to load scheduler status".into());
            
            wasm_bindgen_futures::spawn_local(async move {
                console::log_1(&"SchedulerMonitor: Making authenticated request to /api/ratings/scheduler/status".into());
                let request = authenticated_get("/api/ratings/scheduler/status");
                match request.send().await {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(scheduler_status) = response.json::<SchedulerStatus>().await {
                                console::log_1(&format!("Scheduler status received: {:?}", scheduler_status).into());
                                status.set(Some(scheduler_status));
                            } else {
                                error.set(Some("Failed to parse scheduler status".to_string()));
                            }
                        } else {
                            let status_code = response.status();
                            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            console::error_1(&format!("Scheduler status request failed: {} - {}", status_code, text).into());
                            error.set(Some(format!("Request failed: {} - {}", status_code, text)));
                        }
                    }
                    Err(e) => {
                        console::error_1(&format!("Failed to fetch scheduler status: {}", e).into());
                        error.set(Some(format!("Failed to fetch scheduler status: {}", e)));
                    }
                }
                loading.set(false);
            });
        });
    }

    let on_trigger_click = {
        let trigger_loading = trigger_loading.clone();
        let trigger_message = trigger_message.clone();
        let error = error.clone();
        let period_input = period_input.clone();
        
        Callback::from(move |_| {
            let period_clone = (*period_input).clone();
            trigger_loading.set(true);
            trigger_message.set(None);
            error.set(None);
            
            let trigger_loading = trigger_loading.clone();
            let trigger_message = trigger_message.clone();
            let error = error.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                let url = if !period_clone.is_empty() {
                    format!("/api/ratings/scheduler/trigger?period={}", period_clone)
                } else {
                    "/api/ratings/scheduler/trigger".to_string()
                };
                
                let request = authenticated_post(&url);
                match request.send().await {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(trigger_resp) = response.json::<TriggerResponse>().await {
                                let message = if let Some(p) = trigger_resp.period {
                                    format!("Triggered recalculation for period: {}", p)
                                } else {
                                    "Triggered recalculation for previous month".to_string()
                                };
                                trigger_message.set(Some(message));
                            } else {
                                error.set(Some("Failed to parse trigger response".to_string()));
                            }
                        } else {
                            let status_code = response.status();
                            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            error.set(Some(format!("Trigger request failed: {} - {}", status_code, text)));
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to trigger recalculation: {}", e)));
                    }
                }
                trigger_loading.set(false);
            });
        })
    };

    let on_historical_recalc_click = {
        let historical_loading = historical_loading.clone();
        let historical_message = historical_message.clone();
        let error = error.clone();
        
        Callback::from(move |_| {
            historical_loading.set(true);
            historical_message.set(None);
            error.set(None);
            
            let historical_loading = historical_loading.clone();
            let historical_message = historical_message.clone();
            let error = error.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                let request = authenticated_post("/api/ratings/recalculate/historical");
                match request.send().await {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(historical_resp) = response.json::<serde_json::Value>().await {
                                let message = if let Some(msg) = historical_resp.get("message").and_then(|v| v.as_str()) {
                                    msg.to_string()
                                } else {
                                    "Historical recalculation completed successfully".to_string()
                                };
                                historical_message.set(Some(message));
                            } else {
                                error.set(Some("Failed to parse historical recalculation response".to_string()));
                            }
                        } else {
                            let status_code = response.status();
                            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            error.set(Some(format!("Historical recalculation failed: {} - {}", status_code, text)));
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to start historical recalculation: {}", e)));
                    }
                }
                historical_loading.set(false);
            });
        })
    };

    let on_period_input_change = {
        let period_input = period_input.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            period_input.set(input.value());
        })
    };

    let refresh_status = {
        let status = status.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        Callback::from(move |_| {
            loading.set(true);
            error.set(None);
            
            let status = status.clone();
            let loading = loading.clone();
            let error = error.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                let request = authenticated_get("/api/ratings/scheduler/status");
                match request.send().await {
                    Ok(response) => {
                        if response.ok() {
                            if let Ok(scheduler_status) = response.json::<SchedulerStatus>().await {
                                status.set(Some(scheduler_status));
                            } else {
                                error.set(Some("Failed to parse scheduler status".to_string()));
                            }
                        } else {
                            let status_code = response.status();
                            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            error.set(Some(format!("Request failed: {} - {}", status_code, text)));
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch scheduler status: {}", e)));
                    }
                }
                loading.set(false);
            });
        })
    };

    html! {
        <div class="scheduler-monitor">
            <div class="monitor-header">
                <h2>{"🏆 Glicko2 Ratings Scheduler Monitor"}</h2>
                <p>{"Monitor and control the automatic monthly ratings recalculation"}</p>
            </div>

            <div class="monitor-content">
                if *loading {
                    <div class="loading-container">
                        <p>{"Loading scheduler status..."}</p>
                    </div>
                } else if let Some(err) = (*error).as_ref() {
                    <div class="error-container">
                        <p class="error-text">{"Error: "}{err}</p>
                        <button onclick={refresh_status} class="refresh-btn">{"🔄 Refresh"}</button>
                    </div>
                } else if let Some(scheduler_status) = (*status).as_ref() {
                    <div class="status-grid">
                        <div class="status-card">
                            <h3>{"Scheduler Status"}</h3>
                            <div class="status-indicator">
                                <span class={classes!("status-dot", if scheduler_status.is_running { "running" } else { "stopped" })}></span>
                                <span class="status-text">
                                    {if scheduler_status.is_running { "Running" } else { "Stopped" }}
                                </span>
                            </div>
                        </div>

                        <div class="status-card">
                            <h3>{"Last Run"}</h3>
                            <div class="status-value">
                                {if let Some(last_run) = scheduler_status.last_run {
                                    format!("{}", last_run.format("%Y-%m-%d %H:%M:%S UTC"))
                                } else {
                                    "Never".to_string()
                                }}
                            </div>
                        </div>

                        <div class="status-card">
                            <h3>{"Next Scheduled Run"}</h3>
                            <div class="status-value">
                                {format!("{}", scheduler_status.next_scheduled_run.format("%Y-%m-%d %H:%M:%S UTC"))}
                            </div>
                        </div>
                    </div>

                    <div class="control-section">
                        <h3>{"Manual Control"}</h3>
                        <div class="control-grid">
                            <div class="control-item">
                                <label for="period-input">{"Period (YYYY-MM, optional):"}</label>
                                <input 
                                    id="period-input"
                                    type="text" 
                                    placeholder="e.g., 2024-01" 
                                    value={(*period_input).clone()}
                                    onchange={on_period_input_change}
                                    class="period-input"
                                />
                                <small>{"Leave empty for previous month"}</small>
                            </div>
                            
                            <div class="control-item">
                                <button 
                                    onclick={on_trigger_click}
                                    disabled={*trigger_loading}
                                    class="trigger-btn"
                                >
                                    if *trigger_loading {
                                        {"⏳ Triggering..."}
                                    } else {
                                        {"🚀 Trigger Recalculation"}
                                    }
                                </button>
                            </div>
                        </div>

                        if let Some(message) = (*trigger_message).as_ref() {
                            <div class="success-message">
                                <p>{"✅ "}{message}</p>
                            </div>
                        }
                    </div>

                    <div class="historical-section">
                        <h3>{"Historical Recalculation"}</h3>
                        <div class="historical-info">
                            <p>{"Recalculate all Glicko2 ratings from the first database entry onwards. This will:"}</p>
                            <ul>
                                <li>{"Clear all existing ratings data"}</li>
                                <li>{"Process every month from the actual first contest to present"}</li>
                                <li>{"Use actual contest results and placements"}</li>
                                <li>{"Populate both current and historical ratings"}</li>
                            </ul>
                            <div class="warning">
                                <p>{"⚠️ This operation may take several minutes and should only be run once during initial setup or after major data changes."}</p>
                            </div>
                        </div>
                        
                        <div class="historical-controls">
                            <button 
                                onclick={on_historical_recalc_click}
                                disabled={*historical_loading}
                                class="historical-btn"
                            >
                                if *historical_loading {
                                    {"⏳ Processing Historical Data..."}
                                } else {
                                    {"🔄 Recalculate All Historical Ratings"}
                                }
                            </button>
                        </div>

                        if let Some(message) = (*historical_message).as_ref() {
                            <div class="success-message">
                                <p>{"✅ "}{message}</p>
                            </div>
                        }
                    </div>

                    <div class="info-section">
                        <h3>{"How It Works"}</h3>
                        <div class="info-grid">
                            <div class="info-item">
                                <h4>{"🕐 Automatic Schedule"}</h4>
                                <p>{"The scheduler runs automatically on the 1st of each month at 2:00 AM UTC. "}
                                   {"It processes all contests from the previous month and updates player ratings."}</p>
                            </div>
                            <div class="info-item">
                                <h4>{"🎯 Manual Trigger"}</h4>
                                <p>{"You can manually trigger recalculation for any period. "}
                                   {"This is useful for testing or handling specific time periods."}</p>
                            </div>
                            <div class="info-item">
                                <h4>{"📊 Monitoring"}</h4>
                                <p>{"Monitor the scheduler status, last run time, and next scheduled run. "}
                                   {"All activities are logged for debugging and audit purposes."}</p>
                            </div>
                        </div>
                    </div>

                    <div class="actions">
                        <button onclick={refresh_status} class="refresh-btn">{"🔄 Refresh Status"}</button>
                    </div>
                } else {
                    <div class="no-data">
                        <p>{"No scheduler status available"}</p>
                        <button onclick={refresh_status} class="refresh-btn">{"🔄 Refresh"}</button>
                    </div>
                }
            </div>
        </div>
    }
}
