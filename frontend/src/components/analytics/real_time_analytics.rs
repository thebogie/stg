use yew::prelude::*;
use shared::dto::client_sync::*;
use shared::models::client_analytics::*;
use shared::models::client_storage::*;
use web_sys::console;
use chrono::{DateTime, FixedOffset, Utc};

/// Real-time analytics component that demonstrates client-side computation
#[function_component(RealTimeAnalytics)]
pub fn real_time_analytics() -> Html {
    let player_id = use_state(|| "player/test".to_string());
    
    // Query state
    let date_range = use_state(|| None::<ClientDateRange>);
    let selected_games = use_state(|| Vec::<String>::new());
    let selected_venues = use_state(|| Vec::<String>::new());
    let selected_opponents = use_state(|| Vec::<String>::new());
    let min_players = use_state(|| None::<i32>);
    let max_players = use_state(|| None::<i32>);
    let result_filter = use_state(|| Vec::<String>::new());
    let placement_range = use_state(|| None::<ClientPlacementRange>);

    // Results state
    let analytics_result = use_state(|| None::<ComputedAnalytics>);
    let computation_time = use_state(|| None::<u128>);
    let error = use_state(|| None::<String>);

    // Sample data for demonstration
    let sample_contests = use_state(|| create_sample_contests());

    // Compute analytics when query changes
    {
        let sample_contests = sample_contests.clone();
        let analytics_result = analytics_result.clone();
        let computation_time = computation_time.clone();
        let error = error.clone();
        
        let query = create_current_query(
            date_range.as_ref(),
            selected_games.as_ref(),
            selected_venues.as_ref(),
            selected_opponents.as_ref(),
            *min_players,
            *max_players,
            result_filter.as_ref(),
            placement_range.as_ref(),
        );

        use_effect_with(query, move |query| {
            let start_time = std::time::Instant::now();
            
            // Create a mock cache with sample data
            let mut cache = ClientAnalyticsCache::new(player_id.clone());
            cache.contests = sample_contests.clone();
            cache.compute_core_stats();
            cache.build_lookups();

            // Execute query
            let result = cache.query_analytics(query.clone());
            
            let end_time = std::time::Instant::now();
            let duration = end_time.duration_since(start_time).as_micros();

            match result {
                Ok(analytics) => {
                    analytics_result.set(Some(analytics));
                    computation_time.set(Some(duration));
                    error.set(None);
                    console::log_1(&format!("Analytics computed in {} microseconds", duration).into());
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    analytics_result.set(None);
                    computation_time.set(None);
                }
            }
        });
    }

    let on_date_range_change = {
        let date_range = date_range.clone();
        Callback::from(move |new_range: Option<ClientDateRange>| {
            date_range.set(new_range);
        })
    };

    let on_game_selection_change = {
        let selected_games = selected_games.clone();
        Callback::from(move |games: Vec<String>| {
            selected_games.set(games);
        })
    };

    let on_venue_selection_change = {
        let selected_venues = selected_venues.clone();
        Callback::from(move |venues: Vec<String>| {
            selected_venues.set(venues);
        })
    };

    let on_min_players_change = {
        let min_players = min_players.clone();
        Callback::from(move |value: Option<i32>| {
            min_players.set(value);
        })
    };

    let on_max_players_change = {
        let max_players = max_players.clone();
        Callback::from(move |value: Option<i32>| {
            max_players.set(value);
        })
    };

    let on_result_filter_change = {
        let result_filter = result_filter.clone();
        Callback::from(move |results: Vec<String>| {
            result_filter.set(results);
        })
    };

    html! {
        <div class="real-time-analytics">
            <div class="bg-white shadow-lg rounded-lg p-6">
                <h2 class="text-2xl font-bold text-gray-900 mb-6">
                    {"Real-Time Analytics"}
                </h2>
                
                // Query Controls
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-6">
                    // Date Range
                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-gray-700">
                            {"Date Range"}
                        </label>
                        <DateRangeSelector on_change={on_date_range_change} />
                    </div>

                    // Games
                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-gray-700">
                            {"Games"}
                        </label>
                        <GameSelector on_change={on_game_selection_change} />
                    </div>

                    // Venues
                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-gray-700">
                            {"Venues"}
                        </label>
                        <VenueSelector on_change={on_venue_selection_change} />
                    </div>

                    // Player Count
                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-gray-700">
                            {"Min Players"}
                        </label>
                        <input
                            type="number"
                            min="1"
                            max="10"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                            onchange={on_min_players_change}
                        />
                    </div>

                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-gray-700">
                            {"Max Players"}
                        </label>
                        <input
                            type="number"
                            min="1"
                            max="10"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                            onchange={on_max_players_change}
                        />
                    </div>

                    // Result Filter
                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-gray-700">
                            {"Results"}
                        </label>
                        <ResultFilterSelector on_change={on_result_filter_change} />
                    </div>
                </div>

                // Performance Indicator
                if let Some(computation_time) = *computation_time {
                    <div class="mb-4 p-3 bg-green-50 border border-green-200 rounded-md">
                        <p class="text-sm text-green-800">
                            <span class="font-semibold">{"⚡ "}</span>
                            {"Analytics computed in "} <span class="font-mono">{computation_time}</span> {" microseconds"}
                        </p>
                    </div>
                }

                // Error Display
                if let Some(error_msg) = (*error).as_ref() {
                    <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-md">
                        <p class="text-sm text-red-800">
                            <span class="font-semibold">{"❌ "}</span>
                            {error_msg}
                        </p>
                    </div>
                }

                // Results Display
                if let Some(analytics) = (*analytics_result).as_ref() {
                    <AnalyticsResults analytics={analytics.clone()} />
                }
            </div>
        </div>
    }
}

