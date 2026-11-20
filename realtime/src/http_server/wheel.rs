use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http_server::Context;

#[derive(Debug, Deserialize)]
pub struct WheelQuery {
    /// racing number of the driver to follow, defaults to 3
    driver: Option<String>,
}

fn text(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

fn number(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::String(s)) => s.parse().ok(),
        Some(Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

// "1:23.456" or "83.456" -> seconds
fn lap_seconds(v: &str) -> Option<f64> {
    match v.split_once(':') {
        Some((m, s)) => Some(m.parse::<f64>().ok()? * 60.0 + s.parse::<f64>().ok()?),
        None => v.parse().ok(),
    }
}

/// Small summary of the live state for one driver, meant for tiny clients (the steering wheel
/// screen). Polling this every second or so costs well under 1 KB per response.
pub async fn wheel(
    State(ctx): State<Arc<Context>>,
    Query(query): Query<WheelQuery>,
) -> impl IntoResponse {
    let nr = query.driver.unwrap_or_else(|| "3".to_string());

    let state = match ctx.state_service.get_state().await {
        Ok(state) => state,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": format!("Failed to get current state: {}", e) })),
            );
        }
    };

    let lines = state.pointer("/TimingData/Lines").and_then(|v| v.as_object());
    let timing = lines.and_then(|l| l.get(&nr));
    let driver = state.pointer(&format!("/DriverList/{nr}"));

    let cars = lines
        .map(|l| {
            l.values()
                .filter(|v| v.get("Position").is_some_and(|p| p.is_string()))
                .count()
        })
        .unwrap_or(0);

    let leader_best = lines.and_then(|l| {
        l.values()
            .find(|v| v.get("Position").and_then(|p| p.as_str()) == Some("1"))
            .and_then(|v| text(v.pointer("/BestLapTime/Value")))
    });
    let my_best = timing.and_then(|t| text(t.pointer("/BestLapTime/Value")));
    let best_delta = match (
        my_best.as_deref().and_then(lap_seconds),
        leader_best.as_deref().and_then(lap_seconds),
    ) {
        (Some(mine), Some(leader)) => Some((mine - leader) * 1000.0).map(|ms| ms.round() / 1000.0),
        _ => None,
    };

    let stints = state
        .pointer(&format!("/TimingAppData/Lines/{nr}/Stints"))
        .and_then(|v| v.as_array());
    let stint = stints.and_then(|s| s.last());

    let session = state.get("SessionInfo");

    (
        StatusCode::OK,
        axum::Json(json!({
            "live": timing.is_some(),
            "session": session.and_then(|s| text(s.get("Name"))),
            "location": session.and_then(|s| text(s.pointer("/Meeting/Location"))),
            "session_status": text(state.pointer("/SessionStatus/Status")),
            "track_status": text(state.pointer("/TrackStatus/Status")),
            "air_c": number(state.pointer("/WeatherData/AirTemp")),
            "track_c": number(state.pointer("/WeatherData/TrackTemp")),
            "driver": {
                "number": nr,
                "tla": driver.and_then(|d| text(d.get("Tla"))),
                "team_colour": driver.and_then(|d| text(d.get("TeamColour"))),
            },
            "position": timing.and_then(|t| text(t.get("Position"))),
            "cars": cars,
            "lap": timing.and_then(|t| text(t.get("NumberOfLaps"))),
            "total_laps": number(state.pointer("/LapCount/TotalLaps")),
            "tyre": stint.and_then(|s| text(s.get("Compound"))),
            "tyre_laps": stint.and_then(|s| number(s.get("TotalLaps"))),
            "pit_stops": stints.map(|s| s.len().saturating_sub(1)),
            "in_pit": timing.and_then(|t| t.get("InPit")).and_then(|v| v.as_bool()),
            "last_lap": timing.and_then(|t| text(t.pointer("/LastLapTime/Value"))),
            "best_lap": my_best,
            "interval": timing.and_then(|t| text(t.pointer("/IntervalToPositionAhead/Value"))),
            "gap_to_leader": timing.and_then(|t| text(t.get("GapToLeader"))),
            "best_delta_to_p1": best_delta,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::state_service::StateService;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn summarises_one_driver() {
        let state_service = StateService::new();
        state_service
            .set_state(json!({
                "SessionInfo": { "Name": "Race", "Meeting": { "Location": "Las Vegas" } },
                "TrackStatus": { "Status": "1" },
                "WeatherData": { "AirTemp": "15.2", "TrackTemp": "17.0" },
                "LapCount": { "TotalLaps": 50 },
                "DriverList": { "3": { "Tla": "VER", "TeamColour": "3671C6" } },
                "TimingData": { "Lines": {
                    "1": { "Position": "1", "BestLapTime": { "Value": "1:34.000" } },
                    "3": { "Position": "2", "NumberOfLaps": 12, "InPit": false,
                           "LastLapTime": { "Value": "1:35.100" },
                           "BestLapTime": { "Value": "1:34.500" },
                           "IntervalToPositionAhead": { "Value": "+0.800" } }
                } },
                "TimingAppData": { "Lines": { "3": { "Stints": [
                    { "Compound": "MEDIUM", "TotalLaps": 8 },
                    { "Compound": "HARD", "TotalLaps": 4 }
                ] } } }
            }))
            .await
            .unwrap();
        let (tx, _rx) = broadcast::channel::<String>(1);
        let ctx = Arc::new(Context { state_service, tx });

        let res = wheel(State(ctx), Query(WheelQuery { driver: None }))
            .await
            .into_response();
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(v["live"], true);
        assert_eq!(v["position"], "2");
        assert_eq!(v["cars"], 2);
        assert_eq!(v["lap"], "12");
        assert_eq!(v["total_laps"], 50.0);
        assert_eq!(v["tyre"], "HARD");
        assert_eq!(v["pit_stops"], 1);
        assert_eq!(v["best_delta_to_p1"], 0.5);
        assert_eq!(v["driver"]["tla"], "VER");
        assert_eq!(v["air_c"], 15.2);
    }

    #[tokio::test]
    async fn idle_state_is_not_live() {
        let (tx, _rx) = broadcast::channel::<String>(1);
        let ctx = Arc::new(Context { state_service: StateService::new(), tx });
        let res = wheel(State(ctx), Query(WheelQuery { driver: Some("44".into()) }))
            .await
            .into_response();
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["live"], false);
        assert_eq!(v["driver"]["number"], "44");
    }
}
