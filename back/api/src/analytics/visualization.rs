use serde::{Deserialize, Serialize};
use shared::dto::analytics::*;
use shared::Result;
use std::collections::HashMap;

/// Chart configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub colors: Vec<String>,
    pub show_legend: bool,
    pub show_grid: bool,
    pub animation: bool,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            title: "Analytics Chart".to_string(),
            width: 800,
            height: 500,
            colors: vec![
                "#3B82F6".to_string(), // Blue
                "#EF4444".to_string(), // Red
                "#10B981".to_string(), // Green
                "#F59E0B".to_string(), // Yellow
                "#8B5CF6".to_string(), // Purple
                "#06B6D4".to_string(), // Cyan
                "#F97316".to_string(), // Orange
                "#EC4899".to_string(), // Pink
            ],
            show_legend: true,
            show_grid: true,
            animation: true,
        }
    }
}

/// Chart data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub label: String,
    pub value: f64,
    pub color: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Chart series for multi-series charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSeries {
    pub name: String,
    pub data: Vec<DataPoint>,
    pub color: Option<String>,
}

/// Chart types supported by the visualization system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    Line,
    Bar,
    GroupedBar,
    Pie,
    Doughnut,
    Area,
    Scatter,
    Radar,
    Heatmap,
}

/// Complete chart definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    pub chart_type: ChartType,
    pub config: ChartConfig,
    pub data: ChartData,
    pub metadata: HashMap<String, String>,
}

/// Chart data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartData {
    SingleSeries(Vec<DataPoint>),
    MultiSeries(Vec<ChartSeries>),
    HeatmapData(Vec<Vec<f64>>),
}

/// Visualization service for generating charts
#[derive(Clone)]
pub struct AnalyticsVisualization;

impl AnalyticsVisualization {
    /// Creates a new visualization service
    pub fn new() -> Self {
        Self
    }

    /// Generate player performance trend chart
    pub fn player_performance_trend(
        &self,
        player_stats: &[PlayerStatsDto],
        config: Option<ChartConfig>,
    ) -> Result<Chart> {
        let config = config.unwrap_or_default();

        let data_points: Vec<DataPoint> = player_stats
            .iter()
            .map(|stats| DataPoint {
                label: stats.player_handle.clone(),
                value: stats.win_rate,
                color: None,
                metadata: Some(HashMap::from([
                    (
                        "total_contests".to_string(),
                        stats.total_contests.to_string(),
                    ),
                    ("total_wins".to_string(), stats.total_wins.to_string()),
                    ("skill_rating".to_string(), stats.skill_rating.to_string()),
                ])),
            })
            .collect();

        Ok(Chart {
            chart_type: ChartType::Line,
            config: ChartConfig {
                title: "Player Performance Trends".to_string(),
                ..config
            },
            data: ChartData::SingleSeries(data_points),
            metadata: HashMap::from([
                (
                    "description".to_string(),
                    "Win rate trends across players".to_string(),
                ),
                ("x_axis".to_string(), "Players".to_string()),
                ("y_axis".to_string(), "Win Rate (%)".to_string()),
            ]),
        })
    }

    /// Generate leaderboard visualization
    pub fn leaderboard_chart(
        &self,
        leaderboard: &LeaderboardResponse,
        config: Option<ChartConfig>,
    ) -> Result<Chart> {
        let config = config.unwrap_or_default();

        let data_points: Vec<DataPoint> = leaderboard
            .entries
            .iter()
            .take(10) // Top 10 players
            .enumerate()
            .map(|(index, entry)| DataPoint {
                label: entry.player_handle.clone(),
                value: entry.value,
                color: Some(
                    config
                        .colors
                        .get(index % config.colors.len())
                        .unwrap()
                        .clone(),
                ),
                metadata: Some(HashMap::from([
                    ("rank".to_string(), entry.rank.to_string()),
                    ("player_id".to_string(), entry.player_id.clone()),
                ])),
            })
            .collect();

        Ok(Chart {
            chart_type: ChartType::Bar,
            config: ChartConfig {
                title: "Win Rate Leaderboard".to_string(),
                ..config
            },
            data: ChartData::SingleSeries(data_points),
            metadata: HashMap::from([
                ("description".to_string(), format!("Top 10 players ranked by {}. Shows player performance and ranking in this category.", leaderboard.category.to_string())),
                ("x_axis".to_string(), "Player Names".to_string()),
                ("y_axis".to_string(), format!("{} Score", leaderboard.category.to_string())),
                ("insight".to_string(), "Use this to identify top performers and set benchmarks for other players.".to_string()),
            ]),
        })
    }

