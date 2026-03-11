use crate::pages::profile::ProfilePage;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PlayerProfileProps {
    pub player_id: String,
}

#[function_component(PlayerProfilePage)]
pub fn player_profile_page(props: &PlayerProfileProps) -> Html {
    html! {
        <ProfilePage player_id={Some(props.player_id.clone())} />
    }
}
