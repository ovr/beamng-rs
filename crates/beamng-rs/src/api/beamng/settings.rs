use beamng_proto::Result;

use crate::beamng::BeamNg;

/// API for changing simulator settings.
pub struct SettingsApi<'a> {
    pub(crate) bng: &'a BeamNg,
}

impl SettingsApi<'_> {
    /// Change a game setting.
    pub async fn change(&self, key: &str, value: &str) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "ChangeSetting",
                "SettingsChanged",
                &[
                    ("key", rmpv::Value::from(key)),
                    ("value", rmpv::Value::from(value)),
                ],
            )
            .await
    }

    /// Apply pending graphics settings.
    pub async fn apply_graphics(&self) -> Result<()> {
        self.bng
            .conn()?
            .ack("ApplyGraphicsSetting", "GraphicsSettingApplied", &[])
            .await
    }

    /// Enable deterministic mode.
    pub async fn set_deterministic(
        &self,
        steps_per_second: Option<i32>,
        speed_factor: Option<i32>,
    ) -> Result<()> {
        let mut fields: Vec<(&str, rmpv::Value)> = Vec::new();
        if let Some(sf) = speed_factor {
            fields.push(("speedFactor", rmpv::Value::from(sf)));
        }
        self.bng
            .conn()?
            .ack(
                "SetPhysicsDeterministic",
                "SetPhysicsDeterministic",
                &fields,
            )
            .await?;

        if let Some(sps) = steps_per_second {
            self.set_steps_per_second(sps).await?;
        }
        Ok(())
    }

    /// Disable deterministic mode.
    pub async fn set_nondeterministic(&self) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "SetPhysicsNonDeterministic",
                "SetPhysicsNonDeterministic",
                &[],
            )
            .await
    }

    /// Set the steps per second (temporal resolution).
    pub async fn set_steps_per_second(&self, sps: i32) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "FPSLimit",
                "SetFPSLimit",
                &[("fps", rmpv::Value::from(sps))],
            )
            .await
    }

    /// Remove the steps-per-second limit.
    pub async fn remove_step_limit(&self) -> Result<()> {
        self.bng
            .conn()?
            .ack("RemoveFPSLimit", "RemovedFPSLimit", &[])
            .await
    }

    /// Enable or disable visual particle emission.
    pub async fn set_particles_enabled(&self, enabled: bool) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "ParticlesEnabled",
                "ParticlesSet",
                &[("enabled", rmpv::Value::from(enabled))],
            )
            .await
    }
}
