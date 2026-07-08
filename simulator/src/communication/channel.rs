const REFERENCE_ALTITUDE_KM: f64 = 500.0;

/// Signal-to-noise estimate (falls off with altitude), GPS coordinate drift (small random walk
/// layered on top of the true position), and a simulated per-tick packet drop probability that
/// rises sharply once SNR degrades.
pub struct CommsChannel {
    pub snr_db: f64,
    lat_drift_deg: f64,
    lon_drift_deg: f64,
}

impl CommsChannel {
    pub fn new() -> Self {
        Self {
            snr_db: 25.0,
            lat_drift_deg: 0.0,
            lon_drift_deg: 0.0,
        }
    }

    pub fn tick(&mut self, altitude_km: f64, rng: &mut impl rand::Rng) {
        let path_loss_db = 20.0 * (altitude_km / REFERENCE_ALTITUDE_KM).log10();
        self.snr_db = (25.0 - path_loss_db + rng.gen_range(-1.5..1.5)).max(0.0);

        self.lat_drift_deg = rng.gen_range(-0.01..0.01);
        self.lon_drift_deg = rng.gen_range(-0.01..0.01);
    }

    /// GPS-jittered position for transmission — the "true" physics state stays clean.
    pub fn apply_gps_drift(&self, latitude_deg: f64, longitude_deg: f64) -> (f64, f64) {
        (
            latitude_deg + self.lat_drift_deg,
            longitude_deg + self.lon_drift_deg,
        )
    }

    /// Whether this tick's telemetry send should be simulated as lost in transit.
    pub fn should_drop_packet(&self, rng: &mut impl rand::Rng) -> bool {
        let drop_probability: f64 = if self.snr_db < 5.0 { 0.35 } else { 0.02 };
        rng.gen_bool(drop_probability)
    }
}
