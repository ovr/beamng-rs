use beamng_proto::types::Vec3;
use beamng_proto::Result;
use tracing::info;

use crate::beamng::BeamNg;
use crate::vehicle::Vehicle;

/// Configuration for an [`AdvancedImu`] sensor.
#[derive(Debug, Clone)]
pub struct AdvancedImuConfig {
    pub gfx_update_time: f64,
    pub physics_update_time: f64,
    pub pos: Vec3,
    pub dir: Vec3,
    pub up: Vec3,
    pub smoother_strength: f64,
    pub is_send_immediately: bool,
    pub is_using_gravity: bool,
    pub is_allow_wheel_nodes: bool,
    pub is_visualised: bool,
    pub is_snapping_desired: bool,
    pub is_force_inside_triangle: bool,
    pub is_dir_world_space: bool,
}

impl Default for AdvancedImuConfig {
    fn default() -> Self {
        Self {
            gfx_update_time: 0.0,
            physics_update_time: 0.01,
            pos: (0.0, 0.0, 1.7),
            dir: (0.0, -1.0, 0.0),
            up: (0.0, 0.0, 1.0),
            smoother_strength: 1.0,
            is_send_immediately: false,
            is_using_gravity: false,
            is_allow_wheel_nodes: true,
            is_visualised: true,
            is_snapping_desired: false,
            is_force_inside_triangle: false,
            is_dir_world_space: false,
        }
    }
}

/// A single IMU reading.
#[derive(Debug, Clone, Default)]
pub struct ImuReading {
    pub time: f64,
    pub mass: f64,
    pub acc_raw: Vec3,
    pub acc_smooth: Vec3,
    pub ang_vel: Vec3,
    pub ang_vel_smooth: Vec3,
    pub pos: Vec3,
    pub dir_x: Vec3,
    pub dir_y: Vec3,
    pub dir_z: Vec3,
}

fn extract_vec3(val: &rmpv::Value) -> Vec3 {
    if let Some(arr) = val.as_array() {
        if arr.len() >= 3 {
            return (
                arr[0].as_f64().unwrap_or(0.0),
                arr[1].as_f64().unwrap_or(0.0),
                arr[2].as_f64().unwrap_or(0.0),
            );
        }
    }
    (0.0, 0.0, 0.0)
}

fn parse_reading(map: &beamng_proto::types::StrDict) -> ImuReading {
    ImuReading {
        time: map.get("time").and_then(|v| v.as_f64()).unwrap_or(0.0),
        mass: map.get("mass").and_then(|v| v.as_f64()).unwrap_or(0.0),
        acc_raw: map.get("accRaw").map(extract_vec3).unwrap_or_default(),
        acc_smooth: map.get("accSmooth").map(extract_vec3).unwrap_or_default(),
        ang_vel: map.get("angVel").map(extract_vec3).unwrap_or_default(),
        ang_vel_smooth: map
            .get("angVelSmooth")
            .map(extract_vec3)
            .unwrap_or_default(),
        pos: map.get("pos").map(extract_vec3).unwrap_or_default(),
        dir_x: map.get("dirX").map(extract_vec3).unwrap_or_default(),
        dir_y: map.get("dirY").map(extract_vec3).unwrap_or_default(),
        dir_z: map.get("dirZ").map(extract_vec3).unwrap_or_default(),
    }
}

/// Parse a list of readings from a response value.
///
/// The simulator returns a Map with numeric F64 keys (0.0, 1.0, 2.0, ...)
/// where each value is a reading map with string keys.
fn parse_readings(val: &rmpv::Value) -> Vec<ImuReading> {
    match val {
        rmpv::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| {
                beamng_proto::types::value_to_str_dict(v.clone()).map(|m| parse_reading(&m))
            })
            .collect(),
        rmpv::Value::Map(pairs) => {
            let mut readings: Vec<(f64, ImuReading)> = pairs
                .iter()
                .filter_map(|(k, v)| {
                    let idx = k.as_f64().or_else(|| k.as_u64().map(|i| i as f64))?;
                    let map = beamng_proto::types::value_to_str_dict(v.clone())?;
                    Some((idx, parse_reading(&map)))
                })
                .collect();
            readings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            readings.into_iter().map(|(_, r)| r).collect()
        }
        _ => vec![],
    }
}

