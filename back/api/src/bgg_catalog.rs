//! BGG reference data loaded from `boardgames_ranks.csv` into table `bgg_catalog`.
//! Import via `import_bgg_catalog` binary; search uses Surreal (see `game` repository).

pub mod import;
pub mod import_cli;
pub mod repository;
