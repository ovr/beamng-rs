use beamng_proto::Result;

use crate::vehicle::Vehicle;

/// API for controlling vehicle AI behavior.
pub struct AIApi<'a> {
    pub(crate) vehicle: &'a mut Vehicle,
}

impl AIApi<'_> {
    /// Set the AI mode (e.g. "disabled", "span", "manual", "traffic", "flee", "chase", "random").
    pub async fn set_mode(&mut self, mode: &str) -> Result<()> {
        self.vehicle
            .send_vehicle_request("SetAiMode", &[("mode", rmpv::Value::from(mode))])
            .await?;
        Ok(())
    }

    /// Set the AI target speed in m/s.
    pub async fn set_speed(&mut self, speed: f64, mode: &str) -> Result<()> {
        self.vehicle
            .send_vehicle_request(
                "SetAiSpeed",
                &[
                    ("speed", rmpv::Value::from(speed)),
                    ("mode", rmpv::Value::from(mode)),
                ],
            )
            .await?;
        Ok(())
    }

    /// Set a waypoint for the AI to navigate to.
    pub async fn set_waypoint(&mut self, waypoint: &str) -> Result<()> {
        self.vehicle
            .send_vehicle_request("SetAiTarget", &[("waypoint", rmpv::Value::from(waypoint))])
            .await?;
        Ok(())
    }

    /// Make the AI drive in lane.
    pub async fn drive_in_lane(&mut self, lane: bool) -> Result<()> {
        self.vehicle
            .send_vehicle_request(
                "SetDriveInLane",
                &[("lane", rmpv::Value::from(if lane { "on" } else { "off" }))],
            )
            .await?;
        Ok(())
    }

    /// Set AI aggression (0.0 - 1.0).
    pub async fn set_aggression(&mut self, aggression: f64) -> Result<()> {
        self.vehicle
            .send_vehicle_request(
                "SetAiAggression",
                &[("aggression", rmpv::Value::from(aggression))],
            )
            .await?;
        Ok(())
    }
}
