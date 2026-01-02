use backend::ratings::glicko::{Glicko2Params, RatingState, OpponentSample, update_period, pre_period_inflate_rd};
use approx::assert_relative_eq;

#[test]
fn test_glicko2_integration_workflow() {
    // Test the complete Glicko2 workflow from start to finish
    
    // 1. Create initial rating state
    let initial_state = RatingState {
        rating: 1500.0,
        rd: 350.0,
        vol: 0.06,
    };
    
    // 2. Simulate inactivity (RD inflation)
    let inflated_state = pre_period_inflate_rd(initial_state, 2.0);
    assert!(inflated_state.rd > initial_state.rd);
    assert_eq!(inflated_state.rating, initial_state.rating);
    assert_eq!(inflated_state.vol, initial_state.vol);
    
    // 3. Create opponent samples for a tournament
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
    
    // 4. Update ratings based on tournament results
    let params = Glicko2Params::default();
    let updated_state = update_period(inflated_state, &opponents, params);
    
    // 5. Verify the results make sense
    assert!(updated_state.rating > 0.0);
    assert!(updated_state.rd > 0.0);
    assert!(updated_state.vol > 0.0);
    assert!(updated_state.rd < inflated_state.rd); // RD should decrease with more games
    
    // 6. Test that the rating change is reasonable
    let rating_change = (updated_state.rating - inflated_state.rating).abs();
    assert!(rating_change < 500.0); // Rating shouldn't change by more than 500 points in one tournament
}

#[test]
fn test_glicko2_rating_progression() {
    // Test how ratings progress over multiple periods
    
    let mut current_state = RatingState {
        rating: 1500.0,
        rd: 350.0,
        vol: 0.06,
    };
    
    let params = Glicko2Params::default();
    
    // Simulate 5 periods of games
    for period in 1..=5 {
        // Inflate RD for inactivity
        current_state = pre_period_inflate_rd(current_state, 1.0);
        
        // Create some opponents
        let opponents = vec![
            OpponentSample {
                opp_rating: current_state.rating + 100.0,
                opp_rd: 300.0,
                score: if period % 2 == 0 { 1.0 } else { 0.0 }, // Alternate wins/losses
                weight: 1.0,
            },
            OpponentSample {
                opp_rating: current_state.rating - 100.0,
                opp_rd: 300.0,
                score: if period % 2 == 0 { 0.0 } else { 1.0 }, // Opposite of above
                weight: 1.0,
            },
        ];
        
        // Update ratings
        current_state = update_period(current_state, &opponents, params);
        
        // Verify state remains valid
        assert!(current_state.rating > 0.0);
        assert!(current_state.rd > 0.0);
        assert!(current_state.vol > 0.0);
        assert!(current_state.rd <= 350.0); // RD shouldn't exceed initial value
    }
    
    // After 5 periods, RD should be lower (more certain)
    assert!(current_state.rd < 350.0);
}

#[test]
fn test_glicko2_edge_cases() {
    // Test various edge cases and boundary conditions
    
    let params = Glicko2Params::default();
    
    // Test with very high ratings
    let high_rating_state = RatingState {
        rating: 3000.0,
        rd: 50.0,
        vol: 0.06,
    };
    
    let high_opponent = OpponentSample {
        opp_rating: 2900.0,
        opp_rd: 50.0,
        score: 1.0,
        weight: 1.0,
    };
    
    let high_result = update_period(high_rating_state, &[high_opponent], params);
    assert!(high_result.rating > 0.0);
    assert!(high_result.rd > 0.0);
    
    // Test with very low ratings
    let low_rating_state = RatingState {
        rating: 100.0,
        rd: 50.0,
        vol: 0.06,
    };
    
    let low_opponent = OpponentSample {
        opp_rating: 200.0,
        opp_rd: 50.0,
        score: 0.0,
        weight: 1.0,
    };
    
    let low_result = update_period(low_rating_state, &[low_opponent], params);
    assert!(low_result.rating > 0.0);
    assert!(low_result.rd > 0.0);
    
    // Test with very high RD (uncertainty)
    let uncertain_state = RatingState {
        rating: 1500.0,
        rd: 500.0,
        vol: 0.06,
    };
    
    let certain_opponent = OpponentSample {
        opp_rating: 1500.0,
        opp_rd: 100.0,
        score: 0.5,
        weight: 1.0,
    };
    
    let uncertain_result = update_period(uncertain_state, &[certain_opponent], params);
    assert!(uncertain_result.rd < uncertain_state.rd); // Should become more certain
}

