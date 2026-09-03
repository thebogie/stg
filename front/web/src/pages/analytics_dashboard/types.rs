use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct GameRecommendation {
    pub game_id: String,
    pub game_name: String,
    pub reason: String,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct VenuePerformance {
    pub venue_id: String,
    pub venue_name: String,
    pub total_contests: u64,
    pub win_rate: f64,
}

#[derive(Properties, PartialEq, Clone)]
pub struct AnalyticsDashboardProps {}

#[derive(Clone, PartialEq)]
pub(super) enum AnalyticsTab {
    Overview,
    Contests,
    Venues,
    Games,
    Players,
}
