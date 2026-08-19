//! Exponential time-decay weighting for co-change events per GRAPH-02.
//!
//! Uses an exponential decay model: `weight = 2^(-(t_ref - t_event) / half_life)`.
//! Events at the reference epoch receive weight 1.0, events from `half_life_days` ago
//! receive weight 0.5, and so on.

const SECONDS_PER_DAY: f64 = 86_400.0;

/// Compute exponential time-decay weight for a co-change event.
///
/// # Arguments
/// * `event_epoch` — Unix timestamp (seconds) of the co-change event
/// * `reference_epoch` — Unix timestamp of the reference point (typically HEAD)
/// * `half_life_days` — Number of days for the weight to halve
///
/// # Returns
/// A weight in `[0.0, 1.0]`. Events at the reference epoch return 1.0.
/// Events `half_life_days` in the past return 0.5.
///
/// # Panics
/// Panics if `half_life_days <= 0.0`.
pub fn time_decay_weight(event_epoch: i64, reference_epoch: i64, half_life_days: f64) -> f64 {
    assert!(
        half_life_days > 0.0,
        "half_life_days must be positive, got {half_life_days}"
    );

    let age_seconds = (reference_epoch - event_epoch) as f64;
    if age_seconds <= 0.0 {
        // Future or same-time events get full weight
        return 1.0;
    }

    let half_life_seconds = half_life_days * SECONDS_PER_DAY;
    let exponent = -(age_seconds / half_life_seconds);

    // weight = 2^exponent = e^(exponent * ln(2))
    let weight = (exponent * std::f64::consts::LN_2).exp();
    weight.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REF_EPOCH: i64 = 1_700_000_000; // ~Nov 2023

    #[test]
    fn test_same_time_returns_one() {
        let w = time_decay_weight(REF_EPOCH, REF_EPOCH, 180.0);
        assert!((w - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_future_event_returns_one() {
        let w = time_decay_weight(REF_EPOCH + 86_400, REF_EPOCH, 180.0);
        assert!((w - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_half_life_returns_half() {
        let half_life_days = 180.0;
        let event = REF_EPOCH - (half_life_days * 86_400.0) as i64;
        let w = time_decay_weight(event, REF_EPOCH, half_life_days);
        assert!(
            (w - 0.5).abs() < 1e-10,
            "Expected 0.5 at half-life, got {w}"
        );
    }

    #[test]
    fn test_two_half_lives_returns_quarter() {
        let half_life_days = 180.0;
        let event = REF_EPOCH - (2.0 * half_life_days * 86_400.0) as i64;
        let w = time_decay_weight(event, REF_EPOCH, half_life_days);
        assert!(
            (w - 0.25).abs() < 1e-10,
            "Expected 0.25 at two half-lives, got {w}"
        );
    }

    #[test]
    fn test_very_old_event_near_zero() {
        let half_life_days = 30.0;
        let event = REF_EPOCH - (365 * 86_400); // 1 year ago with 30-day half-life
        let w = time_decay_weight(event, REF_EPOCH, half_life_days);
        assert!(w < 0.01, "Expected near-zero for very old event, got {w}");
    }

    #[test]
    #[should_panic(expected = "half_life_days must be positive")]
    fn test_zero_half_life_panics() {
        time_decay_weight(REF_EPOCH, REF_EPOCH, 0.0);
    }
}
