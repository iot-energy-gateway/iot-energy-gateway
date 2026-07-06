use rand::Rng;

pub struct SimulatedTelemetry {
    base_load_wh: u64,
}

impl SimulatedTelemetry {
    pub fn new(base_load_wh: u64) -> Self {
        Self { base_load_wh }
    }

    /// Returns a simulated watt-hour consumption for one polling interval.
    pub fn sample(&mut self) -> u64 {
        let mut rng = rand::thread_rng();
        let jitter: u64 = rng.gen_range(0..=self.base_load_wh / 2);
        self.base_load_wh + jitter
    }
}
