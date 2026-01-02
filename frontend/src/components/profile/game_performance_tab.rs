use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use shared::models::client_analytics::GamePerformance;
use serde_json::Value;
use yew_router::prelude::*;
use crate::api::utils::authenticated_get;
use crate::components::contests_modal::ContestsModal;
use crate::auth::AuthContext;
use crate::Route;

#[derive(Properties, PartialEq)]
pub struct GamePerformanceTabProps {
	pub game_performance: Option<Vec<GamePerformance>>,
}

#[function_component(GamePerformanceTab)]
pub fn game_performance_tab(props: &GamePerformanceTabProps) -> Html {
	let navigator = use_navigator().unwrap();
	let auth_context = use_context::<AuthContext>().expect("AuthContext not found");
	
	// Game performance sorting and pagination state
	let game_sort_by = use_state(|| "win_rate".to_string());
	let game_sort_asc = use_state(|| false);
	let game_page = use_state(|| 0);
	let games_per_page = 10;
	
	// Game contests modal state
	let game_contests_open = use_state(|| false);
	let game_contests_loading = use_state(|| false);
	let game_contests_error = use_state(|| None::<String>);
	let selected_game_contests = use_state(|| None::<Vec<Value>>);
	let selected_game_name = use_state(|| String::new());
	
	// Store player ID in state
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
									if let Some(contests) = data.get("contests").and_then(|v| v.as_array()) {
										selected_game_contests.set(Some(contests.clone()));
									} else {
										selected_game_contests.set(Some(vec![]));
									}
								}
								Err(e) => {
									game_contests_error.set(Some(format!("Failed to parse contests: {}", e)));
								}
							}
						} else {
							game_contests_error.set(Some(format!("Failed to fetch contests: {}", response.status())));
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
	
	let total_pages = if let Some(games) = &props.game_performance {
		(games.len() + games_per_page - 1) / games_per_page
	} else {
		0
	};
	
	html! {
		<div class="space-y-6">
			<div class="bg-white shadow rounded-lg p-6">
				<div class="flex items-center justify-between mb-4">
					<h3 class="text-lg font-medium text-gray-900">{"🎮 Game Performance"}</h3>
					<p class="text-sm text-gray-600">{"Your performance across all games with sorting and pagination"}</p>
				</div>
				
				{if let Some(games) = &props.game_performance {
					if games.is_empty() {
						html! {
							<div class="text-center py-8 text-gray-500">
								<p>{"No game performance data available"}</p>
							</div>
						}
					} else {
						html! {
							<div class="space-y-4">
								// Sorting controls
								<div class="flex flex-wrap gap-2">
									{["win_rate", "total_plays", "wins", "best_placement", "average_placement"].iter().map(|field| {
										let field_str = field.to_string();
										let is_active = *game_sort_by == field_str;
										let sort_asc = *game_sort_asc;
										
										html! {
											<button
												class={classes!(
													"px-3", "py-1", "text-sm", "font-medium", "rounded-md",
													if is_active {
														classes!("bg-blue-100", "text-blue-800")
													} else {
														classes!("bg-gray-100", "text-gray-700", "hover:bg-gray-200")
													}
												)}
												onclick={let field = field_str.clone(); let game_sort_by = game_sort_by.clone(); let game_sort_asc = game_sort_asc.clone(); let game_page = game_page.clone(); yew::Callback::from(move |_| {
													if *game_sort_by == field {
														game_sort_asc.set(!*game_sort_asc);
													} else {
														game_sort_by.set(field.clone());
														game_sort_asc.set(false);
													}
													// Reset to first page on sort change
													game_page.set(0);
												})}
											>
												{field.replace("_", " ").to_uppercase()}
												{if is_active {
													html! { <span class="ml-1">{if sort_asc { "▲" } else { "▼" }}</span> }
												} else { html! {} }}
											</button>
										}
									}).collect::<Html>()}
								</div>
								
								// Game performance table
								<div class="overflow-x-auto">
									<table class="min-w-full divide-y divide-gray-200">
										<thead class="bg-gray-50">
											<tr>
												<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Game"}</th>
												<th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Win Rate"}</th>
												<th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Total Plays"}</th>
												<th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Wins"}</th>
												<th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Best Placement"}</th>
												<th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">{"Avg Placement"}</th>
											</tr>
										</thead>
										<tbody class="bg-white divide-y divide-gray-200">
											{(|| {
												let mut sorted = games.clone();
												let sort_by = (*game_sort_by).clone();
												let asc = *game_sort_asc;
												// Sort by selected field
												sorted.sort_by(|a, b| {
													let ord = match sort_by.as_str() {
														"win_rate" => a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal),
														"total_plays" => a.total_plays.cmp(&b.total_plays),
														"wins" => a.wins.cmp(&b.wins),
														"best_placement" => b.best_placement.cmp(&a.best_placement), // Lower placement numbers are better
														"average_placement" => b.average_placement.partial_cmp(&a.average_placement).unwrap_or(std::cmp::Ordering::Equal), // Lower placement numbers are better
														_ => std::cmp::Ordering::Equal,
													};
													if asc { ord } else { ord.reverse() }
												});
												// Pagination slice
												let start = (*game_page as usize) * games_per_page;
												let end = std::cmp::min(start + games_per_page, sorted.len());
												let page_items = if start < end { &sorted[start..end] } else { &sorted[0..0] };
												page_items.iter().map(|game| {
													let game_name = game.game.name.clone();
													let game_id = game.game.id.clone();
													html! {
														<tr class="hover:bg-gray-50">
															<td class="px-3 py-2 text-sm">
																<button
																	class="font-medium text-blue-600 hover:text-blue-800 hover:underline cursor-pointer"
																	onclick={let navigator = navigator.clone(); let game_id = game_id.clone(); yew::Callback::from(move |_| {
																		navigator.push(&Route::GameHistory { game_id: game_id.clone() });
																	})}
																>
																	{game_name}
																</button>
															</td>
															<td class="px-3 py-2 text-sm text-center font-medium text-gray-700">
																{format!("{:.1}%", game.win_rate)}
															</td>
															<td class="px-3 py-2 text-sm text-center text-gray-700">
																{game.total_plays}
															</td>
															<td class="px-3 py-2 text-sm text-center text-gray-700">
																{game.wins}
															</td>
															<td class="px-3 py-2 text-sm text-center text-gray-700">
																{if game.best_placement > 0 { format!("#{}", game.best_placement) } else { "N/A".to_string() }}
															</td>
															<td class="px-3 py-2 text-sm text-center font-medium text-gray-700">
																{if game.average_placement > 0.0 { format!("{:.1}", game.average_placement) } else { "N/A".to_string() }}
															</td>
														</tr>
													}
												}).collect::<Html>()
										})()}
									</tbody>
									</table>
								</div>
								
								// Pagination controls
								{if total_pages > 1 {
									html! {
										<div class="flex items-center justify-between">
											<div class="text-sm text-gray-700">
												{"Page "}{*game_page + 1}{" of "}{total_pages}
											</div>
											<div class="flex space-x-2">
												<button
													class={classes!(
														"px-3", "py-1", "text-sm", "font-medium", "rounded-md",
														if *game_page == 0 {
															classes!("bg-gray-100", "text-gray-400", "cursor-not-allowed")
														} else {
															classes!("bg-blue-100", "text-blue-800", "hover:bg-blue-200")
														}
													)}
													disabled={*game_page == 0}
													onclick={let game_page = game_page.clone(); yew::Callback::from(move |_| {
														if *game_page > 0 {
															game_page.set(*game_page - 1);
														}
													})}
												>
													{"Previous"}
												</button>
												<button
													class={classes!(
														"px-3", "py-1", "text-sm", "font-medium", "rounded-md",
														if *game_page >= total_pages - 1 {
															classes!("bg-gray-100", "text-gray-400", "cursor-not-allowed")
														} else {
															classes!("bg-blue-100", "text-blue-800", "hover:bg-blue-200")
														}
													)}
													disabled={*game_page >= total_pages - 1}
													onclick={let game_page = game_page.clone(); yew::Callback::from(move |_| {
														game_page.set(*game_page + 1);
													})}
												>
													{"Next"}
												</button>
											</div>
										</div>
									}
								} else { html! {} }}
							</div>
						}
					}
				} else {
					html! {
						<div class="text-center py-8 text-gray-500">
							<p>{"Loading game performance data..."}</p>
						</div>
					}
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
