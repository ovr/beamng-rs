use beamng_proto::types::StrDict;
use beamng_proto::Result;

use crate::beamng::BeamNg;

/// API for getting info about the host system running the simulator.
pub struct SystemApi<'a> {
    pub(crate) bng: &'a mut BeamNg,
}

impl SystemApi<'_> {
    /// Returns information about the host's system.
    pub async fn get_info(
        &mut self,
        os: bool,
        cpu: bool,
        gpu: bool,
        power: bool,
    ) -> Result<StrDict> {
        self.bng
            .conn()?
            .request(
                "GetSystemInfo",
                &[
                    ("os", rmpv::Value::from(os)),
                    ("cpu", rmpv::Value::from(cpu)),
                    ("gpu", rmpv::Value::from(gpu)),
                    ("power", rmpv::Value::from(power)),
                ],
            )
            .await
    }

    /// Returns the environment filesystem paths of the BeamNG simulator.
    pub async fn get_environment_paths(&mut self) -> Result<StrDict> {
        self.bng.conn()?.request("GetEnvironmentPaths", &[]).await
    }
}
