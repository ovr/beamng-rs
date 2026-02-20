use beamng_proto::Result;

use crate::beamng::BeamNg;

/// API for controlling traffic in the simulation.
pub struct TrafficApi<'a> {
    pub(crate) bng: &'a mut BeamNg,
}

impl TrafficApi<'_> {
    /// Enable traffic simulation for the given vehicle IDs.
    pub async fn start(&mut self, participant_vids: &[&str]) -> Result<()> {
        let participants: Vec<rmpv::Value> = participant_vids
            .iter()
            .map(|v| rmpv::Value::from(*v))
            .collect();
        self.bng
            .conn()?
            .ack(
                "StartTraffic",
                "TrafficStarted",
                &[("participants", rmpv::Value::Array(participants))],
            )
            .await
    }

    /// Spawn traffic vehicles.
    pub async fn spawn(
        &mut self,
        max_amount: Option<i32>,
        police_ratio: f64,
        extra_amount: Option<i32>,
        parked_amount: Option<i32>,
    ) -> Result<()> {
        let mut fields: Vec<(&str, rmpv::Value)> =
            vec![("police_ratio", rmpv::Value::from(police_ratio))];
        if let Some(max) = max_amount {
            fields.push(("max_amount", rmpv::Value::from(max)));
        }
        if let Some(extra) = extra_amount {
            fields.push(("extra_amount", rmpv::Value::from(extra)));
        }
        if let Some(parked) = parked_amount {
            fields.push(("parked_amount", rmpv::Value::from(parked)));
        }
        self.bng
            .conn()?
            .ack("SpawnTraffic", "TrafficSpawned", &fields)
            .await
    }

    /// Reset (force teleport) all traffic vehicles away from the player.
    pub async fn reset(&mut self) -> Result<()> {
        self.bng
            .conn()?
            .ack("ResetTraffic", "TrafficReset", &[])
            .await
    }

    /// Stop the traffic simulation.
    pub async fn stop(&mut self, stop_vehicles: bool) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "StopTraffic",
                "TrafficStopped",
                &[("stop", rmpv::Value::from(stop_vehicles))],
            )
            .await
    }
}