#[test]
fn test_glicko2_parameter_sensitivity() {
    // Test how different parameters affect the results
    
    let base_state = RatingState {
        rating: 1500.0,
        rd: 350.0,
        vol: 0.06,
    };
    
    let opponent = OpponentSample {
        opp_rating: 1600.0,
        opp_rd: 300.0,
        score: 1.0,
        weight: 1.0,
    };
    
    // Test with different tau values
    let low_tau_params = Glicko2Params {
        tau: 0.1,
        ..Glicko2Params::default()
    };
    
    let high_tau_params = Glicko2Params {
        tau: 1.0,
        ..Glicko2Params::default()
    };
    
    let low_tau_result = update_period(base_state.clone(), &[opponent.clone()], low_tau_params);
    let high_tau_result = update_period(base_state.clone(), &[opponent.clone()], high_tau_params);
    
    // Higher tau should allow more volatility change
    let low_tau_vol_change = (low_tau_result.vol - base_state.vol).abs();
    let high_tau_vol_change = (high_tau_result.vol - base_state.vol).abs();
    
    assert!(high_tau_vol_change >= low_tau_vol_change);
}

#[test]
fn test_glicko2_weighted_games() {
    // Test how game weights affect rating updates
    
    let base_state = RatingState {
        rating: 1500.0,
        rd: 350.0,
        vol: 0.06,
    };
    
    let opponent = OpponentSample {
        opp_rating: 1600.0,
        opp_rd: 300.0,
        score: 1.0,
        weight: 1.0,
    };
    
    let params = Glicko2Params::default();
    
    // Single game with weight 1
    let single_result = update_period(base_state.clone(), &[opponent.clone()], params);
    
    // Same game with weight 2 (should have more impact)
    let weighted_opponent = OpponentSample {
        weight: 2.0,
        ..opponent
    };
    
    let weighted_result = update_period(base_state.clone(), &[weighted_opponent], params);
    
    // Higher weight should result in larger rating change
    let single_change = (single_result.rating - base_state.rating).abs();
    let weighted_change = (weighted_result.rating - base_state.rating).abs();
    
    assert!(weighted_change >= single_change);
}

#[test]
fn test_glicko2_consistency() {
    // Test that the system produces consistent results
    
    let base_state = RatingState {
        rating: 1500.0,
        rd: 350.0,
        vol: 0.06,
    };
    
    let opponents = vec![
        OpponentSample {
            opp_rating: 1600.0,
            opp_rd: 300.0,
            score: 1.0,
            weight: 1.0,
        },
        OpponentSample {
            opp_rating: 1400.0,
            opp_rd: 300.0,
            score: 0.0,
            weight: 1.0,
        },
    ];
    
    let params = Glicko2Params::default();
    
    // Run the same update multiple times
    let result1 = update_period(base_state.clone(), &opponents, params);
    let result2 = update_period(base_state.clone(), &opponents, params);
    let result3 = update_period(base_state.clone(), &opponents, params);
    
    // Results should be identical (deterministic)
    assert_relative_eq!(result1.rating, result2.rating, epsilon = 1e-10);
    assert_relative_eq!(result1.rating, result3.rating, epsilon = 1e-10);
    assert_relative_eq!(result1.rd, result2.rd, epsilon = 1e-10);
    assert_relative_eq!(result1.rd, result3.rd, epsilon = 1e-10);
    assert_relative_eq!(result1.vol, result2.vol, epsilon = 1e-10);
    assert_relative_eq!(result1.vol, result3.vol, epsilon = 1e-10);
}
