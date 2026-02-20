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
/// let bng = BeamNg::new("localhost", 25252).connect().await?;
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

    /// Returns a reference to the underlying connection, or an error if not connected.
    pub(crate) fn conn(&self) -> Result<&Connection> {
        self.connection
            .as_ref()
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
    pub fn control(&self) -> ControlApi<'_> {
        ControlApi { bng: self }
    }

    /// Access the system information API.
    pub fn system(&self) -> SystemApi<'_> {
        SystemApi { bng: self }
    }

    /// Access the vehicles management API.
    pub fn vehicles(&self) -> VehiclesApi<'_> {
        VehiclesApi { bng: self }
    }

    /// Access the scenario management API.
    pub fn scenario(&self) -> ScenarioApi<'_> {
        ScenarioApi { bng: self }
    }

    /// Access the environment control API (time of day, weather, gravity).
    pub fn environment(&self) -> EnvironmentApi<'_> {
        EnvironmentApi { bng: self }
    }

    /// Access the debug drawing API.
    pub fn debug(&self) -> DebugApi<'_> {
        DebugApi { bng: self }
    }

    /// Access the traffic control API.
    pub fn traffic(&self) -> TrafficApi<'_> {
        TrafficApi { bng: self }
    }

    /// Access the camera control API.
    pub fn camera(&self) -> CameraApi<'_> {
        CameraApi { bng: self }
    }

    /// Access the settings API.
    pub fn settings(&self) -> SettingsApi<'_> {
        SettingsApi { bng: self }
    }

    /// Access the UI control API.
    pub fn ui(&self) -> UiApi<'_> {
        UiApi { bng: self }
    }
}
