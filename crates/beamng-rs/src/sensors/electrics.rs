use std::collections::HashMap;

use beamng_proto::types::StrDict;

use super::sensor::Sensor;

/// Sensor for retrieving vehicle electrics values (RPM, speed, lights, etc.).
pub struct Electrics;

/// Decoded electrics data (raw string-keyed map).
pub type ElectricsData = StrDict;

/// Mapping from BeamNG internal names to normalized Rust-style names.
const NAME_MAP: &[(&str, &str)] = &[
    ("absActive", "abs_active"),
    ("avgWheelAV", "avg_wheel_av"),
    ("brakelights", "brake_lights"),
    ("checkengine", "check_engine"),
    ("clutchRatio", "clutch_ratio"),
    ("engineLoad", "engine_load"),
    ("engineThrottle", "engine_throttle"),
    ("escActive", "esc_active"),
    ("exhaustFlow", "exhaust_flow"),
    ("fog", "fog_lights"),
    ("fuelVolume", "fuel_volume"),
    ("fuelCapacity", "fuel_capacity"),
    ("gear_A", "gear_a"),
    ("gearIndex", "gear_index"),
    ("gear_M", "gear_m"),
    ("hazard_enabled", "hazard_signal"),
    ("isShifting", "is_shifting"),
    ("lights_state", "headlights"),
    ("oiltemp", "oil_temperature"),
    ("radiatorFanSpin", "radiator_fan_spin"),
    ("rpmTacho", "rpm_tacho"),
    ("signal_L", "signal_l"),
    ("signal_left_input", "left_signal"),
    ("signal_R", "signal_r"),
    ("signal_right_input", "right_signal"),
    ("tcsActive", "tcs_active"),
    ("throttleFactor", "throttle_factor"),
    ("twoStep", "two_step"),
    ("watertemp", "water_temperature"),
];

fn rename_values(mut vals: StrDict) -> StrDict {
    for &(from, to) in NAME_MAP {
        if let Some(v) = vals.remove(from) {
            vals.insert(to.to_string(), v);
        }
    }
    vals
}

impl Sensor for Electrics {
    fn encode_vehicle_request(&self) -> StrDict {
        let mut req = HashMap::new();
        req.insert("type".to_string(), rmpv::Value::from("Electrics"));
        req
    }

    fn decode_response(&self, resp: &StrDict) -> Option<rmpv::Value> {
        let values = resp.get("values")?;
        if let Some(map) = values.as_map() {
            let mut dict = HashMap::new();
            for (k, v) in map {
                if let Some(key) = k.as_str() {
                    dict.insert(key.to_string(), v.clone());
                }
            }
            let renamed = rename_values(dict);
            let pairs: Vec<(rmpv::Value, rmpv::Value)> = renamed
                .into_iter()
                .map(|(k, v)| (rmpv::Value::from(k), v))
                .collect();
            Some(rmpv::Value::Map(pairs))
        } else {
            None
        }
    }
}
