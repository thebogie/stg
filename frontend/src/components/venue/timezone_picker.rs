use yew::prelude::*;
use shared::timezone::get_timezone_abbreviation;

#[derive(Properties, PartialEq)]
pub struct TimezonePickerProps {
    pub selected_timezone: String,
    pub on_timezone_change: Callback<String>,
    #[prop_or_default]
    pub disabled: bool,
}

#[function_component(TimezonePicker)]
pub fn timezone_picker(props: &TimezonePickerProps) -> Html {
    let timezones = vec![
        ("UTC", "UTC"),
        ("America/New_York", "Eastern Time (ET)"),
        ("America/Chicago", "Central Time (CT)"),
        ("America/Denver", "Mountain Time (MT)"),
        ("America/Los_Angeles", "Pacific Time (PT)"),
        ("America/Phoenix", "Mountain Time - Arizona (MT)"),
        ("America/Anchorage", "Alaska Time (AKT)"),
        ("Pacific/Honolulu", "Hawaii Time (HT)"),
        ("Europe/London", "Greenwich Mean Time (GMT)"),
        ("Europe/Paris", "Central European Time (CET)"),
        ("Europe/Berlin", "Central European Time (CET)"),
        ("Asia/Tokyo", "Japan Standard Time (JST)"),
        ("Asia/Shanghai", "China Standard Time (CST)"),
        ("Asia/Kolkata", "India Standard Time (IST)"),
        ("Australia/Sydney", "Australian Eastern Time (AET)"),
        ("Australia/Perth", "Australian Western Time (AWT)"),
    ];

    let on_change = {
        let on_timezone_change = props.on_timezone_change.clone();
        Callback::from(move |event: Event| {
            let target = event.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
            let value = target.value();
            on_timezone_change.emit(value);
        })
    };

    html! {
        <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">
                {"Timezone"}
            </label>
            <select
                value={props.selected_timezone.clone()}
                onchange={on_change}
                disabled={props.disabled}
                class="block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 disabled:bg-gray-100 disabled:cursor-not-allowed"
            >
                {timezones.iter().map(|(tz, display)| {
                    let abbreviation = get_timezone_abbreviation(tz);
                    html! {
                        <option value={*tz} selected={*tz == props.selected_timezone}>
                            {format!("{} ({})", display, abbreviation)}
                        </option>
                    }
                }).collect::<Html>()}
            </select>
            <p class="text-xs text-gray-500">
                {"Select the timezone where this venue is located"}
            </p>
        </div>
    }
}
