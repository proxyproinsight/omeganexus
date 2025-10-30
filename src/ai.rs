use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weights {
    pub latency: f64,
    pub country_diversity: f64,
    pub source_quality: f64,
    pub uptime: f64,
    pub fraud_penalty: f64,
    pub leak_penalty: f64,
    pub elite_bonus: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            latency: 0.4,
            country_diversity: 0.15,
            source_quality: 0.25,
            uptime: 0.2,
            fraud_penalty: 0.5,
            leak_penalty: 0.3,
            elite_bonus: 0.15,
        }
    }
}

/// AI-driven proxy quality prediction with EMA smoothing
pub fn predict_score(
    latency_ms: i64,
    country_rare: bool,
    source_quality: f64,
    age_hours: i64,
    fraud_score: f64,
    dns_leak: bool,
    is_elite: bool,
    weights: &Weights,
) -> f64 {
    let mut score = 0.0;

    // Latency component (lower is better)
    let latency_normalized = 1.0 - (latency_ms as f64 / 5000.0).min(1.0);
    score += latency_normalized * weights.latency;

    // Country diversity bonus
    if country_rare {
        score += weights.country_diversity;
    }

    // Source quality component
    score += source_quality * weights.source_quality;

    // Uptime/freshness component (prefer recently validated)
    let uptime_score = if age_hours < 1 {
        1.0
    } else if age_hours < 6 {
        0.8
    } else if age_hours < 24 {
        0.5
    } else {
        0.2
    };
    score += uptime_score * weights.uptime;

    // Fraud penalty
    score -= fraud_score * weights.fraud_penalty;

    // DNS leak penalty
    if dns_leak {
        score -= weights.leak_penalty;
    }

    // Elite anonymity bonus
    if is_elite {
        score += weights.elite_bonus;
    }

    score.max(0.0).min(1.0)
}

/// Heuristic scoring for quick evaluation
pub fn heuristic_score(latency_ms: i64, country: &str) -> f64 {
    let rare_countries = ["IS", "LU", "CH", "SG", "NL", "SE", "NO", "FI"];
    let country_bonus = if rare_countries.contains(&country) {
        0.2
    } else {
        0.0
    };

    let latency_score: f64 = if latency_ms < 100 {
        1.0
    } else if latency_ms < 300 {
        0.8
    } else if latency_ms < 1000 {
        0.5
    } else if latency_ms < 3000 {
        0.3
    } else {
        0.1
    };

    (latency_score + country_bonus).min(1.0)
}

/// Update source quality using Exponential Moving Average
pub fn update_source_quality(current: f64, success_rate: f64, alpha: f64) -> f64 {
    alpha * success_rate + (1.0 - alpha) * current
}

/// Calculate age in hours from timestamp
pub fn age_hours(timestamp: i64) -> i64 {
    let now = Utc::now().timestamp();
    (now - timestamp) / 3600
}
