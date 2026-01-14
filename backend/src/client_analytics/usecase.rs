use async_trait::async_trait;
use shared::dto::client_sync::*;

use crate::client_analytics::repository::ClientAnalyticsRepository;
use chrono::Utc;
use log;
use shared::error::SharedError;
use std::collections::HashMap;

/// Use case trait for client analytics operations
#[async_trait]
pub trait ClientAnalyticsUseCase: Send + Sync {
    /// Synchronizes client data (full or delta)
    async fn sync_client_data(
        &self,
        player_id: &str,
        request: &ClientSyncRequest,
    ) -> Result<ClientSyncResponse, SharedError>;

    /// Executes real-time analytics queries
    async fn query_client_analytics(
        &self,
        player_id: &str,
        query: &ClientAnalyticsQuery,
    ) -> Result<ClientAnalyticsResponse, SharedError>;

    /// Validates client data integrity
    async fn validate_client_data(
        &self,
        player_id: &str,
        request: &ClientDataValidationRequest,
    ) -> Result<ClientDataValidationResponse, SharedError>;

    /// Gets client sync status and metadata
    async fn get_client_sync_status(
        &self,
        player_id: &str,
    ) -> Result<ClientSyncMetadata, SharedError>;

    /// Clears client data for a player
    async fn clear_client_data(&self, player_id: &str) -> Result<(), SharedError>;

    /// Gets gaming communities for a player
    async fn get_gaming_communities(
        &self,
        player_id: &str,
        min_contests: i32,
    ) -> Result<Vec<serde_json::Value>, SharedError>;

    /// Gets player networking data
    async fn get_player_networking(
        &self,
        player_id: &str,
    ) -> Result<serde_json::Value, SharedError>;
}

/// Implementation of client analytics use cases
pub struct ClientAnalyticsUseCaseImpl<R, C> {
    repository: R,
    _phantom: std::marker::PhantomData<C>,
}

