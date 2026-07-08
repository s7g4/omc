const BASE_PANEL_OUTPUT_W: f64 = 160.0;
const MAX_CAPACITY_FADE: f64 = 0.15;

/// Solar output follows a cosine sun-angle law (panel output falls off as the angle between
/// the panel normal and the sun grows) gated by eclipse, rather than independent noise. Battery
/// charge/discharge is coupled to that output, with a slow capacity-fade factor over mission
/// time standing in for cell degradation.
pub struct PowerState {
    pub solar_power_w: f64,
    pub battery_level_pct: f64,
    capacity_fade: f64,
}

impl PowerState {
    pub fn new(initial_battery_pct: f64) -> Self {
        Self {
            solar_power_w: 0.0,
            battery_level_pct: initial_battery_pct,
            capacity_fade: 0.0,
        }
    }

    pub fn tick(
        &mut self,
        in_sunlight: bool,
        sun_angle_rad: f64,
        solar_fault_multiplier: f64,
        time_elapsed_secs: f64,
        rng: &mut impl rand::Rng,
    ) {
        self.capacity_fade = (time_elapsed_secs / 1_000_000.0).min(MAX_CAPACITY_FADE);

        if in_sunlight {
            let angle_factor = sun_angle_rad.cos().max(0.0);
            self.solar_power_w = (BASE_PANEL_OUTPUT_W
                * angle_factor
                * (1.0 - self.capacity_fade)
                * solar_fault_multiplier
                + rng.gen_range(-3.0..3.0))
            .max(0.0);

            let charge_efficiency = 1.0 - self.capacity_fade;
            self.battery_level_pct = (self.battery_level_pct + 0.05 * charge_efficiency).min(100.0);
        } else {
            self.solar_power_w = 0.0;
            self.battery_level_pct = (self.battery_level_pct - 0.08).max(0.0);
        }
    }

    pub fn apply_drain(&mut self, amount_pct: f64) {
        self.battery_level_pct = (self.battery_level_pct - amount_pct).max(0.0);
    }
}
