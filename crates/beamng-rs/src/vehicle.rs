use beamng_proto::types::{Color, StrDict};
use beamng_proto::Connection;

use crate::api::vehicle::{AIApi, RootApi};

/// A vehicle in the BeamNG.tech simulation.
pub struct Vehicle {
    /// The unique vehicle identifier.
    pub vid: String,
    /// The vehicle model name.
    pub model: String,
    /// The per-vehicle TCP connection (established after spawn + connect).
    pub(crate) connection: Option<Connection>,
    /// Vehicle options passed at spawn time.
    pub(crate) options: VehicleOptions,
}

/// Options for constructing a vehicle.
#[derive(Debug, Clone, Default)]
pub struct VehicleOptions {
    pub license: Option<String>,
    pub color: Option<Color>,
    pub color2: Option<Color>,
    pub color3: Option<Color>,
    pub part_config: Option<String>,
    pub extensions: Option<Vec<String>>,
}

/// Builder for constructing a [`Vehicle`] with optional parameters.
pub struct VehicleBuilder {
    vid: String,
    model: String,
    options: VehicleOptions,
}

impl VehicleBuilder {
    pub fn new(vid: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            vid: vid.into().replace(' ', "_"),
            model: model.into(),
            options: VehicleOptions::default(),
        }
    }

    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.options.license = Some(license.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.options.color = Some(color);
        self
    }

    pub fn color2(mut self, color: Color) -> Self {
        self.options.color2 = Some(color);
        self
    }

    pub fn color3(mut self, color: Color) -> Self {
        self.options.color3 = Some(color);
        self
    }

    pub fn part_config(mut self, config: impl Into<String>) -> Self {
        self.options.part_config = Some(config.into());
        self
    }

    pub fn extensions(mut self, exts: Vec<String>) -> Self {
        self.options.extensions = Some(exts);
        self
    }

    pub fn build(self) -> Vehicle {
        Vehicle {
            vid: self.vid,
            model: self.model,
            connection: None,
            options: self.options,
        }
    }
}

impl Vehicle {
    /// Create a new vehicle with the given ID and model.
    pub fn new(vid: impl Into<String>, model: impl Into<String>) -> Self {
        VehicleBuilder::new(vid, model).build()
    }

    /// Returns a builder for more complex vehicle configuration.
    pub fn builder(vid: impl Into<String>, model: impl Into<String>) -> VehicleBuilder {
        VehicleBuilder::new(vid, model)
    }

    /// Whether this vehicle has an active per-vehicle connection.
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Send a request over the per-vehicle connection.
    pub(crate) async fn send_vehicle_request(
        &self,
        req_type: &str,
        fields: &[(&str, rmpv::Value)],
    ) -> beamng_proto::Result<StrDict> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| beamng_proto::BngError::Disconnected("Vehicle not connected".into()))?;
        conn.request(req_type, fields).await
    }

    /// Access the AI control API for this vehicle.
    pub fn ai(&self) -> AIApi<'_> {
        AIApi { vehicle: self }
    }

    /// Access the root-level vehicle API (position, bounding box, direct control).
    pub fn root(&self) -> RootApi<'_> {
        RootApi { vehicle: self }
    }

    /// Disconnect the per-vehicle connection.
    pub fn disconnect(&mut self) {
        self.connection = None;
    }
}