    /// Generate achievement distribution pie chart
    pub fn achievement_distribution(
        &self,
        achievements: &PlayerAchievementsDto,
        config: Option<ChartConfig>,
    ) -> Result<Chart> {
        let config = config.unwrap_or_default();

        let mut category_counts: HashMap<String, i32> = HashMap::new();
        for achievement in &achievements.achievements {
            let category = achievement.category.to_string();
            *category_counts.entry(category).or_insert(0) += 1;
        }

        let data_points: Vec<DataPoint> = category_counts
            .iter()
            .enumerate()
            .map(|(index, (category, count))| DataPoint {
                label: category.clone(),
                value: *count as f64,
                color: Some(
                    config
                        .colors
                        .get(index % config.colors.len())
                        .unwrap()
                        .clone(),
                ),
                metadata: Some(HashMap::from([
                    ("category".to_string(), category.clone()),
                    ("count".to_string(), count.to_string()),
                ])),
            })
            .collect();

        Ok(Chart {
            chart_type: ChartType::Pie,
            config: ChartConfig {
                title: "Achievement Distribution".to_string(),
                ..config
            },
            data: ChartData::SingleSeries(data_points),
            metadata: HashMap::from([
                (
                    "description".to_string(),
                    "Achievements by category".to_string(),
                ),
                (
                    "total_achievements".to_string(),
                    achievements.total_achievements.to_string(),
                ),
                (
                    "unlocked_achievements".to_string(),
                    achievements.unlocked_achievements.to_string(),
                ),
            ]),
        })
    }

    /// Generate contest trends bar chart
    pub fn contest_trends(
        &self,
        trends: &[MonthlyContestsDto],
        config: Option<ChartConfig>,
    ) -> Result<Chart> {
        let config = config.unwrap_or_default();

        let data_points: Vec<DataPoint> = trends
            .iter()
            .map(|trend| DataPoint {
                label: format!("{}-{:02}", trend.year, trend.month),
                value: trend.contests as f64,
                color: Some("#3B82F6".to_string()), // Blue bars
                metadata: Some(HashMap::from([
                    ("year".to_string(), trend.year.to_string()),
                    ("month".to_string(), trend.month.to_string()),
                ])),
            })
            .collect();

        Ok(Chart {
            chart_type: ChartType::Bar,
            config: ChartConfig {
                title: "Monthly Contest Activity".to_string(),
                ..config
            },
            data: ChartData::SingleSeries(data_points),
            metadata: HashMap::from([
                (
                    "description".to_string(),
                    "Monthly contest frequency showing activity patterns over time".to_string(),
                ),
                ("x_axis".to_string(), "Month".to_string()),
                ("y_axis".to_string(), "Number of Contests".to_string()),
                (
                    "insight".to_string(),
                    "Identify peak contest months and seasonal patterns for strategic planning."
                        .to_string(),
                ),
            ]),
        })
    }

