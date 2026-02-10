use gloo_utils::document;
use web_sys::HtmlElement;
use yew::prelude::*;

/// Chart renderer component for displaying analytics charts
#[derive(Properties, PartialEq)]
pub struct ChartRendererProps {
    pub chart_data: String, // JSON string of chart data
    pub chart_id: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[function_component(ChartRenderer)]
pub fn chart_renderer(props: &ChartRendererProps) -> Html {
    let chart_container_ref = use_node_ref();
    let chart_data = props.chart_data.clone();
    let chart_id = props.chart_id.clone();
    let width = props.width.unwrap_or(800);
    let height = props.height.unwrap_or(500);

    {
        let chart_container_ref = chart_container_ref.clone();
        let chart_data = chart_data.clone();
        let chart_id = chart_id.clone();

        use_effect_with((chart_data, chart_id), move |(chart_data, chart_id)| {
            if let Some(container) = chart_container_ref.cast::<HtmlElement>() {
                // Clear previous chart
                container.set_inner_html("");

                // Create chart element with proper dimensions
                let chart_element = document().create_element("div").unwrap();
                chart_element.set_id(&format!("chart-{}", chart_id));
                chart_element
                    .set_attribute(
                        "style",
                        &format!(
                            "width: {}px; height: {}px; overflow: visible;",
                            width, height
                        ),
                    )
                    .unwrap();
                container.append_child(&chart_element).unwrap();

                // Parse chart data
                if let Ok(chart) = serde_json::from_str::<ChartData>(chart_data) {
                    render_chart(&chart, &chart_id);
                }
            }
            || ()
        });
    }

    html! {
        <div class="chart-container" ref={chart_container_ref}>
            <div class="chart-loading">
                {"Loading chart..."}
            </div>
        </div>
    }
}

/// Chart data structure for frontend
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ChartData {
    pub chart_type: String,
    pub config: ChartConfig,
    pub data: ChartDataContent,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ChartConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub colors: Vec<String>,
    pub show_legend: bool,
    pub show_grid: bool,
    pub animation: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ChartDataContent {
    #[serde(rename = "SingleSeries")]
    pub single_series: Option<Vec<DataPoint>>,
    #[serde(rename = "MultiSeries")]
    pub multi_series: Option<Vec<ChartSeries>>,
    #[serde(rename = "HeatmapData")]
    pub heatmap_data: Option<Vec<Vec<f64>>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct DataPoint {
    pub label: String,
    pub value: f64,
    pub color: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ChartSeries {
    pub name: String,
    pub data: Vec<DataPoint>,
    pub color: Option<String>,
}

/// Render chart using Chart.js
fn render_chart(chart_data: &ChartData, chart_id: &str) {
    // This would integrate with Chart.js or another charting library
    // For now, we'll create a simple HTML representation

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    if let Some(chart_element) = document.get_element_by_id(&format!("chart-{}", chart_id)) {
        let chart_html = generate_chart_html(chart_data);
        chart_element.set_inner_html(&chart_html);
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Generate HTML representation of the chart
fn generate_chart_html(chart_data: &ChartData) -> String {
    let _title = &chart_data.config.title;
    let chart_type = &chart_data.chart_type;

    match chart_type.as_str() {
        "Line" => generate_line_chart_html(chart_data),
        "Bar" => generate_bar_chart_html(chart_data),
        "GroupedBar" => generate_grouped_bar_chart_html(chart_data),
        "Pie" => generate_pie_chart_html(chart_data),
        "Doughnut" => generate_doughnut_chart_html(chart_data),
        "Scatter" => generate_scatter_chart_html(chart_data),
        "Radar" => generate_radar_chart_html(chart_data),
        "Heatmap" => generate_heatmap_chart_html(chart_data),
        _ => generate_generic_chart_html(chart_data),
    }
}

fn generate_line_chart_html(chart_data: &ChartData) -> String {
    let _title = &chart_data.config.title;
    let colors = &chart_data.config.colors;

    if let Some(data_points) = &chart_data.data.single_series {
        let _labels: Vec<String> = data_points.iter().map(|p| p.label.clone()).collect();
        let _values: Vec<f64> = data_points.iter().map(|p| p.value).collect();

        let title_html = if chart_data.config.show_legend {
            format!("<h3 class=\"chart-title\">{}</h3>", _title)
        } else {
            String::new()
        };

        let legend_html = if chart_data.config.show_legend {
            format!(
                "<div class=\"chart-legend\">{}</div>",
                generate_legend_html(data_points, colors)
            )
        } else {
            String::new()
        };

        format!(
            r#"
            <div class="chart-wrapper">
                {}
                <div class="chart-content">
                    <div class="line-chart">
                        <svg width="{}" height="{}" viewBox="0 0 {} {}">
                            <defs>
                                <linearGradient id="lineGradient" x1="0%" y1="0%" x2="0%" y2="100%">
                                    <stop offset="0%" style="stop-color:{};stop-opacity:0.8" />
                                    <stop offset="100%" style="stop-color:{};stop-opacity:0.2" />
                                </linearGradient>
                            </defs>
                            <g class="chart-area">
                                <path d="{}" fill="url(#lineGradient)" stroke="{}" stroke-width="2"/>
                            </g>
                        </svg>
                    </div>
                    {}
                </div>
            </div>
            "#,
            title_html,
            chart_data.config.width,
            chart_data.config.height,
            chart_data.config.width,
            chart_data.config.height,
            colors.get(0).unwrap_or(&"#3B82EB".to_string()),
            colors.get(0).unwrap_or(&"#3B82EB".to_string()),
            generate_line_path(
                data_points,
                chart_data.config.width,
                chart_data.config.height
            ),
            colors.get(0).unwrap_or(&"#3B82EB".to_string()),
            legend_html
        )
    } else if let Some(series_list) = &chart_data.data.multi_series {
        // Render multiple line series
        let paths_html: String = series_list.iter().enumerate().map(|(i, series)| {
            let default_color = "#3B82F6".to_string();
            let color = colors.get(i % colors.len()).unwrap_or(&default_color);
            let path = generate_line_path(&series.data, chart_data.config.width, chart_data.config.height);
            format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="2" class="line-series" data-series="{}"/>"#,
                path,
                color,
                escape_html(&series.name)
            )
        }).collect();

        let legend_html: String = series_list
            .iter()
            .enumerate()
            .map(|(i, series)| {
                let default_color = "#3B82F6".to_string();
                let color = colors.get(i % colors.len()).unwrap_or(&default_color);
                format!(
                    r#"
                <div class="legend-item">
                    <span class="legend-color" style="background-color: {}"></span>
                    <span class="legend-label">{}</span>
                </div>
                "#,
                    color,
                    escape_html(&series.name)
                )
            })
            .collect();

        // Add axis labels only (no grid lines to avoid SVG attribute issues)
        // Support dual Y-axis labels when provided
        let y_left = escape_html(
            chart_data
                .metadata
                .get("y_axis_left")
                .or_else(|| chart_data.metadata.get("y_axis"))
                .unwrap_or(&"Y Axis".to_string()),
        );
        let y_right_opt = chart_data.metadata.get("y_axis_right").cloned();
        let axis_html = if let Some(y_right) = y_right_opt {
            let y_right_esc = escape_html(&y_right);
            format!(
                "<g class=\"chart-axes\">\
                    <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" class=\"x-axis-label\">{}</text>\
                    <text x=\"20\" y=\"{}\" text-anchor=\"middle\" transform=\"rotate(-90, 20, {})\" class=\"y-axis-label-left\">{}</text>\
                    <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" transform=\"rotate(90, {}, {})\" class=\"y-axis-label-right\">{}</text>\
                </g>",
                chart_data.config.width / 2,
                chart_data.config.height - 25,
                escape_html(chart_data.metadata.get("x_axis").unwrap_or(&"X Axis".to_string())),
                chart_data.config.height / 2,
                chart_data.config.height / 2,
                y_left,
                chart_data.config.width - 20,
                chart_data.config.height / 2,
                chart_data.config.width - 20,
                chart_data.config.height / 2,
                y_right_esc
            )
        } else {
            format!(
                "<g class=\"chart-axes\">\
                    <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" class=\"x-axis-label\">{}</text>\
                    <text x=\"20\" y=\"{}\" text-anchor=\"middle\" transform=\"rotate(-90, 20, {})\" class=\"y-axis-label\">{}</text>\
                </g>",
                chart_data.config.width / 2,
                chart_data.config.height - 25,
                escape_html(chart_data.metadata.get("x_axis").unwrap_or(&"X Axis".to_string())),
                chart_data.config.height / 2,
                chart_data.config.height / 2,
                y_left
            )
        };

        // Get additional metadata for better chart description
        let description = escape_html(
            chart_data
                .metadata
                .get("description")
                .unwrap_or(&"Chart data".to_string()),
        );
        let insight = escape_html(
            chart_data
                .metadata
                .get("insight")
                .unwrap_or(&"".to_string()),
        );

        format!(
            r#"
            <div class="chart-wrapper">
                <h3 class="chart-title">{}</h3>
                <div class="chart-description">{}</div>
                <div class="chart-content">
                    <div class="line-chart">
                        <svg width="{}" height="{}" viewBox="0 0 {} {}">
                            <g class="chart-area">
                                {}
                                {}
                            </g>
                        </svg>
                    </div>
                    <div class="chart-legend">
                        {}
                    </div>
                </div>
                {}
            </div>
            "#,
            escape_html(&_title),
            description,
            chart_data.config.width,
            chart_data.config.height,
            chart_data.config.width,
            chart_data.config.height,
            axis_html,
            paths_html,
            legend_html,
            if !insight.is_empty() {
                format!(
                    "<div class=\"chart-insight\"><strong>💡 Insight:</strong> {}</div>",
                    insight
                )
            } else {
                "".to_string()
            }
        )
    } else {
        format!("<div class='chart-error'>No data available for line chart</div>")
    }
}

fn generate_bar_chart_html(chart_data: &ChartData) -> String {
    let title = &escape_html(&chart_data.config.title);
    let colors = &chart_data.config.colors;

    if let Some(data_points) = &chart_data.data.single_series {
        let max_value = data_points.iter().map(|p| p.value).fold(0.0, f64::max);
        let bar_width = chart_data.config.width as f64 / data_points.len() as f64 * 0.8;
        let bar_spacing = chart_data.config.width as f64 / data_points.len() as f64 * 0.2;

        let bars_html: String = data_points
            .iter()
            .enumerate()
            .map(|(i, point)| {
                let x = i as f64 * (bar_width + bar_spacing);
                let height = (point.value / max_value) * (chart_data.config.height as f64 * 0.7);
                let y = chart_data.config.height as f64 - height - 80.0; // 80px for labels and axis
                let default_color = "#3B82F6".to_string();
                let color = colors.get(i % colors.len()).unwrap_or(&default_color);

                format!(
                    r#"
                <g class="bar-group">
                    <rect x="{}" y="{}" width="{}" height="{}" fill="{}" class="bar"/>
                    <text x="{}" y="{}" text-anchor="middle" class="bar-label">{}</text>
                    <text x="{}" y="{}" text-anchor="middle" class="bar-value">{:.1}</text>
                </g>
                "#,
                    x,
                    y,
                    bar_width,
                    height,
                    color,
                    x + bar_width / 2.0,
                    chart_data.config.height as f64 - 30.0,
                    escape_html(&point.label),
                    x + bar_width / 2.0,
                    y - 5.0,
                    point.value
                )
            })
            .collect();

        // Add axis labels and horizontal baseline
        let axis_html = format!(
            "<g class=\"chart-axes\">\
                <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" class=\"x-axis-label\">{}</text>\
                <text x=\"20\" y=\"{}\" text-anchor=\"middle\" transform=\"rotate(-90, 20, {})\" class=\"y-axis-label\">{}</text>\
                <line x1=\"50\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e5e7eb\" stroke-width=\"1\"/>\
            </g>",
            chart_data.config.width / 2,
            chart_data.config.height - 25,
            escape_html(chart_data.metadata.get("x_axis").unwrap_or(&"Players".to_string())),
            chart_data.config.height / 2,
            chart_data.config.height / 2,
            escape_html(chart_data.metadata.get("y_axis").unwrap_or(&"Win Rate Value".to_string())),
            chart_data.config.height - 80,
            chart_data.config.width - 50,
            chart_data.config.height - 80
        );

        // Get additional metadata for better chart description
        let description = escape_html(
            chart_data
                .metadata
                .get("description")
                .unwrap_or(&"Chart data".to_string()),
        );
        let insight = escape_html(
            chart_data
                .metadata
                .get("insight")
                .unwrap_or(&"".to_string()),
        );

        format!(
            r#"
            <div class="chart-wrapper">
                <h3 class="chart-title">{}</h3>
                <div class="chart-description">{}</div>
                <div class="chart-content">
                    <svg width="{}" height="{}" viewBox="0 0 {} {}">
                        <g class="chart-area">
                            {}
                            {}
                        </g>
                    </svg>
                    {}
                </div>
            </div>
            "#,
            title,
            description,
            chart_data.config.width,
            chart_data.config.height,
            chart_data.config.width,
            chart_data.config.height,
            axis_html,
            bars_html,
            if !insight.is_empty() {
                format!(
                    "<div class=\"chart-insight\"><strong>💡 Insight:</strong> {}</div>",
                    insight
                )
            } else {
                "".to_string()
            }
        )
    } else {
        format!("<div class='chart-error'>No data available for bar chart</div>")
    }
}

fn generate_grouped_bar_chart_html(chart_data: &ChartData) -> String {
    let title = &escape_html(&chart_data.config.title);
    let colors = &chart_data.config.colors;

    if let Some(series_list) = &chart_data.data.multi_series {
        // Calculate dimensions for stacked bars
        let bar_width = 80; // Width of each bar
        let bar_spacing = 60; // Space between bars

        let mut bars_html = String::new();
        let mut legend_html = String::new();

        // Find the maximum value for proper scaling
        let max_value = series_list
            .iter()
            .flat_map(|series| series.data.iter())
            .map(|dp| dp.value)
            .fold(0.0, f64::max);

        for (series_idx, series) in series_list.iter().enumerate() {
            if series.data.is_empty() {
                continue; // Skip empty series
            }

            let player_count = escape_html(&series.name); // e.g., "2 Players"
            let x = series_idx as f64 * (bar_width + bar_spacing) as f64;
            let mut current_y = chart_data.config.height as f64 - 80.0; // Start from bottom

            // Add legend item for this player count
            legend_html.push_str(&format!(
                r#"
                <div class="legend-item">
                    <span class="legend-color" style="background-color: {}"></span>
                    <span class="legend-label">{}</span>
                </div>
                "#,
                colors
                    .get(series_idx % colors.len())
                    .unwrap_or(&"#3B82F6".to_string()),
                player_count
            ));

            // Generate stacked segments for this player count
            for (game_idx, data_point) in series.data.iter().enumerate() {
                let segment_height =
                    (data_point.value / max_value) * (chart_data.config.height as f64 * 0.6);
                let segment_y = current_y - segment_height;

                // Use different color for each game
                let default_color = "#3B82F6".to_string();
                let game_color = colors
                    .get(game_idx % colors.len())
                    .unwrap_or(&default_color);

                bars_html.push_str(&format!(
                    r#"
                    <g class="stacked-segment">
                        <rect x="{}" y="{}" width="{}" height="{}" fill="{}" class="bar-segment"/>
                        <text x="{}" y="{}" text-anchor="middle" class="game-name" font-size="10">{}</text>
                        <text x="{}" y="{}" text-anchor="middle" class="play-count" font-size="9">{}</text>
                    </g>
                    "#,
                    x, segment_y, bar_width, segment_height, game_color,
                    x + (bar_width as f64) / 2.0, segment_y + segment_height / 2.0, escape_html(&data_point.label),
                    x + (bar_width as f64) / 2.0, segment_y + segment_height + 15.0, data_point.value
                ));

                current_y = segment_y; // Move up for next segment
            }

            // Add player count label below the bar
            bars_html.push_str(&format!(
                r#"
                <text x="{}" y="{}" text-anchor="middle" class="player-count-label" font-size="12" font-weight="bold">{}</text>
                "#,
                x + (bar_width as f64) / 2.0, chart_data.config.height as f64 - 20.0, player_count
            ));
        }

        // Add axis labels
        let axis_html = format!(
            "<g class=\"chart-axes\">\
                <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" class=\"x-axis-label\">{}</text>\
                <text x=\"20\" y=\"{}\" text-anchor=\"middle\" transform=\"rotate(-90, 20, {})\" class=\"y-axis-label\">{}</text>\
            </g>",
            chart_data.config.width / 2,
            chart_data.config.height - 25,
            escape_html(chart_data.metadata.get("x_axis").unwrap_or(&"Number of Players".to_string())),
            chart_data.config.height / 2,
            chart_data.config.height / 2,
            escape_html(chart_data.metadata.get("y_axis").unwrap_or(&"Times Played".to_string()))
        );

        let description = escape_html(
            chart_data
                .metadata
                .get("description")
                .unwrap_or(&"Chart data".to_string()),
        );
        let insight = escape_html(
            chart_data
                .metadata
                .get("insight")
                .unwrap_or(&"".to_string()),
        );

        format!(
            r#"
            <div class="chart-wrapper">
                <h3 class="chart-title">{}</h3>
                <div class="chart-description">{}</div>
                <div class="chart-content">
                    <svg width="{}" height="{}" viewBox="0 0 {} {}">
                        <g class="chart-area">
                            {}
                            {}
                        </g>
                    </svg>
                    <div class="chart-legend">
                        {}
                    </div>
                    {}
                </div>
            </div>
            "#,
            title,
            description,
            chart_data.config.width,
            chart_data.config.height,
            chart_data.config.width,
            chart_data.config.height,
            axis_html,
            bars_html,
            legend_html,
            if !insight.is_empty() {
                format!(
                    "<div class=\"chart-insight\"><strong>💡 Insight:</strong> {}</div>",
                    insight
                )
            } else {
                "".to_string()
            }
        )
    } else {
        format!("<div class='chart-error'>No data available for grouped bar chart</div>")
    }
}

fn generate_pie_chart_html(chart_data: &ChartData) -> String {
    let title = &chart_data.config.title;
    let colors = &chart_data.config.colors;

    if let Some(data_points) = &chart_data.data.single_series {
        let total: f64 = data_points.iter().map(|p| p.value).sum();
        let center_x = chart_data.config.width as f64 / 2.0;
        let center_y = chart_data.config.height as f64 / 2.0;
        let radius = (chart_data.config.width.min(chart_data.config.height) as f64 / 2.0) * 0.8;

        let mut current_angle = 0.0;
        let slices_html: String = data_points
            .iter()
            .enumerate()
            .map(|(i, point)| {
                let slice_angle = (point.value / total) * 2.0 * std::f64::consts::PI;
                let end_angle = current_angle + slice_angle;
                let default_color = "#3B82F6".to_string();
                let color = colors.get(i % colors.len()).unwrap_or(&default_color);

                let x2 = center_x + radius * end_angle.cos();
                let y2 = center_y + radius * end_angle.sin();

                let large_arc_flag = if slice_angle > std::f64::consts::PI {
                    1
                } else {
                    0
                };

                let path_data = if slice_angle > 0.0 {
                    format!(
                        "M {},{} A {},{} 0 {},1 {},{} L {},{} Z",
                        center_x,
                        center_y,
                        radius,
                        radius,
                        large_arc_flag,
                        x2,
                        y2,
                        center_x,
                        center_y
                    )
                } else {
                    format!("M {},{} L {},{} Z", center_x, center_y, center_x, center_y)
                };

                current_angle = end_angle;

                format!(
                    r#"
                <g class="pie-slice">
                    <path d="{}" fill="{}" class="slice"/>
                    <text x="{}" y="{}" text-anchor="middle" class="slice-label">{}</text>
                </g>
                "#,
                    path_data,
                    color,
                    center_x + (radius * 0.7) * (current_angle - slice_angle / 2.0).cos(),
                    center_y + (radius * 0.7) * (current_angle - slice_angle / 2.0).sin(),
                    escape_html(&point.label)
                )
            })
            .collect();

        format!(
            r#"
            <div class="chart-wrapper">
                <h3 class="chart-title">{}</h3>
                <div class="chart-content">
                    <svg width="{}" height="{}" viewBox="0 0 {} {}">
                        <g class="chart-area">
                            {}
                        </g>
                    </svg>
                    <div class="chart-legend">
                        {}
                    </div>
                </div>
            </div>
            "#,
            title,
            chart_data.config.width,
            chart_data.config.height,
            chart_data.config.width,
            chart_data.config.height,
            slices_html,
            generate_legend_html(data_points, colors)
        )
    } else {
        format!("<div class='chart-error'>No data available for pie chart</div>")
    }
}

fn generate_doughnut_chart_html(chart_data: &ChartData) -> String {
    // Similar to pie chart but with inner radius
    generate_pie_chart_html(chart_data)
}

fn generate_scatter_chart_html(chart_data: &ChartData) -> String {
    let title = &escape_html(&chart_data.config.title);
    let colors = &chart_data.config.colors;

    if let Some(data_points) = &chart_data.data.single_series {
        let max_value = data_points.iter().map(|p| p.value).fold(0.0, f64::max);
        let points_html: String = data_points
            .iter()
            .enumerate()
            .map(|(i, point)| {
                let x = (i as f64 / data_points.len() as f64) * chart_data.config.width as f64;
                let y = chart_data.config.height as f64
                    - (point.value / max_value) * (chart_data.config.height as f64 * 0.8)
                    - 50.0;
                let default_color = "#3B82F6".to_string();
                let color = point
                    .color
                    .as_ref()
                    .unwrap_or(colors.get(i % colors.len()).unwrap_or(&default_color));

                format!(
                    r#"
                <g class="scatter-point">
                    <circle cx="{}" cy="{}" r="5" fill="{}" class="point"/>
                    <text x="{}" y="{}" text-anchor="middle" class="point-label">{}</text>
                </g>
                "#,
                    x,
                    y,
                    color,
                    x,
                    y - 10.0,
                    escape_html(&point.label)
                )
            })
            .collect();

        format!(
            r#"
            <div class="chart-wrapper">
                <h3 class="chart-title">{}</h3>
                <div class="chart-content">
                    <svg width="{}" height="{}" viewBox="0 0 {} {}">
                        <g class="chart-area">
                            {}
                        </g>
                    </svg>
                </div>
            </div>
            "#,
            title,
            chart_data.config.width,
            chart_data.config.height,
            chart_data.config.width,
            chart_data.config.height,
            points_html
        )
    } else {
        format!("<div class='chart-error'>No data available for scatter chart</div>")
    }
}

fn generate_radar_chart_html(chart_data: &ChartData) -> String {
    let title = &escape_html(&chart_data.config.title);
    let colors = &chart_data.config.colors;

    if let Some(series_list) = &chart_data.data.multi_series {
        if series_list.is_empty() {
            return format!("<div class='chart-error'>No data available for radar chart</div>");
        }

        let axis_labels: Vec<String> = series_list[0]
            .data
            .iter()
            .map(|p| escape_html(&p.label))
            .collect();
        let axis_count = axis_labels.len();
        if axis_count == 0 {
            return format!("<div class='chart-error'>No data available for radar chart</div>");
        }

        let width = chart_data.config.width as f64;
        let height = chart_data.config.height as f64;
        let cx = width / 2.0;
        let cy = height / 2.0;
        let radius = (width.min(height) * 0.35).max(1.0);
        let angle_step = 2.0 * std::f64::consts::PI / axis_count as f64;

        let mut grid_html = String::new();
        for level in 1..=5 {
            let r = radius * (level as f64 / 5.0);
            let points: Vec<String> = (0..axis_count)
                .map(|i| {
                    let angle = -std::f64::consts::PI / 2.0 + i as f64 * angle_step;
                    format!("{},{}", cx + r * angle.cos(), cy + r * angle.sin())
                })
                .collect();
            grid_html.push_str(&format!(
                "<polygon points='{}' fill='none' stroke='#e5e7eb' stroke-width='1'/>",
                points.join(" ")
            ));
        }

        let axis_html: String = (0..axis_count)
            .map(|i| {
                let angle = -std::f64::consts::PI / 2.0 + i as f64 * angle_step;
                let x = cx + radius * angle.cos();
                let y = cy + radius * angle.sin();
                format!(
                    "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#e5e7eb' stroke-width='1'/>",
                    cx, cy, x, y
                )
            })
            .collect();

        let label_html: String = (0..axis_count)
            .map(|i| {
                let angle = -std::f64::consts::PI / 2.0 + i as f64 * angle_step;
                let x = cx + (radius * 1.15) * angle.cos();
                let y = cy + (radius * 1.15) * angle.sin();
                let anchor = if angle.cos() > 0.2 {
                    "start"
                } else if angle.cos() < -0.2 {
                    "end"
                } else {
                    "middle"
                };
                let tooltip = match axis_labels[i].as_str() {
                    "Win Rate" => "Share of contests you won.",
                    "Skill Rating" => "Normalized skill rating (Glicko2).",
                    "Total Contests" => "Normalized total contests played.",
                    "Best Placement" => "Best finish (1st = 100%).",
                    "Longest Streak" => "Best consecutive win streak.",
                    _ => "Metric detail.",
                };
                format!(
                    r#"<text x="{}" y="{}" text-anchor="{}" class="radar-axis-label"><title>{}</title>{}</text>"#,
                    x,
                    y,
                    anchor,
                    escape_html(tooltip),
                    axis_labels[i]
                )
            })
            .collect();

        let series_html: String = series_list
            .iter()
            .enumerate()
            .map(|(i, series)| {
                let default_color = "#3B82F6".to_string();
                let color = series
                    .color
                    .clone()
                    .or_else(|| colors.get(i % colors.len()).cloned())
                    .unwrap_or(default_color);
                let points: Vec<String> = series
                    .data
                    .iter()
                    .enumerate()
                    .map(|(idx, point)| {
                        let angle = -std::f64::consts::PI / 2.0 + idx as f64 * angle_step;
                        let value = point.value.max(0.0).min(100.0) / 100.0;
                        let r = radius * value;
                        format!("{},{}", cx + r * angle.cos(), cy + r * angle.sin())
                    })
                    .collect();
                let series_id = format!("series-{}", i);
                format!(
                    r#"<polygon points="{}" data-radar-series="{}" opacity="0.6" fill="{}" fill-opacity="0.2" stroke="{}" stroke-width="2"/>"#,
                    points.join(" "),
                    series_id,
                    color,
                    color
                )
            })
            .collect();

        let legend_html: String = series_list
            .iter()
            .enumerate()
            .map(|(i, series)| {
                let default_color = "#3B82F6".to_string();
                let color = series
                    .color
                    .clone()
                    .or_else(|| colors.get(i % colors.len()).cloned())
                    .unwrap_or(default_color);
                let series_id = format!("series-{}", i);
                format!(
                    r#"<div class="legend-item" data-radar-legend="{}" onmouseenter="document.querySelectorAll('[data-radar-series]').forEach(e=>e.setAttribute('opacity','0.15')); document.querySelectorAll('[data-radar-series=&quot;{}&quot;]').forEach(e=>e.setAttribute('opacity','0.9'));" onmouseleave="document.querySelectorAll('[data-radar-series]').forEach(e=>e.setAttribute('opacity','0.6'));"><span class="legend-color" style="background-color: {}"></span><span class="legend-label">{}</span></div>"#,
                    series_id,
                    series_id,
                    color,
                    escape_html(&series.name)
                )
            })
            .collect();

        format!(
            r#"
        <div class="chart-wrapper">
            <h3 class="chart-title">{}</h3>
            <div class="chart-content">
                <svg width="{}" height="{}" viewBox="0 0 {} {}">
                    <g class="chart-area">
                        {}
                        {}
                        {}
                        {}
                    </g>
                </svg>
                <div class="chart-legend">
                    {}
                </div>
            </div>
        </div>
        "#,
            title,
            chart_data.config.width,
            chart_data.config.height,
            chart_data.config.width,
            chart_data.config.height,
            grid_html,
            axis_html,
            series_html,
            label_html,
            legend_html
        )
    } else {
        format!("<div class='chart-error'>No data available for radar chart</div>")
    }
}

fn generate_heatmap_chart_html(chart_data: &ChartData) -> String {
    let title = &escape_html(&chart_data.config.title);
    let description = escape_html(
        chart_data
            .metadata
            .get("description")
            .unwrap_or(&"Chart data".to_string()),
    );
    let insight = escape_html(
        chart_data
            .metadata
            .get("insight")
            .unwrap_or(&"".to_string()),
    );

    if let Some(heatmap_data) = &chart_data.data.heatmap_data {
        let rows = heatmap_data.len();
        let cols = if rows > 0 { heatmap_data[0].len() } else { 0 };

        if rows > 0 && cols > 0 {
            let max_value: f64 = heatmap_data
                .iter()
                .flat_map(|row| row.iter())
                .fold(0.0, |acc, &val| acc.max(val));

            let cell_width = 80;
            let cell_height = 60;
            let total_width = cols * cell_width;
            let total_height = rows * cell_height;

            let cells_html: String = heatmap_data.iter().enumerate().map(|(row_idx, row)| {
                row.iter().enumerate().map(|(col_idx, &value)| {
                    let intensity = if max_value > 0.0 { (value / max_value).min(1.0) } else { 0.0 };
                    let color = format!("rgba(59, 130, 246, {})", intensity.max(0.1));
                    let x = col_idx * cell_width;
                    let y = row_idx * cell_height;

                    format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#e5e7eb\" stroke-width=\"1\" class=\"heatmap-cell\">\
                            <title>Value: {:.1}</title>\
                        </rect>",
                        x, y, cell_width - 1, cell_height - 1, color, value
                    )
                }).collect::<Vec<String>>().join("")
            }).collect();

            let labels_html = format!(
                r#"
                <g class="heatmap-labels">
                    <text x="{}" y="{}" text-anchor="middle" class="heatmap-label">Plays</text>
                    <text x="{}" y="{}" text-anchor="middle" class="heatmap-label">Popularity</text>
                    {}
                </g>
                "#,
                total_width / 2, 20,
                total_width / 2, total_height + 40,
                (0..rows).map(|i| format!(
                    r#"<text x="10" y="{}" text-anchor="end" class="heatmap-label" dominant-baseline="middle">Game {}</text>"#,
                    i * cell_height + cell_height / 2, i + 1
                )).collect::<Vec<String>>().join("")
            );

            format!(
                r#"
                <div class="chart-wrapper">
                    <h3 class="chart-title">{}</h3>
                    <div class="chart-description">{}</div>
                    <div class="chart-content">
                        <svg width="{}" height="{}" viewBox="0 0 {} {}">
                            <g class="heatmap-area">
                                {}
                                {}
                            </g>
                        </svg>
                        {}
                    </div>
                </div>
                "#,
                title,
                description,
                total_width + 40,
                total_height + 80,
                total_width + 40,
                total_height + 80,
                cells_html,
                labels_html,
                if !insight.is_empty() {
                    format!(
                        "<div class=\"chart-insight\"><strong>💡 Insight:</strong> {}</div>",
                        insight
                    )
                } else {
                    "".to_string()
                }
            )
        } else {
            format!("<div class='chart-error'>No heatmap data available</div>")
        }
    } else {
        format!("<div class='chart-error'>No heatmap data available</div>")
    }
}

fn generate_generic_chart_html(chart_data: &ChartData) -> String {
    let title = &chart_data.config.title;

    format!(
        r#"
        <div class="chart-wrapper">
            <h3 class="chart-title">{}</h3>
            <div class="chart-content">
                <div class="generic-chart-placeholder">
                    <p>Chart type '{}' visualization would be rendered here</p>
                    <p>Chart data: {}</p>
                </div>
            </div>
        </div>
        "#,
        title,
        chart_data.chart_type,
        serde_json::to_string_pretty(chart_data).unwrap_or_default()
    )
}

fn generate_line_path(data_points: &[DataPoint], width: u32, height: u32) -> String {
    if data_points.is_empty() {
        return String::new();
    }

    let max_value = data_points.iter().map(|p| p.value).fold(0.0, f64::max);
    let min_value = data_points
        .iter()
        .map(|p| p.value)
        .fold(f64::INFINITY, f64::min);
    let value_range = max_value - min_value;

    let points: Vec<String> = data_points
        .iter()
        .enumerate()
        .map(|(i, point)| {
            let x = (i as f64 / (data_points.len() - 1) as f64) * width as f64;
            let normalized_value = if value_range > 0.0 {
                (point.value - min_value) / value_range
            } else {
                0.5
            };
            let y = height as f64 - (normalized_value * (height as f64 * 0.8) + 50.0);
            format!("{},{}", x, y)
        })
        .collect();

    format!("M {}", points.join(" L "))
}

fn generate_legend_html(data_points: &[DataPoint], colors: &[String]) -> String {
    data_points
        .iter()
        .enumerate()
        .map(|(i, point)| {
            let default_color = "#3B82F6".to_string();
            let color = colors.get(i % colors.len()).unwrap_or(&default_color);
            format!(
                r#"
            <div class="legend-item">
                <span class="legend-color" style="background-color: {}"></span>
                <span class="legend-label">{}</span>
                <span class="legend-value">{:.1}</span>
            </div>
            "#,
                color, point.label, point.value
            )
        })
        .collect::<Vec<String>>()
        .join("")
}
