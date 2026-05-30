OPTION IMPORT;

-- Legacy resulted_in rows (Arango import) store numeric scores in `points`; API/UI read `score`.
-- Redefine contest_with_edges so outcomes expose coalesced score without a data backfill.

REMOVE FUNCTION IF EXISTS fn::contest_with_edges;

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
		outcomes: (SELECT `out` AS player_id, place, result,
			IF score != NONE AND string::trim(<string>score) != "" THEN <string>score
			ELSE IF points != NONE THEN <string>points
			ELSE "" END AS score
			FROM resulted_in WHERE `in` = $cid ORDER BY place ASC)
	};
};
