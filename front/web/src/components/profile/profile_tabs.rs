use crate::pages::profile::ProfileTab;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ProfileTabsProps {
    pub current_tab: ProfileTab,
    pub on_tab_click: Callback<ProfileTab>,
    pub show_settings: bool,
}

#[function_component(ProfileTabs)]
pub fn profile_tabs(props: &ProfileTabsProps) -> Html {
    let mut tabs = vec![
        (ProfileTab::OverallStats, "📊", "Overall Stats"),
        (ProfileTab::Ratings, "🎖️", "Ratings"),
        (ProfileTab::Achievements, "🏅", "Achievements"),
        (ProfileTab::Nemesis, "⚔️", "Nemesis"),
        (ProfileTab::Owned, "🎯", "Owned"),
        (ProfileTab::GamePerformance, "🎮", "Game Performance"),
        (ProfileTab::Trends, "📈", "Trends"),
        (ProfileTab::Comparison, "🧭", "Comparison"),
        (ProfileTab::Settings, "⚙️", "Settings"),
    ];
    if !props.show_settings {
        tabs.retain(|(tab, _, _)| *tab != ProfileTab::Settings);
    }

    let current_label = tabs
        .iter()
        .find(|(t, _, _)| *t == props.current_tab)
        .map(|(_, _, label)| *label)
        .unwrap_or("Select tab");

    html! {
        <div class="border-b border-gray-200">
            // Dropdown on all screen sizes (prevents any horizontal overflow)
            <div class="pb-3">
                <label class="sr-only" for="profile-tab-select">{"Profile tab"}</label>
                <div class="relative">
                    <select
                        id="profile-tab-select"
                        class="w-full appearance-none rounded-lg border border-gray-300 bg-white px-3 py-2 pr-10 text-sm font-medium text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        onchange={{
                            let tab_click = props.on_tab_click.clone();
                            Callback::from(move |e: Event| {
                                let input: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                let value = input.value();
                                let next = match value.as_str() {
                                    "OverallStats" => ProfileTab::OverallStats,
                                    "Ratings" => ProfileTab::Ratings,
                                    "Achievements" => ProfileTab::Achievements,
                                    "Nemesis" => ProfileTab::Nemesis,
                                    "Owned" => ProfileTab::Owned,
                                    "GamePerformance" => ProfileTab::GamePerformance,
                                    "Trends" => ProfileTab::Trends,
                                    "Comparison" => ProfileTab::Comparison,
                                    "Settings" => ProfileTab::Settings,
                                    _ => ProfileTab::OverallStats,
                                };
                                tab_click.emit(next);
                            })
                        }}
                    >
                        {tabs.iter().map(|(tab, _icon, label)| {
                            let value = match tab {
                                ProfileTab::OverallStats => "OverallStats",
                                ProfileTab::Ratings => "Ratings",
                                ProfileTab::Achievements => "Achievements",
                                ProfileTab::Nemesis => "Nemesis",
                                ProfileTab::Owned => "Owned",
                                ProfileTab::GamePerformance => "GamePerformance",
                                ProfileTab::Trends => "Trends",
                                ProfileTab::Comparison => "Comparison",
                                ProfileTab::Settings => "Settings",
                            };
                            html! { <option value={value} selected={props.current_tab == *tab}>{*label}</option> }
                        }).collect::<Html>()}
                    </select>
                    <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-3 text-gray-500">
                        {"▾"}
                    </div>
                </div>
                <div class="mt-1 text-xs text-gray-500">
                    {"Current: "}{current_label}
                </div>
            </div>
        </div>
    }
}
