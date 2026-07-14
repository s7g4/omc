use std::f64::consts::PI;

const EARTH_MU_KM3_S2: f64 = 398_600.441_8;
const EARTH_RADIUS_KM: f64 = 6371.0;
const SIDEREAL_DAY_SECS: f64 = 86_164.0;
const REENTRY_FLOOR_KM: f64 = 120.0;

/// Near-circular orbit propagation. Velocity comes from the vis-viva equation
/// (`v = sqrt(mu/r)` for a circular orbit) instead of a linear altitude->velocity guess,
/// and ground track (lat/lon) comes from the standard circular-orbit ground-track formulas
/// (argument of latitude + inclination + Earth rotation), not independent sinusoids.
pub struct OrbitalState {
    pub altitude_km: f64,
    pub velocity_km_s: f64,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    argument_of_latitude_rad: f64,
    inclination_rad: f64,
    ascending_node_deg: f64,
    time_elapsed_secs: f64,
}

impl OrbitalState {
    pub fn new(initial_altitude_km: f64, inclination_deg: f64) -> Self {
        Self {
            altitude_km: initial_altitude_km,
            velocity_km_s: Self::circular_velocity(initial_altitude_km),
            latitude_deg: 0.0,
            longitude_deg: 0.0,
            argument_of_latitude_rad: 0.0,
            inclination_rad: inclination_deg.to_radians(),
            ascending_node_deg: 0.0,
            time_elapsed_secs: 0.0,
        }
    }

    fn orbital_radius_km(&self) -> f64 {
        EARTH_RADIUS_KM + self.altitude_km
    }

    fn circular_velocity(altitude_km: f64) -> f64 {
        (EARTH_MU_KM3_S2 / (EARTH_RADIUS_KM + altitude_km)).sqrt()
    }

    fn orbital_period_secs(&self) -> f64 {
        2.0 * PI * (self.orbital_radius_km().powi(3) / EARTH_MU_KM3_S2).sqrt()
    }

    /// Advances the orbit by `dt_secs`. `altitude_decay_km` models atmospheric drag
    /// (positive = altitude lost this tick).
    pub fn tick(&mut self, dt_secs: f64, altitude_decay_km: f64) {
        self.time_elapsed_secs += dt_secs;

        let mean_motion = 2.0 * PI / self.orbital_period_secs();
        self.argument_of_latitude_rad =
            (self.argument_of_latitude_rad + mean_motion * dt_secs) % (2.0 * PI);

        self.altitude_km = (self.altitude_km - altitude_decay_km).max(REENTRY_FLOOR_KM);
        self.velocity_km_s = Self::circular_velocity(self.altitude_km);

        let u = self.argument_of_latitude_rad;
        self.latitude_deg = (self.inclination_rad.sin() * u.sin()).asin().to_degrees();

        let raw_lon_rad = (self.inclination_rad.cos() * u.sin()).atan2(u.cos());
        let earth_rotation_deg = (self.time_elapsed_secs / SIDEREAL_DAY_SECS) * 360.0;
        let lon = raw_lon_rad.to_degrees() + self.ascending_node_deg - earth_rotation_deg;
        self.longitude_deg = ((lon + 180.0).rem_euclid(360.0)) - 180.0;
    }

    /// Argument of latitude, also used as the sun-relative angle for the eclipse/power model
    /// (the sun direction is fixed at angle 0 in this simplified model).
    pub fn sun_relative_angle(&self) -> f64 {
        self.argument_of_latitude_rad
    }

    pub fn is_in_sunlight(&self) -> bool {
        self.argument_of_latitude_rad.cos() > -0.3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iss_altitude_gives_iss_like_velocity() {
        // The ISS orbits at ~400-420km and ~7.66 km/s. Sanity-checks the vis-viva
        // implementation against a real, known reference point rather than just "it runs."
        let velocity = OrbitalState::circular_velocity(400.0);
        assert!(
            (7.6..7.7).contains(&velocity),
            "expected ISS-like velocity ~7.66 km/s at 400km, got {velocity}"
        );
    }

    #[test]
    fn higher_orbits_are_slower() {
        // Vis-viva: velocity decreases monotonically with altitude for a circular orbit.
        let low = OrbitalState::circular_velocity(300.0);
        let high = OrbitalState::circular_velocity(1000.0);
        assert!(
            low > high,
            "a 300km orbit should be faster than a 1000km orbit"
        );
    }

    #[test]
    fn atmospheric_drag_decreases_altitude_and_never_goes_below_reentry_floor() {
        let mut state = OrbitalState::new(125.0, 51.6);
        for _ in 0..100 {
            state.tick(1.0, 10.0); // aggressive decay
        }
        assert!(
            state.altitude_km >= REENTRY_FLOOR_KM,
            "altitude should clamp at the reentry floor, got {}",
            state.altitude_km
        );
    }

    #[test]
    fn tick_advances_argument_of_latitude() {
        let mut state = OrbitalState::new(500.0, 51.6);
        let before = state.argument_of_latitude_rad;
        state.tick(60.0, 0.0);
        assert_ne!(before, state.argument_of_latitude_rad);
    }
}
