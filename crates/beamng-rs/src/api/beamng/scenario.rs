use beamng_proto::types::StrDict;
use beamng_proto::{BngError, Result};

use crate::beamng::BeamNg;
use crate::scenario::Scenario;

/// API for working with scenarios, levels and scenario objects.
pub struct ScenarioApi<'a> {
    pub(crate) bng: &'a BeamNg,
}

impl ScenarioApi<'_> {
    /// Query available levels.
    pub async fn get_levels(&self) -> Result<Option<rmpv::Value>> {
        self.bng.conn()?.message("GetLevels", &[]).await
    }

    /// Query available scenarios, optionally filtered by level names.
    pub async fn get_scenarios(&self, levels: &[&str]) -> Result<Option<rmpv::Value>> {
        let levels_val: Vec<rmpv::Value> = levels.iter().map(|l| rmpv::Value::from(*l)).collect();
        self.bng
            .conn()?
            .message("GetScenarios", &[("levels", rmpv::Value::Array(levels_val))])
            .await
    }

    /// Get the name of the currently loaded scenario.
    pub async fn get_name(&self) -> Result<String> {
        let resp = self.bng.conn()?.request("GetScenarioName", &[]).await?;
        resp.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| BngError::ValueError("Missing scenario name".into()))
    }

    /// Load a [`Scenario`] that was previously created with [`Scenario::make`].
    pub async fn load_scenario(
        &self,
        scenario: &Scenario,
        precompile_shaders: bool,
    ) -> Result<()> {
        let path = scenario
            .path()
            .ok_or_else(|| BngError::ValueError("Scenario has no path; call make() first".into()))?;
        self.load(path, precompile_shaders).await
    }

    /// Load a scenario by its path.
    pub async fn load(&self, path: &str, precompile_shaders: bool) -> Result<()> {
        self.bng.conn()?.ack(
            "LoadScenario",
            "MapLoaded",
            &[
                ("path", rmpv::Value::from(path)),
                ("precompileShaders", rmpv::Value::from(precompile_shaders)),
            ],
        ).await
    }

    /// Start the currently loaded scenario.
    pub async fn start(&self, restrict_actions: bool) -> Result<()> {
        self.bng.conn()?.ack(
            "StartScenario",
            "ScenarioStarted",
            &[("restrict_actions", rmpv::Value::from(restrict_actions))],
        ).await
    }

    /// Restart the currently running scenario.
    pub async fn restart(&self, restrict_actions: bool) -> Result<()> {
        self.bng.conn()?.ack(
            "RestartScenario",
            "ScenarioRestarted",
            &[("restrict_actions", rmpv::Value::from(restrict_actions))],
        ).await
    }

    /// Stop the current scenario and return to the main menu.
    pub async fn stop(&self) -> Result<()> {
        self.bng
            .conn()?
            .ack("StopScenario", "ScenarioStopped", &[])
            .await
    }

    /// Get the current scenario info.
    pub async fn get_current(&self) -> Result<Option<rmpv::Value>> {
        self.bng.conn()?.message("GetCurrentScenario", &[]).await
    }

    /// Retrieve the road network data.
    pub async fn get_road_network(
        &self,
        include_edges: bool,
        drivable_only: bool,
    ) -> Result<StrDict> {
        self.bng
            .conn()?
            .request(
                "GetRoadNetwork",
                &[
                    ("includeEdges", rmpv::Value::from(include_edges)),
                    ("drivableOnly", rmpv::Value::from(drivable_only)),
                ],
            )
            .await
    }

    /// Retrieve edges of a named road.
    pub async fn get_road_edges(&self, road: &str) -> Result<StrDict> {
        self.bng
            .conn()?
            .request("GetDecalRoadEdges", &[("road", rmpv::Value::from(road))])
            .await
    }

    /// Find objects of a given class.
    pub async fn find_objects_class(&self, class: &str) -> Result<StrDict> {
        self.bng
            .conn()?
            .request("FindObjectsClass", &[("class", rmpv::Value::from(class))])
            .await
    }

    /// Teleport a scenario object.
    pub async fn teleport_object(
        &self,
        id: i64,
        pos: beamng_proto::types::Vec3,
        rot_quat: Option<beamng_proto::types::Quat>,
    ) -> Result<()> {
        let mut fields: Vec<(&str, rmpv::Value)> = vec![
            ("id", rmpv::Value::from(id)),
            ("pos", rmpv::Value::Array(vec![
                rmpv::Value::from(pos.0),
                rmpv::Value::from(pos.1),
                rmpv::Value::from(pos.2),
            ])),
        ];
        if let Some(rot) = rot_quat {
            fields.push(("rot", rmpv::Value::Array(vec![
                rmpv::Value::from(rot.0),
                rmpv::Value::from(rot.1),
                rmpv::Value::from(rot.2),
                rmpv::Value::from(rot.3),
            ])));
        }
        self.bng
            .conn()?
            .ack("TeleportScenarioObject", "ScenarioObjectTeleported", &fields)
            .await
    }

    /// Load a TrackBuilder track.
    pub async fn load_trackbuilder_track(&self, path: &str) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "LoadTrackBuilderTrack",
                "TrackBuilderTrackLoaded",
                &[("path", rmpv::Value::from(path))],
            )
            .await
    }
}
