use yew::prelude::*;
use shared::dto::analytics::HeadToHeadRecordDto;

#[derive(Properties, PartialEq, Clone)]
pub struct HeadToHeadModalProps {
    pub record: Option<HeadToHeadRecordDto>,
    pub opponent_handle: String,
    pub opponent_name: String,
    pub loading: bool,
    pub error: Option<String>,
    pub on_close: Callback<MouseEvent>,
}

#[function_component(HeadToHeadModal)]
pub fn head_to_head_modal(props: &HeadToHeadModalProps) -> Html {
    let record = props.record.clone();
    
    html! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
            <div class="bg-white rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-auto p-4">
                <div class="flex items-start justify-between mb-3">
                    <div>
                        <h3 class="text-lg font-semibold">{"Head-to-Head vs "}{&props.opponent_name}</h3>
                        <p class="text-sm text-gray-600">{"@"}{&props.opponent_handle}</p>
                    </div>
                    <button class="text-gray-500 hover:text-gray-700" onclick={props.on_close.clone()}>
                        <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>
                
                if props.loading {
                    <div class="p-6 text-center text-gray-600">{"Loading..."}</div>
                } else if let Some(err) = &props.error {
                    <div class="p-4 text-red-600 bg-red-50 rounded">{err.clone()}</div>
                } else if let Some(rec) = record.as_ref() {
                    <div class="grid grid-cols-2 gap-4 mb-4">
                        <div class="p-3 bg-gray-50 rounded">
                            <div class="text-sm text-gray-600">{"Total Contests"}</div>
                            <div class="text-xl font-semibold">{rec.total_contests}</div>
                        </div>
                        <div class="p-3 bg-gray-50 rounded">
                            <div class="text-sm text-gray-600">{"My Wins"}</div>
                            <div class="text-xl font-semibold">{rec.my_wins}</div>
                        </div>
                        <div class="p-3 bg-gray-50 rounded">
                            <div class="text-sm text-gray-600">{"Their Wins"}</div>
                            <div class="text-xl font-semibold">{rec.opponent_wins}</div>
                        </div>
                        <div class="p-3 bg-gray-50 rounded">
                            <div class="text-sm text-gray-600">{"My Win Rate"}</div>
                            <div class="text-xl font-semibold">{format!("{:.1}%", rec.my_win_rate)}</div>
                        </div>
                    </div>
                    
                    <div class="table-container overflow-x-auto">
                        <table class="game-performance-table w-full divide-y divide-gray-200">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Date"}</th>
                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Contest"}</th>
                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Game"}</th>
                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Venue"}</th>
                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"My Place"}</th>
                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Their Place"}</th>
                                    <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Result"}</th>
                                </tr>
                            </thead>
                            <tbody class="bg-white divide-y divide-gray-200">
                                {rec.contest_history.iter().map(|c| {
                                    let date_str = c.contest_date.format("%Y-%m-%d").to_string();
                                    html! {
                                        <tr>
                                            <td class="px-3 py-2 text-sm text-gray-700">{date_str}</td>
                                            <td class="px-3 py-2 text-sm text-gray-700">{&c.contest_name}</td>
                                            <td class="px-3 py-2 text-sm text-gray-700">{&c.game_name}</td>
                                            <td class="px-3 py-2 text-sm text-gray-700">{&c.venue_name}</td>
                                            <td class="px-3 py-2 text-sm text-gray-700">{c.my_placement}</td>
                                            <td class="px-3 py-2 text-sm text-gray-700">{c.opponent_placement}</td>
                                            <td class="px-3 py-2 text-sm text-gray-700">{if c.i_won { "Won" } else { "Lost" }}</td>
                                        </tr>
                                    }
                                }).collect::<Html>()}
                            </tbody>
                        </table>
                    </div>
                } else {
                    <div class="p-6 text-center text-gray-600">{"No data available."}</div>
                }
            </div>
        </div>
    }
}
