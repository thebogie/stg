//! Frontend hooks for shared data and behavior.
//!
//! Profile page uses `use_profile_data` as the single Tauri-standard source for profile + ratings
//! (see docs/PROFILE_PAGE_DESIGN.md).

pub mod use_profile_data;

pub use use_profile_data::use_profile_data;
pub use use_profile_data::ProfileDataHandles;
