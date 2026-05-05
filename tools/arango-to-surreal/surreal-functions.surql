OPTION IMPORT;

-- SurrealDB custom functions for contest lookups (v3-compatible).
-- Canonical location: tools/arango-to-surreal/surreal-functions.surql
-- Apply: ./scripts/apply-surreal-functions.sh (or paste in Surrealist with NS/DB set).
-- See docs/SURREALDB_FUNCTIONS.md. Edge convention: in = contest, out = venue | game | player.
-- v3: type::record (replaces type::thing); DEFINE FUNCTION uses { } not BLOCK { }.

-- contest_row(key) — one contest by string key (backend still does numeric-key fallback in Rust).
DEFINE FUNCTION fn::contest_row($key: string) {
	RETURN (SELECT * FROM contest WHERE id = type::record('contest', $key) LIMIT 1)[0];
};

-- contest_with_edges(key) — contest + venue_id, game_ids, outcomes in one round-trip.
DEFINE FUNCTION fn::contest_with_edges($key: string) {
	LET $contest = (SELECT * FROM contest WHERE id = type::record('contest', $key) LIMIT 1)[0];
	IF $contest == NONE THEN
		RETURN NONE;
	END;
	LET $cid = $contest.id;
	RETURN {
		contest: $contest,
		venue_id: (SELECT `out` FROM played_at WHERE `in` = $cid LIMIT 1)[0].out,
		game_ids: (SELECT VALUE `out` FROM played_with WHERE `in` = $cid),
		outcomes: (SELECT `out` AS player_id, place, result, score FROM resulted_in WHERE `in` = $cid ORDER BY place ASC)
	};
};

-- contest_venue(key) — venue row for a contest in one round-trip (replaces: played_at edge + venue by id).
DEFINE FUNCTION fn::contest_venue($key: string) {
	LET $row = (SELECT * FROM contest WHERE id = type::record('contest', $key) LIMIT 1)[0];
	IF $row == NONE THEN
		RETURN NONE;
	END;
	LET $edge = (SELECT `out` FROM played_at WHERE `in` = $row.id LIMIT 1)[0];
	IF $edge == NONE THEN
		RETURN NONE;
	END;
	RETURN (SELECT * FROM venue WHERE id = $edge.out LIMIT 1)[0];
};

-- contest_game(key) — game row for a contest in one round-trip (replaces: played_with edge + game by id).
DEFINE FUNCTION fn::contest_game($key: string) {
	LET $row = (SELECT * FROM contest WHERE id = type::record('contest', $key) LIMIT 1)[0];
	IF $row == NONE THEN
		RETURN NONE;
	END;
	LET $edge = (SELECT `out` FROM played_with WHERE `in` = $row.id LIMIT 1)[0];
	IF $edge == NONE THEN
		RETURN NONE;
	END;
	RETURN (SELECT * FROM game WHERE id = $edge.out LIMIT 1)[0];
};

-- player_contest_ids(player_key) — contest record ids for a player (one round-trip).
DEFINE FUNCTION fn::player_contest_ids($player_key: string) {
	RETURN (SELECT VALUE `in` FROM resulted_in WHERE `out` = type::record('player', $player_key));
};

-- contest_participants(key) — edges (player_id, place, result, points) for a contest; backend batches player lookup.
DEFINE FUNCTION fn::contest_participants($key: string) {
	LET $row = (SELECT id FROM contest WHERE id = type::record('contest', $key) LIMIT 1)[0];
	IF $row == NONE THEN
		RETURN [];
	END;
	RETURN (SELECT `out` AS player_id, place, result, points, score FROM resulted_in WHERE `in` = $row.id ORDER BY place ASC);
};

