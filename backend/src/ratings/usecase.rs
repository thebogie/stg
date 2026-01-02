use arangors::{client::ClientExt, AqlQuery};
use chrono::{Datelike, Utc};
use shared::{Result, SharedError};
use shared::dto::ratings::{PlayerRatingDto, PlayerRatingHistoryPointDto, RatingScope, RatingLeaderboardEntryDto};
use serde_json::Value;

use super::glicko::{Glicko2Params, RatingState, OpponentSample, update_period, pre_period_inflate_rd};
use super::repository::RatingsRepository;

#[derive(Clone)]
pub struct RatingsUsecase<C: ClientExt> {
    repo: RatingsRepository<C>,
    params: Glicko2Params,
}

impl<C: ClientExt> RatingsUsecase<C> {
    pub fn new(repo: RatingsRepository<C>) -> Self {
        Self { repo, params: Glicko2Params::default() }
    }

    /// Recalculate all ratings from the beginning of time (2000) to build proper historical data
    pub async fn recalculate_all_historical_ratings(&self) -> Result<()> {
        log::info!("Starting complete historical ratings recalculation from 2000...");
        
        // Clear existing ratings to start fresh
        self.repo.clear_all_ratings().await?;
        
        // Get the earliest contest date from the database
        let earliest_contest = self.repo.get_earliest_contest_date().await?;
        log::info!("Earliest contest date found: {}", earliest_contest);
        
        // Parse the earliest date to get year and month
        let date_parts: Vec<&str> = earliest_contest.split('-').collect();
        if date_parts.len() < 2 {
            return Err(SharedError::BadRequest("Invalid earliest contest date format".into()));
        }
        
        let start_year = date_parts[0].parse::<i32>()
            .map_err(|_| SharedError::BadRequest("Invalid year in earliest contest date".into()))?;
        let start_month = date_parts[1].parse::<u32>()
            .map_err(|_| SharedError::BadRequest("Invalid month in earliest contest date".into()))?;
        
        log::info!("Starting historical recalculation from {}-{:02} (actual first database entry)", start_year, start_month);
        
        // Process each month from the actual first entry until now
        let now = Utc::now();
        let mut year = start_year;
        let mut month = start_month;
        
        while year < now.year() || (year == now.year() && month <= now.month()) {
            let period = format!("{:04}-{:02}", year, month);
            log::info!("Processing period: {}", period);
            
            match self.recompute_month_with_history(Some(period)).await {
                Ok(_) => log::info!("Successfully processed period {}-{:02}", year, month),
                Err(e) => {
                    log::error!("Failed to process period {}-{:02}: {}", year, month, e);
                    // Continue processing other months even if one fails
                }
            }
            
            // Move to next month
            if month == 12 {
                year += 1;
                month = 1;
            } else {
                month += 1;
            }
        }
        
        log::info!("Historical ratings recalculation completed!");
        Ok(())
    }

