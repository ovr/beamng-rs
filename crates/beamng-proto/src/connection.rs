use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::error::{BngError, Result};
use crate::frame::{read_frame, write_frame};
use crate::types::{value_as_str, value_as_u64, value_to_str_dict, StrDict};

/// The protocol version this client speaks.
pub const PROTOCOL_VERSION: &str = "v1.26";

/// A connection to a BeamNG.tech instance.
///
/// Handles TCP framing, msgpack serialization, hello handshake,
/// and request/response correlation via `_id` fields.
pub struct Connection {
    reader: Mutex<ReadHalf<TcpStream>>,
    writer: Mutex<WriteHalf<TcpStream>>,
    req_id: AtomicU64,
    /// Buffer for out-of-order responses (keyed by their `_id`).
    buffered: Mutex<HashMap<u64, ResponsePayload>>,
}

/// The payload of a successfully received response, or an error from the sim.
#[derive(Debug)]
enum ResponsePayload {
    Ok(StrDict),
    SimError(BngError),
}

impl Connection {
    /// Establish a TCP connection to BeamNG.tech and perform the hello handshake.
    pub async fn open(host: &str, port: u16) -> Result<Self> {
        let addr = format!("{host}:{port}");
        info!("Connecting to BeamNG.tech at {addr}");
        let stream = TcpStream::connect(&addr).await?;
        stream.set_nodelay(true)?;

        let (reader, writer) = tokio::io::split(stream);
        let conn = Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
            req_id: AtomicU64::new(0),
            buffered: Mutex::new(HashMap::new()),
        };

