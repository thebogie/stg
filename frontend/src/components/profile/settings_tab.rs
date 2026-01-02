use yew::prelude::*;
use crate::components::profile_editor::ProfileEditor;

#[derive(Properties, PartialEq)]
pub struct SettingsTabProps {}

#[function_component(SettingsTab)]
pub fn settings_tab(_props: &SettingsTabProps) -> Html {
    let on_profile_update = Callback::from(|_| {
        // This callback can be used to refresh the profile data after updates
        // For now, we'll just log that an update occurred
        log::info!("Profile updated");
    });

    html! {
        <div class="space-y-6">
            <ProfileEditor on_update={on_profile_update} />
        </div>
    }
}
