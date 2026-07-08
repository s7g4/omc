pub mod orbital;
pub mod power;
pub mod thermal;

use orbital::OrbitalState;
use power::PowerState;
use thermal::ThermalState;

/// Facade over the orbital/thermal/power submodels, exposing the same flat field set the
/// telemetry payload needs (`battery_level`, `battery_temp`, `solar_power`, `velocity`,
/// `altitude`, `latitude`, `longitude`) so callers don't need to reach into each submodel.
pub struct SatelliteState {
    orbital: OrbitalState,
    thermal: ThermalState,
    power: PowerState,
    time_elapsed: f64,

    pub battery_level: f64,
    pub battery_temp: f64,
    pub solar_power: f64,
    pub velocity: f64,
    pub altitude: f64,
    pub latitude: f64,
    pub longitude: f64,
}

impl SatelliteState {
    pub fn new() -> Self {
        let orbital = OrbitalState::new(500.0, 51.6); // ISS-like altitude/inclination
        let thermal = ThermalState::new(20.0);
        let power = PowerState::new(90.0);

        let mut state = Self {
            battery_level: power.battery_level_pct,
            battery_temp: thermal.battery_temp_c,
            solar_power: power.solar_power_w,
            velocity: orbital.velocity_km_s,
            altitude: orbital.altitude_km,
            latitude: orbital.latitude_deg,
            longitude: orbital.longitude_deg,
            orbital,
            thermal,
            power,
            time_elapsed: 0.0,
        };
        state.sync_public_fields();
        state
    }

    pub fn tick(&mut self, active_fault: Option<&str>) {
        let mut rng = rand::thread_rng();
        self.time_elapsed += 1.0;

        let orbit_decay_fault = active_fault == Some("orbit_decay");
        let altitude_decay_km = if orbit_decay_fault { 2.0 } else { 0.01 };
        self.orbital.tick(1.0, altitude_decay_km);

        let in_sunlight = self.orbital.is_in_sunlight();
        let sun_angle = self.orbital.sun_relative_angle();

        let thruster_fault = active_fault == Some("thruster_overheat");
        self.thermal.tick(in_sunlight, thruster_fault, &mut rng);

        let solar_fault_multiplier = if active_fault == Some("solar_degrade") {
            0.15
        } else {
            1.0
        };
        self.power.tick(
            in_sunlight,
            sun_angle,
            solar_fault_multiplier,
            self.time_elapsed,
            &mut rng,
        );

        if active_fault == Some("battery_drain") {
            self.power.apply_drain(1.5);
        }

        self.sync_public_fields();
    }

    fn sync_public_fields(&mut self) {
        self.battery_level = self.power.battery_level_pct;
        self.battery_temp = self.thermal.battery_temp_c;
        self.solar_power = self.power.solar_power_w;
        self.velocity = self.orbital.velocity_km_s;
        self.altitude = self.orbital.altitude_km;
        self.latitude = self.orbital.latitude_deg;
        self.longitude = self.orbital.longitude_deg;
    }
}
