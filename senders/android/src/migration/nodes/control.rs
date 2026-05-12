use crate::migration::protocol::{ControlMode, ControlPoint};
use chrono::{DateTime, Utc};
use serde_json::Value;

fn interpolate_values(start: &Value, end: &Value, ratio: f64) -> Option<Value> {
    let start = start.as_f64()?;
    let end = end.as_f64()?;
    Some(Value::from(start + (end - start) * ratio))
}

/// Evaluates control points at `at` using old-node semantics:
/// - `set`: use the latest point at/before time.
/// - `interpolate`: linearly interpolate to the next point when both are numeric.
pub fn evaluate_control_points(points: &[ControlPoint], at: DateTime<Utc>) -> Option<Value> {
    if points.is_empty() {
        return None;
    }

    let mut before: Option<&ControlPoint> = None;
    let mut after: Option<&ControlPoint> = None;
    for point in points {
        if point.time <= at {
            if before.is_none_or(|candidate| point.time >= candidate.time) {
                before = Some(point);
            }
        } else if after.is_none_or(|candidate| point.time < candidate.time) {
            after = Some(point);
        }
    }

    let current = before.or(after)?;

    if current.mode == ControlMode::Interpolate {
        if let Some(next) = after {
            let total_ms = (next.time - current.time).num_milliseconds();
            if total_ms > 0 {
                let elapsed_ms = (at - current.time).num_milliseconds().max(0);
                let ratio = (elapsed_ms as f64 / total_ms as f64).clamp(0.0, 1.0);
                if let Some(value) = interpolate_values(&current.value, &next.value, ratio) {
                    return Some(value);
                }
            }
        }
    }

    Some(current.value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    fn point(id: &str, time: DateTime<Utc>, value: Value, mode: ControlMode) -> ControlPoint {
        ControlPoint {
            id: id.to_string(),
            time,
            value,
            mode,
        }
    }

    #[test]
    fn evaluate_returns_none_for_empty_points() {
        assert!(evaluate_control_points(&[], Utc::now()).is_none());
    }

    #[test]
    fn evaluate_set_mode_uses_latest_point_at_or_before_timestamp() {
        let now = Utc::now();
        let points = vec![
            point(
                "a",
                now - Duration::seconds(10),
                json!(0.1),
                ControlMode::Set,
            ),
            point(
                "b",
                now - Duration::seconds(2),
                json!(0.5),
                ControlMode::Set,
            ),
            point(
                "c",
                now + Duration::seconds(10),
                json!(0.9),
                ControlMode::Set,
            ),
        ];

        let value = evaluate_control_points(&points, now).unwrap();
        assert_eq!(value, json!(0.5));
    }

    #[test]
    fn evaluate_before_first_point_returns_first_value() {
        let now = Utc::now();
        let points = vec![
            point("a", now + Duration::seconds(3), json!(7), ControlMode::Set),
            point("b", now + Duration::seconds(5), json!(9), ControlMode::Set),
        ];

        let value = evaluate_control_points(&points, now).unwrap();
        assert_eq!(value, json!(7));
    }

    #[test]
    fn evaluate_interpolates_numeric_values() {
        let now = Utc::now();
        let points = vec![
            point(
                "a",
                now - Duration::seconds(10),
                json!(10.0),
                ControlMode::Interpolate,
            ),
            point(
                "b",
                now + Duration::seconds(10),
                json!(30.0),
                ControlMode::Set,
            ),
        ];

        let value = evaluate_control_points(&points, now).unwrap();
        let interpolated = value.as_f64().unwrap();
        assert!((interpolated - 20.0).abs() < 0.001);
    }

    #[test]
    fn evaluate_interpolate_with_non_numeric_values_falls_back_to_current() {
        let now = Utc::now();
        let points = vec![
            point(
                "a",
                now - Duration::seconds(1),
                json!("left"),
                ControlMode::Interpolate,
            ),
            point(
                "b",
                now + Duration::seconds(1),
                json!("right"),
                ControlMode::Set,
            ),
        ];

        let value = evaluate_control_points(&points, now).unwrap();
        assert_eq!(value, json!("left"));
    }

    #[test]
    fn evaluate_interpolate_with_same_timestamp_does_not_divide_by_zero() {
        let now = Utc::now();
        let points = vec![point(
            "a",
            now + Duration::seconds(1),
            json!(42.0),
            ControlMode::Interpolate,
        )];

        let value = evaluate_control_points(&points, now).unwrap();
        assert_eq!(value, json!(42.0));
    }
}
