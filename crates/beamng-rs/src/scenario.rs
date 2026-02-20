use beamng_proto::types::{Quat, Vec3};
use beamng_proto::{BngError, Result};
use serde_json::{json, Map, Value as JsonValue};

use crate::beamng::BeamNg;
use crate::vehicle::VehicleOptions;

/// A lightweight vehicle descriptor stored in a [`Scenario`].
///
/// This is intentionally separate from [`Vehicle`](crate::vehicle::Vehicle) to avoid
/// shared mutable reference complexity. After loading a scenario, vehicles are connected
/// independently.
#[derive(Debug, Clone)]
pub struct ScenarioVehicle {
    pub vid: String,
    pub model: String,
    pub pos: Vec3,
    pub rot_quat: Quat,
    pub options: VehicleOptions,
    uuid: String,
}

/// A programmatically-created scenario that can be sent to BeamNG.tech.
///
/// # Example
/// ```no_run
/// # async fn example() -> beamng_proto::Result<()> {
/// use beamng_rs::{BeamNg, Scenario};
/// use beamng_rs::vehicle::VehicleOptions;
///
/// let bng = BeamNg::new("localhost", 25252).connect().await?;
/// let mut scenario = Scenario::new("italy", "my_scenario");
/// scenario.add_vehicle("ego", "etk800", (0.0, 0.0, 0.0), (0.0, 0.0, 0.0, 1.0), VehicleOptions::default());
/// scenario.make(&bng).await?;
/// # Ok(())
/// # }
/// ```
pub struct Scenario {
    pub level: String,
    pub name: String,
    path: Option<String>,
    vehicles: Vec<ScenarioVehicle>,
    uuid: String,
}