    /// Enhanced month recalculation that properly loads existing ratings
    pub async fn recompute_month_with_history(&self, period: Option<String>) -> Result<()> {
        // Determine previous month if None
        let (year, month) = if let Some(p) = period {
            let parts: Vec<_> = p.split('-').collect();
            if parts.len() != 2 { return Err(SharedError::BadRequest(format!("Invalid period: {}", p))); }
            (parts[0].parse::<i32>().map_err(|_| SharedError::BadRequest("invalid year".into()))?, parts[1].parse::<u32>().map_err(|_| SharedError::BadRequest("invalid month".into()))?)
        } else {
            let now = Utc::now();
            if now.month() == 1 { (now.year() - 1, 12) } else { (now.year(), now.month() - 1) }
        };
        let start = format!("{:04}-{:02}-01T00:00:00Z", year, month);
        let next_month = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        let end = format!("{:04}-{:02}-01T00:00:00Z", next_month.0, next_month.1);

        // Fetch contests in the month
        let contests = self.repo.get_contests_in_period(&start, &end).await?;
        if contests.is_empty() {
            log::info!("No contests found for period {}-{:02}; applying inactivity RD inflation for all players with latest ratings", year, month);
        } else {
            log::info!("Processing {} contests for period {}-{:02}", contests.len(), year, month);
        }

        // Load existing latest ratings per player/scope (THIS IS THE KEY FIX!)
        let mut latest: std::collections::HashMap<(String, RatingScope), RatingState> = std::collections::HashMap::new();
        
        // Get all unique players from the contests first
        let mut all_players = std::collections::HashSet::new();
        for c in contests.iter() {
            let cid = c.get("_id").and_then(|v| v.as_str()).ok_or(SharedError::Database("contest missing _id".into()))?;
            let players = self.repo.get_contest_players(cid).await?;
            for player in players {
                all_players.insert(player);
            }
        }
        
        // Also include all players that currently have a latest rating (to inflate RD when inactive)
        if let Ok(existing_latest_players) = self.repo.get_all_latest_player_ids("global", None).await {
            for pid in existing_latest_players {
                all_players.insert(pid);
            }
        }

        // Load existing ratings for all players in scope (contest participants plus anyone with a latest rating)
        let mut last_period_end_by_player: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for player_id in &all_players {
            if let Some(existing_rating) = self.repo.get_latest_rating("global", None, player_id).await? {
                let rating = existing_rating.get("rating").and_then(|x| x.as_f64()).unwrap_or(self.params.default_rating);
                let rd = existing_rating.get("rd").and_then(|x| x.as_f64()).unwrap_or(self.params.default_rd);
                let volatility = existing_rating.get("volatility").and_then(|x| x.as_f64()).unwrap_or(self.params.default_vol);
                if let Some(lpe) = existing_rating.get("last_period_end").and_then(|x| x.as_str()) {
                    last_period_end_by_player.insert(player_id.clone(), lpe.to_string());
                }
                
                latest.insert((player_id.clone(), RatingScope::Global), RatingState {
                    rating,
                    rd,
                    vol: volatility,
                });
            }
        }

        // Build samples per player for this month using REAL contest results
        let mut samples_by_player: std::collections::HashMap<String, Vec<OpponentSample>> = std::collections::HashMap::new();
        let mut games_played: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        let mut wins_by_player: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        let mut losses_by_player: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

        for c in contests.iter() {
            let cid = c.get("_id").and_then(|v| v.as_str()).ok_or(SharedError::Database("contest missing _id".into()))?;
            let contest_results = self.repo.get_contest_results(cid).await?;
            if contest_results.len() < 2 { continue; }
            
            let weight = 1.0; // Each contest has equal weight
            
            // Process actual contest results and track wins/losses
            for (player_id, player_place) in &contest_results {
                *games_played.entry(player_id.clone()).or_insert(0) += 1;
                
                // Track wins and losses based on placement
                if let Some(place) = player_place {
                    if *place == 1 {
                        *wins_by_player.entry(player_id.clone()).or_insert(0) += 1;
                    } else {
                        *losses_by_player.entry(player_id.clone()).or_insert(0) += 1;
                    }
                }
                
                // Compare this player against all other players
                for (opponent_id, opponent_place) in &contest_results {
                    if player_id == opponent_id { continue; }
                    
                    // Determine score based on placements
                    let score = match (player_place, opponent_place) {
                        (Some(p_place), Some(o_place)) => {
                            if p_place < o_place { 1.0 }
                            else if p_place > o_place { 0.0 }
                            else { 0.5 }
                        }
                        _ => 0.5,
                    };
                    
                    // Get opponent's current rating
                    let opp_state = *latest.entry((opponent_id.clone(), RatingScope::Global))
                        .or_insert(RatingState { 
                            rating: self.params.default_rating, 
                            rd: self.params.default_rd, 
                            vol: self.params.default_vol 
                        });
                    
                    samples_by_player.entry(player_id.clone()).or_default().push(OpponentSample {
                        opp_rating: opp_state.rating,
                        opp_rd: opp_state.rd,
                        score,
                        weight,
                    });
                }
            }
        }

        // Helper: months difference between two ISO dates (YYYY-MM-01T00:00:00Z)
        fn months_between(a: &str, b: &str) -> i32 {
            let parse = |s: &str| -> Option<(i32, i32)> {
                let date_part = s.split('T').next()?;
                let parts: Vec<&str> = date_part.split('-').collect();
                if parts.len() < 2 { return None; }
                let y = parts[0].parse::<i32>().ok()?;
                let m = parts[1].parse::<i32>().ok()?;
                Some((y, m))
            };
            match (parse(a), parse(b)) {
                (Some((y1, m1)), Some((y2, m2))) => (y2 - y1) * 12 + (m2 - m1),
                _ => 1,
            }
        }

        // Apply Glicko2 updates across all players in scope, inflating RD for those with no games
        for player in all_players.into_iter() {
            let key = (player.clone(), RatingScope::Global);
            let current_state = *latest.entry(key.clone()).or_insert(RatingState { 
                rating: self.params.default_rating, 
                rd: self.params.default_rd, 
                vol: self.params.default_vol 
            });

            let samples = samples_by_player.remove(&player).unwrap_or_default();
            let updated = if samples.is_empty() {
                // Player had no games this period - apply RD inflation by exact months inactive
                let t = if let Some(prev_end) = last_period_end_by_player.get(&player) {
                    let diff = months_between(prev_end, &end);
                    if diff <= 0 { 1 } else { diff }
                } else { 1 };
                let inflated = pre_period_inflate_rd(current_state, t as f64);
                RatingState { rd: inflated.rd.min(self.params.default_rd), ..inflated }
            } else {
                // Player had games - apply full Glicko2 update
                update_period(current_state, &samples, self.params)
            };
            latest.insert(key, updated);
        }

        // Persist latest and history docs (global scope)
        let period_end = end.clone();
        let now = Utc::now().to_rfc3339();
        for ((player_id, scope), state) in latest.into_iter() {
            if let RatingScope::Global = scope {
                let gp = *games_played.get(&player_id).unwrap_or(&0);
                let wins = *wins_by_player.get(&player_id).unwrap_or(&0);
                let losses = *losses_by_player.get(&player_id).unwrap_or(&0);
                let latest_doc = serde_json::json!({
                    "player_id": player_id,
                    "scope_type": "global",
                    "scope_id": serde_json::Value::Null,
                    "rating": state.rating,
                    "rd": state.rd,
                    "volatility": state.vol,
                    "games_played": gp,
                    "last_period_end": period_end,
                    "updated_at": now,
                });
                self.repo.upsert_latest_rating(latest_doc).await?;

                let history_doc = serde_json::json!({
                    "player_id": player_id,
                    "scope_type": "global",
                    "scope_id": serde_json::Value::Null,
                    "period_end": period_end,
                    "rating": state.rating,
                    "rd": state.rd,
                    "volatility": state.vol,
                    "period_games": gp,
                    "wins": wins,
                    "losses": losses,
                    "draws": 0, // No draws in this monthly recompute
                    "created_at": now,
                });
                self.repo.insert_rating_history(history_doc).await?;
            }
        }
        Ok(())
    }

