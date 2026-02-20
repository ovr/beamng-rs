use beamng_proto::types::{Quat, StrDict, Vec3};
use beamng_proto::Result;

use crate::vehicle::Vehicle;

/// Root-level vehicle API for direct vehicle control and info.
pub struct RootApi<'a> {
    pub(crate) vehicle: &'a Vehicle,
}

impl RootApi<'_> {
    /// Set the vehicle's position and optional rotation.
    pub async fn set_position(&self, pos: Vec3, rot: Option<Quat>) -> Result<()> {
        let mut fields: Vec<(&str, rmpv::Value)> = vec![(
            "pos",
            rmpv::Value::Array(vec![
                rmpv::Value::from(pos.0),
                rmpv::Value::from(pos.1),
                rmpv::Value::from(pos.2),
            ]),
        )];
        if let Some(r) = rot {
            fields.push((
                "rot",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(r.0),
                    rmpv::Value::from(r.1),
                    rmpv::Value::from(r.2),
                    rmpv::Value::from(r.3),
                ]),
            ));
        }
        self.vehicle
            .send_vehicle_request("SetPosition", &fields)
            .await?;
        Ok(())
    }

    /// Get the vehicle's bounding box.
    pub async fn get_bbox(&self) -> Result<StrDict> {
        self.vehicle
            .send_vehicle_request("GetBBoxPoints", &[])
            .await
    }

    /// Apply vehicle input (steering, throttle, brake, etc.).
    pub async fn control(
        &self,
        steering: Option<f64>,
        throttle: Option<f64>,
        brake: Option<f64>,
        parkingbrake: Option<f64>,
        clutch: Option<f64>,
        gear: Option<i32>,
    ) -> Result<()> {
        let mut fields: Vec<(&str, rmpv::Value)> = Vec::new();
        if let Some(v) = steering {
            fields.push(("steering", rmpv::Value::from(v)));
        }
        if let Some(v) = throttle {
            fields.push(("throttle", rmpv::Value::from(v)));
        }
        if let Some(v) = brake {
            fields.push(("brake", rmpv::Value::from(v)));
        }
        if let Some(v) = parkingbrake {
            fields.push(("parkingbrake", rmpv::Value::from(v)));
        }
        if let Some(v) = clutch {
            fields.push(("clutch", rmpv::Value::from(v)));
        }
        if let Some(v) = gear {
            fields.push(("gear", rmpv::Value::from(v)));
        }
        self.vehicle
            .send_vehicle_request("Control", &fields)
            .await?;
        Ok(())
    }
}
