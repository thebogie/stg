use yew::prelude::*;
use crate::pages::profile::ProfileTab;

#[derive(Properties, PartialEq, Clone)]
pub struct ProfileNavProps {
    pub current_tab: ProfileTab,
    pub on_tab_click: Callback<ProfileTab>,
}

#[function_component(ProfileNav)]
pub fn profile_nav(props: &ProfileNavProps) -> Html {
    html! {
        <div class="profile-nav">
            <button 
                class={classes!("nav-tab", if props.current_tab == ProfileTab::Ratings { "active" } else { "" })}
                onclick={props.on_tab_click.clone().reform(|_| ProfileTab::Ratings)}
            >
                {"🏆 Ratings"}
            </button>
            <button 
                class={classes!("nav-tab", if props.current_tab == ProfileTab::Nemesis { "active" } else { "" })}
                onclick={props.on_tab_click.clone().reform(|_| ProfileTab::Nemesis)}
            >
                {"😈 Nemesis"}
            </button>
            <button 
                class={classes!("nav-tab", if props.current_tab == ProfileTab::Owned { "active" } else { "" })}
                onclick={props.on_tab_click.clone().reform(|_| ProfileTab::Owned)}
            >
                {"💪 Owned"}
            </button>
            <button 
                class={classes!("nav-tab", if props.current_tab == ProfileTab::GamePerformance { "active" } else { "" })}
                onclick={props.on_tab_click.clone().reform(|_| ProfileTab::GamePerformance)}
            >
                {"🎮 Games"}
            </button>
            <button 
                class={classes!("nav-tab", if props.current_tab == ProfileTab::Trends { "active" } else { "" })}
                onclick={props.on_tab_click.clone().reform(|_| ProfileTab::Trends)}
            >
                {"📈 Trends"}
            </button>
        </div>
    }
}
