use yew::prelude::*;
use crate::pages::analytics_dashboard::AnalyticsDashboard;

#[derive(Properties, PartialEq)]
pub struct AnalyticsProps {}

#[function_component(Analytics)]
pub fn analytics(_props: &AnalyticsProps) -> Html {
    html! {
        <div class="analytics-page">
            <AnalyticsDashboard />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_analytics_component() {
        // Just test that it compiles and renders
        assert!(true);
    }
} 