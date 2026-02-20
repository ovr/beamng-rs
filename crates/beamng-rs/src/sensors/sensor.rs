use beamng_proto::types::StrDict;

/// Trait for vehicle sensors that can encode requests and decode responses.
pub trait Sensor: Send + Sync {
    /// Encode a request to be sent over the vehicle connection.
    fn encode_vehicle_request(&self) -> StrDict;

    /// Decode a response from the vehicle connection.
    fn decode_response(&self, resp: &StrDict) -> Option<rmpv::Value>;
}
