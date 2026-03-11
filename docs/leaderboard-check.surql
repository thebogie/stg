-- Leaderboard check: same queries as backend analytics repository
-- Run: source scripts/load-env.sh dev && ./scripts/run-surreal-script.sh docs/leaderboard-check.surql

SELECT count() AS resulted_in_count FROM resulted_in GROUP ALL;
SELECT `in` AS player_id, count() AS total_plays FROM resulted_in GROUP BY `in` LIMIT 5;
SELECT `in` AS player_id, count() AS wins FROM resulted_in WHERE place = 1 GROUP BY `in` LIMIT 5;
