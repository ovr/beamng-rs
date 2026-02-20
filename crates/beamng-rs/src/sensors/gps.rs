use beamng_proto::types::Vec3;
use beamng_proto::Result;
use tracing::info;

use crate::beamng::BeamNg;
use crate::vehicle::Vehicle;

/// Configuration for a [`Gps`] sensor.
#[derive(Debug, Clone)]
pub struct GpsConfig {
    pub gfx_update_time: f64,
    pub physics_update_time: f64,
    pub pos: Vec3,
    pub ref_lon: f64,
    pub ref_lat: f64,
    pub is_send_immediately: bool,
    pub is_visualised: bool,
    pub is_snapping_desired: bool,
    pub is_force_inside_triangle: bool,
    pub is_dir_world_space: bool,
}

impl Default for GpsConfig {
    fn default() -> Self {
        Self {
            gfx_update_time: 0.0,
            physics_update_time: 0.01,
            pos: (0.0, 0.0, 1.7),
            ref_lon: 0.0,
            ref_lat: 0.0,
            is_send_immediately: false,
            is_visualised: true,
            is_snapping_desired: false,
            is_force_inside_triangle: false,
            is_dir_world_space: false,
        }
    }
}

/// A single GPS reading.
#[derive(Debug, Clone, Default)]
pub struct GpsReading {
    pub time: f64,
    pub x: f64,
    pub y: f64,
    pub lon: f64,
    pub lat: f64,
}

fn parse_reading(map: &beamng_proto::types::StrDict) -> GpsReading {
    GpsReading {
        time: map.get("time").and_then(|v| v.as_f64()).unwrap_or(0.0),
        x: map.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
        y: map.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
        lon: map.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0),
        lat: map.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0),
    }
}

fn parse_readings(val: &rmpv::Value) -> Vec<GpsReading> {
    match val {
        rmpv::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| {
                beamng_proto::types::value_to_str_dict(v.clone()).map(|m| parse_reading(&m))
            })
            .collect(),
        rmpv::Value::Map(pairs) => {
            let mut readings: Vec<(f64, GpsReading)> = pairs
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

/// A GPS sensor attached to a vehicle (GE-level).
pub struct Gps {
    name: String,
    vid: String,
    #[allow(dead_code)]
    is_send_immediately: bool,
}

impl Gps {
    /// Open a GPS sensor in the simulator, attached to the given vehicle.
    pub async fn open(
        name: impl Into<String>,
        bng: &mut BeamNg,
        vehicle: &Vehicle,
        config: GpsConfig,
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
            ("refLon", rmpv::Value::from(config.ref_lon)),
            ("refLat", rmpv::Value::from(config.ref_lat)),
            (
                "isSendImmediately",
                rmpv::Value::from(config.is_send_immediately),
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

        bng.conn()?.ack("OpenGPS", "OpenedGPS", &fields).await?;

        info!("Opened GPS: \"{}\"", name);

        Ok(Self {
            name,
            vid,
            is_send_immediately: config.is_send_immediately,
        })
    }

    /// Poll the sensor for readings.
    pub async fn poll(&self, bng: &mut BeamNg) -> Result<Vec<GpsReading>> {
        let resp = bng
            .conn()?
            .request(
                "PollGPSGE",
                &[("name", rmpv::Value::from(self.name.as_str()))],
            )
            .await?;

        let readings = resp.get("data").map(parse_readings).unwrap_or_default();

        Ok(readings)
    }

    /// Close the sensor.
    pub async fn close(self, bng: &mut BeamNg) -> Result<()> {
        bng.conn()?
            .ack(
                "CloseGPS",
                "ClosedGPS",
                &[
                    ("name", rmpv::Value::from(self.name.as_str())),
                    ("vid", rmpv::Value::from(self.vid.as_str())),
                ],
            )
            .await?;
        info!("Closed GPS: \"{}\"", self.name);
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