    /// Generate platform statistics dashboard
    pub fn platform_stats_dashboard(
        &self,
        stats: &PlatformStatsDto,
        config: Option<ChartConfig>,
    ) -> Result<Vec<Chart>> {
        let config = config.unwrap_or_default();
        let mut charts = Vec::new();

        // Activity metrics chart
        let activity_data = vec![
            DataPoint {
                label: "Active (7d)".to_string(),
                value: stats.active_players_7d as f64,
                color: Some("#10B981".to_string()),
                metadata: None,
            },
            DataPoint {
                label: "Active (30d)".to_string(),
                value: stats.active_players_30d as f64,
                color: Some("#3B82F6".to_string()),
                metadata: None,
            },
            DataPoint {
                label: "Total Players".to_string(),
                value: stats.total_players as f64,
                color: Some("#8B5CF6".to_string()),
                metadata: None,
            },
        ];

        charts.push(Chart {
            chart_type: ChartType::Bar,
            config: ChartConfig {
                title: "Player Activity Metrics".to_string(),
                ..config.clone()
            },
            data: ChartData::SingleSeries(activity_data),
            metadata: HashMap::from([
                (
                    "description".to_string(),
                    "Player activity comparison".to_string(),
                ),
                ("x_axis".to_string(), "Metric".to_string()),
                ("y_axis".to_string(), "Number of Players".to_string()),
            ]),
        });

        // Contest activity chart
        let contest_data = vec![
            DataPoint {
                label: "Contests (30d)".to_string(),
                value: stats.contests_30d as f64,
                color: Some("#F59E0B".to_string()),
                metadata: None,
            },
            DataPoint {
                label: "Total Contests".to_string(),
                value: stats.total_contests as f64,
                color: Some("#EF4444".to_string()),
                metadata: None,
            },
        ];

        charts.push(Chart {
            chart_type: ChartType::Bar,
            config: ChartConfig {
                title: "Contest Activity".to_string(),
                ..config.clone()
            },
            data: ChartData::SingleSeries(contest_data),
            metadata: HashMap::from([
                (
                    "description".to_string(),
                    "Contest activity comparison".to_string(),
                ),
                ("x_axis".to_string(), "Metric".to_string()),
                ("y_axis".to_string(), "Number of Contests".to_string()),
            ]),
        });

        // Platform overview pie chart
        let overview_data = vec![
            DataPoint {
                label: "Players".to_string(),
                value: stats.total_players as f64,
                color: Some("#3B82F6".to_string()),
                metadata: None,
            },
            DataPoint {
                label: "Games".to_string(),
                value: stats.total_games as f64,
                color: Some("#10B981".to_string()),
                metadata: None,
            },
            DataPoint {
                label: "Venues".to_string(),
                value: stats.total_venues as f64,
                color: Some("#F59E0B".to_string()),
                metadata: None,
            },
        ];

        charts.push(Chart {
            chart_type: ChartType::Doughnut,
            config: ChartConfig {
                title: "Platform Overview".to_string(),
                ..config.clone()
            },
            data: ChartData::SingleSeries(overview_data),
            metadata: HashMap::from([
                (
                    "description".to_string(),
                    "Platform entity distribution".to_string(),
                ),
                (
                    "total_entities".to_string(),
                    (stats.total_players + stats.total_games + stats.total_venues).to_string(),
                ),
            ]),
        });

        // Top games chart
        let top_games_data: Vec<DataPoint> = stats
            .top_games
            .iter()
            .enumerate()
            .map(|(index, game)| DataPoint {
                label: game.game_name.clone(),
                value: game.plays as f64,
                color: Some(
                    config
                        .colors
                        .get(index % config.colors.len())
                        .unwrap()
                        .clone(),
                ),
                metadata: Some(HashMap::from([
                    ("game_id".to_string(), game.game_id.clone()),
                    (
                        "popularity_score".to_string(),
                        format!("{:.2}", game.popularity_score),
                    ),
                ])),
            })
            .collect();

        charts.push(Chart {
            chart_type: ChartType::Bar,
            config: ChartConfig {
                title: "Top Games by Plays".to_string(),
                ..config.clone()
            },
            data: ChartData::SingleSeries(top_games_data),
            metadata: HashMap::from([
                ("description".to_string(), "Most played games".to_string()),
                ("x_axis".to_string(), "Game".to_string()),
                ("y_axis".to_string(), "Plays".to_string()),
            ]),
        });

        // Top venues chart
        let top_venues_data: Vec<DataPoint> = stats
            .top_venues
            .iter()
            .enumerate()
            .map(|(index, venue)| DataPoint {
                label: venue.venue_name.clone(),
                value: venue.contests_held as f64,
                color: Some(
                    config
                        .colors
                        .get(index % config.colors.len())
                        .unwrap()
                        .clone(),
                ),
                metadata: Some(HashMap::from([
                    ("venue_id".to_string(), venue.venue_id.clone()),
                    (
                        "total_participants".to_string(),
                        venue.total_participants.to_string(),
                    ),
                ])),
            })
            .collect();

        charts.push(Chart {
            chart_type: ChartType::Bar,
            config: ChartConfig {
                title: "Top Venues by Contests".to_string(),
                ..config.clone()
            },
            data: ChartData::SingleSeries(top_venues_data),
            metadata: HashMap::from([
                ("description".to_string(), "Most active venues".to_string()),
                ("x_axis".to_string(), "Venue".to_string()),
                ("y_axis".to_string(), "Contests".to_string()),
            ]),
        });

        Ok(charts)
    }

