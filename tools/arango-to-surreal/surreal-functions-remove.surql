OPTION IMPORT;

-- Remove application functions so DEFINE in surreal-functions.surql is idempotent.
-- Run this before importing surreal-functions.surql (e.g. in apply-surreal-functions.sh).
-- Safe to run even when functions do not exist (script may ignore import errors).

REMOVE FUNCTION fn::contest_row;
REMOVE FUNCTION fn::contest_with_edges;
REMOVE FUNCTION fn::contest_venue;
REMOVE FUNCTION fn::contest_game;
REMOVE FUNCTION fn::player_contest_ids;
REMOVE FUNCTION fn::contest_participants;
REMOVE FUNCTION fn::player_contests_with_edges;
REMOVE FUNCTION fn::venue_with_contest_ids;
REMOVE FUNCTION fn::game_with_contest_ids;
REMOVE FUNCTION fn::player_game_performance_data;
REMOVE FUNCTION fn::daily_active_players_data;
REMOVE FUNCTION fn::player_stats_by_id_str;
REMOVE FUNCTION fn::player_stats_by_key;
REMOVE FUNCTION fn::player_game_performance_detail_data;