impl Scenario {
    /// Create a new scenario descriptor for the given level and name.
    pub fn new(level: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            level: level.into(),
            name: name.into(),
            path: None,
            vehicles: Vec::new(),
            uuid: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Add a vehicle descriptor to the scenario.
    /// The first vehicle added will receive `startFocus`.
    pub fn add_vehicle(
        &mut self,
        vid: impl Into<String>,
        model: impl Into<String>,
        pos: Vec3,
        rot_quat: Quat,
        options: VehicleOptions,
    ) {
        self.vehicles.push(ScenarioVehicle {
            vid: vid.into().replace(' ', "_"),
            model: model.into(),
            pos,
            rot_quat,
            options,
            uuid: uuid::Uuid::new_v4().to_string(),
        });
    }

    /// Get the scenario path (set after `make()`).
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Get the vehicle IDs in the scenario.
    pub fn vehicle_ids(&self) -> Vec<&str> {
        self.vehicles.iter().map(|v| v.vid.as_str()).collect()
    }

    /// Delete a previously-created scenario from the simulator's filesystem.
    ///
    /// Useful to clean up stale scenarios before re-creating them.
    pub async fn delete(bng: &BeamNg, path: &str) -> Result<()> {
        bng.conn()?
            .message("DeleteScenario", &[("path", rmpv::Value::from(path))])
            .await?;
        Ok(())
    }

    /// Build the scenario in the simulator.
    ///
    /// Generates a prefab JSON and info dict, sends a `CreateScenario` message,
    /// and stores the returned path.
    pub async fn make(&mut self, bng: &BeamNg) -> Result<()> {
        if self.path.is_some() {
            return Err(BngError::ValueError(
                "This scenario already has an info file.".into(),
            ));
        }

        let prefab = self.build_prefab();
        let info = self.build_info_dict();

        let conn = bng.conn()?;
        let resp = conn
            .request(
                "CreateScenario",
                &[
                    ("level", rmpv::Value::from(self.level.as_str())),
                    ("name", rmpv::Value::from(self.name.as_str())),
                    ("prefab", rmpv::Value::from(prefab.as_str())),
                    ("info", info),
                    ("json", rmpv::Value::from(true)),
                ],
            )
            .await?;

        let path = resp
            .get("result")
            .and_then(|v| beamng_proto::types::value_to_string(v))
            .ok_or_else(|| {
                BngError::ValueError("Missing path in CreateScenario response".into())
            })?;

        self.path = Some(path);
        Ok(())
    }

    /// Build the prefab string matching BeamNGpy's format.
    ///
    /// BeamNG's `deserializeLineObjects` reads **one JSON object per line**.
    /// Each line must be a complete, single-line JSON object.
    ///
    /// Key order matters — we use `serde_json::Map` (backed by `IndexMap` via the
    /// `preserve_order` feature) to guarantee insertion order matches the Python template.
    fn build_prefab(&self) -> String {
        let mut lines = Vec::new();

        let mut sorted_vehicles = self.vehicles.clone();
        sorted_vehicles.sort_by(|a, b| a.vid.cmp(&b.vid));

        for v in &sorted_vehicles {
            let rot_mat = beamng_proto::types::quat_to_rotation_matrix(v.rot_quat);
            lines
                .push(serde_json::to_string(&build_vehicle_json(v, &self.name, &rot_mat)).unwrap());
        }

        // Footer: SimGroup
        let footer = build_footer_json(&self.name, &self.uuid);
        lines.push(serde_json::to_string(&footer).unwrap());

        lines.join("\n")
    }

    /// Build the info dict as an rmpv::Value map matching Python's `_get_info_dict`.
    fn build_info_dict(&self) -> rmpv::Value {
        let mut vehicles_map: Vec<(rmpv::Value, rmpv::Value)> = Vec::new();

        for (i, v) in self.vehicles.iter().enumerate() {
            let mut props: Vec<(rmpv::Value, rmpv::Value)> =
                vec![(rmpv::Value::from("playerUsable"), rmpv::Value::from(true))];
            if i == 0 {
                props.push((rmpv::Value::from("startFocus"), rmpv::Value::from(true)));
            }
            vehicles_map.push((rmpv::Value::from(v.vid.as_str()), rmpv::Value::Map(props)));
        }

        let prefab_path = format!(
            "levels/{}/scenarios/{}/{}.prefab.json",
            self.level, self.name, self.name
        );

        rmpv::Value::Map(vec![
            (
                rmpv::Value::from("name"),
                rmpv::Value::from(self.name.as_str()),
            ),
            (rmpv::Value::from("description"), rmpv::Value::from("")),
            (rmpv::Value::from("difficulty"), rmpv::Value::from(0)),
            (rmpv::Value::from("authors"), rmpv::Value::from("")),
            (rmpv::Value::from("lapConfig"), rmpv::Value::Array(vec![])),
            (
                rmpv::Value::from("forceNoCountDown"),
                rmpv::Value::from(true),
            ),
            (
                rmpv::Value::from("vehicles"),
                rmpv::Value::Map(vehicles_map),
            ),
            (
                rmpv::Value::from("prefabs"),
                rmpv::Value::Array(vec![rmpv::Value::from(prefab_path.as_str())]),
            ),
        ])
    }
}

/// Build a vehicle JSON object with keys in the exact order BeamNG expects.
fn build_vehicle_json(v: &ScenarioVehicle, scenario_name: &str, rot_mat: &[f64; 9]) -> JsonValue {
    let mut obj = Map::new();
    obj.insert("name".into(), json!(v.vid));
    obj.insert("class".into(), json!("BeamNGVehicle"));
    obj.insert("persistentId".into(), json!(v.uuid));
    obj.insert("__parent".into(), json!(format!("{scenario_name}_group")));
    obj.insert("position".into(), json!([v.pos.0, v.pos.1, v.pos.2]));

    if let Some(color) = v.options.color {
        let c = json!([color.0, color.1, color.2, color.3]);
        obj.insert("color".into(), c.clone());
        obj.insert("colorPalette0".into(), c.clone());
        obj.insert("colorPalette1".into(), c);
    }

    obj.insert("dataBlock".into(), json!("default_vehicle"));
    obj.insert("jBeam".into(), json!(v.model));

    if let Some(ref license) = v.options.license {
        obj.insert("licenseText".into(), json!(license));
    }

    if let Some(ref part_config) = v.options.part_config {
        obj.insert("partConfig".into(), json!(part_config));
    }

    obj.insert("rotationMatrix".into(), json!(rot_mat.to_vec()));
    obj.insert("autoEnterVehicle".into(), json!("false"));

    JsonValue::Object(obj)
}

/// Build the SimGroup footer JSON with correct key order.
fn build_footer_json(scenario_name: &str, uuid: &str) -> JsonValue {
    let mut obj = Map::new();
    obj.insert("name".into(), json!(format!("{scenario_name}_group")));
    obj.insert("class".into(), json!("SimGroup"));
    obj.insert("persistentId".into(), json!(uuid));
    obj.insert("groupPosition".into(), json!("0.000000 0.000000 0.000000"));
    JsonValue::Object(obj)
}
