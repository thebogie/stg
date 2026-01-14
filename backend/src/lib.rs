pub mod player;
pub mod venue;
pub mod game;
pub mod contest;
pub mod config;
pub mod error;
pub mod health;
pub mod middleware;
pub mod auth;
pub mod third_party;
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

    pub use cache::{AnalyticsCache, CacheKeys, CacheTTL, CacheStats};
    pub use controller::AnalyticsController;
    pub use engine::AnalyticsEngine;
    pub use repository::AnalyticsRepository;
    pub use usecase::AnalyticsUseCase;
    pub use visualization::{AnalyticsVisualization, Chart, ChartConfig, ChartType, DataPoint, ChartSeries, ChartData, ExportOptions, ChartFormat};
}

pub mod client_analytics {
    pub mod controller;
    pub mod usecase;
    pub mod repository;
    
    pub use controller::ClientAnalyticsController;
    pub use usecase::{ClientAnalyticsUseCase, ClientAnalyticsUseCaseImpl};
    pub use repository::{ClientAnalyticsRepository, ClientAnalyticsRepositoryImpl};
}

pub mod ratings {
    pub mod glicko;
    pub mod repository;
    pub mod usecase;
    pub mod controller;
    pub mod scheduler;
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
