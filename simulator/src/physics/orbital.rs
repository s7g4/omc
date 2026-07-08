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
