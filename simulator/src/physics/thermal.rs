const SPACE_TEMP_C: f64 = -270.0;
// Scaled so radiative loss balances against ~0.5-0.7 units/tick of heat input at ~300K,
// i.e. tuned for a believable equilibrium rather than physically exact units.
const RADIATIVE_SCALE: f64 = 7.4e-11;
const AVIONICS_BASELINE_HEAT: f64 = 0.15;
const SOLAR_HEATING: f64 = 0.55;
const THRUSTER_FAULT_HEAT: f64 = 1.2;

/// Simplified radiative heat balance: heat in (solar absorption + baseline avionics load,
/// plus thruster load during a fault) vs. heat out proportional to (T^4 - T_space^4), instead
/// of the previous flat +/- per-tick increments.
pub struct ThermalState {
    pub battery_temp_c: f64,
}

impl ThermalState {
    pub fn new(initial_temp_c: f64) -> Self {
        Self {
            battery_temp_c: initial_temp_c,
        }
    }

    pub fn tick(&mut self, in_sunlight: bool, thruster_fault: bool, rng: &mut impl rand::Rng) {
        let absolute_temp_k = self.battery_temp_c + 273.15;
        let space_temp_k = SPACE_TEMP_C + 273.15;
        let radiative_loss = RADIATIVE_SCALE * (absolute_temp_k.powi(4) - space_temp_k.powi(4));

        let mut heat_in = AVIONICS_BASELINE_HEAT;
        if in_sunlight {
            heat_in += SOLAR_HEATING;
        }
        if thruster_fault {
            heat_in += THRUSTER_FAULT_HEAT;
        }

        self.battery_temp_c += heat_in - radiative_loss + rng.gen_range(-0.02..0.02);
        self.battery_temp_c = self.battery_temp_c.clamp(-40.0, 85.0);
    }
}
