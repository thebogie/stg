use chrono::DateTime;
use chrono::FixedOffset;
use shared::contest_name::default_contest_name;

/// Generates a default contest name from game(s), start time, and venue timezone.
pub fn generate_contest_name(
    game_names: &[&str],
    start: DateTime<FixedOffset>,
    venue_timezone: &str,
) -> String {
    default_contest_name(game_names, start, venue_timezone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_generate_contest_name_includes_game_and_date() {
        let start = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 5, 28, 12, 0, 0)
            .unwrap();
        let name = generate_contest_name(&["Terraforming Mars"], start, "America/Chicago");
        assert!(name.contains("Terraforming Mars"));
        assert!(name.contains("—"));
    }
}
