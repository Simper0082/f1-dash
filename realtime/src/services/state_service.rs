use anyhow::Error;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

use super::pit_tracker::{PitRecord, PitTracker};

#[derive(Clone)]
pub struct StateService {
    state: Arc<RwLock<Value>>,
    pits: Arc<Mutex<PitTracker>>,
}

impl StateService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(Value::Object(serde_json::Map::new()))),
            pits: Arc::new(Mutex::new(PitTracker::default())),
        }
    }

    pub async fn get_state(&self) -> Result<Value, Error> {
        let state = self.state.read().await;
        Ok(state.clone())
    }

    pub async fn get_state_string(&self) -> Result<String, Error> {
        let state = self.state.read().await;
        Ok(state.to_string())
    }

    pub async fn set_state(&self, new_state: Value) -> Result<(), Error> {
        let mut state = self.state.write().await;
        *state = new_state;
        Ok(())
    }

    /// feed timestamp of every update, used to time pit stops
    pub fn note_update(&self, topic: &str, data: &Value, ts: &str) {
        if let Ok(mut p) = self.pits.lock() {
            p.note(topic, data, ts);
        }
    }

    pub fn pit_snapshot(&self, nr: &str) -> (Option<f64>, PitRecord) {
        let p = self.pits.lock().unwrap();
        (p.now, p.drivers.get(nr).cloned().unwrap_or_default())
    }

    pub async fn update_state(&self, update: Value) -> Result<(), Error> {
        let mut state = self.state.write().await;
        merge(&mut state, update);
        Ok(())
    }
}

pub fn merge(base: &mut Value, update: Value) {
    match (base, update) {
        (Value::Object(prev), Value::Object(update)) => {
            for (k, v) in update {
                merge(prev.entry(k).or_insert(Value::Null), v);
            }
        }
        (Value::Array(prev), Value::Object(update)) => {
            for (k, v) in update {
                if let Ok(index) = k.parse::<usize>() {
                    if let Some(item) = prev.get_mut(index) {
                        merge(item, v);
                    } else {
                        prev.push(v);
                    }
                }
            }
        }
        (a, b) => *a = b,
    }
}
