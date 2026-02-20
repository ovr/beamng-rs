use beamng_proto::types::StrDict;
use beamng_proto::Result;

use crate::beamng::BeamNg;

/// API for controlling in-game environment variables: time of day, weather, gravity.
pub struct EnvironmentApi<'a> {
    pub(crate) bng: &'a BeamNg,
}

impl EnvironmentApi<'_> {
    /// Get the current time-of-day object.
    pub async fn get_tod(&self) -> Result<StrDict> {
        self.bng.conn()?.request("GetTimeOfDay", &[]).await
    }

    /// Set the time of day and related parameters.
    pub async fn set_tod(
        &self,
        tod: Option<f64>,
        play: Option<bool>,
        day_scale: Option<f64>,
        night_scale: Option<f64>,
        day_length: Option<f64>,
        azimuth_override: Option<f64>,
    ) -> Result<()> {
        let mut fields: Vec<(&str, rmpv::Value)> = Vec::new();
        if let Some(t) = tod {
            fields.push(("time", rmpv::Value::from(t)));
        }
        if let Some(p) = play {
            fields.push(("play", rmpv::Value::from(p)));
        }
        if let Some(ds) = day_scale {
            fields.push(("dayScale", rmpv::Value::from(ds)));
        }
        if let Some(ns) = night_scale {
            fields.push(("nightScale", rmpv::Value::from(ns)));
        }
        if let Some(dl) = day_length {
            fields.push(("dayLength", rmpv::Value::from(dl)));
        }
        if let Some(az) = azimuth_override {
            fields.push(("azimuthOverride", rmpv::Value::from(az)));
        }
        self.bng
            .conn()?
            .ack("TimeOfDayChange", "TimeOfDayChanged", &fields)
            .await
    }

    /// Set a weather preset.
    pub async fn set_weather_preset(&self, preset: &str, time: f64) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "SetWeatherPreset",
                "WeatherPresetChanged",
                &[
                    ("preset", rmpv::Value::from(preset)),
                    ("time", rmpv::Value::from(time)),
                ],
            )
            .await
    }

    /// Get the current gravity value.
    pub async fn get_gravity(&self) -> Result<f64> {
        let resp = self.bng.conn()?.request("GetGravity", &[]).await?;
        resp.get("gravity")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| beamng_proto::BngError::ValueError("Missing gravity value".into()))
    }

    /// Set the gravity value. Earth default is -9.807.
    pub async fn set_gravity(&self, gravity: f64) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "SetGravity",
                "GravitySet",
                &[("gravity", rmpv::Value::from(gravity))],
            )
            .await
    }
}