/// Date range selector component
#[function_component(DateRangeSelector)]
fn date_range_selector(props: &DateRangeSelectorProps) -> Html {
    let start_date = use_state(|| Utc::now().fixed_offset());
    let end_date = use_state(|| Utc::now().fixed_offset());

    let on_start_change = {
        let start_date = start_date.clone();
        Callback::from(move |value: String| {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&value) {
                start_date.set(dt.fixed_offset());
            }
        })
    };

    let on_end_change = {
        let end_date = end_date.clone();
        Callback::from(move |value: String| {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&value) {
                end_date.set(dt.fixed_offset());
            }
        })
    };

    let on_apply = {
        let start_date = start_date.clone();
        let end_date = end_date.clone();
        let on_change = props.on_change.clone();
        
        Callback::from(move |_| {
            let range = ClientDateRange {
                start: *start_date,
                end: *end_date,
            };
            on_change.emit(Some(range));
        })
    };

    html! {
        <div class="space-y-2">
            <input
                type="datetime-local"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                onchange={on_start_change}
            />
            <input
                type="datetime-local"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                onchange={on_end_change}
            />
            <button
                onclick={on_apply}
                class="w-full px-3 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
                {"Apply Date Range"}
            </button>
        </div>
    }
}

/// Game selector component
#[function_component(GameSelector)]
fn game_selector(props: &GameSelectorProps) -> Html {
    let selected_games = use_state(|| Vec::<String>::new());
    
    let games = vec![
        "Catan".to_string(),
        "Ticket to Ride".to_string(),
        "Pandemic".to_string(),
        "Carcassonne".to_string(),
        "Dominion".to_string(),
    ];

    let on_game_toggle = {
        let selected_games = selected_games.clone();
        let on_change = props.on_change.clone();
        
        Callback::from(move |game: String| {
            let mut current = selected_games.as_ref().clone();
            if current.contains(&game) {
                current.retain(|g| g != &game);
            } else {
                current.push(game);
            }
            selected_games.set(current.clone());
            on_change.emit(current);
        })
    };

    html! {
        <div class="space-y-2">
            {games.into_iter().map(|game| {
                let is_selected = selected_games.contains(&game);
                html! {
                    <label class="flex items-center space-x-2">
                        <input
                            type="checkbox"
                            checked={is_selected}
                            class="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                            onchange={let game = game.clone(); Callback::from(move |_| on_game_toggle.emit(game.clone()))}
                        />
                        <span class="text-sm text-gray-700">{game}</span>
                    </label>
                }
            }).collect::<Html>()}
        </div>
    }
}

