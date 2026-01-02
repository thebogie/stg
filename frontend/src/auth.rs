use shared::PlayerDto;
use log::error;
use yew::prelude::*;
use yew::functional::use_reducer_eq;
use gloo_storage::{LocalStorage, Storage};
use wasm_bindgen_futures::spawn_local;
use crate::api::auth;
use std::rc::Rc;
use gloo_timers::callback::Interval;

#[derive(Clone, Debug)]
pub struct AuthState {
    pub player: Option<PlayerDto>,
    pub loading: bool,
    pub error: Option<String>,
    pub heartbeat_active: bool,
}

impl PartialEq for AuthState {
    fn eq(&self, other: &Self) -> bool {
        self.loading == other.loading && 
        self.error == other.error && 
        self.heartbeat_active == other.heartbeat_active &&
        match (&self.player, &other.player) {
            (Some(a), Some(b)) => a.id == b.id,
            (None, None) => true,
            _ => false,
        }
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            player: None,
            loading: false,
            error: None,
            heartbeat_active: false,
        }
    }
}

impl AuthState {
    /// Check if the current player has administrative privileges
    pub fn is_admin(&self) -> bool {
        self.player.as_ref().map(|p| p.is_admin).unwrap_or(false)
    }
}

#[derive(Clone, Debug)]
pub enum AuthAction {
    Login { email: String, password: String },
    LoginSuccess { player: PlayerDto, session_id: String },
    LoginError(String),
    Logout,
    LogoutSuccess,
    LogoutError(String),
    SetLoading(bool),
    SetError(Option<String>),
    StartHeartbeat,
    StopHeartbeat,
    HeartbeatCheck,
    HeartbeatSuccess(PlayerDto),
    HeartbeatError(String),
    RefreshPlayer,
}

impl PartialEq for AuthAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AuthAction::Login { email: a, password: b }, AuthAction::Login { email: c, password: d }) => 
                a == c && b == d,
            (AuthAction::LoginSuccess { player: a, session_id: sa }, AuthAction::LoginSuccess { player: b, session_id: sb }) => 
                a.id == b.id && sa == sb,
            (AuthAction::LoginError(a), AuthAction::LoginError(b)) => 
                a == b,
            (AuthAction::Logout, AuthAction::Logout) => true,
            (AuthAction::LogoutSuccess, AuthAction::LogoutSuccess) => true,
            (AuthAction::LogoutError(a), AuthAction::LogoutError(b)) => 
                a == b,
            (AuthAction::SetLoading(a), AuthAction::SetLoading(b)) => 
                a == b,
            (AuthAction::SetError(a), AuthAction::SetError(b)) => 
                a == b,
            (AuthAction::StartHeartbeat, AuthAction::StartHeartbeat) => true,
            (AuthAction::StopHeartbeat, AuthAction::StopHeartbeat) => true,
            (AuthAction::HeartbeatCheck, AuthAction::HeartbeatCheck) => true,
            (AuthAction::HeartbeatSuccess(a), AuthAction::HeartbeatSuccess(b)) => 
                a.id == b.id,
            (AuthAction::HeartbeatError(a), AuthAction::HeartbeatError(b)) => 
                a == b,
            _ => false,
        }
    }
}