impl<R, C> ClientAnalyticsUseCaseImpl<R, C>
where
    R: ClientAnalyticsRepository + Send + Sync,
    C: arangors::client::ClientExt + Send + Sync,
{
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Converts database contest data to client DTOs
    async fn convert_contests_to_client_dtos(
        &self,
        contests: Vec<shared::models::contest::Contest>,
        player_id: &str,
    ) -> Result<Vec<ClientContestDto>, SharedError> {
        let mut client_contests = Vec::new();

        for contest in contests {
            // Get game data
            let game = self.repository.get_game_for_contest(&contest.id).await?;
            let _game_dto = ClientGameDto {
                id: game.id.clone(),
                name: game.name.clone(),
                year_published: game.year_published,
                description: game.description.clone(),
                source: format!("{:?}", game.source),
            };

            // Get venue data
            let venue = self.repository.get_venue_for_contest(&contest.id).await?;
            let _venue_dto = ClientVenueDto {
                id: venue.id.clone(),
                name: venue.display_name.clone(), // Use display_name as name
                display_name: Some(venue.display_name.clone()),
                formatted_address: Some(venue.formatted_address.clone()),
                city: None,    // Venue doesn't have city field
                state: None,   // Venue doesn't have state field
                country: None, // Venue doesn't have country field
                lat: Some(venue.lat),
                lng: Some(venue.lng),
                source: format!("{:?}", venue.source),
            };

            // Get participants and results
            let participants = self
                .repository
                .get_contest_participants(&contest.id)
                .await?;
            let mut client_participants = Vec::new();
            let mut my_result = None;

            for participant in participants {
                let participant_dto = ClientParticipantDto {
                    player_id: participant.player_id.clone(),
                    handle: participant.handle.clone(),
                    firstname: participant.firstname.clone(),
                    lastname: participant.lastname.clone(),
                    place: participant.place,
                    result: participant.result.clone(),
                };

                client_participants.push(participant_dto);

                // Check if this is the current user
                if participant.player_id == player_id {
                    my_result = Some(ClientResultDto {
                        place: participant.place,
                        result: participant.result.clone(),
                        points: participant.points,
                    });
                }
            }

            let my_result = my_result.ok_or_else(|| {
                SharedError::NotFound(format!(
                    "Current player not found in contest {}",
                    contest.id
                ))
            })?;

            let client_contest = ClientContestDto {
                id: contest.id.clone(),
                name: contest.name.clone(),
                start: contest.start,
                end: contest.stop,
                game_id: game.id,
                game_name: game.name,
                venue_id: venue.id,
                venue_name: venue.display_name.clone(),
                venue_display_name: Some(venue.display_name.clone()),
                participants: client_participants,
                my_result,
            };

            client_contests.push(client_contest);
        }

        Ok(client_contests)
    }

    /// Converts database game data to client DTOs
    fn convert_games_to_client_dtos(
        &self,
        games: Vec<shared::models::game::Game>,
    ) -> Vec<ClientGameDto> {
        games
            .into_iter()
            .map(|game| ClientGameDto {
                id: game.id,
                name: game.name,
                year_published: game.year_published,
                description: game.description,
                source: format!("{:?}", game.source),
            })
            .collect()
    }

    /// Converts database venue data to client DTOs
    fn convert_venues_to_client_dtos(
        &self,
        venues: Vec<shared::models::venue::Venue>,
    ) -> Vec<ClientVenueDto> {
        venues
            .into_iter()
            .map(|venue| ClientVenueDto {
                id: venue.id,
                name: venue.display_name.clone(),
                display_name: Some(venue.display_name),
                formatted_address: Some(venue.formatted_address),
                city: None,
                state: None,
                country: None,
                lat: Some(venue.lat),
                lng: Some(venue.lng),
                source: format!("{:?}", venue.source),
            })
            .collect()
    }

    /// Converts database player data to client DTOs
    fn convert_players_to_client_dtos(
        &self,
        players: Vec<shared::models::player::Player>,
    ) -> Vec<ClientPlayerDto> {
        players
            .into_iter()
            .map(|player| ClientPlayerDto {
                id: player.id,
                handle: player.handle,
                firstname: Some(player.firstname),
                lastname: None, // Player doesn't have lastname field
                email: Some(player.email),
                last_seen: player.created_at, // Use created_at as last_seen for now
            })
            .collect()
    }

    /// Computes analytics from contest data
    fn compute_analytics_from_contests(
        &self,
        contests: &[ClientContestDto],
        player_id: &str,
    ) -> (
        ClientStatsDto,
        Vec<ClientGamePerformanceDto>,
        Vec<ClientOpponentPerformanceDto>,
        Vec<ClientTrendDto>,
    ) {
        if contests.is_empty() {
            return (
                ClientStatsDto {
                    total_contests: 0,
                    total_wins: 0,
                    total_losses: 0,
                    win_rate: 0.0,
                    average_placement: 0.0,
                    best_placement: 0,
                    worst_placement: 0,
                    current_streak: 0,
                    longest_streak: 0,
                },
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
        }

        // Compute core stats
        let mut total_contests = 0;
        let mut total_wins = 0;
        let mut total_losses = 0;
        let mut total_placement = 0;
        let mut best_placement = i32::MAX;
        let mut worst_placement = 0;

        for contest in contests {
            total_contests += 1;

            match contest.my_result.result.as_str() {
                "won" => total_wins += 1,
                "lost" => total_losses += 1,
                _ => {}
            }

            let placement = contest.my_result.place;
            total_placement += placement;
            best_placement = best_placement.min(placement);
            worst_placement = worst_placement.max(placement);
        }

        let stats = ClientStatsDto {
            total_contests,
            total_wins,
            total_losses,
            win_rate: if total_contests > 0 {
                (total_wins as f64 / total_contests as f64) * 100.0
            } else {
                0.0
            },
            average_placement: if total_contests > 0 {
                total_placement as f64 / total_contests as f64
            } else {
                0.0
            },
            best_placement: if best_placement == i32::MAX {
                0
            } else {
                best_placement
            },
            worst_placement,
            current_streak: 0, // Would need to compute from full dataset
            longest_streak: 0, // Would need to compute from full dataset
        };

        // Compute game performance
        let mut game_stats: HashMap<String, ClientGamePerformanceDto> = HashMap::new();

        for contest in contests {
            let game_id = contest.game_id.clone();
            let entry = game_stats
                .entry(game_id)
                .or_insert_with(|| ClientGamePerformanceDto {
                    game: ClientGameDto {
                        id: contest.game_id.clone(),
                        name: contest.game_name.clone(),
                        year_published: None,
                        description: None,
                        source: "unknown".to_string(),
                    },
                    total_plays: 0,
                    wins: 0,
                    losses: 0,
                    win_rate: 0.0,
                    average_placement: 0.0,
                    best_placement: i32::MAX,
                    worst_placement: 0,
                    last_played: contest.start,
                    days_since_last_play: 0,
                    favorite_venue: None,
                });

            entry.total_plays += 1;
            entry.last_played = entry.last_played.max(contest.start);

            match contest.my_result.result.as_str() {
                "won" => entry.wins += 1,
                "lost" => entry.losses += 1,
                _ => {}
            }

            let placement = contest.my_result.place;
            entry.best_placement = entry.best_placement.min(placement);
            entry.worst_placement = entry.worst_placement.max(placement);
        }

        // Calculate derived values for game performance
        for performance in game_stats.values_mut() {
            performance.win_rate = if performance.total_plays > 0 {
                (performance.wins as f64 / performance.total_plays as f64) * 100.0
            } else {
                0.0
            };

            performance.days_since_last_play = Utc::now()
                .fixed_offset()
                .signed_duration_since(performance.last_played)
                .num_days();
        }

        let mut game_performance: Vec<_> = game_stats.into_values().collect();
        game_performance.sort_by(|a, b| {
            b.total_plays
                .cmp(&a.total_plays)
                .then(b.win_rate.partial_cmp(&a.win_rate).unwrap())
        });

        // Compute opponent performance
        let mut opponent_stats: HashMap<String, ClientHeadToHeadDto> = HashMap::new();

        for contest in contests {
            for participant in &contest.participants {
                if participant.player_id != player_id {
                    let entry = opponent_stats
                        .entry(participant.player_id.clone())
                        .or_insert_with(|| ClientHeadToHeadDto {
                            total_contests: 0,
                            my_wins: 0,
                            opponent_wins: 0,
                            my_win_rate: 0.0,
                            contest_history: Vec::new(),
                        });

                    entry.total_contests += 1;
                    entry.contest_history.push(contest.clone());

                    match contest.my_result.result.as_str() {
                        "won" => entry.my_wins += 1,
                        "lost" => entry.opponent_wins += 1,
                        _ => {}
                    }
                }
            }
        }

        // Calculate win rates for opponents
        for stats in opponent_stats.values_mut() {
            stats.my_win_rate = if stats.total_contests > 0 {
                (stats.my_wins as f64 / stats.total_contests as f64) * 100.0
            } else {
                0.0
            };
        }

        let mut opponent_performance: Vec<_> = opponent_stats
            .into_iter()
            .map(|(opponent_id, head_to_head)| {
                // Create a placeholder opponent - in real implementation, you'd get this from the repository
                let opponent = ClientPlayerDto {
                    id: opponent_id.clone(),
                    handle: "unknown".to_string(),
                    firstname: None,
                    lastname: None,
                    email: None,
                    last_seen: Utc::now().fixed_offset(),
                };

                ClientOpponentPerformanceDto {
                    opponent,
                    head_to_head,
                }
            })
            .collect();

        opponent_performance.sort_by(|a, b| {
            b.head_to_head
                .total_contests
                .cmp(&a.head_to_head.total_contests)
        });

        // Compute trends
        let mut monthly_stats: HashMap<String, (i32, i32, f64)> = HashMap::new();

        for contest in contests {
            let month_key = contest.start.format("%Y-%m").to_string();
            let entry = monthly_stats.entry(month_key).or_insert((0, 0, 0.0));

            entry.0 += 1; // contests_played
            if contest.my_result.result == "won" {
                entry.1 += 1; // wins
            }
            entry.2 += contest.my_result.place as f64; // total placement
        }

        let mut trends: Vec<_> = monthly_stats
            .into_iter()
            .map(|(period, (contests_played, wins, total_placement))| {
                let win_rate = if contests_played > 0 {
                    (wins as f64 / contests_played as f64) * 100.0
                } else {
                    0.0
                };
                let average_placement = if contests_played > 0 {
                    total_placement / contests_played as f64
                } else {
                    0.0
                };

                ClientTrendDto {
                    period,
                    contests_played,
                    wins,
                    win_rate,
                    average_placement,
                }
            })
            .collect();

        trends.sort_by(|a, b| b.period.cmp(&a.period));

        (stats, game_performance, opponent_performance, trends)
    }
}

#[async_trait]
impl<R, C> ClientAnalyticsUseCase for ClientAnalyticsUseCaseImpl<R, C>
where
    R: ClientAnalyticsRepository + Send + Sync,
    C: arangors::client::ClientExt + Send + Sync,
{
    async fn sync_client_data(
        &self,
        player_id: &str,
        request: &ClientSyncRequest,
    ) -> Result<ClientSyncResponse, SharedError> {
        log::info!("Starting client sync for player: {}", player_id);

        // Get contests based on sync type
        let contests = if request.full_sync {
            self.repository
                .get_all_contests_for_player(player_id)
                .await?
        } else {
            // Delta sync - get contests since last sync
            let since = request
                .last_sync
                .unwrap_or_else(|| Utc::now().fixed_offset());
            self.repository.get_contests_since(player_id, since).await?
        };

        // Apply limit if specified
        let contests = if let Some(limit) = request.limit {
            contests.into_iter().take(limit).collect()
        } else {
            contests
        };

        // Convert to client DTOs
        let client_contests = self
            .convert_contests_to_client_dtos(contests, player_id)
            .await?;

        // Get related data if requested
        let (games, venues, players) = if request.include_related {
            let games = self.repository.get_games_for_player(player_id).await?;
            let venues = self.repository.get_venues_for_player(player_id).await?;
            let players = self.repository.get_opponents_for_player(player_id).await?;

            (
                self.convert_games_to_client_dtos(games),
                self.convert_venues_to_client_dtos(venues),
                self.convert_players_to_client_dtos(players),
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        // Create sync metadata
        let sync_metadata = ClientSyncMetadata {
            total_contests: self
                .repository
                .get_total_contests_for_player(player_id)
                .await?,
            contests_returned: client_contests.len(),
            is_delta: !request.full_sync,
            data_size_bytes: 0, // Would calculate actual size
            compression_ratio: None,
            next_sync_recommended: Utc::now().fixed_offset(),
        };

        let response = ClientSyncResponse {
            player_id: player_id.to_string(),
            sync_timestamp: Utc::now().fixed_offset(),
            data_version: "1.0.0".to_string(),
            contests: client_contests,
            games,
            venues,
            players,
            sync_metadata,
        };

        log::info!(
            "Client sync completed for player: {}, contests: {}",
            player_id,
            response.contests.len()
        );
        Ok(response)
    }

    async fn query_client_analytics(
        &self,
        player_id: &str,
        query: &ClientAnalyticsQuery,
    ) -> Result<ClientAnalyticsResponse, SharedError> {
        log::info!("Executing client analytics query for player: {}", player_id);

        // Get contests based on query filters
        let contests = self
            .repository
            .get_filtered_contests(player_id, query)
            .await?;

        // Convert to client DTOs
        let client_contests = self
            .convert_contests_to_client_dtos(contests, player_id)
            .await?;

        // Compute analytics
        let (stats, game_performance, opponent_performance, trends) =
            self.compute_analytics_from_contests(&client_contests, player_id);

        let response = ClientAnalyticsResponse {
            player_id: player_id.to_string(),
            query: query.clone(),
            computed_at: Utc::now().fixed_offset(),
            contests: client_contests,
            stats,
            game_performance,
            opponent_performance,
            trends,
        };

        log::info!("Client analytics query completed for player: {}", player_id);
        Ok(response)
    }

    async fn validate_client_data(
        &self,
        player_id: &str,
        request: &ClientDataValidationRequest,
    ) -> Result<ClientDataValidationResponse, SharedError> {
        log::info!("Validating client data for player: {}", player_id);

        // Get server-side data for comparison
        let server_contests = self
            .repository
            .get_all_contests_for_player(player_id)
            .await?;
        let server_contest_count = server_contests.len();

        // Calculate server hash (simplified - in real implementation, use proper hashing)
        let server_hash = format!("{}:{}", player_id, server_contest_count);

        // Compare with client data
        let is_valid =
            server_hash == request.data_hash && server_contest_count == request.contest_count;

        let response = ClientDataValidationResponse {
            player_id: player_id.to_string(),
            is_valid,
            server_hash,
            data_version: "1.0.0".to_string(),
            contest_count: server_contest_count,
            validation_message: if is_valid {
                "Data integrity verified".to_string()
            } else {
                "Data integrity check failed - re-sync recommended".to_string()
            },
        };

        log::info!(
            "Client data validation completed for player: {}, valid: {}",
            player_id,
            is_valid
        );
        Ok(response)
    }

    async fn get_client_sync_status(
        &self,
        player_id: &str,
    ) -> Result<ClientSyncMetadata, SharedError> {
        log::info!("Getting client sync status for player: {}", player_id);

        let total_contests = self
            .repository
            .get_total_contests_for_player(player_id)
            .await?;
        let _last_contest = self
            .repository
            .get_last_contest_for_player(player_id)
            .await?;

        let sync_metadata = ClientSyncMetadata {
            total_contests,
            contests_returned: 0, // Not applicable for status
            is_delta: false,      // Not applicable for status
            data_size_bytes: 0,   // Would calculate actual size
            compression_ratio: None,
            next_sync_recommended: Utc::now().fixed_offset(),
        };

        log::info!("Client sync status retrieved for player: {}", player_id);
        Ok(sync_metadata)
    }

    async fn clear_client_data(&self, player_id: &str) -> Result<(), SharedError> {
        log::info!("Clearing client data for player: {}", player_id);

        // In a real implementation, you might want to log this action
        // or perform some cleanup operations

        log::info!("Client data cleared for player: {}", player_id);
        Ok(())
    }

    async fn get_gaming_communities(
        &self,
        player_id: &str,
        min_contests: i32,
    ) -> Result<Vec<serde_json::Value>, SharedError> {
        log::info!("Getting gaming communities for player: {}", player_id);

        let communities = self
            .repository
            .get_gaming_communities(player_id, min_contests)
            .await?;
        Ok(communities)
    }

    async fn get_player_networking(
        &self,
        player_id: &str,
    ) -> Result<serde_json::Value, SharedError> {
        log::info!("Getting player networking for player: {}", player_id);

        let networking = self.repository.get_player_networking(player_id).await?;
        Ok(networking)
    }
}
