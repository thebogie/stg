OPTION IMPORT;

-- Prod may still have legacy fn::contest_with_edges that omits score from outcomes.
-- Pass through resulted_in.score/points; backend coalesces via outcome_score_from_json.

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
		outcomes: (SELECT `out` AS player_id, place, result, points, score
			FROM resulted_in WHERE `in` = $cid ORDER BY place ASC)
	};
};