    /// Generate player comparison radar chart
    pub fn player_comparison_radar(
        &self,
        players: &[PlayerStatsDto],
        config: Option<ChartConfig>,
    ) -> Result<Chart> {
        let config = config.unwrap_or_default();

        let mut series = Vec::new();

        for (index, player) in players.iter().take(5).enumerate() {
            // Compare top 5 players
            let best_placement_value = if player.best_placement > 0 {
                (1.0 / player.best_placement as f64) * 100.0
            } else {
                0.0
            };
            let data_points = vec![
                DataPoint {
                    label: "Win Rate".to_string(),
                    value: player.win_rate,
                    color: None,
                    metadata: None,
                },
                DataPoint {
                    label: "Skill Rating".to_string(),
                    value: player.skill_rating / 2000.0 * 100.0, // Normalize to 0-100
                    color: None,
                    metadata: None,
                },
                DataPoint {
                    label: "Total Contests".to_string(),
                    value: (player.total_contests as f64 / 100.0 * 100.0).min(100.0), // Normalize to 0-100
                    color: None,
                    metadata: None,
                },
                DataPoint {
                    label: "Best Placement".to_string(),
                    value: best_placement_value, // Invert so 1st = 100%, avoid division by zero
                    color: None,
                    metadata: None,
                },
                DataPoint {
                    label: "Longest Streak".to_string(),
                    value: (player.longest_streak as f64 / 10.0 * 100.0).min(100.0), // Normalize to 0-100
                    color: None,
                    metadata: None,
                },
            ];

            series.push(ChartSeries {
                name: player.player_handle.clone(),
                data: data_points,
                color: Some(
                    config
                        .colors
                        .get(index % config.colors.len())
                        .unwrap()
                        .clone(),
                ),
            });
        }

        Ok(Chart {
            chart_type: ChartType::Radar,
            config: ChartConfig {
                title: "Player Performance Comparison".to_string(),
                ..config
            },
            data: ChartData::MultiSeries(series),
            metadata: HashMap::from([
                (
                    "description".to_string(),
                    "Multi-dimensional player comparison".to_string(),
                ),
                (
                    "metrics".to_string(),
                    "Win Rate, Skill Rating, Total Contests, Best Placement, Longest Streak"
                        .to_string(),
                ),
            ]),
        })
    }

    /// Generate games by player count distribution chart with individual game breakdowns
    pub fn game_popularity_heatmap(
        &self,
        player_count_data: &[(i32, Vec<(String, i32)>)],
        config: Option<ChartConfig>,
    ) -> Result<Chart> {
        let config = config.unwrap_or_default();

        // Create a grouped bar chart showing player counts on X-axis and game counts on Y-axis
        // Each bar represents a player count, and we'll show the total games for that count

        let _data_points: Vec<DataPoint> = player_count_data
            .iter()
            .map(|(player_count, games)| {
                let total_games = games.iter().map(|(_, count)| *count).sum::<i32>();
                let label = if *player_count == 10 {
                    "10+ Players".to_string()
                } else {
                    format!("{} Players", player_count)
                };

                DataPoint {
                    label,
                    value: total_games as f64,
                    color: Some(match player_count {
                        2 => "#3B82F6".to_string(),  // Blue
                        3 => "#EF4444".to_string(),  // Red
                        4 => "#10B981".to_string(),  // Green
                        5 => "#F59E0B".to_string(),  // Yellow
                        6 => "#8B5CF6".to_string(),  // Purple
                        7 => "#06B6D4".to_string(),  // Cyan
                        8 => "#F97316".to_string(),  // Orange
                        9 => "#EC4899".to_string(),  // Pink
                        10 => "#059669".to_string(), // Emerald
                        _ => "#6B7280".to_string(),  // Gray
                    }),
                    metadata: Some({
                        let mut meta = HashMap::new();
                        meta.insert("player_count".to_string(), player_count.to_string());
                        meta.insert("total_games".to_string(), total_games.to_string());
                        meta.insert("unique_games".to_string(), games.len().to_string());

                        // Add top 3 games for this player count
                        let top_games: Vec<String> = games
                            .iter()
                            .take(3)
                            .map(|(game_name, count)| format!("{} ({}x)", game_name, count))
                            .collect();
                        meta.insert("top_games".to_string(), top_games.join(", "));

                        meta
                    }),
                }
            })
            .collect();

        // Create a multi-series chart where each series represents a player count
        let mut chart_series = Vec::new();

        for (player_count, games) in player_count_data {
            let series = ChartSeries {
                name: format!("{} Players", player_count),
                data: games
                    .into_iter()
                    .map(|(game_name, count)| DataPoint {
                        label: game_name.clone(),
                        value: *count as f64,
                        color: None, // Will be assigned by the chart renderer
                        metadata: Some(HashMap::from([
                            ("player_count".to_string(), player_count.to_string()),
                            ("game_name".to_string(), game_name.clone()),
                            ("count".to_string(), count.to_string()),
                        ])),
                    })
                    .collect(),
                color: None,
            };
            chart_series.push(series);
        }

        Ok(Chart {
            chart_type: ChartType::GroupedBar,
            config: ChartConfig {
                title: "Games by Player Count Distribution".to_string(),
                ..config
            },
            data: ChartData::MultiSeries(chart_series),
            metadata: HashMap::from([
                ("description".to_string(), "Individual games played at different group sizes, showing which games are most popular for each player count".to_string()),
                ("x_axis".to_string(), "Number of Players".to_string()),
                ("y_axis".to_string(), "Times Played".to_string()),
                ("insight".to_string(), "Use this to see which specific games dominate each group size and plan contests accordingly.".to_string()),
            ]),
        })
    }