/// Venue selector component
#[function_component(VenueSelector)]
fn venue_selector(props: &VenueSelectorProps) -> Html {
    let selected_venues = use_state(|| Vec::<String>::new());
    
    let venues = vec![
        "Home".to_string(),
        "Game Store".to_string(),
        "Community Center".to_string(),
        "Library".to_string(),
        "Friend's House".to_string(),
    ];

    let on_venue_toggle = {
        let selected_venues = selected_venues.clone();
        let on_change = props.on_change.clone();
        
        Callback::from(move |venue: String| {
            let mut current = selected_venues.as_ref().clone();
            if current.contains(&venue) {
                current.retain(|v| v != &venue);
            } else {
                current.push(venue);
            }
            selected_venues.set(current.clone());
            on_change.emit(current);
        })
    };

    html! {
        <div class="space-y-2">
            {venues.into_iter().map(|venue| {
                let is_selected = selected_venues.contains(&venue);
                html! {
                    <label class="flex items-center space-x-2">
                        <input
                            type="checkbox"
                            checked={is_selected}
                            class="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                            onchange={let venue = venue.clone(); Callback::from(move |_| on_venue_toggle.emit(venue.clone()))}
                        />
                        <span class="text-sm text-gray-700">{venue}</span>
                    </label>
                }
            }).collect::<Html>()}
        </div>
    }
}

/// Result filter selector component
#[function_component(ResultFilterSelector)]
fn result_filter_selector(props: &ResultFilterSelectorProps) -> Html {
    let selected_results = use_state(|| Vec::<String>::new());
    
    let results = vec![
        "won".to_string(),
        "lost".to_string(),
        "tied".to_string(),
    ];

    let on_result_toggle = {
        let selected_results = selected_results.clone();
        let on_change = props.on_change.clone();
        
        Callback::from(move |result: String| {
            let mut current = selected_results.as_ref().clone();
            if current.contains(&result) {
                current.retain(|r| r != &result);
            } else {
                current.push(result);
            }
            selected_results.set(current.clone());
            on_change.emit(current);
        })
    };

    html! {
        <div class="space-y-2">
            {results.into_iter().map(|result| {
                let is_selected = selected_results.contains(&result);
                html! {
                    <label class="flex items-center space-x-2">
                        <input
                            type="checkbox"
                            checked={is_selected}
                            class="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                            onchange={let result = result.clone(); Callback::from(move |_| on_result_toggle.emit(result.clone()))}
                        />
                        <span class="text-sm text-gray-700">{result}</span>
                    </label>
                }
            }).collect::<Html>()}
        </div>
    }
}

/// Analytics results display component
#[function_component(AnalyticsResults)]
fn analytics_results(props: &AnalyticsResultsProps) -> Html {
    let analytics = &props.analytics;

    html! {
        <div class="space-y-6">
            // Core Stats
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div class="bg-blue-50 p-4 rounded-lg text-center">
                    <div class="text-2xl font-bold text-blue-600">{analytics.stats.total_contests}</div>
                    <div class="text-sm text-blue-800">{"Total Contests"}</div>
                </div>
                <div class="bg-green-50 p-4 rounded-lg text-center">
                    <div class="text-2xl font-bold text-green-600">{analytics.stats.total_wins}</div>
                    <div class="text-sm text-green-800">{"Wins"}</div>
                </div>
                <div class="bg-red-50 p-4 rounded-lg text-center">
                    <div class="text-2xl font-bold text-red-600">{analytics.stats.total_losses}</div>
                    <div class="text-sm text-red-800">{"Losses"}</div>
                </div>
                <div class="bg-purple-50 p-4 rounded-lg text-center">
                    <div class="text-2xl font-bold text-purple-600">{format!("{:.1}%", analytics.stats.win_rate)}</div>
                    <div class="text-sm text-purple-800">{"Win Rate"}</div>
                </div>
            </div>

            // Game Performance
            if !analytics.game_performance.is_empty() {
                <div class="bg-white border border-gray-200 rounded-lg p-4">
                    <h3 class="text-lg font-semibold text-gray-900 mb-4">{"Game Performance"}</h3>
                    <div class="overflow-x-auto">
                        <table class="min-w-full divide-y divide-gray-200">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Game"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Plays"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Wins"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Win Rate"}</th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Best Place"}</th>
                                </tr>
                            </thead>
                            <tbody class="bg-white divide-y divide-gray-200">
                                {analytics.game_performance.iter().take(5).map(|game| html! {
                                    <tr>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                                            {&game.game.name}
                                        </td>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                            {game.total_plays}
                                        </td>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                            {game.wins}
                                        </td>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                            {format!("{:.1}%", game.win_rate)}
                                        </td>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                            {game.best_placement}
                                        </td>
                                    </tr>
                                }).collect::<Html>()}
                            </tbody>
                        </table>
                    </div>
                </div>
            }

            // Trends
            if !analytics.trends.is_empty() {
                <div class="bg-white border border-gray-200 rounded-lg p-4">
                    <h3 class="text-lg font-semibold text-gray-900 mb-4">{"Performance Trends"}</h3>
                    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                        {analytics.trends.iter().take(4).map(|trend| html! {
                            <div class="text-center">
                                <div class="text-lg font-semibold text-gray-900">{&trend.period}</div>
                                <div class="text-sm text-gray-600">{trend.contests_played} {" contests"}</div>
                                <div class="text-sm text-gray-600">{format!("{:.1}%", trend.win_rate)} {" win rate"}</div>
                            </div>
                        }).collect::<Html>()}
                    </div>
                </div>
            }
        </div>
    }
}