/// An Advanced IMU sensor attached to a vehicle (GE-level).
pub struct AdvancedImu {
    name: String,
    vid: String,
    is_send_immediately: bool,
}

impl AdvancedImu {
    /// Open an Advanced IMU sensor in the simulator, attached to the given vehicle.
    pub async fn open(
        name: impl Into<String>,
        bng: &mut BeamNg,
        vehicle: &Vehicle,
        config: AdvancedImuConfig,
    ) -> Result<Self> {
        let name = name.into();
        let vid = vehicle.vid.clone();

        let fields: Vec<(&str, rmpv::Value)> = vec![
            ("name", rmpv::Value::from(name.as_str())),
            ("vid", rmpv::Value::from(vid.as_str())),
            ("GFXUpdateTime", rmpv::Value::from(config.gfx_update_time)),
            (
                "physicsUpdateTime",
                rmpv::Value::from(config.physics_update_time),
            ),
            (
                "pos",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(config.pos.0),
                    rmpv::Value::from(config.pos.1),
                    rmpv::Value::from(config.pos.2),
                ]),
            ),
            (
                "dir",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(config.dir.0),
                    rmpv::Value::from(config.dir.1),
                    rmpv::Value::from(config.dir.2),
                ]),
            ),
            (
                "up",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(config.up.0),
                    rmpv::Value::from(config.up.1),
                    rmpv::Value::from(config.up.2),
                ]),
            ),
            (
                "smootherStrength",
                rmpv::Value::from(config.smoother_strength),
            ),
            (
                "isSendImmediately",
                rmpv::Value::from(config.is_send_immediately),
            ),
            ("isUsingGravity", rmpv::Value::from(config.is_using_gravity)),
            (
                "isAllowWheelNodes",
                rmpv::Value::from(config.is_allow_wheel_nodes),
            ),
            ("isVisualised", rmpv::Value::from(config.is_visualised)),
            (
                "isSnappingDesired",
                rmpv::Value::from(config.is_snapping_desired),
            ),
            (
                "isForceInsideTriangle",
                rmpv::Value::from(config.is_force_inside_triangle),
            ),
            (
                "isDirWorldSpace",
                rmpv::Value::from(config.is_dir_world_space),
            ),
        ];

        bng.conn()?
            .ack("OpenAdvancedIMU", "OpenedAdvancedIMU", &fields)
            .await?;

        info!("Opened AdvancedIMU: \"{}\"", name);

        Ok(Self {
            name,
            vid,
            is_send_immediately: config.is_send_immediately,
        })
    }

    /// Poll the sensor for readings.
    ///
    /// Returns a list of readings accumulated since the last poll (bulk mode)
    /// or the single latest reading (immediate mode).
    pub async fn poll(&self, bng: &mut BeamNg) -> Result<Vec<ImuReading>> {
        let resp = if self.is_send_immediately {
            // VE poll would require the sensorId and vehicle connection.
            // For simplicity, use GE poll which works for both modes.
            bng.conn()?
                .request(
                    "PollAdvancedImuGE",
                    &[("name", rmpv::Value::from(self.name.as_str()))],
                )
                .await?
        } else {
            bng.conn()?
                .request(
                    "PollAdvancedImuGE",
                    &[("name", rmpv::Value::from(self.name.as_str()))],
                )
                .await?
        };

        let readings = resp.get("data").map(parse_readings).unwrap_or_default();

        Ok(readings)
    }

    /// Close the sensor.
    pub async fn close(self, bng: &mut BeamNg) -> Result<()> {
        bng.conn()?
            .ack(
                "CloseAdvancedIMU",
                "ClosedAdvancedIMU",
                &[
                    ("name", rmpv::Value::from(self.name.as_str())),
                    ("vid", rmpv::Value::from(self.vid.as_str())),
                ],
            )
            .await?;
        info!("Closed AdvancedIMU: \"{}\"", self.name);
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
