use crate::pages::profile::ProfileTab;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ProfileTabsProps {
    pub current_tab: ProfileTab,
    pub on_tab_click: Callback<ProfileTab>,
}

#[function_component(ProfileTabs)]
pub fn profile_tabs(props: &ProfileTabsProps) -> Html {
    let tabs = vec![
        (ProfileTab::OverallStats, "📊", "Overall Stats"),
        (ProfileTab::Ratings, "🎖️", "Ratings"),
        (ProfileTab::Nemesis, "⚔️", "Nemesis"),
        (ProfileTab::Owned, "🎯", "Owned"),
        (ProfileTab::GamePerformance, "🎮", "Game Performance"),
        (ProfileTab::Trends, "📈", "Trends"),
        (ProfileTab::Comparison, "🧭", "Comparison"),
        (ProfileTab::Settings, "⚙️", "Settings"),
    ];

    html! {
        <div class="border-b border-gray-200">
            <nav class="-mb-px flex space-x-8">
                {tabs.iter().map(|(tab, icon, label)| {
                    let is_active = props.current_tab == *tab;
                    let tab_click = props.on_tab_click.clone();

                    html! {
                        <button
                            class={classes!(
                                "py-2", "px-1", "border-b-2", "font-medium", "text-sm",
                                if is_active {
                                    classes!("border-blue-500", "text-blue-600")
                                } else {
                                    classes!("border-transparent", "text-gray-500", "hover:text-gray-700", "hover:border-gray-300")
                                }
                            )}
                            onclick={let tab = tab.clone(); yew::Callback::from(move |_| tab_click.emit(tab.clone()))}
                        >
                            <span class="mr-2">{icon}</span>
                            {label}
                        </button>
                    }
                }).collect::<Html>()}
            </nav>
        </div>
    }
}
