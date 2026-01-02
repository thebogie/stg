use yew::prelude::*;
use shared::dto::analytics::HeadToHeadRecordDto;

#[derive(Properties, PartialEq)]
pub struct OwnedTabProps {
    pub opponents_i_beat: Option<Vec<HeadToHeadRecordDto>>,
    pub on_open_contest_modal: Callback<(String, String, String)>,
}

#[function_component(OwnedTab)]
pub fn owned_tab(props: &OwnedTabProps) -> Html {
    html! {
        <div class="space-y-6">
            <div class="bg-white rounded-lg shadow p-6">
                <h2 class="text-2xl font-bold text-gray-900 mb-4">{"💪 Players I Own"}</h2>
                <div class="mb-4">
                    <p class="text-gray-600">
                        <strong>{"Players I Beat:"}</strong> {"These opponents struggle against you. Keep dominating and maintain your winning streak!"}
                    </p>
                </div>
                { if let Some(opponents) = &props.opponents_i_beat {
                    if opponents.is_empty() {
                        html! { <p class="text-gray-600 text-center py-8">{"No dominated opponents yet. Start winning some contests! 💪"}</p> }
                    } else {
                        html! {
                            <div class="overflow-x-auto rounded-lg shadow">
                                <table class="min-w-full divide-y divide-gray-200">
                                    <thead class="bg-gray-50">
                                        <tr>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"#"}</th>
                                            <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"Opponent"}</th>
                                            <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"My Wins vs Them"}</th>
                                            <th class="px-3 py-2 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"My Win Rate"}</th>
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
                                                    <td class="px-3 py-2 text-sm text-gray-700">{opponent.my_wins}</td>
                                                    <td class="px-3 py-2 text-sm text-gray-700">{format!("{:.1}%", opponent.my_win_rate)}</td>
                                                    <td class="px-3 py-2 text-sm text-gray-700">{opponent.total_contests}</td>
                                                </tr>
                                            }
                                        }).collect::<Html>()}
                                    </tbody>
                                </table>
                            </div>
                        }
                    }
                } else { html! { <p class="text-gray-600 text-center py-8">{"Loading domination data..."}</p> } } }
            </div>
        </div>
    }
}
