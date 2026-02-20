use thiserror::Error;

/// Errors that can occur when communicating with BeamNG.tech.
#[derive(Debug, Error)]
pub enum BngError {
    /// An error reported by the simulator (`bngError` field in response).
    #[error("Simulator error: {0}")]
    SimulatorError(String),

    /// A value error reported by the simulator (`bngValueError` field in response).
    #[error("Value error: {0}")]
    ValueError(String),

    /// The connection to the simulator was lost or not established.
    #[error("Disconnected: {0}")]
    Disconnected(String),

    /// Protocol version mismatch between client and simulator.
    #[error("Protocol mismatch: {0}")]
    ProtocolMismatch(String),

    /// Unexpected response type from the simulator.
    #[error("Unexpected response type: expected \"{expected}\", got \"{got}\"")]
    UnexpectedResponseType { expected: String, got: String },

    /// A response was missing the `_id` field.
    #[error("Invalid message: missing _id field. The version of BeamNG.tech may be incompatible.")]
    MissingId,

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// MessagePack encoding error.
    #[error("Msgpack encode error: {0}")]
    MsgpackEncode(#[from] rmp_serde::encode::Error),

    /// MessagePack decoding error.
    #[error("Msgpack decode error: {0}")]
    MsgpackDecode(#[from] rmp_serde::decode::Error),

    /// Timeout waiting for a response.
    #[error("Timeout: {0}")]
    Timeout(String),
}

pub type Result<T> = std::result::Result<T, BngError>;