    pub async fn recompute_month(&self, period: Option<String>) -> Result<()> {
        // Determine previous month if None
        let (year, month) = if let Some(p) = period {
            let parts: Vec<_> = p.split('-').collect();
            if parts.len() != 2 { return Err(SharedError::BadRequest(format!("Invalid period: {}", p))); }
            (parts[0].parse::<i32>().map_err(|_| SharedError::BadRequest("invalid year".into()))?, parts[1].parse::<u32>().map_err(|_| SharedError::BadRequest("invalid month".into()))?)
        } else {
            let now = Utc::now();
            if now.month() == 1 { (now.year() - 1, 12) } else { (now.year(), now.month() - 1) }
        };
        let start = format!("{:04}-{:02}-01T00:00:00Z", year, month);
        let next_month = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        let end = format!("{:04}-{:02}-01T00:00:00Z", next_month.0, next_month.1);

        // Fetch contests in the month
        let contests = self.repo.get_contests_in_period(&start, &end).await?;
        if contests.is_empty() { return Ok(()); }

        // TODO: Fetch existing latest ratings per player/scope; for now use defaults
        let mut latest: std::collections::HashMap<(String, RatingScope), RatingState> = std::collections::HashMap::new();

        // For simplicity, compute only global scope in this first pass
        // Build samples per player for this month
        let mut samples_by_player: std::collections::HashMap<String, Vec<OpponentSample>> = std::collections::HashMap::new();
        let mut games_played: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        let mut wins_by_player: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        let mut losses_by_player: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

        for c in contests.iter() {
            let cid = c.get("_id").and_then(|v| v.as_str()).ok_or(SharedError::Database("contest missing _id".into()))?;
            let contest_results = self.repo.get_contest_results(cid).await?;
            if contest_results.len() < 2 { continue; }
            
            let weight = 1.0; // Each contest has equal weight
            
            // Process actual contest results and track wins/losses
            for (player_id, player_place) in &contest_results {
                *games_played.entry(player_id.clone()).or_insert(0) += 1;
                
                // Track wins and losses based on placement
                if let Some(place) = player_place {
                    if *place == 1 {
                        *wins_by_player.entry(player_id.clone()).or_insert(0) += 1;
                    } else {
                        *losses_by_player.entry(player_id.clone()).or_insert(0) += 1;
                    }
                }
                
                // Compare this player against all other players
                for (opponent_id, opponent_place) in &contest_results {
                    if player_id == opponent_id { continue; }
                    
                    // Determine score based on placements
                    let score = match (player_place, opponent_place) {
                        (Some(p_place), Some(o_place)) => {
                            if p_place < o_place { 1.0 }
                            else if p_place > o_place { 0.0 }
                            else { 0.5 }
                        }
                        _ => 0.5,
                    };
                    
                    // Get opponent's current rating
                    let opp_state = *latest.entry((opponent_id.clone(), RatingScope::Global))
                        .or_insert(RatingState { 
                            rating: self.params.default_rating, 
                            rd: self.params.default_rd, 
                            vol: self.params.default_vol 
                        });
                    
                    samples_by_player.entry(player_id.clone()).or_default().push(OpponentSample {
                        opp_rating: opp_state.rating,
                        opp_rd: opp_state.rd,
                        score,
                        weight,
                    });
                }
            }
        }

        // Apply updates
        for (player, samples) in samples_by_player.into_iter() {
            let key = (player.clone(), RatingScope::Global);
            let state = *latest.entry(key.clone()).or_insert(RatingState { rating: self.params.default_rating, rd: self.params.default_rd, vol: self.params.default_vol });
            // No inactivity model here (monthly recompute processes only this month)
            let updated = update_period(state, &samples, self.params);
            latest.insert(key, updated);
        }

        // Persist latest and history docs (global scope)
        let period_end = end.clone();
        let now = Utc::now().to_rfc3339();
        for ((player_id, scope), state) in latest.into_iter() {
            if let RatingScope::Global = scope {
                let gp = *games_played.get(&player_id).unwrap_or(&0);
                let wins = *wins_by_player.get(&player_id).unwrap_or(&0);
                let losses = *losses_by_player.get(&player_id).unwrap_or(&0);
                let latest_doc = serde_json::json!({
                    "player_id": player_id,
                    "scope_type": "global",
                    "scope_id": serde_json::Value::Null,
                    "rating": state.rating,
                    "rd": state.rd,
                    "volatility": state.vol,
                    "games_played": gp,
                    "last_period_end": period_end,
                    "updated_at": now,
                });
                self.repo.upsert_latest_rating(latest_doc).await?;

                let history_doc = serde_json::json!({
                    "player_id": player_id,
                    "scope_type": "global",
                    "scope_id": serde_json::Value::Null,
                    "period_end": period_end,
                    "rating": state.rating,
                    "rd": state.rd,
                    "volatility": state.vol,
                    "period_games": gp,
                    "wins": wins,
                    "losses": losses,
                    "draws": 0, // No draws in this monthly recompute
                    "created_at": now,
                });
                self.repo.insert_rating_history(history_doc).await?;
            }
        }
        Ok(())
    }

