use yew::prelude::*;
use yew_router::prelude::*;
use log::{info, debug};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use crate::auth::{AuthProvider, AuthContext};
use crate::components::nav::Nav;
use crate::components::footer::Footer;
use crate::components::common::toast::{ToastProvider, ToastContext, Toast, ToastType};



pub mod api;
pub mod auth;
pub mod components;
pub mod config;
pub mod version;
pub mod flatpickr;
pub mod analytics {
    pub mod client_manager;
    pub mod events;
    pub use client_manager::*;
    pub use events::*;
}
pub mod pages {
    pub mod home;
    pub mod login;
    pub mod venues;
    pub mod venue_details;
    pub mod games;
    pub mod game_details;
    pub mod analytics;
    pub mod analytics_dashboard;
    pub mod analytics_test;
    pub mod admin;

    pub mod contests;
    pub mod contest;
    pub mod contest_details;
    pub mod game_history;
    pub mod venue_history;
    pub mod not_found;
    pub mod profile;
}

use pages::{home::Home, login::Login, venues::Venues, venue_details::VenueDetails, games::Games, game_details::GameDetails, analytics::Analytics, analytics_test::AnalyticsTest, admin::AdminPage, contests::Contests, contest::Contest, contest_details::ContestDetails, game_history::GameHistory, venue_history::VenueHistory, not_found::NotFound, profile::ProfilePage};

// Unit test modules only
#[cfg(test)]
mod tests;

#[derive(Clone, Routable, PartialEq, Debug)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/login")]
    Login,
    #[at("/venues")]
    Venues,
    #[at("/venue/:venue_id")]
    VenueDetails { venue_id: String },
    #[at("/games")]
    Games,
    #[at("/game/:game_id")]
    GameDetails { game_id: String },
    #[at("/analytics")]
    Analytics,
    #[at("/analytics/test")]
    AnalyticsTest,
    #[at("/admin")]
    Admin,

    #[at("/profile")]
    Profile,
    #[at("/contests")]
    Contests,
    #[at("/contest/:contest_id")]
    ContestDetails { contest_id: String },
    #[at("/game/:game_id/history")]
    GameHistory { game_id: String },
    #[at("/venue/:venue_id/history")]
    VenueHistory { venue_id: String },
    #[at("/contest/create")]
    Contest,
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[function_component(App)]
fn app() -> Html {
    debug!("App component rendering");
    html! {
        <ToastProvider>
            <AuthProvider>
                <BrowserRouter>
                    <div class="app-container">
                        <Nav />
                        <main class="flex-1">
                            <Switch<Route> render={switch} />
                        </main>
                        <Footer />
                    </div>
                </BrowserRouter>
            </AuthProvider>
        </ToastProvider>
    }
}

