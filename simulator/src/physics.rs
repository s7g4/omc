use rand::Rng;

pub struct SatelliteState {
    pub battery_level: f64,
    pub battery_temp: f64,
    pub solar_power: f64,
    pub velocity: f64,
    pub altitude: f64,
    pub latitude: f64,
    pub longitude: f64,
    time_elapsed: f64,
}

impl SatelliteState {
    pub fn new() -> Self {
        Self {
            battery_level: 90.0,
            battery_temp: 20.0,
            solar_power: 120.0,
            velocity: 7.67,
            altitude: 500.0,
            latitude: 0.0,
            longitude: 0.0,
            time_elapsed: 0.0,
        }
    }

    pub fn tick(&mut self) {
        let mut rng = rand::thread_rng();
        self.time_elapsed += 1.0;

        // 1. Orbit Simulation (Circular LEO path)
        let orbital_freq = 0.001; // Orbital frequency
                                  // Inclination of 51.6 degrees (similar to ISS orbit)
        self.latitude = (self.time_elapsed * orbital_freq).sin() * 51.6;
        self.longitude =
            ((self.time_elapsed * 0.0005).sin() * 180.0 + rng.gen_range(-0.01..0.01)) % 180.0;

        // 2. Solar exposure check (True in sunlight, False in Earth shadow)
        // Cosine wave representing day/night cycles
        let in_sunlight = (self.time_elapsed * 0.001).cos() > -0.3;

        // 3. Power & Thermal simulation
        if in_sunlight {
            self.solar_power = 140.0 + rng.gen_range(-5.0..5.0);
            self.battery_level = (self.battery_level + 0.05).min(100.0);
            self.battery_temp = (self.battery_temp + 0.1).min(45.0);
        } else {
            self.solar_power = 0.0;
            self.battery_level = (self.battery_level - 0.08).max(0.0);
            self.battery_temp = (self.battery_temp - 0.15).max(-10.0);
        }

        // Apply slight thermal noise
        self.battery_temp += rng.gen_range(-0.02..0.02);

        // 4. Minor orbital perturbations
        self.altitude += rng.gen_range(-0.05..0.05);
        self.velocity = 7.67 + (500.0 - self.altitude) * 0.001; // Keplerian approximation (velocity decreases as altitude increases)
    }
}
