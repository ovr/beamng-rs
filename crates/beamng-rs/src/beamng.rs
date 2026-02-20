use beamng_proto::{BngError, Connection, Result};
use tracing::info;

use crate::api::beamng::*;

/// The main handle to a BeamNG.tech simulator instance.
///
/// # Example
/// ```no_run
/// # async fn example() -> beamng_proto::Result<()> {
/// use beamng_rs::BeamNg;
///
/// let mut bng = BeamNg::new("localhost", 25252).connect().await?;
/// bng.control().pause().await?;
/// bng.control().step(60, true).await?;
/// bng.control().resume().await?;
/// # Ok(())
/// # }
/// ```
pub struct BeamNg {
    host: String,
    port: u16,
    connection: Option<Connection>,
}

impl BeamNg {
    /// Create a new BeamNg handle targeting the given host and port.
    /// Does not connect immediately — call [`connect()`](Self::connect) to establish a connection.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            connection: None,
        }
    }

    /// Connect to the simulator and perform the hello handshake.
    pub async fn connect(mut self) -> Result<Self> {
        let conn = Connection::open(&self.host, self.port).await?;
        self.connection = Some(conn);
        Ok(self)
    }

    /// Returns a mutable reference to the underlying connection, or an error if not connected.
    pub(crate) fn conn(&mut self) -> Result<&mut Connection> {
        self.connection
            .as_mut()
            .ok_or_else(|| BngError::Disconnected("Not connected to BeamNG.tech".into()))
    }

    /// Returns the host address.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Disconnect from the simulator.
    pub fn disconnect(&mut self) {
        self.connection = None;
        info!("Disconnected from BeamNG.tech");
    }

    // --- API accessors ---

    /// Access the simulation control API (pause, resume, step, etc.).
    pub fn control(&mut self) -> ControlApi<'_> {
        ControlApi { bng: self }
    }

    /// Access the system information API.
    pub fn system(&mut self) -> SystemApi<'_> {
        SystemApi { bng: self }
    }

    /// Access the vehicles management API.
    pub fn vehicles(&mut self) -> VehiclesApi<'_> {
        VehiclesApi { bng: self }
    }

    /// Access the scenario management API.
    pub fn scenario(&mut self) -> ScenarioApi<'_> {
        ScenarioApi { bng: self }
    }

    /// Access the environment control API (time of day, weather, gravity).
    pub fn environment(&mut self) -> EnvironmentApi<'_> {
        EnvironmentApi { bng: self }
    }

    /// Access the debug drawing API.
    pub fn debug(&mut self) -> DebugApi<'_> {
        DebugApi { bng: self }
    }

    /// Access the traffic control API.
    pub fn traffic(&mut self) -> TrafficApi<'_> {
        TrafficApi { bng: self }
    }

    /// Access the camera control API.
    pub fn camera(&mut self) -> CameraApi<'_> {
        CameraApi { bng: self }
    }

    /// Access the settings API.
    pub fn settings(&mut self) -> SettingsApi<'_> {
        SettingsApi { bng: self }
    }

    /// Access the UI control API.
    pub fn ui(&mut self) -> UiApi<'_> {
        UiApi { bng: self }
    }
}