#[function_component(ProtectedRoute)]
pub fn protected_route(props: &Props) -> Html {
    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let toast_context = use_context::<ToastContext>().expect("Toast context not found");
    let is_authenticated = auth.state.player.as_ref()
        .map(|player| !player.id.is_empty())
        .unwrap_or(false);
    let session_expired = auth.state.error.as_ref().map(|e| e.contains("Session expired")).unwrap_or(false);
    let navigator = use_navigator().unwrap();

    // Show toast when session expires
    {
        let toast_context = toast_context.clone();
        use_effect_with(session_expired, move |expired| {
            if *expired {
                let toast = Toast::new(
                    "Your session has expired. Please log in again.".to_string(),
                    ToastType::Warning
                ).with_duration(8000); // Show for 8 seconds
                toast_context.add_toast.emit(toast);
            }
            || ()
        });
    }

    {
        let navigator = navigator.clone();
        use_effect_with((is_authenticated, session_expired), move |(is_auth, session_expired)| {
            if !*is_auth || *session_expired {
                navigator.push(&Route::Login);
            }
            || ()
        });
    }

    if is_authenticated && !session_expired {
        html! {
            <>
                {props.children.clone()}
            </>
        }
    } else {
        html! {}
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub children: Children,
}



fn switch(routes: Route) -> Html {
    debug!("Route switch: {:?}", routes);
    match routes {
        Route::Home => {
            debug!("Rendering Home component (protected)");
            html! {
                <ProtectedRoute>
                    <Home />
                </ProtectedRoute>
            }
        },
        Route::Login => {
            debug!("Rendering Login component");
            html! { <Login /> }
        },
        Route::Profile => {
            debug!("Rendering Profile component (protected)");
            html! {
                <ProtectedRoute>
                    <ProfilePage />
                </ProtectedRoute>
            }
        },
        Route::Contests => {
            debug!("Rendering Contests component (protected)");
            html! {
                <ProtectedRoute>
                    <Contests />
                </ProtectedRoute>
            }
        },
        Route::Contest => {
            debug!("Rendering Contest creation component (protected)");
            html! {
                <ProtectedRoute>
                    <Contest />
                </ProtectedRoute>
            }
        },
        Route::ContestDetails { contest_id } => {
            debug!("Rendering Contest details component (protected) with contest_id: {}", contest_id);
            html! {
                <ProtectedRoute>
                    <ContestDetails contest_id={contest_id} />
                </ProtectedRoute>
            }
        },
        Route::GameHistory { game_id } => {
            debug!("Rendering Game history component (protected) for game: {}", game_id);
            html! {
                <ProtectedRoute>
                    <GameHistory game_id={game_id} />
                </ProtectedRoute>
            }
        },
        Route::VenueHistory { venue_id } => {
            debug!("Rendering Venue history component (protected) for venue: {}", venue_id);
            html! {
                <ProtectedRoute>
                    <VenueHistory venue_id={venue_id} />
                </ProtectedRoute>
            }
        },

        Route::Venues => {
            debug!("Rendering Venues component (protected)");
            html! {
                <ProtectedRoute>
                    <Venues />
                </ProtectedRoute>
            }
        },
        Route::VenueDetails { venue_id } => {
            debug!("Rendering Venue details component (protected) for venue: {}", venue_id);
            html! {
                <ProtectedRoute>
                    <VenueDetails venue_id={venue_id} />
                </ProtectedRoute>
            }
        },
        Route::Games => {
            debug!("Rendering Games component (protected)");
            html! {
                <ProtectedRoute>
                    <Games />
                </ProtectedRoute>
            }
        },
        Route::GameDetails { game_id } => {
            debug!("Rendering Game details component (protected) for game: {}", game_id);
            html! {
                <ProtectedRoute>
                    <GameDetails game_id={game_id} />
                </ProtectedRoute>
            }
        },
        Route::Analytics => {
            debug!("Rendering Analytics component (protected)");
            html! {
                <ProtectedRoute>
                    <Analytics />
                </ProtectedRoute>
            }
        },
        Route::AnalyticsTest => {
            debug!("Rendering Analytics Test component (protected)");
            html! {
                <ProtectedRoute>
                    <AnalyticsTest />
                </ProtectedRoute>
            }
        },
        Route::Admin => {
            debug!("Rendering Admin component (protected)");
            html! {
                <ProtectedRoute>
                    <AdminPage />
                </ProtectedRoute>
            }
        },

        Route::NotFound => {
            debug!("Rendering 404 Not Found");
            html! { <NotFound /> }
        },
    }
}

#[wasm_bindgen]
pub async fn run_app() -> Result<(), JsValue> {
    info!("Initializing application...");

    // Initialize logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    info!("Logger initialized");

    // Set up panic hook
    console_error_panic_hook::set_once();
    info!("Panic hook set");

    // Mount the app
    info!("Mounting application to #app");
    yew::Renderer::<App>::new().render();
    info!("Application mounted");

    Ok(())
}

// Add a start function that Trunk can call
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    wasm_bindgen_futures::spawn_local(async {
        run_app().await.expect("Failed to run app");
    });
    Ok(())
}

// Remove test code since we don't need it
// pub fn add(left: u64, right: u64) -> u64 {
//     left + right
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
