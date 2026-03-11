use crate::api::utils::authenticated_get;
use crate::auth::AuthContext;
use crate::Route;
use gloo_storage::Storage;
use serde_json::Value;
use yew::prelude::*;
use yew_router::prelude::*;

// Helper functions for formatting
fn format_date(date_str: &str, timezone_name: &str) -> String {
    if let Ok(utc_dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        // Convert UTC to the venue's timezone using our enhanced timezone utilities
        shared::timezone::format_with_timezone(utc_dt.with_timezone(&chrono::Utc), timezone_name)
    } else {
        date_str.to_string()
    }
}

fn format_duration(minutes: i32) -> String {
    if minutes == 0 {
        "Unknown".to_string()
    } else if minutes < 60 {
        format!("{} minutes", minutes)
    } else {
        let hours = minutes / 60;
        let remaining_minutes = minutes % 60;
        if remaining_minutes == 0 {
            format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
        } else {
            format!(
                "{} hour{} {} minute{}",
                hours,
                if hours == 1 { "" } else { "s" },
                remaining_minutes,
                if remaining_minutes == 1 { "" } else { "s" }
            )
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct ContestDetailsProps {
    pub contest_id: String,
}

#[derive(Clone, PartialEq)]
struct ContestData {
    id: String,
    name: String,
    start: String,
    stop: String,
    venue: VenueInfo,
    games: Vec<GameInfo>,
    participants: Vec<ParticipantInfo>,
    stats: Option<ContestStats>,
}

#[derive(Clone, PartialEq)]
struct VenueInfo {
    id: String,
    name: String,
    display_name: Option<String>,
    formatted_address: Option<String>,
    lat: f64,
    lng: f64,
    timezone: String,
}

#[derive(Clone, PartialEq)]
struct GameInfo {
    id: String,
    name: String,
    bgg_id: Option<i64>,
}

#[derive(Clone, PartialEq)]
struct ParticipantInfo {
    player_id: String,
    handle: String,
    firstname: Option<String>,
    lastname: Option<String>,
    place: i32,
    result: String,
}

#[derive(Clone, PartialEq)]
struct ContestStats {
    participant_count: i32,
    completion_count: i32,
    completion_rate: f64,
    average_placement: f64,
    duration_minutes: i32,
    most_popular_game: Option<String>,
    difficulty_rating: f64,
    excitement_rating: f64,
}

#[function_component(ContestDetails)]
pub fn contest_details(props: &ContestDetailsProps) -> Html {
    let _auth = use_context::<AuthContext>().expect("Auth context not found");
    let navigator = use_navigator().unwrap();

    // Use contest_id from props instead of URL
    let contest_id = props.contest_id.clone();

    let contest_details = use_state(|| None::<ContestData>);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let _came_from = use_state(|| {
        gloo_storage::LocalStorage::get::<String>("profile_last_tab")
            .unwrap_or_else(|_| "Profile".to_string())
    });

    // Load contest details
    {
        let contest_details = contest_details.clone();
        let loading = loading.clone();
        let error = error.clone();
        let contest_id = contest_id.clone();

        use_effect_with(contest_id.clone(), move |_| {
            let contest_details = contest_details.clone();
            let loading = loading.clone();
            let error = error.clone();
            let contest_id = contest_id.clone();

            wasm_bindgen_futures::spawn_local(async move {
                loading.set(true);
                error.set(None);

                gloo::console::log!("Fetching contest details for ID:", &contest_id);

                // Extract just the numeric part from contest IDs like "contest/4127490"
                let numeric_id = if contest_id.starts_with("contest/") {
                    contest_id.strip_prefix("contest/").unwrap_or(&contest_id)
                } else {
                    &contest_id
                };

                // First get contest stats
                let stats_url = format!("/api/analytics/contests/{}/stats", numeric_id);
                let stats_result = authenticated_get(&stats_url).send().await;

                let contest_url = format!("/api/contests/{}", numeric_id);
                gloo::console::log!("Fetching from URL:", &contest_url);
                let contest_result = authenticated_get(&contest_url).send().await;

                match (stats_result, contest_result) {
                    (Ok(stats_response), Ok(contest_response)) => {
                        if stats_response.ok() && contest_response.ok() {
                            if let (Ok(stats_data), Ok(contest_data)) = (
                                stats_response.json::<Value>().await,
                                contest_response.json::<Value>().await,
                            ) {
                                // Debug: Log venue data structure
                                log::debug!("🔍 Venue data structure: {:?}", contest_data["venue"]);
                                log::debug!(
                                    "🔍 Venue display_name: {:?}",
                                    contest_data["venue"]["display_name"]
                                );
                                log::debug!("🔍 Venue name: {:?}", contest_data["venue"]["name"]);
                                log::debug!(
                                    "🔍 Venue formatted_address: {:?}",
                                    contest_data["venue"]["formatted_address"]
                                );
                                log::debug!("🔍 Venue lat: {:?}", contest_data["venue"]["lat"]);
                                log::debug!("🔍 Venue lng: {:?}", contest_data["venue"]["lng"]);
                                log::debug!(
                                    "🔍 All venue fields: {:?}",
                                    contest_data["venue"]
                                        .as_object()
                                        .map(|obj| obj.keys().collect::<Vec<_>>())
                                );

                                // Parse contest details
                                let contest = ContestData {
                                    id: contest_id,
                                    name: contest_data["name"]
                                        .as_str()
                                        .unwrap_or("Unknown Contest")
                                        .to_string(),
                                    start: contest_data["start"].as_str().unwrap_or("").to_string(),
                                    stop: contest_data["stop"].as_str().unwrap_or("").to_string(),
                                    venue: VenueInfo {
                                        id: contest_data["venue"]["id"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string(),
                                        name: contest_data["venue"]["displayName"]
                                            .as_str()
                                            .unwrap_or("Venue")
                                            .to_string(),
                                        display_name: contest_data["venue"]["displayName"]
                                            .as_str()
                                            .map(|s| s.to_string()),
                                        formatted_address: contest_data["venue"]
                                            ["formattedAddress"]
                                            .as_str()
                                            .map(|s| s.to_string()),
                                        lat: contest_data["venue"]["lat"].as_f64().unwrap_or(0.0),
                                        lng: contest_data["venue"]["lng"].as_f64().unwrap_or(0.0),
                                        timezone: contest_data["venue"]["timezone"]
                                            .as_str()
                                            .unwrap_or("UTC")
                                            .to_string(),
                                    },
                                    games: contest_data["games"]
                                        .as_array()
                                        .map(|games| {
                                            games
                                                .iter()
                                                .map(|g| {
                                                    // Extract bgg_id from URL string if it's stored as a URL
                                                    let bgg_id = if let Some(bgg_id_value) =
                                                        g["bgg_id"].as_str()
                                                    {
                                                        if bgg_id_value.contains(
                                                            "boardgamegeek.com/boardgame/",
                                                        ) {
                                                            // Extract the numeric ID from the URL
                                                            bgg_id_value
                                                                .split(
                                                                    "boardgamegeek.com/boardgame/",
                                                                )
                                                                .last()
                                                                .and_then(|id_str| {
                                                                    id_str.parse::<i64>().ok()
                                                                })
                                                        } else {
                                                            // Try to parse as direct numeric value
                                                            bgg_id_value.parse::<i64>().ok()
                                                        }
                                                    } else {
                                                        // Try to parse as numeric value directly
                                                        g["bgg_id"].as_i64()
                                                    };

                                                    GameInfo {
                                                        id: g["id"]
                                                            .as_str()
                                                            .unwrap_or("")
                                                            .to_string(),
                                                        name: g["name"]
                                                            .as_str()
                                                            .unwrap_or("Unknown Game")
                                                            .to_string(),
                                                        bgg_id,
                                                    }
                                                })
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                    participants: {
                                        let mut participants: Vec<ParticipantInfo> = contest_data
                                            ["outcomes"]
                                            .as_array()
                                            .map(|outcomes| {
                                                outcomes
                                                    .iter()
                                                    .map(|o| ParticipantInfo {
                                                        player_id: o["player_id"]
                                                            .as_str()
                                                            .unwrap_or("")
                                                            .to_string(),
                                                        handle: o["handle"]
                                                            .as_str()
                                                            .unwrap_or("Unknown Player")
                                                            .to_string(),
                                                        firstname: None, // Backend doesn't send firstname/lastname
                                                        lastname: None,
                                                        place: o["place"]
                                                            .as_str()
                                                            .unwrap_or("0")
                                                            .parse()
                                                            .unwrap_or(0),
                                                        result: o["result"]
                                                            .as_str()
                                                            .unwrap_or("")
                                                            .to_string(),
                                                    })
                                                    .collect()
                                            })
                                            .unwrap_or_default();

                                        // Sort participants by place (1st place first, then 2nd, etc.)
                                        participants.sort_by_key(|p| p.place);
                                        participants
                                    },
                                    stats: Some(ContestStats {
                                        participant_count: stats_data["participant_count"]
                                            .as_i64()
                                            .unwrap_or(0)
                                            as i32,
                                        completion_count: stats_data["completion_count"]
                                            .as_i64()
                                            .unwrap_or(0)
                                            as i32,
                                        completion_rate: stats_data["completion_rate"]
                                            .as_f64()
                                            .unwrap_or(0.0),
                                        average_placement: stats_data["average_placement"]
                                            .as_f64()
                                            .unwrap_or(0.0),
                                        duration_minutes: {
                                            // Calculate duration from start/stop times
                                            if let (Ok(start_time), Ok(stop_time)) = (
                                                chrono::DateTime::parse_from_rfc3339(
                                                    &contest_data["start"].as_str().unwrap_or(""),
                                                ),
                                                chrono::DateTime::parse_from_rfc3339(
                                                    &contest_data["stop"].as_str().unwrap_or(""),
                                                ),
                                            ) {
                                                let duration = stop_time - start_time;
                                                duration.num_minutes() as i32
                                            } else {
                                                // Fallback to stats duration if parsing fails
                                                stats_data["duration_minutes"].as_i64().unwrap_or(0)
                                                    as i32
                                            }
                                        },
                                        most_popular_game: stats_data["most_popular_game"]
                                            .as_str()
                                            .map(|s| s.to_string()),
                                        difficulty_rating: stats_data["difficulty_rating"]
                                            .as_f64()
                                            .unwrap_or(5.0),
                                        excitement_rating: stats_data["excitement_rating"]
                                            .as_f64()
                                            .unwrap_or(5.0),
                                    }),
                                };

                                // Debug: Log the final parsed venue data
                                log::debug!("🔍 Final parsed venue: id={}, name={}, display_name={:?}, formatted_address={:?}, lat={}, lng={}, timezone={}",
                                    contest.venue.id, contest.venue.name, contest.venue.display_name,
                                    contest.venue.formatted_address, contest.venue.lat, contest.venue.lng, contest.venue.timezone);

                                contest_details.set(Some(contest));
                            } else {
                                error.set(Some("Failed to parse contest data".to_string()));
                            }
                        } else {
                            gloo::console::error!(
                                "Failed to fetch contest data - stats ok:",
                                stats_response.ok(),
                                "contest ok:",
                                contest_response.ok()
                            );
                            error.set(Some("Failed to fetch contest data".to_string()));
                        }
                    }
                    (Err(e), _) => {
                        gloo::console::error!("Stats fetch error:", &e.to_string());
                        error.set(Some(format!("Failed to fetch contest stats: {}", e)));
                    }
                    (_, Err(e)) => {
                        gloo::console::error!("Contest fetch error:", &e.to_string());
                        error.set(Some(format!("Failed to fetch contest details: {}", e)));
                    }
                }

                loading.set(false);
            });

            || ()
        });
    }

    let on_back = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            // Always route back to Profile, Profile page will restore the last tab
            navigator.push(&Route::Profile)
        })
    };

    html! {
        <div class="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100">
            <header class="bg-white shadow-sm border-b border-gray-200">
                <div class="container mx-auto px-4 py-3">
                    <div class="flex items-center space-x-4">
                        <button
                            onclick={on_back}
                            class="flex items-center space-x-2 px-3 py-1.5 bg-gradient-to-r from-blue-600 to-purple-600 text-white rounded-md hover:from-blue-700 hover:to-purple-700 transition-all duration-200 shadow-sm hover:shadow-md"
                        >
                            <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M9.707 16.707a1 1 0 01-1.414 0l-6-6a1 1 0 010-1.414l6-6a1 1 0 011.414 1.414L5.414 9H17a1 1 0 110 2H5.414l4.293 4.293a1 1 0 010 1.414z" clip-rule="evenodd" />
                            </svg>
                            <span class="font-medium text-sm">{"Back"}</span>
                        </button>
                        <div class="flex-1">
                            <h1 class="text-xl font-bold text-gray-900">{"Contest Details"}</h1>
                            <p class="text-sm text-gray-600">{"View contest information, participants, and statistics"}</p>
                        </div>
                    </div>
                </div>
            </header>

            <main class="container mx-auto px-4 py-4">
                if *loading {
                    <div class="flex items-center justify-center py-12">
                        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
                        <span class="ml-3 text-lg text-gray-600">{"Loading contest details..."}</span>
                    </div>
                } else if let Some(err) = &*error {
                    <div class="bg-red-50 border border-red-200 rounded-lg p-6 shadow-sm">
                        <div class="flex items-center">
                            <div class="flex-shrink-0">
                                <svg class="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
                                </svg>
                            </div>
                            <div class="ml-3">
                                <h3 class="text-sm font-medium text-red-800">{"Error loading contest details"}</h3>
                                <div class="mt-2 text-sm text-red-700">{err}</div>
                            </div>
                        </div>
                    </div>
                } else if let Some(contest) = &*contest_details {
                    <div class="space-y-6">
                        // Contest Header with gradient background
                        <div class="relative overflow-hidden rounded-lg bg-gradient-to-br from-blue-500 via-purple-500 to-indigo-600 p-4 text-white shadow-lg">
                            <div class="absolute inset-0 bg-black opacity-5"></div>
                            <div class="relative z-10">
                                <div class="flex justify-between items-start mb-3">
                                    <div class="flex-1">
                                        <h2 class="text-2xl font-bold mb-2 drop-shadow-lg">{&contest.name}</h2>
                                        <div class="flex items-center space-x-4 text-blue-100 text-sm">
                                            <div class="flex items-center space-x-1">
                                                <svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
                                                    <path fill-rule="evenodd" d="M6 2a1 1 0 00-1 1v1H4a2 2 0 00-2 2v10a2 2 0 002 2h12a2 2 0 002-2V6a2 2 0 00-2-2h-1V3a1 1 0 10-2 0v1H7V3a1 1 0 00-1-1zm0 5a1 1 0 000 2h8a1 1 0 100-2H6z" clip-rule="evenodd" />
                                                </svg>
                                                <span>{format_date(&contest.start, &contest.venue.timezone)}</span>
                                            </div>
                                            <div class="flex items-center space-x-1">
                                                <svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
                                                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z" clip-rule="evenodd" />
                                                </svg>
                                                <span>{"Duration: "}{format_duration(contest.stats.as_ref().map(|s| s.duration_minutes).unwrap_or(0))}</span>
                                            </div>
                                        </div>
                                    </div>
                                    if let Some(stats) = &contest.stats {
                                        <div class="text-right bg-white bg-opacity-20 rounded-md p-2 backdrop-blur-sm">
                                            <div class="text-2xl font-bold">{stats.participant_count}</div>
                                            <div class="text-xs text-blue-100">{"Participants"}</div>
                                        </div>
                                    }
                                </div>
                            </div>
                        </div>

                                                // Venue and Games in a tighter grid
                        <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
                            <div class="bg-white rounded-md shadow-sm border border-gray-100 p-3 hover:shadow-md transition-shadow duration-200">
                                <div class="flex items-center mb-2">
                                    <div class="p-1.5 bg-blue-100 rounded-md mr-2">
                                        <svg class="h-4 w-4 text-blue-600" fill="currentColor" viewBox="0 0 20 20">
                                            <path fill-rule="evenodd" d="M5.05 4.05a7 7 0 119.9 9.9L10 18.9l-4.95-4.95a7 7 0 010-9.9zM10 11a2 2 0 100-4 2 2 0 000 4z" clip-rule="evenodd" />
                                        </svg>
                                    </div>
                                    <h3 class="text-base font-semibold text-gray-900">{"Venue"}</h3>
                                </div>
                                <div class="space-y-1.5">
                                    <p class="text-sm font-medium text-gray-900">
                                        {if let Some(display_name) = &contest.venue.display_name {
                                            if !display_name.is_empty() && display_name != "Unknown Venue" {
                                                display_name.clone()
                                            } else if !contest.venue.name.is_empty() && contest.venue.name != "Venue" {
                                                contest.venue.name.clone()
                                            } else {
                                                "Venue".to_string()
                                            }
                                        } else if !contest.venue.name.is_empty() && contest.venue.name != "Venue" {
                                            contest.venue.name.clone()
                                        } else {
                                            "Venue".to_string()
                                        }}
                                    </p>
                                    if contest.venue.lat != 0.0 && contest.venue.lng != 0.0 {
                                        <p class="text-xs text-gray-600 flex items-center">
                                            <svg class="h-3 w-3 mr-1.5 text-gray-400" fill="currentColor" viewBox="0 0 20 20">
                                                <path fill-rule="evenodd" d="M5.05 4.05a7 7 0 119.9 9.9L10 18.9l-4.95-4.95a7 7 0 010-9.9zM10 11a2 2 0 100-4 2 2 0 000 4z" clip-rule="evenodd" />
                                            </svg>
                                            <a
                                                href={format!("https://www.google.com/maps?q={},{}", contest.venue.lat, contest.venue.lng)}
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                class="text-blue-600 hover:text-blue-800 underline hover:no-underline transition-colors"
                                            >
                                                {format!("{:.4}, {:.4}", contest.venue.lat, contest.venue.lng)}
                                            </a>
                                        </p>
                                    }
                                    if let Some(address) = &contest.venue.formatted_address {
                                        if !address.is_empty() && address != "Address not available" {
                                            <p class="text-xs text-gray-500 flex items-center">
                                                <svg class="h-3 w-3 mr-1.5 text-gray-400" fill="currentColor" viewBox="0 0 20 20">
                                                    <path fill-rule="evenodd" d="M5.05 4.05a7 7 0 119.9 9.9L10 18.9l-4.95-4.95a7 7 0 010-9.9zM10 11a2 2 0 100-4 2 2 0 000 4z" clip-rule="evenodd" />
                                                </svg>
                                                {address}
                                            </p>
                                        }
                                    }
                                </div>
                            </div>

                            <div class="bg-white rounded-md shadow-sm border border-gray-100 p-3 hover:shadow-md transition-shadow duration-200">
                                <div class="flex items-center mb-2">
                                    <div class="p-1.5 bg-green-100 rounded-md mr-2">
                                        <svg class="h-4 w-4 text-green-600" fill="currentColor" viewBox="0 0 20 20">
                                            <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                                        </svg>
                                    </div>
                                    <h3 class="text-base font-semibold text-gray-900">{"Games Played"}</h3>
                                </div>
                                <div class="space-y-1.5">
                                    {contest.games.iter().map(|game| {
                                        html! {
                                            <div class="flex items-center justify-between p-1.5 bg-gray-50 rounded-md hover:bg-gray-100 transition-colors">
                                                <div class="flex items-center">
                                                    {if let Some(bgg_id) = game.bgg_id {
                                                        html! {
                                                            <a
                                                                href={format!("https://boardgamegeek.com/boardgame/{}", bgg_id)}
                                                                target="_blank"
                                                                rel="noopener noreferrer"
                                                                class="font-medium text-blue-600 hover:text-blue-800 underline hover:no-underline transition-colors text-sm"
                                                            >
                                                                {&game.name}
                                                            </a>
                                                        }
                                                    } else {
                                                        html! {
                                                            <span class="font-medium text-gray-900 text-sm">{&game.name}</span>
                                                        }
                                                    }}

                                                </div>
                                                <div class="w-1.5 h-1.5 bg-green-400 rounded-full"></div>
                                            </div>
                                        }
                                    }).collect::<Html>()}
                                </div>
                            </div>
                        </div>

                        // Participants Table with enhanced styling
                        <div class="bg-white rounded-md shadow-sm border border-gray-100 overflow-hidden">
                            <div class="px-3 py-2 border-b border-gray-200 bg-gradient-to-r from-gray-50 to-gray-100">
                                <h3 class="text-base font-semibold text-gray-900 flex items-center">
                                    <svg class="h-4 w-4 mr-2 text-blue-600" fill="currentColor" viewBox="0 0 20 20">
                                        <path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3z" />
                                    </svg>
                                    {"Participants"}
                                </h3>
                            </div>
                            <div class="overflow-x-auto">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-3 py-1.5 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Place"}</th>
                                            <th class="px-3 py-1.5 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Player"}</th>
                                            <th class="px-3 py-1.5 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Result"}</th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white divide-y divide-gray-200">
                                        {contest.participants.iter().map(|participant| {
                                            let place_class = if participant.place == 1 {
                                                "bg-gradient-to-r from-yellow-400 to-yellow-500 text-white"
                                            } else if participant.place == 2 {
                                                "bg-gradient-to-r from-gray-400 to-gray-500 text-white"
                                            } else if participant.place == 3 {
                                                "bg-gradient-to-r from-orange-400 to-orange-500 text-white"
                                            } else {
                                                "bg-gray-100 text-gray-700"
                                            };

                                            html! {
                                                <tr class="hover:bg-gray-50 transition-colors">
                                                    <td class="px-3 py-2 whitespace-nowrap">
                                                        <span class={classes!("inline-flex", "items-center", "px-1.5", "py-0.5", "rounded-full", "text-xs", "font-medium", place_class)}>
                                                            {if participant.place > 0 { format!("#{}", participant.place) } else { "N/A".to_string() }}
                                                        </span>
                                                    </td>
                                                    <td class="px-3 py-2 whitespace-nowrap">
                                                        <div class="flex items-center">
                                                            <div class="flex-shrink-0 h-6 w-6">
                                                                <div class="h-6 w-6 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center text-white font-bold text-xs">
                                                                    {participant.handle.chars().next().unwrap_or('?').to_uppercase().next().unwrap_or('?')}
                                                                </div>
                                                            </div>
                                                            <div class="ml-2">
                                                                <div class="text-sm font-medium text-gray-900">{&participant.handle}</div>
                                                                if let (Some(first), Some(last)) = (&participant.firstname, &participant.lastname) {
                                                                    <div class="text-xs text-gray-500">{first} {" "} {last}</div>
                                                                }
                                                            </div>
                                                        </div>
                                                    </td>
                                                    <td class="px-3 py-2 whitespace-nowrap">
                                                        <span class={classes!("inline-flex", "items-center", "px-1.5", "py-0.5", "rounded-full", "text-xs", "font-medium",
                                                            if participant.result.to_lowercase() == "won" { "bg-green-100 text-green-800" }
                                                            else if participant.result.to_lowercase() == "lost" { "bg-red-100 text-red-800" }
                                                            else { "bg-gray-100 text-gray-800" }
                                                        )}>
                                                            {&participant.result}
                                                        </span>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect::<Html>()}
                                    </tbody>
                                </table>
                            </div>
                        </div>

                        // Contest Statistics with beautiful cards
                        if let Some(stats) = &contest.stats {
                            <div class="bg-white rounded-md shadow-sm border border-gray-100 p-3">
                                <h3 class="text-base font-semibold text-gray-900 mb-3 flex items-center">
                                    <svg class="h-4 w-4 mr-2 text-purple-600" fill="currentColor" viewBox="0 0 20 20">
                                        <path d="M2 11a1 1 0 011-1h2a1 1 0 011 1v5a1 1 0 01-1 1H3a1 1 0 01-1-1v-5zM8 7a1 1 0 011-1h2a1 1 0 011 1v9a1 1 0 01-1 1H9a1 1 0 01-1-1V7zM14 4a1 1 0 011-1h2a1 1 0 011 1v12a1 1 0 01-1 1h-2a1 1 0 01-1-1V4z" />
                                    </svg>
                                    {"Contest Statistics"}
                                </h3>
                                <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
                                    <div class="text-center p-2 bg-gradient-to-br from-blue-50 to-blue-100 rounded-md border border-blue-200">
                                        <div class="text-xl font-bold text-blue-600">{format!("{:.1}%", stats.completion_rate)}</div>
                                        <div class="text-xs text-blue-700 font-medium">{"Completion Rate"}</div>
                                    </div>
                                    <div class="text-center p-2 bg-gradient-to-br from-green-50 to-green-100 rounded-md border border-green-200">
                                        <div class="text-xl font-bold text-green-600">{format!("{:.1}", stats.average_placement)}</div>
                                        <div class="text-xs text-green-700 font-medium">{"Avg Placement"}</div>
                                    </div>
                                    <div class="text-center p-2 bg-gradient-to-br from-purple-50 to-purple-100 rounded-md border border-purple-200">
                                        <div class="text-xl font-bold text-purple-600">{format!("{:.1}", stats.difficulty_rating)}</div>
                                        <div class="text-xs text-purple-700 font-medium">{"Difficulty"}</div>
                                    </div>
                                    <div class="text-center p-2 bg-gradient-to-br from-orange-50 to-orange-100 rounded-md border border-orange-200">
                                        <div class="text-xl font-bold text-orange-600">{format!("{:.1}", stats.excitement_rating)}</div>
                                        <div class="text-xs text-orange-700 font-medium">{"Excitement"}</div>
                                    </div>
                                </div>
                            </div>
                        }
                    </div>
                }
            </main>
        </div>
    }
}