        conn.hello().await?;
        info!("Successfully connected to BeamNG.tech");
        Ok(conn)
    }

    /// Create a connection from an already-connected TCP stream and perform hello.
    pub async fn from_stream(stream: TcpStream) -> Result<Self> {
        stream.set_nodelay(true)?;
        let (reader, writer) = tokio::io::split(stream);
        let conn = Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
            req_id: AtomicU64::new(0),
            buffered: Mutex::new(HashMap::new()),
        };

        conn.hello().await?;
        Ok(conn)
    }

    /// Perform the Hello handshake, verifying protocol version.
    async fn hello(&self) -> Result<()> {
        let resp = self
            .request(
                "Hello",
                &[("protocolVersion", rmpv::Value::from(PROTOCOL_VERSION))],
            )
            .await?;

        let version = resp
            .get("protocolVersion")
            .and_then(|v| value_as_str(v))
            .unwrap_or("");

        if version != PROTOCOL_VERSION {
            return Err(BngError::ProtocolMismatch(format!(
                "BeamNGpy's is: {PROTOCOL_VERSION}, BeamNG.tech's is: {version}"
            )));
        }

        // Verify the response type is Hello
        let resp_type = resp.get("type").and_then(|v| value_as_str(v)).unwrap_or("");
        if resp_type != "Hello" {
            return Err(BngError::UnexpectedResponseType {
                expected: "Hello".into(),
                got: resp_type.into(),
            });
        }

        Ok(())
    }

    /// Allocate the next request ID.
    fn next_id(&self) -> u64 {
        self.req_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Send a request and wait for the correlated response.
    ///
    /// The `req_type` becomes the `"type"` field.
    /// Additional fields are passed as `fields`.
    pub async fn request(&self, req_type: &str, fields: &[(&str, rmpv::Value)]) -> Result<StrDict> {
        let req_id = self.send_raw(req_type, fields).await?;
        self.recv(req_id).await
    }

    /// Send a request and return the assigned request ID without waiting for a response.
    pub async fn send_raw(
        &self,
        req_type: &str,
        fields: &[(&str, rmpv::Value)],
    ) -> Result<u64> {
        let req_id = self.next_id();

        let mut pairs: Vec<(rmpv::Value, rmpv::Value)> =
            Vec::with_capacity(fields.len() + 2);
        pairs.push((
            rmpv::Value::from("type"),
            rmpv::Value::from(req_type),
        ));
        pairs.push((
            rmpv::Value::from("_id"),
            rmpv::Value::from(req_id),
        ));
        for (k, v) in fields {
            pairs.push((rmpv::Value::from(*k), v.clone()));
        }

        let msg = rmpv::Value::Map(pairs);
        let mut packed = Vec::new();
        rmpv::encode::write_value(&mut packed, &msg)
            .map_err(|e| BngError::Io(std::io::Error::other(e)))?;
        debug!("Sending {req_type} (id={req_id})");

        let mut writer = self.writer.lock().await;
        write_frame(&mut *writer, &packed).await?;

        Ok(req_id)
    }

    /// Wait for a response with the given request ID.
    ///
    /// If a response with a different ID arrives, it is buffered for later retrieval.
    pub async fn recv(&self, req_id: u64) -> Result<StrDict> {
        // Check the buffer first.
        {
            let mut buffered = self.buffered.lock().await;
            if let Some(payload) = buffered.remove(&req_id) {
                return match payload {
                    ResponsePayload::Ok(dict) => Ok(dict),
                    ResponsePayload::SimError(e) => Err(e),
                };
            }
        }

        // Read frames until we find ours.
        loop {
            let dict = self.read_one_message().await?;
            let msg_id = dict
                .get("_id")
                .and_then(value_as_u64)
                .ok_or(BngError::MissingId)?;

            let payload = Self::check_sim_error(&dict);

            if msg_id == req_id {
                return match payload {
                    Some(e) => Err(e),
                    None => Ok(dict),
                };
            }

            // Buffer out-of-order message.
            let stored = match payload {
                Some(e) => ResponsePayload::SimError(e),
                None => ResponsePayload::Ok(dict),
            };
            self.buffered.lock().await.insert(msg_id, stored);
        }
    }

    /// Send a typed request and verify the response type matches (ack pattern).
    pub async fn ack(
        &self,
        req_type: &str,
        ack_type: &str,
        fields: &[(&str, rmpv::Value)],
    ) -> Result<()> {
        let resp = self.request(req_type, fields).await?;
        let got = resp
            .get("type")
            .and_then(|v| value_as_str(v))
            .unwrap_or("");
        if got != ack_type {
            return Err(BngError::UnexpectedResponseType {
                expected: ack_type.into(),
                got: got.into(),
            });
        }
        Ok(())
    }

    /// High-level message helper: sends a typed request with kwargs,
    /// checks response type matches, and returns the `"result"` field if present.
    pub async fn message(
        &self,
        req_type: &str,
        fields: &[(&str, rmpv::Value)],
    ) -> Result<Option<rmpv::Value>> {
        let resp = self.request(req_type, fields).await?;
        let resp_type = resp
            .get("type")
            .and_then(|v| value_as_str(v))
            .unwrap_or("");
        if resp_type != req_type {
            return Err(BngError::UnexpectedResponseType {
                expected: req_type.into(),
                got: resp_type.into(),
            });
        }
        Ok(resp.get("result").cloned())
    }

    /// Read and decode one msgpack message from the wire.
    async fn read_one_message(&self) -> Result<StrDict> {
        let mut reader = self.reader.lock().await;
        let data = read_frame(&mut *reader).await?;
        drop(reader);

        let value = rmpv::decode::read_value(&mut &data[..])
            .map_err(|e| BngError::Io(std::io::Error::other(e)))?;
        debug!("Received: {:?}", value);

        value_to_str_dict(value).ok_or(BngError::MissingId)
    }

    /// Check if a response dict contains a simulator error.
    fn check_sim_error(dict: &StrDict) -> Option<BngError> {
        if let Some(val) = dict.get("bngError") {
            let msg = val.as_str().unwrap_or("unknown error").to_string();
            return Some(BngError::SimulatorError(msg));
        }
        if let Some(val) = dict.get("bngValueError") {
            let msg = val.as_str().unwrap_or("unknown value error").to_string();
            return Some(BngError::ValueError(msg));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    fn encode(val: &rmpv::Value) -> Vec<u8> {
        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, val).unwrap();
        buf
    }

    fn decode(data: &[u8]) -> rmpv::Value {
        rmpv::decode::read_value(&mut &data[..]).unwrap()
    }

    /// A minimal mock server that responds to the Hello handshake.
    async fn mock_hello_server(listener: TcpListener) {
        let (stream, _) = listener.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (mut reader, mut writer) = tokio::io::split(stream);

        // Read the Hello request.
        let data = read_frame(&mut reader).await.unwrap();
        let value = decode(&data);
        let map = value.as_map().unwrap();

        // Extract _id from request.
        let id = map
            .iter()
            .find(|(k, _)| k.as_str() == Some("_id"))
            .map(|(_, v)| v.clone())
            .unwrap();

        // Build Hello response.
        let resp = rmpv::Value::Map(vec![
            (rmpv::Value::from("type"), rmpv::Value::from("Hello")),
            (rmpv::Value::from("_id"), id),
            (
                rmpv::Value::from("protocolVersion"),
                rmpv::Value::from(PROTOCOL_VERSION),
            ),
        ]);
        write_frame(&mut writer, &encode(&resp)).await.unwrap();
    }

    #[tokio::test]
    async fn test_connect_and_hello() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(mock_hello_server(listener));
        let conn = Connection::open("127.0.0.1", addr.port()).await.unwrap();
        server.await.unwrap();

        // Connection should have id counter at 1 after hello.
        assert_eq!(conn.req_id.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_protocol_mismatch() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut reader, mut writer) = tokio::io::split(stream);

            let data = read_frame(&mut reader).await.unwrap();
            let value: rmpv::Value = decode(&data);
            let map = value.as_map().unwrap();
            let id = map
                .iter()
                .find(|(k, _)| k.as_str() == Some("_id"))
                .map(|(_, v)| v.clone())
                .unwrap();

            let resp = rmpv::Value::Map(vec![
                (rmpv::Value::from("type"), rmpv::Value::from("Hello")),
                (rmpv::Value::from("_id"), id),
                (
                    rmpv::Value::from("protocolVersion"),
                    rmpv::Value::from("v0.99"),
                ),
            ]);
            let packed = encode(&resp);
            write_frame(&mut writer, &packed).await.unwrap();
        });

        let result = Connection::open("127.0.0.1", addr.port()).await;
        assert!(matches!(result, Err(BngError::ProtocolMismatch(_))));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_request_response_correlation() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            stream.set_nodelay(true).unwrap();
            let (mut reader, mut writer) = tokio::io::split(stream);

            // Respond to Hello first.
            let data = read_frame(&mut reader).await.unwrap();
            let value: rmpv::Value = decode(&data);
            let id = value
                .as_map()
                .unwrap()
                .iter()
                .find(|(k, _)| k.as_str() == Some("_id"))
                .map(|(_, v)| v.clone())
                .unwrap();
            let resp = rmpv::Value::Map(vec![
                (rmpv::Value::from("type"), rmpv::Value::from("Hello")),
                (rmpv::Value::from("_id"), id),
                (
                    rmpv::Value::from("protocolVersion"),
                    rmpv::Value::from(PROTOCOL_VERSION),
                ),
            ]);
            write_frame(&mut writer, &encode(&resp))
                .await
                .unwrap();

            // Read the Pause request and respond out of order:
            // First send a future response (id=99), then the actual one.
            let data = read_frame(&mut reader).await.unwrap();
            let value: rmpv::Value = decode(&data);
            let id = value
                .as_map()
                .unwrap()
                .iter()
                .find(|(k, _)| k.as_str() == Some("_id"))
                .map(|(_, v)| v.clone())
                .unwrap();

            // Send an out-of-order response with id=99 first.
            let future_resp = rmpv::Value::Map(vec![
                (rmpv::Value::from("type"), rmpv::Value::from("Future")),
                (rmpv::Value::from("_id"), rmpv::Value::from(99u64)),
            ]);
            write_frame(&mut writer, &encode(&future_resp))
                .await
                .unwrap();

            // Then send the actual Paused response.
            let resp = rmpv::Value::Map(vec![
                (rmpv::Value::from("type"), rmpv::Value::from("Paused")),
                (rmpv::Value::from("_id"), id),
            ]);
            write_frame(&mut writer, &encode(&resp))
                .await
                .unwrap();
        });

        let conn = Connection::open("127.0.0.1", addr.port()).await.unwrap();
        let resp = conn.request("Pause", &[]).await.unwrap();
        assert_eq!(resp.get("type").unwrap().as_str().unwrap(), "Paused");

        // The out-of-order message should be buffered.
        let buffered = conn.buffered.lock().await;
        assert!(buffered.contains_key(&99));

        server.await.unwrap();
    }
}
