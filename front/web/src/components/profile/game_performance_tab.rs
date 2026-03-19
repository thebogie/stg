use crate::api::utils::authenticated_get;
use crate::auth::AuthContext;
use crate::components::contests_modal::ContestsModal;
use crate::Route;
use serde_json::Value;
use shared::dto::analytics::GamePerformanceDetailDto;
use shared::models::client_analytics::GamePerformance;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct GamePerformanceTabProps {
    pub game_performance: Option<Vec<GamePerformance>>,
    pub player_id_override: Option<String>,
}

#[function_component(GamePerformanceTab)]
pub fn game_performance_tab(props: &GamePerformanceTabProps) -> Html {
    let navigator = use_navigator().unwrap();
    let auth_context = use_context::<AuthContext>().expect("AuthContext not found");

    // Enriched game performance (best/toughest opponent, best venue) + interactive table state
    let detail_rows = use_state(|| None::<Vec<GamePerformanceDetailDto>>);
    let detail_loading = use_state(|| false);
    let detail_error = use_state(|| None::<String>);

    let filter_search = use_state(|| "".to_string());
    let filter_min_plays = use_state(|| 1i32);
    let filter_window = use_state(|| "all".to_string()); // all | 12m | 3m

    let sort_by = use_state(|| "total_plays".to_string());
    let sort_asc = use_state(|| false);
    let page = use_state(|| 0usize);
    let rows_per_page = 12usize;

    // Game contests modal state
    let game_contests_open = use_state(|| false);
    let game_contests_loading = use_state(|| false);
    let game_contests_error = use_state(|| None::<String>);
    let selected_game_contests = use_state(|| None::<Vec<Value>>);
    let selected_game_name = use_state(|| String::new());

    // Store player ID in state (kept for future use in modal)
    let player_id = use_state(|| {
        if let Some(player) = &auth_context.state.player {
            if player.id.starts_with("player/") {
                player.id.trim_start_matches("player/").to_string()
            } else {
                player.id.clone()
            }
        } else {
            String::new()
        }
    });

    // Resolve "me" vs other player key for analytics endpoint
    let player_param = if let Some(ref override_id) = props.player_id_override {
        override_id
            .strip_prefix("player/")
            .unwrap_or(override_id)
            .to_string()
    } else {
        "me".to_string()
    };

    // Fetch detail rows on mount / when viewing a different player.
    {
        let detail_rows = detail_rows.clone();
        let detail_loading = detail_loading.clone();
        let detail_error = detail_error.clone();
        let player_param = player_param.clone();
        use_effect_with(player_param.clone(), move |_p| {
            detail_loading.set(true);
            detail_error.set(None);
            spawn_local(async move {
                let url = format!("/api/analytics/players/{}/game-performance/detail", player_param);
                match authenticated_get(&url).send().await {
                    Ok(resp) if resp.ok() => match resp.json::<Vec<GamePerformanceDetailDto>>().await {
                        Ok(v) => detail_rows.set(Some(v)),
                        Err(_) => detail_error.set(Some("Failed to parse game performance detail".to_string())),
                    },
                    Ok(resp) => detail_error.set(Some(format!("Failed to load detail: {}", resp.status()))),
                    Err(e) => detail_error.set(Some(format!("Failed to load detail: {}", e))),
                }
                detail_loading.set(false);
            });
            || {}
        });
    }

    // Fetch game contests function (kept for future use in modal)
    let _fetch_game_contests = {
        let game_contests_open = game_contests_open.clone();
        let game_contests_loading = game_contests_loading.clone();
        let game_contests_error = game_contests_error.clone();
        let selected_game_contests = selected_game_contests.clone();
        let selected_game_name = selected_game_name.clone();
        let player_id = player_id.clone();

        Callback::from(move |game_name: String| {
            let game_contests_open = game_contests_open.clone();
            let game_contests_loading = game_contests_loading.clone();
            let game_contests_error = game_contests_error.clone();
            let selected_game_contests = selected_game_contests.clone();
            let selected_game_name = selected_game_name.clone();
            let player_id = player_id.clone();

            spawn_local(async move {
                game_contests_open.set(true);
                game_contests_loading.set(true);
                game_contests_error.set(None);
                selected_game_name.set(game_name.clone());

                // Check if we have a valid player ID
                if player_id.is_empty() {
                    game_contests_error.set(Some("Player not authenticated".to_string()));
                    game_contests_loading.set(false);
                    return;
                }

                let url = format!("/api/contests/player/{}/game/{}", *player_id, game_name);

                match authenticated_get(&url).send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Value>().await {
                                Ok(data) => {
                                    if let Some(contests) =
                                        data.get("contests").and_then(|v| v.as_array())
                                    {
                                        selected_game_contests.set(Some(contests.clone()));
                                    } else {
                                        selected_game_contests.set(Some(vec![]));
                                    }
                                }
                                Err(e) => {
                                    game_contests_error
                                        .set(Some(format!("Failed to parse contests: {}", e)));
                                }
                            }
                        } else {
                            game_contests_error.set(Some(format!(
                                "Failed to fetch contests: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => {
                        game_contests_error.set(Some(format!("Failed to fetch contests: {}", e)));
                    }
                }

                game_contests_loading.set(false);
            });
        })
    };

    let now_ms = js_sys::Date::now();
    let cutoff_ms = match (*filter_window).as_str() {
        "3m" => now_ms - 1000.0 * 60.0 * 60.0 * 24.0 * 30.0 * 3.0,
        "12m" => now_ms - 1000.0 * 60.0 * 60.0 * 24.0 * 30.0 * 12.0,
        _ => 0.0,
    };

    html! {
        <div class="space-y-6">
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="flex items-center justify-between mb-4">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-900">{"Game Performance"}</h2>
                        <p class="mt-1 text-gray-600">
                            {"Your performance across all games with sorting and pagination."}
                        </p>
                    </div>
                    <div class="text-4xl">{"🎮"}</div>
                </div>

                {if *detail_loading {
                    html! { <div class="py-8 text-center text-gray-600">{"Loading game performance..."}</div> }
                } else if let Some(err) = &*detail_error {
                    html! { <div class="py-6 text-sm text-red-700">{err.clone()}</div> }
                } else if let Some(rows) = &*detail_rows {
                    let search_lc = (*filter_search).to_lowercase();
                    let min_plays = *filter_min_plays;
                    let mut filtered: Vec<GamePerformanceDetailDto> = rows
                        .iter()
                        .cloned()
                        .filter(|r| r.total_plays >= min_plays)
                        .filter(|r| search_lc.is_empty() || r.game_name.to_lowercase().contains(&search_lc))
                        .filter(|r| {
                            if cutoff_ms <= 0.0 { return true; }
                            (r.last_played.timestamp_millis() as f64) >= cutoff_ms
                        })
                        .collect();

                    let sb = (*sort_by).clone();
                    let asc = *sort_asc;
                    filtered.sort_by(|a, b| {
                        let ord = match sb.as_str() {
                            "wins" => a.wins.cmp(&b.wins),
                            "win_rate" => a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal),
                            "average_placement" => b.average_placement.partial_cmp(&a.average_placement).unwrap_or(std::cmp::Ordering::Equal),
                            "last_played" => a.last_played.cmp(&b.last_played),
                            _ => a.total_plays.cmp(&b.total_plays),
                        };
                        if asc { ord } else { ord.reverse() }
                    });

                    let total_pages = (filtered.len() + rows_per_page - 1) / rows_per_page;
                    let start = (*page) * rows_per_page;
                    let end = std::cmp::min(start + rows_per_page, filtered.len());
                    let page_rows = if start < end { &filtered[start..end] } else { &filtered[0..0] };

                    html! {
                        <div class="space-y-3">
                            <div class="rounded-lg bg-gray-50 border border-gray-200 p-3 text-sm text-gray-600">
                                <p class="font-medium text-gray-700 mb-1">{"Definitions"}</p>
                                <ul class="list-disc list-inside space-y-0.5">
                                    <li><strong>{"Best opponent"}</strong>{" — Opponent you’ve beaten most often in this game (min 3 contests together). Shown as handle, your win % vs them, and number of contests."}</li>
                                    <li><strong>{"Toughest opponent"}</strong>{" — Opponent you’ve lost to most often in this game (min 3 contests together). Shown as handle, your win % vs them, and number of contests."}</li>
                                </ul>
                            </div>
                            <div class="flex flex-wrap gap-2">
                                {for [("total_plays","Plays"),("wins","Wins"),("win_rate","Win%"),("average_placement","Avg place"),("last_played","Last played")].iter().map(|(k,l)|{
                                    let key = k.to_string();
                                    let active = *sort_by == key;
                                    html!{
                                        <button
                                            class={classes!("px-3","py-1","text-sm","font-medium","rounded-md",
                                                if active { "bg-blue-100 text-blue-800" } else { "bg-gray-100 text-gray-700 hover:bg-gray-200" }
                                            )}
                                            onclick={{
                                                let sort_by = sort_by.clone();
                                                let sort_asc = sort_asc.clone();
                                                let page = page.clone();
                                                Callback::from(move |_|{
                                                    if *sort_by == key {
                                                        sort_asc.set(!*sort_asc);
                                                    } else {
                                                        sort_by.set(key.clone());
                                                        sort_asc.set(false);
                                                    }
                                                    page.set(0);
                                                })
                                            }}
                                        >
                                            {l.to_string()}
                                            {if active { html!{ <span class="ml-1">{if *sort_asc {"▲"} else {"▼"}}</span> } } else { html!{} }}
                                        </button>
                                    }
                                })}
                            </div>

                            <div class="overflow-x-auto">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Game"}</th>
                                            <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Plays"}</th>
                                            <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Win%"}</th>
                                            <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Avg place"}</th>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Best opponent"}</th>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Toughest opponent"}</th>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Best venue"}</th>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Last played"}</th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white divide-y divide-gray-200">
                                        {for page_rows.iter().map(|r| {
                                            let game_id = r.game_id.clone();
                                            html!{
                                                <tr class="hover:bg-gray-50">
                                                    <td class="px-3 py-2 text-sm">
                                                        <button
                                                            class="font-medium text-blue-600 hover:text-blue-800 hover:underline"
                                                            title={format!("View all contests you played {}", r.game_name)}
                                                            onclick={{
                                                                let navigator = navigator.clone();
                                                                let game_key = game_id.strip_prefix("game/").unwrap_or_else(|| game_id.strip_prefix("game:").unwrap_or(&game_id)).to_string();
                                                                Callback::from(move |_| navigator.push(&Route::ContestsWithGame { game_id: game_key.clone() }))
                                                            }}
                                                        >
                                                            {r.game_name.clone()}
                                                        </button>
                                                    </td>
                                                    <td class="px-3 py-2 text-sm text-center text-gray-700">{r.total_plays}</td>
                                                    <td class="px-3 py-2 text-sm text-center text-gray-700">{format!("{:.1}%", r.win_rate)}</td>
                                                    <td class="px-3 py-2 text-sm text-center text-gray-700">{format!("{:.1}", r.average_placement)}</td>
                                                    <td class="px-3 py-2 text-sm text-gray-700">
                                                        {r.best_opponent.as_ref().map(|o| format!("{} ({:.0}% / {}c)", o.player_handle, o.my_win_rate, o.contests_played)).unwrap_or_else(|| "-".into())}
                                                    </td>
                                                    <td class="px-3 py-2 text-sm text-gray-700">
                                                        {r.toughest_opponent.as_ref().map(|o| format!("{} ({:.0}% / {}c)", o.player_handle, o.my_win_rate, o.contests_played)).unwrap_or_else(|| "-".into())}
                                                    </td>
                                                    <td class="px-3 py-2 text-sm text-gray-700">
                                                        {r.best_venue.as_ref().map(|v| format!("{} ({} plays)", v.venue_name, v.plays)).unwrap_or_else(|| "-".into())}
                                                    </td>
                                                    <td class="px-3 py-2 text-sm text-gray-700">{r.last_played.to_rfc3339()}</td>
                                                </tr>
                                            }
                                        })}
                                    </tbody>
                                </table>
                            </div>

                            <div class="flex items-center justify-between text-sm text-gray-600">
                                <span>{format!("{} games", filtered.len())}</span>
                                <div class="flex items-center gap-2">
                                    <button
                                        class="px-2 py-1 rounded bg-gray-100 hover:bg-gray-200 disabled:opacity-50"
                                        disabled={*page == 0}
                                        onclick={{ let page = page.clone(); Callback::from(move |_| page.set(page.saturating_sub(1))) }}
                                    >{"Prev"}</button>
                                    <span class="font-mono">{format!("{}/{}", (*page + 1).min(total_pages.max(1)), total_pages.max(1))}</span>
                                    <button
                                        class="px-2 py-1 rounded bg-gray-100 hover:bg-gray-200 disabled:opacity-50"
                                        disabled={*page + 1 >= total_pages.max(1)}
                                        onclick={{ let page = page.clone(); Callback::from(move |_| page.set(*page + 1)) }}
                                    >{"Next"}</button>
                                </div>
                            </div>
                        </div>
                    }
                } else {
                    html! { <div class="text-center py-8 text-gray-500"><p>{"Loading game performance..."}</p></div> }
                }}
            </div>

            // Game Contests Modal
            <ContestsModal
                is_open={*game_contests_open}
                on_close={Callback::from(move |_| game_contests_open.set(false))}
                title={format!("🎮 Contests for {}", (*selected_game_name).clone())}
                subtitle={Some::<String>("Click any contest to view details".to_string())}
                contests={(*selected_game_contests).clone()}
                loading={*game_contests_loading}
                error={(*game_contests_error).clone()}
                show_bgg_link={None::<String>}
            />
        </div>
    }
}