-- player_contests_with_edges(player_key, start_date, end_date) — contests with venue_id, game_id, outcomes. Pass "" for start_date/end_date for no filter.
DEFINE FUNCTION fn::player_contests_with_edges($player_key: string, $start_date: string, $end_date: string) {
	LET $cids = (SELECT VALUE `in` FROM resulted_in WHERE `out` = type::record('player', $player_key));
	IF array::len($cids) == 0 THEN
		RETURN [];
	END;
	RETURN (SELECT id AS contest_id, name, start, stop,
		(SELECT VALUE `out` FROM played_at WHERE `in` = id LIMIT 1)[0] AS venue_id,
		(SELECT VALUE `out` FROM played_with WHERE `in` = id LIMIT 1)[0] AS game_id,
		(SELECT `out` AS player_id, place, result, score FROM resulted_in WHERE `in` = id ORDER BY place ASC) AS outcomes
		FROM contest WHERE id INSIDE $cids
		AND ($start_date == "" OR start >= $start_date)
		AND ($end_date == "" OR start <= $end_date)
		ORDER BY start DESC);
};

-- venue_with_contest_ids(key) — venue row plus list of contest ids at that venue.
DEFINE FUNCTION fn::venue_with_contest_ids($key: string) {
	LET $v = (SELECT * FROM venue WHERE id = type::record('venue', $key) LIMIT 1)[0];
	IF $v == NONE THEN
		RETURN NONE;
	END;
	RETURN {
		venue: $v,
		contest_ids: (SELECT VALUE `in` FROM played_at WHERE `out` = $v.id)
	};
};

-- game_with_contest_ids(key) — game row plus list of contest ids for that game.
DEFINE FUNCTION fn::game_with_contest_ids($key: string) {
	LET $g = (SELECT * FROM game WHERE id = type::record('game', $key) LIMIT 1)[0];
	IF $g == NONE THEN
		RETURN NONE;
	END;
	RETURN {
		game: $g,
		contest_ids: (SELECT VALUE `in` FROM played_with WHERE `out` = $g.id)
	};
};

-- player_game_performance_data(player_key) — one round-trip: resulted_in (contest_id, place), contest (id, start), played_with (contest_id, game_id). Backend aggregates to GamePerformanceDto.
DEFINE FUNCTION fn::player_game_performance_data($player_key: string) {
	LET $cids = (SELECT VALUE `in` FROM resulted_in WHERE `out` = type::record('player', $player_key));
	IF array::len($cids) == 0 THEN
		RETURN { resulted_in: [], contest_starts: [], played_with: [] };
	END;
	RETURN {
		resulted_in: (SELECT `in` AS contest_id, place FROM resulted_in WHERE `out` = type::record('player', $player_key)),
		contest_starts: (SELECT id, start FROM contest WHERE id INSIDE $cids),
		played_with: (SELECT `in` AS contest_id, `out` AS game_id FROM played_with WHERE `in` INSIDE $cids)
	};
};

-- daily_active_players_data(days) — one round-trip: contest_days (id, day), resulted_in (contest_id, player_id). Backend groups by day and counts distinct players.
DEFINE FUNCTION fn::daily_active_players_data($days: int) {
	LET $cids = (SELECT id FROM contest WHERE start >= time::now() - duration::from_days($days));
	RETURN {
		contest_days: (SELECT id, time::format(start, '%Y-%m-%d') AS day FROM contest WHERE id INSIDE $cids),
		resulted_in: (SELECT `in` AS contest_id, `out` AS player_id FROM resulted_in WHERE `in` INSIDE $cids)
	};
};

