use std::collections::HashMap;

use beamng_proto::types::StrDict;

use super::sensor::Sensor;

/// The state sensor monitors general stats of the vehicle:
/// position, direction, velocity, rotation, time.
pub struct State;

impl Sensor for State {
    fn encode_vehicle_request(&self) -> StrDict {
        let mut req = HashMap::new();
        req.insert("type".to_string(), rmpv::Value::from("State"));
        req
    }

    fn decode_response(&self, resp: &StrDict) -> Option<rmpv::Value> {
        resp.get("state").cloned()
    }
}