impl Reducible for AuthState {
    type Action = AuthAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            AuthAction::Login { .. } => {
                Rc::new(Self {
                    loading: true,
                    error: None,
                    ..(*self).clone()
                })
            }
            AuthAction::LoginSuccess { player, session_id } => {
                // Store player in local storage
                if let Err(e) = LocalStorage::set("player", &player) {
                    error!("Failed to store player in local storage: {}", e);
                }

                // Store session_id in local storage for API authentication
                if let Err(e) = LocalStorage::set("session_id", &session_id) {
                    error!("Failed to store session_id in local storage: {}", e);
                }

                Rc::new(Self {
                    player: Some(player),
                    loading: false,
                    error: None,
                    heartbeat_active: true,
                })
            }
            AuthAction::LoginError(error) => {
                Rc::new(Self {
                    player: None,
                    loading: false,
                    error: Some(error),
                    heartbeat_active: false,
                })
            }
            AuthAction::Logout => {
                Rc::new(Self {
                    loading: true,
                    error: None,
                    heartbeat_active: false,
                    ..(*self).clone()
                })
            }
            AuthAction::LogoutSuccess => {
                // Clear player and session_id from local storage
                let _ = LocalStorage::delete("player");
                let _ = LocalStorage::delete("session_id");
                Rc::new(Self {
                    player: None,
                    loading: false,
                    error: None,
                    heartbeat_active: false,
                })
            }
            AuthAction::LogoutError(error) => {
                Rc::new(Self {
                    loading: false,
                    error: Some(error),
                    heartbeat_active: false,
                    ..(*self).clone()
                })
            }
            AuthAction::SetLoading(loading) => {
                Rc::new(Self {
                    loading,
                    ..(*self).clone()
                })
            }
            AuthAction::SetError(error) => {
                Rc::new(Self {
                    error,
                    ..(*self).clone()
                })
            }
            AuthAction::StartHeartbeat => {
                Rc::new(Self {
                    heartbeat_active: true,
                    ..(*self).clone()
                })
            }
            AuthAction::StopHeartbeat => {
                Rc::new(Self {
                    heartbeat_active: false,
                    ..(*self).clone()
                })
            }
            AuthAction::HeartbeatCheck => {
                // Don't change state, just trigger the check
                self
            }
            AuthAction::HeartbeatSuccess(player) => {
                // Update player data in case it changed
                if let Err(e) = LocalStorage::set("player", &player) {
                    error!("Failed to update player in local storage: {}", e);
                }
                Rc::new(Self {
                    player: Some(player),
                    heartbeat_active: true,
                    ..(*self).clone()
                })
            }
            AuthAction::HeartbeatError(_) => {
                // Session expired, logout and redirect
                let _ = LocalStorage::delete("player");
                let _ = LocalStorage::delete("session_id");
                
                Rc::new(Self {
                    player: None,
                    loading: false,
                    error: Some("Session expired. Please log in again.".to_string()),
                    heartbeat_active: false,
                })
            }
            AuthAction::RefreshPlayer => {
                // Don't change state, just trigger the refresh
                self
            }
        }
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct AuthProviderProps {
    #[prop_or_default]
    pub children: Children,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuthContext {
    pub state: AuthState,
    pub login: Callback<(String, String)>,
    pub logout: Callback<()>,
    pub on_session_expired: Callback<()>,
    pub refresh: Callback<()>,
}

#[function_component(AuthProvider)]
pub fn auth_provider(props: &AuthProviderProps) -> Html {
    // Try to load player from local storage
    let player = LocalStorage::get("player").ok();
    let heartbeat_active = player.as_ref().is_some();
    let auth = use_reducer_eq(move || AuthState {
        player,
        heartbeat_active,
        ..Default::default()
    });

    // Heartbeat effect - runs every 5 minutes (300 seconds)
    {
        let auth = auth.clone();
        use_effect_with(auth.heartbeat_active, move |heartbeat_active| {
            if *heartbeat_active {
                let auth = auth.clone();
                let interval = Interval::new(300_000, move || {
                    let auth = auth.clone();
                    spawn_local(async move {
                        auth.dispatch(AuthAction::HeartbeatCheck);
                        
                        match auth::get_current_player().await {
                            Ok(player) => {
                                auth.dispatch(AuthAction::HeartbeatSuccess(player));
                            }
                            Err(_) => {
                                auth.dispatch(AuthAction::HeartbeatError("Session expired".to_string()));
                            }
                        }
                    });
                });
                
                Box::new(move || {
                    interval.cancel();
                }) as Box<dyn FnOnce()>
            } else {
                Box::new(|| {}) as Box<dyn FnOnce()>
            }
        });
    }

    // Handle login
    let login = {
        let auth = auth.clone();
        Callback::from(move |(email, password): (String, String)| {
            let auth = auth.clone();
            spawn_local(async move {
                auth.dispatch(AuthAction::SetLoading(true));
                auth.dispatch(AuthAction::SetError(None));

                match auth::login(&email, &password).await {
                    Ok(response) => {
                        auth.dispatch(AuthAction::LoginSuccess { 
                    player: response.player, 
                    session_id: response.session_id 
                });
                        auth.dispatch(AuthAction::StartHeartbeat);
                    }
                    Err(e) => {
                        auth.dispatch(AuthAction::LoginError(e));
                    }
                }
            });
        })
    };

    // Handle logout
    let logout = {
        let auth = auth.clone();
        Callback::from(move |_: ()| {
            let auth = auth.clone();
            spawn_local(async move {
                auth.dispatch(AuthAction::SetLoading(true));
                auth.dispatch(AuthAction::SetError(None));
                auth.dispatch(AuthAction::StopHeartbeat);

                match auth::logout().await {
                    Ok(()) => {
                        auth.dispatch(AuthAction::LogoutSuccess);
                    }
                    Err(e) => {
                        auth.dispatch(AuthAction::LogoutError(e));
                    }
                }
            });
        })
    };

    // Handle refresh
    let refresh = {
        let auth = auth.clone();
        Callback::from(move |_: ()| {
            let auth = auth.clone();
            spawn_local(async move {
                auth.dispatch(AuthAction::RefreshPlayer);
                
                match auth::get_current_player().await {
                    Ok(player) => {
                        auth.dispatch(AuthAction::HeartbeatSuccess(player));
                    }
                    Err(e) => {
                        error!("Failed to refresh player data: {}", e);
                    }
                }
            });
        })
    };

    let context = AuthContext {
        state: (*auth).clone(),
        login,
        logout,
        on_session_expired: Callback::from(|_| {
            // This will be handled by the component that uses the auth context
        }),
        refresh,
    };

    html! {
        <ContextProvider<AuthContext> context={context}>
            {props.children.clone()}
        </ContextProvider<AuthContext>>
    }
} 