    pub async fn get_leaderboard(&self, scope: RatingScope, min_games: i32, limit: i32) -> Result<Vec<RatingLeaderboardEntryDto>> {
        let (scope_type, scope_id_opt) = match scope {
            RatingScope::Global => ("global", None),
            RatingScope::Game(ref gid) => ("game", Some(gid.as_str())),
        };
        let rows = self.repo.get_leaderboard(scope_type, scope_id_opt, min_games, limit).await?;
        
        // Log the raw data for debugging
        log::info!("Raw leaderboard data: {:?}", rows);
        
        let mut out = Vec::new();
        for v in rows.into_iter() {
            let player_id = v.get("player_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let rating = v.get("rating").and_then(|x| x.as_f64()).unwrap_or(1500.0);
            let rd = v.get("rd").and_then(|x| x.as_f64()).unwrap_or(350.0);
            let games_played = v.get("games_played").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
            let last_active = v.get("last_active").and_then(|x| x.as_str()).map(|s| s.to_string());
            let contest_id = v.get("contest_id").and_then(|x| x.as_str()).map(|s| s.to_string());
            
            // Get player information from the joined data
            let handle = v.get("handle").and_then(|x| x.as_str()).map(|s| s.to_string());
            let firstname = v.get("firstname").and_then(|x| x.as_str()).unwrap_or("Unknown").to_string();
            
            // Log individual player data for debugging
            log::info!("Player data: player_id={}, handle={:?}, firstname={}, last_active={:?}, contest_id={:?}", player_id, handle, firstname, last_active, contest_id);
            
            out.push(RatingLeaderboardEntryDto { 
                player_id, 
                handle, 
                rating, 
                rd, 
                games_played, 
                last_active,
                contest_id
            });
        }
        Ok(out)
    }

    pub async fn get_player_ratings(&self, player_id: &str) -> Result<Vec<PlayerRatingDto>> {
        let rows = self.repo.get_player_latest_ratings(player_id).await?;
        let mut out = Vec::new();
        for v in rows.into_iter() {
            let scope_type = v.get("scope_type").and_then(|x| x.as_str()).unwrap_or("global");
            let scope = if scope_type == "game" {
                let sid = v.get("scope_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                RatingScope::Game(sid)
            } else {
                RatingScope::Global
            };
            out.push(PlayerRatingDto {
                player_id: player_id.to_string(),
                scope,
                rating: v.get("rating").and_then(|x| x.as_f64()).unwrap_or(1500.0),
                rd: v.get("rd").and_then(|x| x.as_f64()).unwrap_or(350.0),
                volatility: v.get("volatility").and_then(|x| x.as_f64()).unwrap_or(0.06),
                games_played: v.get("games_played").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                last_period_end: v.get("last_period_end").and_then(|x| x.as_str()).map(|s| s.to_string()),
                updated_at: v.get("updated_at").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            });
        }
        Ok(out)
    }

    pub async fn get_player_rating_history(&self, player_id: &str, scope: RatingScope) -> Result<Vec<PlayerRatingHistoryPointDto>> {
        let (scope_type, scope_id_opt) = match scope {
            RatingScope::Global => ("global", None),
            RatingScope::Game(ref gid) => ("game", Some(gid.as_str())),
        };
        let rows = self.repo.get_rating_history(player_id, scope_type, scope_id_opt, 180).await?;
        let mut out = Vec::new();
        for v in rows.into_iter() {
            out.push(PlayerRatingHistoryPointDto {
                player_id: player_id.to_string(),
                scope: scope.clone(),
                period_end: v.get("period_end").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                rating: v.get("rating").and_then(|x| x.as_f64()).unwrap_or(1500.0),
                rd: v.get("rd").and_then(|x| x.as_f64()).unwrap_or(350.0),
                volatility: v.get("volatility").and_then(|x| x.as_f64()).unwrap_or(0.06),
                period_games: v.get("period_games").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                wins: v.get("wins").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                losses: v.get("losses").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                draws: v.get("draws").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
            });
        }
        Ok(out)
    }

    /// Debug a single player's activity and RD update decision within a given period (YYYY-MM)
    pub async fn debug_player_period_activity(&self, email: &str, period: &str) -> Result<Value> {
        // Resolve player_id from email
        let player_id_opt = self.get_player_id_by_email(email).await?;
        let player_id = if let Some(pid) = player_id_opt { pid } else {
            return Ok(serde_json::json!({
                "email": email,
                "error": "player_not_found"
            }));
        };

        // Parse period boundaries
        let parts: Vec<_> = period.split('-').collect();
        if parts.len() != 2 { return Err(SharedError::BadRequest(format!("Invalid period: {}", period))); }
        let year = parts[0].parse::<i32>().map_err(|_| SharedError::BadRequest("invalid year".into()))?;
        let month = parts[1].parse::<u32>().map_err(|_| SharedError::BadRequest("invalid month".into()))?;
        let start = format!("{:04}-{:02}-01T00:00:00Z", year, month);
        let next_month = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        let end = format!("{:04}-{:02}-01T00:00:00Z", next_month.0, next_month.1);

        // Gather contests in period and build samples for this player
        let contests = self.repo.get_contests_in_period(&start, &end).await?;
        let mut samples: Vec<OpponentSample> = Vec::new();
        let mut contests_with_player = 0usize;

        // Load latest rating for this player (if any)
        let current_state = if let Some(existing_rating) = self.repo.get_latest_rating("global", None, &player_id).await? {
            let rating = existing_rating.get("rating").and_then(|x| x.as_f64()).unwrap_or(self.params.default_rating);
            let rd = existing_rating.get("rd").and_then(|x| x.as_f64()).unwrap_or(self.params.default_rd);
            let volatility = existing_rating.get("volatility").and_then(|x| x.as_f64()).unwrap_or(self.params.default_vol);
            RatingState { rating, rd, vol: volatility }
        } else {
            RatingState { rating: self.params.default_rating, rd: self.params.default_rd, vol: self.params.default_vol }
        };

        // For collecting opponent RDs
        let mut opponent_rds: Vec<f64> = Vec::new();

        for c in contests.iter() {
            let cid = c.get("_id").and_then(|v| v.as_str()).ok_or(SharedError::Database("contest missing _id".into()))?;
            let contest_results = self.repo.get_contest_results(cid).await?;
            if contest_results.len() < 2 { continue; }
            let weight = 1.0;

            // Determine if player participates
            let mut player_present = false;
            for (pid, _) in &contest_results {
                if *pid == player_id { player_present = true; break; }
            }
            if !player_present { continue; }
            contests_with_player += 1;

            // Compute player's place for scoring
            let mut player_place_opt: Option<i32> = None;
            for (pid, place) in &contest_results {
                if *pid == player_id { player_place_opt = *place; break; }
            }
            for (opp_id, opp_place) in &contest_results {
                if *opp_id == player_id { continue; }
                let score = match (player_place_opt, *opp_place) {
                    (Some(p_place), Some(o_place)) => {
                        if p_place < o_place { 1.0 } else if p_place > o_place { 0.0 } else { 0.5 }
                    }
                    _ => 0.5,
                };
                // Load opponent latest rating from DB for accuracy
                let (opp_rating, opp_rd) = if let Some(r) = self.repo.get_latest_rating("global", None, opp_id).await? {
                    let r0 = r.get("rating").and_then(|x| x.as_f64()).unwrap_or(self.params.default_rating);
                    let rd0 = r.get("rd").and_then(|x| x.as_f64()).unwrap_or(self.params.default_rd);
                    (r0, rd0)
                } else {
                    (self.params.default_rating, self.params.default_rd)
                };
                opponent_rds.push(opp_rd);
                samples.push(OpponentSample { opp_rating, opp_rd, score, weight });
            }
        }

        // Decide path and compute after state
        let before = current_state;
        let after = if samples.is_empty() {
            // Inactivity inflation path
            RatingState {
                rating: before.rating,
                rd: (before.rd.powi(2) + before.vol.powi(2)).sqrt().min(self.params.default_rd),
                vol: before.vol,
            }
        } else {
            update_period(before, &samples, self.params)
        };

        // Compute Glicko-2 internals for transparency when there are samples
        let mut internals = serde_json::json!({});
        if !samples.is_empty() {
            // Helpers (duplicate minimal logic here)
            let to_mu = |r: f64| (r - 1500.0) / 173.7178;
            let to_phi = |rd: f64| rd / 173.7178;
            let g = |phi_j: f64| 1.0 / (1.0 + 3.0 * phi_j.powi(2) / std::f64::consts::PI.powi(2)).sqrt();
            let e = |mu: f64, mu_j: f64, phi_j: f64| 1.0 / (1.0 + (-g(phi_j) * (mu - mu_j)).exp());

            let mu = to_mu(before.rating);
            let phi = to_phi(before.rd);

            let mut v_inv = 0.0f64;
            let mut delta_num = 0.0f64;
            let mut exp_sum = 0.0f64;
            let mut score_sum = 0.0f64;
            for s in &samples {
                let mu_j = to_mu(s.opp_rating);
                let phi_j = to_phi(s.opp_rd);
                let g_phi = g(phi_j);
                let e_val = e(mu, mu_j, phi_j);
                v_inv += s.weight * (g_phi * g_phi * e_val * (1.0 - e_val));
                delta_num += s.weight * g_phi * (s.score - e_val);
                exp_sum += e_val;
                score_sum += s.score;
            }
            let v = if v_inv > 0.0 { 1.0 / v_inv } else { f64::INFINITY };
            let delta = if v.is_finite() { v * delta_num } else { 0.0 };

            // Volatility update approximation (copy of our method's structure)
            let sigma = before.vol;
            let a = (sigma * sigma).ln();
            let tau = self.params.tau;
            let mut a_low = a - 10.0;
            let mut a_high = a + 10.0;
            let f = |x: f64| {
                let ex = (x).exp();
                let phi_sq = phi * phi;
                let top = ex * (delta * delta - phi_sq - v - ex);
                let bot = 2.0 * (phi_sq + v + ex) * (phi_sq + v + ex);
                (top / bot) - ((x - a) / (tau * tau))
            };
            for _ in 0..30 {
                let mid = (a_low + a_high) / 2.0;
                let f_low = f(a_low);
                let f_mid = f(mid);
                if f_mid * f_low < 0.0 { a_high = mid; } else { a_low = mid; }
                if (a_high - a_low).abs() < 1e-6 { break; }
            }
            let a_new = (a_low + a_high) / 2.0;
            let sigma_prime = (a_new / 2.0).exp().sqrt();
            let phi_star = (phi.powi(2) + sigma_prime.powi(2)).sqrt();

            internals = serde_json::json!({
                "expected_score_sum": exp_sum,
                "actual_score_sum": score_sum,
                "expected_score_avg": exp_sum / (samples.len() as f64),
                "actual_score_avg": score_sum / (samples.len() as f64),
                "v": v,
                "delta": delta,
                "sigma_before": sigma,
                "sigma_after": sigma_prime,
                "phi_before": phi,
                "phi_star": phi_star
            });
        }

        // Opponent RD stats
        let opp_rd_stats = if opponent_rds.is_empty() { None } else {
            let mut v = opponent_rds.clone();
            v.sort_by(|a,b| a.partial_cmp(b).unwrap());
            let sum: f64 = v.iter().sum();
            let avg = sum / v.len() as f64;
            let min = *v.first().unwrap();
            let max = *v.last().unwrap();
            Some(serde_json::json!({
                "count": v.len(),
                "avg": avg,
                "min": min,
                "max": max
            }))
        };

        Ok(serde_json::json!({
            "email": email,
            "player_id": player_id,
            "period": period,
            "contests_total": contests.len(),
            "contests_with_player": contests_with_player,
            "samples_count": samples.len(),
            "path": if samples.is_empty() { "inflation" } else { "update" },
            "rd_before": before.rd,
            "rd_after": after.rd,
            "rating_before": before.rating,
            "rating_after": after.rating,
            "opponent_rd_stats": opp_rd_stats,
            "internals": internals
        }))
    }

    /// Debug function to check what's happening with player IDs
    pub async fn debug_player_ids(&self) -> Result<Vec<serde_json::Value>> {
        self.repo.debug_player_ids().await
    }

    /// Debug function to check what collections exist in the database
    pub async fn debug_collections(&self) -> Result<Vec<Value>> {
        self.repo.debug_collections().await
    }

    /// Debug function to check resulted_in vs players mismatch
    pub async fn debug_resulted_in_vs_players(&self) -> Result<Vec<Value>> {
        self.repo.debug_resulted_in_vs_players().await
    }

    /// Get simple leaderboard without complex player joins
    pub async fn get_simple_leaderboard(&self, scope: RatingScope, min_games: i32, limit: i32) -> Result<Vec<Value>> {
        let (scope_type, scope_id_opt) = match scope {
            RatingScope::Global => ("global", None),
            RatingScope::Game(ref gid) => ("game", Some(gid.as_str())),
        };
        self.repo.get_simple_leaderboard(scope_type, scope_id_opt, min_games, limit).await
    }

    /// Get leaderboard with player info from contest data
    pub async fn get_leaderboard_with_contest_data(&self, scope: RatingScope, min_games: i32, limit: i32) -> Result<Vec<Value>> {
        let (scope_type, scope_id_opt) = match scope {
            RatingScope::Global => ("global", None),
            RatingScope::Game(ref gid) => ("game", Some(gid.as_str())),
        };
        self.repo.get_leaderboard_with_contest_data(scope_type, scope_id_opt, min_games, limit).await
    }

    /// Get player ID by email
    pub async fn get_player_id_by_email(&self, email: &str) -> Result<Option<String>> {
        let query = "FOR p IN player FILTER LOWER(p.email) == LOWER(@email) LIMIT 1 RETURN p._id";
        
        let aql = AqlQuery::builder()
            .query(query)
            .bind_var("email", email)
            .build();
            
        match self.repo.db.aql_query::<String>(aql).await {
            Ok(results) => {
                if let Some(player_id) = results.into_iter().next() {
                    Ok(Some(player_id))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                log::error!("Failed to query player ID from email: {}", e);
                Err(shared::SharedError::Database(e.to_string()))
            }
        }
    }
}


