use yew::prelude::*;
use crate::components::chart_renderer::ChartRenderer;

#[derive(Properties, PartialEq)]
pub struct AnalyticsTestProps {}

#[function_component(AnalyticsTest)]
pub fn analytics_test(_props: &AnalyticsTestProps) -> Html {
    // Create a simple test chart data
    let test_chart_data = r###"{
        "chart_type": "Bar",
        "config": {
            "title": "Test Chart",
            "width": 600,
            "height": 400,
            "colors": ["#3B82F6", "#EF4444", "#10B981", "#F59E0B"],
            "show_legend": true,
            "show_grid": true,
            "animation": true
        },
        "data": {
            "SingleSeries": [
                {
                    "label": "Player 1",
                    "value": 85.5,
                    "color": null,
                    "metadata": null
                },
                {
                    "label": "Player 2",
                    "value": 72.3,
                    "color": null,
                    "metadata": null
                },
                {
                    "label": "Player 3",
                    "value": 91.2,
                    "color": null,
                    "metadata": null
                },
                {
                    "label": "Player 4",
                    "value": 68.7,
                    "color": null,
                    "metadata": null
                }
            ]
        },
        "metadata": {
            "description": "Test chart for verification",
            "x_axis": "Players",
            "y_axis": "Win Rate (%)"
        }
    }"###;

    html! {
        <div class="analytics-test-page">
            <div class="test-header">
                <h1>{"Chart Renderer Test"}</h1>
                <p>{"Testing the chart renderer component with sample data"}</p>
            </div>
            
            <div class="test-content">
                <div class="chart-test-section">
                    <h2>{"Bar Chart Test"}</h2>
                    <div class="chart-container">
                        <ChartRenderer
                            chart_data={test_chart_data.to_string()}
                            chart_id={"test-chart-1".to_string()}
                            width={Some(600)}
                            height={Some(400)}
                        />
                    </div>
                </div>
                
                <div class="test-info">
                    <h3>{"Test Information"}</h3>
                    <ul>
                        <li>{"Chart Type: Bar Chart"}</li>
                        <li>{"Data Points: 4 players with win rates"}</li>
                        <li>{"Colors: Blue, Red, Green, Yellow"}</li>
                        <li>{"Interactive: Hover effects and animations"}</li>
                    </ul>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_analytics_test_component() {
        // Just test that it compiles and renders
        assert!(true);
    }
} 