// Props for components
#[derive(Properties, PartialEq, Clone)]
pub struct DateRangeSelectorProps {
    pub on_change: Callback<Option<ClientDateRange>>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct GameSelectorProps {
    pub on_change: Callback<Vec<String>>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct VenueSelectorProps {
    pub on_change: Callback<Vec<String>>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct ResultFilterSelectorProps {
    pub on_change: Callback<Vec<String>>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct AnalyticsResultsProps {
    pub analytics: ComputedAnalytics,
}

// Helper functions
fn create_current_query(
    date_range: Option<&ClientDateRange>,
    games: &[String],
    venues: &[String],
    opponents: &[String],
    min_players: Option<i32>,
    max_players: Option<i32>,
    result_filter: &[String],
    placement_range: Option<&ClientPlacementRange>,
) -> AnalyticsQuery {
    AnalyticsQuery {
        date_range: date_range.cloned(),
        games: if games.is_empty() { None } else { Some(games.to_vec()) },
        venues: if venues.is_empty() { None } else { Some(venues.to_vec()) },
        opponents: if opponents.is_empty() { None } else { Some(opponents.to_vec()) },
        min_players,
        max_players,
        result_filter: if result_filter.is_empty() { None } else { Some(result_filter.to_vec()) },
        placement_range: placement_range.cloned(),
    }
}

fn create_sample_contests() -> Vec<ClientContest> {
    vec![
        ClientContest {
            id: "contest/1".to_string(),
            name: "Friday Night Games".to_string(),
            start: Utc::now().fixed_offset(),
            end: Utc::now().fixed_offset(),
            game: ClientGame {
                id: "game/1".to_string(),
                name: "Catan".to_string(),
                year_published: Some(1995),
            },
            venue: ClientVenue {
                id: "venue/1".to_string(),
                name: "Home".to_string(),
                display_name: Some("My House".to_string()),
                city: Some("Anytown".to_string()),
                state: Some("ST".to_string()),
            },
            participants: vec![
                ClientParticipant {
                    player_id: "player/test".to_string(),
                    handle: "testuser".to_string(),
                    firstname: Some("Test".to_string()),
                    lastname: Some("User".to_string()),
                    place: 1,
                    result: "won".to_string(),
                },
                ClientParticipant {
                    player_id: "player/opponent".to_string(),
                    handle: "opponent".to_string(),
                    firstname: Some("Opponent".to_string()),
                    lastname: Some("Player".to_string()),
                    place: 2,
                    result: "lost".to_string(),
                },
            ],
            my_result: ClientResult {
                place: 1,
                result: "won".to_string(),
                points: Some(10),
            },
        },
        // Add more sample contests as needed
    ]
}
