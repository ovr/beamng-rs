use beamng_proto::types::{Quat, StrDict, Vec3};
use beamng_proto::{BngError, Connection, Result};

use crate::beamng::BeamNg;
use crate::vehicle::Vehicle;

/// API for vehicle manipulation in the simulator.
pub struct VehiclesApi<'a> {
    pub(crate) bng: &'a BeamNg,
}

impl VehiclesApi<'_> {
    /// Start a per-vehicle connection. Returns the dynamic port info from the simulator.
    pub async fn start_connection(
        &self,
        vehicle: &Vehicle,
        extensions: Option<&[String]>,
    ) -> Result<StrDict> {
        let conn = self.bng.conn()?;
        let mut fields: Vec<(&str, rmpv::Value)> = vec![("vid", rmpv::Value::from(vehicle.vid.as_str()))];
        if let Some(exts) = extensions {
            let exts_val: Vec<rmpv::Value> = exts.iter().map(|s| rmpv::Value::from(s.as_str())).collect();
            fields.push(("exts", rmpv::Value::Array(exts_val)));
        }
        conn.request("StartVehicleConnection", &fields).await
    }

    /// Spawn a vehicle in the simulation at the given position.
    pub async fn spawn(
        &self,
        vehicle: &mut Vehicle,
        pos: Vec3,
        rot_quat: Quat,
        cling: bool,
        connect: bool,
    ) -> Result<bool> {
        let conn = self.bng.conn()?;
        let mut fields: Vec<(&str, rmpv::Value)> = vec![
            ("name", rmpv::Value::from(vehicle.vid.as_str())),
            ("model", rmpv::Value::from(vehicle.model.as_str())),
            ("pos", rmpv::Value::Array(vec![
                rmpv::Value::from(pos.0),
                rmpv::Value::from(pos.1),
                rmpv::Value::from(pos.2),
            ])),
            ("rot", rmpv::Value::Array(vec![
                rmpv::Value::from(rot_quat.0),
                rmpv::Value::from(rot_quat.1),
                rmpv::Value::from(rot_quat.2),
                rmpv::Value::from(rot_quat.3),
            ])),
            ("cling", rmpv::Value::from(cling)),
        ];

        if let Some(ref license) = vehicle.options.license {
            fields.push(("licenseText", rmpv::Value::from(license.as_str())));
        }
        if let Some(ref pc) = vehicle.options.part_config {
            fields.push(("partConfig", rmpv::Value::from(pc.as_str())));
        }

        let resp = conn.request("SpawnVehicle", &fields).await?;
        let success = resp
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if success && connect {
            self.connect_vehicle(vehicle).await?;
        }

        Ok(success)
    }

    /// Establish a per-vehicle TCP connection.
    pub async fn connect_vehicle(&self, vehicle: &mut Vehicle) -> Result<()> {
        let resp = self.start_connection(vehicle, vehicle.options.extensions.as_deref()).await?;
        let port = resp
            .get("result")
            .and_then(|v| v.as_u64())
            .or_else(|| resp.get("port").and_then(|v| v.as_u64()))
            .ok_or_else(|| BngError::ValueError("Missing port in StartVehicleConnection response".into()))?;

        let host = &self.bng.host();
        let stream = tokio::net::TcpStream::connect(format!("{host}:{port}")).await?;
        let veh_conn = Connection::from_stream(stream).await?;
        vehicle.connection = Some(veh_conn);
        Ok(())
    }

    /// Despawn a vehicle from the simulation.
    pub async fn despawn(&self, vehicle: &mut Vehicle) -> Result<()> {
        vehicle.disconnect();
        self.bng.conn()?.ack(
            "DespawnVehicle",
            "VehicleDespawned",
            &[("vid", rmpv::Value::from(vehicle.vid.as_str()))],
        ).await
    }

    /// Retrieve a dictionary of available vehicle models.
    pub async fn get_available(&self) -> Result<StrDict> {
        self.bng
            .conn()?
            .request("GetAvailableVehicles", &[])
            .await
    }

    /// Teleport a vehicle to a new position.
    pub async fn teleport(
        &self,
        vid: &str,
        pos: Vec3,
        rot_quat: Option<Quat>,
        reset: bool,
    ) -> Result<bool> {
        let mut fields: Vec<(&str, rmpv::Value)> = vec![
            ("vehicle", rmpv::Value::from(vid)),
            ("pos", rmpv::Value::Array(vec![
                rmpv::Value::from(pos.0),
                rmpv::Value::from(pos.1),
                rmpv::Value::from(pos.2),
            ])),
            ("reset", rmpv::Value::from(reset)),
        ];
        if let Some(rot) = rot_quat {
            fields.push(("rot", rmpv::Value::Array(vec![
                rmpv::Value::from(rot.0),
                rmpv::Value::from(rot.1),
                rmpv::Value::from(rot.2),
                rmpv::Value::from(rot.3),
            ])));
        }

        let resp = self.bng.conn()?.request("Teleport", &fields).await?;
        Ok(resp
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    /// Switch the active (player-focused) vehicle.
    pub async fn switch(&self, vid: &str) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "SwitchVehicle",
                "VehicleSwitched",
                &[("vid", rmpv::Value::from(vid))],
            )
            .await
    }

    /// Wait for a vehicle with the given name to spawn.
    pub async fn await_spawn(&self, vid: &str) -> Result<()> {
        self.bng
            .conn()?
            .request("WaitForSpawn", &[("name", rmpv::Value::from(vid))])
            .await?;
        Ok(())
    }

    /// Get the states of the given vehicles (position, direction, velocity).
    pub async fn get_states(&self, vids: &[&str]) -> Result<StrDict> {
        let vehicles: Vec<rmpv::Value> = vids.iter().map(|v| rmpv::Value::from(*v)).collect();
        let resp = self
            .bng
            .conn()?
            .request(
                "UpdateScenario",
                &[("vehicles", rmpv::Value::Array(vehicles))],
            )
            .await?;
        Ok(resp)
    }

    /// Query the currently active vehicles in the simulator.
    pub async fn get_current_info(&self, include_config: bool) -> Result<Option<rmpv::Value>> {
        self.bng
            .conn()?
            .message(
                "GetCurrentVehicles",
                &[("include_config", rmpv::Value::from(include_config))],
            )
            .await
    }

    /// Get the current player vehicle ID.
    pub async fn get_player_vehicle_id(&self) -> Result<StrDict> {
        self.bng
            .conn()?
            .request("GetPlayerVehicleID", &[])
            .await
    }

    /// Set a vehicle's license plate text.
    pub async fn set_license_plate(&self, vid: &str, text: &str) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "SetLicensePlate",
                "SetLicensePlate",
                &[
                    ("vid", rmpv::Value::from(vid)),
                    ("text", rmpv::Value::from(text)),
                ],
            )
            .await
    }
}