-- player_stats_by_id_str(id_str) — one round-trip: aggregates by out and by in (backend picks non-zero set). Use when you have the exact id string from string::concat(id).
DEFINE FUNCTION fn::player_stats_by_id_str($id_str: string) {
	LET $co = (SELECT count() FROM resulted_in WHERE string::replace(string::concat(`out`), '`', '') = $id_str GROUP ALL)[0];
	LET $wo = (SELECT count() FROM resulted_in WHERE string::replace(string::concat(`out`), '`', '') = $id_str AND place = 1 GROUP ALL)[0];
	LET $ao = (SELECT math::mean(place) FROM resulted_in WHERE string::replace(string::concat(`out`), '`', '') = $id_str GROUP ALL)[0];
	LET $bo = (SELECT math::min(place) FROM resulted_in WHERE string::replace(string::concat(`out`), '`', '') = $id_str GROUP ALL)[0];
	LET $ci = (SELECT count() FROM resulted_in WHERE string::replace(string::concat(`in`), '`', '') = $id_str GROUP ALL)[0];
	LET $wi = (SELECT count() FROM resulted_in WHERE string::replace(string::concat(`in`), '`', '') = $id_str AND place = 1 GROUP ALL)[0];
	LET $ai = (SELECT math::mean(place) FROM resulted_in WHERE string::replace(string::concat(`in`), '`', '') = $id_str GROUP ALL)[0];
	LET $bi = (SELECT math::min(place) FROM resulted_in WHERE string::replace(string::concat(`in`), '`', '') = $id_str GROUP ALL)[0];
	RETURN { contests_out: $co, wins_out: $wo, avg_out: $ao, best_out: $bo, contests_in: $ci, wins_in: $wi, avg_in: $ai, best_in: $bi };
};

-- player_stats_by_key(key) — one round-trip: aggregates by out and by in using type::record('player', $key). Backend picks non-zero set.
DEFINE FUNCTION fn::player_stats_by_key($key: string) {
	LET $rid = type::record('player', $key);
	LET $co = (SELECT count() FROM resulted_in WHERE `out` = $rid GROUP ALL)[0];
	LET $wo = (SELECT count() FROM resulted_in WHERE `out` = $rid AND place = 1 GROUP ALL)[0];
	LET $ao = (SELECT math::mean(place) FROM resulted_in WHERE `out` = $rid GROUP ALL)[0];
	LET $bo = (SELECT math::min(place) FROM resulted_in WHERE `out` = $rid GROUP ALL)[0];
	LET $ci = (SELECT count() FROM resulted_in WHERE `in` = $rid GROUP ALL)[0];
	LET $wi = (SELECT count() FROM resulted_in WHERE `in` = $rid AND place = 1 GROUP ALL)[0];
	LET $ai = (SELECT math::mean(place) FROM resulted_in WHERE `in` = $rid GROUP ALL)[0];
	LET $bi = (SELECT math::min(place) FROM resulted_in WHERE `in` = $rid GROUP ALL)[0];
	RETURN { contests_out: $co, wins_out: $wo, avg_out: $ao, best_out: $bo, contests_in: $ci, wins_in: $wi, avg_in: $ai, best_in: $bi };
};

-- player_game_performance_detail_data(player_key) — one round-trip for Game Performance tab (best/toughest opponent, best venue).
-- Returns raw arrays; backend aggregates and resolves names.
DEFINE FUNCTION fn::player_game_performance_detail_data($player_key: string) {
	LET $cids = (SELECT VALUE `in` FROM resulted_in WHERE `out` = type::record('player', $player_key));
	IF array::len($cids) == 0 THEN
		RETURN { resulted_in: [], contest_starts: [], played_with: [], played_at: [], participants: [] };
	END;
	RETURN {
		resulted_in: (SELECT `in` AS contest_id, place FROM resulted_in WHERE `out` = type::record('player', $player_key)),
		contest_starts: (SELECT id, start FROM contest WHERE id INSIDE $cids),
		played_with: (SELECT `in` AS contest_id, `out` AS game_id FROM played_with WHERE `in` INSIDE $cids),
		played_at: (SELECT `in` AS contest_id, `out` AS venue_id FROM played_at WHERE `in` INSIDE $cids),
		participants: (SELECT `in` AS contest_id, `out` AS player_id, place, score FROM resulted_in WHERE `in` INSIDE $cids)
	};
};
