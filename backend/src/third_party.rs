pub mod google;
pub mod bgg;

// Re-export commonly used services for convenience
pub use google::places::GooglePlacesService;
pub use bgg::games::BGGService; 