    /// Generate contest difficulty vs excitement scatter plot
    pub fn contest_analysis_scatter(
        &self,
        contests: &[ContestStatsDto],
        config: Option<ChartConfig>,
    ) -> Result<Chart> {
        let config = config.unwrap_or_default();

        let data_points: Vec<DataPoint> = contests
            .iter()
            .map(|contest| DataPoint {
                label: contest.contest_name.clone(),
                value: contest.excitement_rating,
                color: Some(if contest.difficulty_rating > 7.0 {
                    "#EF4444".to_string() // Red for high difficulty
                } else if contest.difficulty_rating > 4.0 {
                    "#F59E0B".to_string() // Yellow for medium difficulty
                } else {
                    "#10B981".to_string() // Green for low difficulty
                }),
                metadata: Some(HashMap::from([
                    (
                        "difficulty".to_string(),
                        contest.difficulty_rating.to_string(),
                    ),
                    (
                        "excitement".to_string(),
                        contest.excitement_rating.to_string(),
                    ),
                    (
                        "participants".to_string(),
                        contest.participant_count.to_string(),
                    ),
                    (
                        "completion_rate".to_string(),
                        contest.completion_rate.to_string(),
                    ),
                ])),
            })
            .collect();

        Ok(Chart {
            chart_type: ChartType::Scatter,
            config: ChartConfig {
                title: "Contest Difficulty vs Excitement".to_string(),
                ..config
            },
            data: ChartData::SingleSeries(data_points),
            metadata: HashMap::from([
                (
                    "description".to_string(),
                    "Contest analysis scatter plot".to_string(),
                ),
                ("x_axis".to_string(), "Difficulty Rating".to_string()),
                ("y_axis".to_string(), "Excitement Rating".to_string()),
            ]),
        })
    }
}

/// Chart export formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartFormat {
    Json,
    Svg,
    Png,
    Html,
}

/// Chart export options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ChartFormat,
    pub include_data: bool,
    pub include_metadata: bool,
    pub theme: String,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ChartFormat::Json,
            include_data: true,
            include_metadata: true,
            theme: "default".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_config_default() {
        let config = ChartConfig::default();
        assert_eq!(config.title, "Analytics Chart");
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 500);
        assert!(!config.colors.is_empty());
    }

    #[test]
    fn test_data_point_creation() {
        let data_point = DataPoint {
            label: "Test".to_string(),
            value: 42.0,
            color: Some("#FF0000".to_string()),
            metadata: None,
        };
        assert_eq!(data_point.label, "Test");
        assert_eq!(data_point.value, 42.0);
    }

    #[test]
    fn test_visualization_service_creation() {
        let _viz = AnalyticsVisualization::new();
        assert!(true); // Just test that it compiles
    }
}
