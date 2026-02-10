use shared::dto::analytics::HeadToHeadRecordDto;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NemesisTabProps {
    pub opponents_who_beat_me: Option<Vec<HeadToHeadRecordDto>>,
    pub on_open_contest_modal: Callback<(String, String, String)>,
}

#[function_component(NemesisTab)]
pub fn nemesis_tab(props: &NemesisTabProps) -> Html {
    html! {
        <div class="space-y-6">
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="flex items-center justify-between mb-4">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-900">{"My Nemeses"}</h2>
                        <p class="mt-1 text-gray-600">
                            {"Opponents with the upper hand—study them to turn the tables."}
                        </p>
                    </div>
                    <div class="text-4xl">{"😈"}</div>
                </div>
                { if let Some(opponents) = &props.opponents_who_beat_me {
                    if opponents.is_empty() {
                        html! { <p class="text-gray-600 text-center py-8">{"No nemeses yet - you're undefeated! 🏆"}</p> }
                    } else {
                        html! {
                            <div class="overflow-x-auto rounded-lg shadow">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"#"}</th>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Opponent"}</th>
                                            <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"Their Wins vs Me"}</th>
                                            <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"Their Win Rate"}</th>
                                            <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"Total Contests"}</th>
                                        </tr>
                                    </thead>
                                    <tbody class="bg-white divide-y divide-gray-200">
                                        {opponents.iter().enumerate().map(|(index, opponent)| {
                                            let rank = index + 1;
                                            let opp_id = opponent.opponent_id.clone();
                                            let opp_handle = opponent.opponent_handle.clone();
                                            let opp_name = opponent.opponent_name.clone();
                                            html! {
                                                <tr class="hover:bg-gray-50 cursor-pointer" onclick={let opponent_data = (opp_id.clone(), opp_handle.clone(), opp_name.clone()); let callback = props.on_open_contest_modal.clone(); yew::Callback::from(move |_| callback.emit(opponent_data.clone()))}>
                                                    <td class="px-3 py-2 text-sm text-gray-900">{rank}</td>
                                                    <td class="px-3 py-2 text-sm text-gray-900">
                                                        <div>
                                                            <div class="font-medium">{opponent.opponent_name.clone()}</div>
                                                            <div class="text-gray-500">{"@"}{opponent.opponent_handle.clone()}</div>
                                                        </div>
                                                    </td>
                                                    <td class="px-3 py-2 text-right text-sm text-gray-700">{opponent.opponent_wins}</td>
                                                    <td class="px-3 py-2 text-right text-sm text-gray-700">{format!("{:.1}%", 100.0 - opponent.my_win_rate)}</td>
                                                    <td class="px-3 py-2 text-right text-sm text-gray-700">{opponent.total_contests}</td>
                                                </tr>
                                            }
                                        }).collect::<Html>()}
                                    </tbody>
                                </table>
                            </div>
                        }
                    }
                } else { html! { <p class="text-gray-600 text-center py-8">{"Loading nemesis data..."}</p> } } }
            </div>
        </div>
    }
}
