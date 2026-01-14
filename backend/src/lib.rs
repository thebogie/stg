pub mod auth;
pub mod config;
pub mod contest;
pub mod error;
pub mod game;
pub mod health;
pub mod metrics;
pub mod middleware;
pub mod player;
pub mod third_party;
pub mod venue;
pub mod timezone {
    pub mod controller;
}
pub mod analytics {
    pub mod cache;
    pub mod controller;
    pub mod engine;
    pub mod repository;
    pub mod usecase;
    pub mod visualization;

    pub use cache::{AnalyticsCache, CacheKeys, CacheStats, CacheTTL};
    pub use controller::AnalyticsController;
    pub use engine::AnalyticsEngine;
    pub use repository::AnalyticsRepository;
    pub use usecase::AnalyticsUseCase;
    pub use visualization::{
        AnalyticsVisualization, Chart, ChartConfig, ChartData, ChartFormat, ChartSeries, ChartType,
        DataPoint, ExportOptions,
    };
}

pub mod client_analytics {
    pub mod controller;
    pub mod repository;
    pub mod usecase;

    pub use controller::ClientAnalyticsController;
    pub use repository::{ClientAnalyticsRepository, ClientAnalyticsRepositoryImpl};
    pub use usecase::{ClientAnalyticsUseCase, ClientAnalyticsUseCaseImpl};
}

pub mod ratings {
    pub mod controller;
    pub mod glicko;
    pub mod repository;
    pub mod scheduler;
    pub mod usecase;
}

pub mod migration {
    pub mod timezone_migration;
}

pub mod openapi;

// Unit test modules only
#[cfg(test)]
mod tests;

#[cfg(test)]
mod ratings_tests;

#[cfg(test)]
mod normalization_tests;

#[cfg(test)]
mod game_venue_history_tests;

#[cfg(test)]
mod utility_tests;

#[cfg(test)]
mod auth_tests;

#[cfg(test)]
mod error_tests;

#[cfg(test)]
mod config_tests;

#[cfg(test)]
mod game_tests;

#[cfg(test)]
mod contest_tests;

#[cfg(test)]
mod cache_tests;

#[cfg(test)]
mod visualization_tests;

#[cfg(test)]
mod analytics_tests;

#[cfg(test)]
mod player_tests;

#[cfg(test)]
mod game_controller_tests;

// Controller tests are in their respective modules
