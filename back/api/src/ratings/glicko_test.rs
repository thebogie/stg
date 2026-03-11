#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_glicko2_params_default() {
        let params = Glicko2Params::default();
        assert_eq!(params.default_rating, 1500.0);
        assert_eq!(params.default_rd, 350.0);
        assert_eq!(params.default_vol, 0.06);
        assert_eq!(params.tau, 0.5);
    }

    #[test]
    fn test_rating_state_creation() {
        let state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        assert_eq!(state.rating, 1500.0);
        assert_eq!(state.rd, 350.0);
        assert_eq!(state.vol, 0.06);
    }

    #[test]
    fn test_opponent_sample_creation() {
        let sample = OpponentSample {
            opp_rating: 1600.0,
            opp_rd: 300.0,
            score: 1.0, // Win
            weight: 1.0,
        };
        assert_eq!(sample.opp_rating, 1600.0);
        assert_eq!(sample.opp_rd, 300.0);
        assert_eq!(sample.score, 1.0);
        assert_eq!(sample.weight, 1.0);
    }

    #[test]
    fn test_g_function() {
        // Test g(φ) function with various phi values
        let phi_350 = 350.0 / 173.7178; // Convert RD to phi
        let g_350 = g(phi_350);
        assert!(g_350 > 0.0 && g_350 < 1.0);
        
        let phi_100 = 100.0 / 173.7178; // Lower RD
        let g_100 = g(phi_100);
        assert!(g_100 > g_350); // Lower RD should give higher g value
        
        let phi_500 = 500.0 / 173.7178; // Higher RD
        let g_500 = g(phi_500);
        assert!(g_500 < g_350); // Higher RD should give lower g value
    }

    #[test]
    fn test_e_function() {
        // Test e(μ, μ_j, φ_j) function
        let mu = to_mu(1500.0); // Player rating
        let mu_j = to_mu(1600.0); // Opponent rating
        let phi_j = to_phi(300.0); // Opponent RD
        
        let e_val = e(mu, mu_j, phi_j);
        assert!(e_val > 0.0 && e_val < 1.0);
        
        // When player rating is much lower than opponent, expected score should be low
        let mu_lower = to_mu(1200.0);
        let e_lower = e(mu_lower, mu_j, phi_j);
        assert!(e_lower < e_val);
        
        // When player rating is much higher than opponent, expected score should be high
        let mu_higher = to_mu(1800.0);
        let e_higher = e(mu_higher, mu_j, phi_j);
        assert!(e_higher > e_val);
    }

    #[test]
    fn test_rating_conversion_functions() {
        // Test conversion between rating scale and μ scale
        let rating = 1500.0;
        let mu = to_mu(rating);
        let rating_back = from_mu(mu);
        assert_relative_eq!(rating, rating_back, epsilon = 1e-10);
        
        // Test conversion between RD and φ scale
        let rd = 350.0;
        let phi = to_phi(rd);
        let rd_back = from_phi(phi);
        assert_relative_eq!(rd, rd_back, epsilon = 1e-10);
        
        // Test edge cases
        let mu_zero = to_mu(0.0);
        let rating_zero = from_mu(mu_zero);
        assert!(rating_zero < 0.0); // Should be negative
        
        let mu_high = to_mu(3000.0);
        let rating_high = from_mu(mu_high);
        assert!(rating_high > 2000.0); // Should be very high
    }

    #[test]
    fn test_pre_period_inflate_rd() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        // Test RD inflation after 1 period of inactivity
        let inflated_1 = pre_period_inflate_rd(initial_state, 1.0);
        assert!(inflated_1.rd > initial_state.rd);
        assert_eq!(inflated_1.rating, initial_state.rating);
        assert_eq!(inflated_1.vol, initial_state.vol);
        
        // Test RD inflation after 5 periods of inactivity
        let inflated_5 = pre_period_inflate_rd(initial_state, 5.0);
        assert!(inflated_5.rd > inflated_1.rd);
        
        // Test RD inflation after 0 periods (should be unchanged)
        let inflated_0 = pre_period_inflate_rd(initial_state, 0.0);
        assert_relative_eq!(inflated_0.rd, initial_state.rd, epsilon = 1e-10);
    }

    #[test]
    fn test_update_period_empty_samples() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        let params = Glicko2Params::default();
        let result = update_period(initial_state, &[], params);
        
        // Should return unchanged state when no samples
        assert_eq!(result.rating, initial_state.rating);
        assert_eq!(result.rd, initial_state.rd);
        assert_eq!(result.vol, initial_state.vol);
    }

    #[test]
    fn test_update_period_single_win() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        let opponent = OpponentSample {
            opp_rating: 1600.0,
            opp_rd: 300.0,
            score: 1.0, // Win
            weight: 1.0,
        };
        
        let params = Glicko2Params::default();
        let result = update_period(initial_state, &[opponent], params);
        
        // Rating should increase after beating higher-rated opponent
        assert!(result.rating > initial_state.rating);
        // RD should decrease (more certain)
        assert!(result.rd < initial_state.rd);
        // Volatility should change
        assert!(result.vol != initial_state.vol);
    }

    #[test]
    fn test_update_period_single_loss() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        let opponent = OpponentSample {
            opp_rating: 1400.0,
            opp_rd: 300.0,
            score: 0.0, // Loss
            weight: 1.0,
        };
        
        let params = Glicko2Params::default();
        let result = update_period(initial_state, &[opponent], params);
        
        // Rating should decrease after losing to lower-rated opponent
        assert!(result.rating < initial_state.rating);
        // RD should decrease (more certain)
        assert!(result.rd < initial_state.rd);
    }

    #[test]
    fn test_update_period_draw() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        let opponent = OpponentSample {
            opp_rating: 1500.0,
            opp_rd: 350.0,
            score: 0.5, // Draw
            weight: 1.0,
        };
        
        let params = Glicko2Params::default();
        let result = update_period(initial_state, &[opponent], params);
        
        // Rating should change minimally for draw against equal opponent
        assert_relative_eq!(result.rating, initial_state.rating, epsilon = 50.0);
        // RD should decrease
        assert!(result.rd < initial_state.rd);
    }

    #[test]
    fn test_update_period_multiple_opponents() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        let opponents = vec![
            OpponentSample {
                opp_rating: 1600.0,
                opp_rd: 300.0,
                score: 1.0, // Win
                weight: 1.0,
            },
            OpponentSample {
                opp_rating: 1400.0,
                opp_rd: 300.0,
                score: 0.0, // Loss
                weight: 1.0,
            },
            OpponentSample {
                opp_rating: 1500.0,
                opp_rd: 350.0,
                score: 0.5, // Draw
                weight: 1.0,
            },
        ];
        
        let params = Glicko2Params::default();
        let result = update_period(initial_state, &opponents, params);
        
        // Should handle multiple opponents
        assert!(result.rating != initial_state.rating);
        assert!(result.rd < initial_state.rd);
        assert!(result.vol != initial_state.vol);
    }

    #[test]
    fn test_update_period_weighted_samples() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        let opponents = vec![
            OpponentSample {
                opp_rating: 1600.0,
                opp_rd: 300.0,
                score: 1.0, // Win
                weight: 2.0, // Double weight
            },
            OpponentSample {
                opp_rating: 1400.0,
                opp_rd: 300.0,
                score: 0.0, // Loss
                weight: 1.0, // Normal weight
            },
        ];
        
        let params = Glicko2Params::default();
        let result = update_period(initial_state, &opponents, params);
        
        // Higher weight should have more influence
        assert!(result.rating != initial_state.rating);
    }

    #[test]
    fn test_update_period_zero_weight_samples() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        let opponents = vec![
            OpponentSample {
                opp_rating: 1600.0,
                opp_rd: 300.0,
                score: 1.0,
                weight: 0.0, // Zero weight
            },
        ];
        
        let params = Glicko2Params::default();
        let result = update_period(initial_state, &opponents, params);
        
        // Zero weight samples should be ignored
        assert_eq!(result.rating, initial_state.rating);
        assert_eq!(result.rd, initial_state.rd);
        assert_eq!(result.vol, initial_state.vol);
    }

    #[test]
    fn test_edge_cases() {
        // Test with very high ratings
        let high_rating_state = RatingState {
            rating: 3000.0,
            rd: 100.0,
            vol: 0.06,
        };
        
        let opponent = OpponentSample {
            opp_rating: 2900.0,
            opp_rd: 100.0,
            score: 1.0,
            weight: 1.0,
        };
        
        let params = Glicko2Params::default();
        let result = update_period(high_rating_state, &[opponent], params);
        
        assert!(result.rating > 0.0);
        assert!(result.rd > 0.0);
        assert!(result.vol > 0.0);
        
        // Test with very low ratings
        let low_rating_state = RatingState {
            rating: 100.0,
            rd: 100.0,
            vol: 0.06,
        };
        
        let result_low = update_period(low_rating_state, &[opponent], params);
        
        assert!(result_low.rating > 0.0);
        assert!(result_low.rd > 0.0);
        assert!(result_low.vol > 0.0);
    }

    #[test]
    fn test_volatility_constraints() {
        let initial_state = RatingState {
            rating: 1500.0,
            rd: 350.0,
            vol: 0.06,
        };
        
        let opponent = OpponentSample {
            opp_rating: 1500.0,
            opp_rd: 350.0,
            score: 0.5,
            weight: 1.0,
        };
        
        let params = Glicko2Params::default();
        let result = update_period(initial_state, &[opponent], params);
        
        // Volatility should remain within reasonable bounds
        assert!(result.vol > 0.0);
        assert!(result.vol < 1.0);
    }
}
