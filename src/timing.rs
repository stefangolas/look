use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Timings {
    #[serde(flatten)]
    stages_ms: BTreeMap<String, f64>,
}

impl Timings {
    pub fn get(&self, stage: &str) -> Option<f64> {
        self.stages_ms.get(stage).copied()
    }

    pub fn set_ms(&mut self, stage: impl Into<String>, milliseconds: f64) {
        self.stages_ms.insert(stage.into(), milliseconds);
    }

    pub fn record(&mut self, stage: impl Into<String>, elapsed: Duration) {
        self.stages_ms
            .insert(stage.into(), elapsed.as_secs_f64() * 1_000.0);
    }

    pub fn accumulate(&mut self, stage: impl Into<String>, elapsed: Duration) {
        let entry = self.stages_ms.entry(stage.into()).or_insert(0.0);
        *entry += elapsed.as_secs_f64() * 1_000.0;
    }

    pub fn measure<T>(&mut self, stage: impl Into<String>, operation: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let result = operation();
        self.record(stage, started.elapsed());
        result
    }

    pub fn merge(&mut self, other: &Timings) {
        self.stages_ms.extend(
            other
                .stages_ms
                .iter()
                .map(|(key, value)| (key.clone(), *value)),
        );
    }
}
