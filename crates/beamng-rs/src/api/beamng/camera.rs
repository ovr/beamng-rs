use beamng_proto::types::{StrDict, Vec3};
use beamng_proto::Result;

use crate::beamng::BeamNg;

fn vec3_val(v: Vec3) -> rmpv::Value {
    rmpv::Value::Array(vec![
        rmpv::Value::from(v.0),
        rmpv::Value::from(v.1),
        rmpv::Value::from(v.2),
    ])
}

/// API for controlling the in-game camera and annotation info.
pub struct CameraApi<'a> {
    pub(crate) bng: &'a BeamNg,
}

impl CameraApi<'_> {
    /// Set the position and direction of the free camera.
    pub async fn set_free(&self, pos: Vec3, direction: Vec3) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "SetFreeCamera",
                "FreeCameraSet",
                &[("pos", vec3_val(pos)), ("dir", vec3_val(direction))],
            )
            .await
    }

    /// Switch the camera to relative mode for the current vehicle.
    pub async fn set_relative(&self, pos: Vec3, dir: Vec3, up: Vec3) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "SetRelativeCam",
                "RelativeCamSet",
                &[
                    ("pos", vec3_val(pos)),
                    ("dir", vec3_val(dir)),
                    ("up", vec3_val(up)),
                ],
            )
            .await
    }

    /// Set the camera mode for a vehicle.
    pub async fn set_player_mode(&self, vid: &str, mode: &str, config: &StrDict) -> Result<()> {
        let config_val = rmpv::Value::Map(
            config
                .iter()
                .map(|(k, v)| (rmpv::Value::from(k.as_str()), v.clone()))
                .collect(),
        );
        self.bng
            .conn()?
            .ack(
                "SetPlayerCameraMode",
                "PlayerCameraModeSet",
                &[
                    ("vid", rmpv::Value::from(vid)),
                    ("mode", rmpv::Value::from(mode)),
                    ("config", config_val),
                ],
            )
            .await
    }

    /// Get camera modes for a vehicle.
    pub async fn get_player_modes(&self, vid: &str) -> Result<StrDict> {
        self.bng
            .conn()?
            .request("GetPlayerCameraMode", &[("vid", rmpv::Value::from(vid))])
            .await
    }

    /// Get annotation configuration (class → RGB color mapping).
    pub async fn get_annotations(&self) -> Result<StrDict> {
        self.bng.conn()?.request("GetAnnotations", &[]).await
    }
}
