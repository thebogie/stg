// Minimal, clear Glicko-2 implementation (per Mark Glickman)
// Rating scale uses default μ=1500, φ (RD)=350, volatility τ=0.06

#[derive(Debug, Clone, Copy)]
pub struct Glicko2Params {
    pub default_rating: f64,    // typically 1500
    pub default_rd: f64,        // typically 350
    pub default_vol: f64,       // typically 0.06
    pub tau: f64,               // volatility constraint, 0.5–1.2; we’ll use 0.5
}

impl Default for Glicko2Params {
    fn default() -> Self {
        Self {
            default_rating: 1500.0,
            default_rd: 350.0,
            default_vol: 0.06,
            tau: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RatingState {
    pub rating: f64,
    pub rd: f64,
    pub vol: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct OpponentSample {
    pub opp_rating: f64,
    pub opp_rd: f64,
    pub score: f64, // 1.0 win, 0.5 draw, 0.0 loss
    pub weight: f64,
}

// Helpers per Glicko-2 formulas
fn g(phi: f64) -> f64 {
    1.0 / (1.0 + 3.0 * phi.powi(2) / std::f64::consts::PI.powi(2)).sqrt()
}

fn e(mu: f64, mu_j: f64, phi_j: f64) -> f64 {
    1.0 / (1.0 + (-g(phi_j) * (mu - mu_j)).exp())
}

fn to_mu(r: f64) -> f64 { (r - 1500.0) / 173.7178 }
fn to_phi(rd: f64) -> f64 { rd / 173.7178 }
fn from_mu(mu: f64) -> f64 { mu * 173.7178 + 1500.0 }
fn from_phi(phi: f64) -> f64 { phi * 173.7178 }

pub fn pre_period_inflate_rd(state: RatingState, periods_inactive: f64) -> RatingState {
    // Simple inactivity inflation per period: φ' = sqrt(φ^2 + σ^2 * t)
    let phi = to_phi(state.rd);
    let inflated_phi = (phi.powi(2) + state.vol.powi(2) * periods_inactive).sqrt();
    RatingState { rd: from_phi(inflated_phi), ..state }
}

// Core Glicko-2 update for a batch vs multiple opponents (single period)
pub fn update_period(
    current: RatingState,
    samples: &[OpponentSample],
    params: Glicko2Params,
) -> RatingState {
    if samples.is_empty() {
        return current;
    }

    let mu = to_mu(current.rating);
    let phi = to_phi(current.rd);
    let sigma = current.vol;

    // Compute variance v and delta
    let mut v_inv = 0.0;
    let mut delta_num = 0.0;
    for s in samples.iter() {
        if s.weight <= 0.0 { continue; }
        let mu_j = to_mu(s.opp_rating);
        let phi_j = to_phi(s.opp_rd);
        let g_phi = g(phi_j);
        let e_val = e(mu, mu_j, phi_j);
        v_inv += s.weight * (g_phi * g_phi * e_val * (1.0 - e_val));
        delta_num += s.weight * g_phi * (s.score - e_val);
    }
    if v_inv <= 0.0 {
        return current;
    }
    let v = 1.0 / v_inv;
    let delta = v * delta_num;

    // Volatility update via iterative method (simplified, few iterations)
    let a = (sigma * sigma).ln();
    let tau = params.tau;
    let mut a_low = a - 10.0;
    let mut a_high = a + 10.0;
    let f = |x: f64| {
        let ex = (x).exp();
        let phi_sq = phi * phi;
        let top = ex * (delta * delta - phi_sq - v - ex);
        let bot = 2.0 * (phi_sq + v + ex) * (phi_sq + v + ex);
        (top / bot) - ((x - a) / (tau * tau))
    };

    // Bisection
    for _ in 0..30 {
        let mid = (a_low + a_high) / 2.0;
        let f_low = f(a_low);
        let f_mid = f(mid);
        if f_mid * f_low < 0.0 { a_high = mid; } else { a_low = mid; }
        if (a_high - a_low).abs() < 1e-6 { break; }
    }
    let a_new = (a_low + a_high) / 2.0;
    // Per Glickman: sigma' = exp(a'/2)
    let sigma_prime = (a_new / 2.0).exp();

    // New pre-rating φ* combining old φ and volatility
    let phi_star = (phi.powi(2) + sigma_prime.powi(2)).sqrt();
    // New φ
    let phi_prime = 1.0 / (1.0 / (phi_star * phi_star) + 1.0 / v).sqrt();
    // New μ
    let mu_prime = mu + phi_prime * phi_prime * delta_num;

    RatingState {
        rating: from_mu(mu_prime),
        rd: from_phi(phi_prime),
        vol: sigma_prime,
    }
}


