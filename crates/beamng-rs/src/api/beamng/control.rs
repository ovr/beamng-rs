use beamng_proto::types::StrDict;
use beamng_proto::Result;

use crate::beamng::BeamNg;

/// API for controlling the flow of the simulation — pausing, resuming, stepping,
/// and executing custom Lua code.
pub struct ControlApi<'a> {
    pub(crate) bng: &'a mut BeamNg,
}

impl ControlApi<'_> {
    /// Pause the simulation.
    pub async fn pause(&mut self) -> Result<()> {
        self.bng.conn()?.ack("Pause", "Paused", &[]).await
    }

    /// Resume the simulation.
    pub async fn resume(&mut self) -> Result<()> {
        self.bng.conn()?.ack("Resume", "Resumed", &[]).await
    }

    /// Advance the simulation by `count` steps.
    ///
    /// If `wait` is true, blocks until the simulator has finished simulating the steps.
    pub async fn step(&mut self, count: u32, wait: bool) -> Result<()> {
        let conn = self.bng.conn()?;
        let fields = &[
            ("count", rmpv::Value::from(count)),
            ("ack", rmpv::Value::from(wait)),
        ];
        if wait {
            conn.ack("Step", "Stepped", fields).await
        } else {
            conn.send_raw("Step", fields).await?;
            Ok(())
        }
    }

    /// Get the current game state.
    ///
    /// Returns a dict with a `"state"` key that is either `"scenario"` or `"menu"`.
    pub async fn get_gamestate(&mut self) -> Result<StrDict> {
        let conn = self.bng.conn()?;
        let resp = conn.request("GameStateRequest", &[]).await?;
        Ok(resp)
    }

    /// Execute a Lua chunk in the game engine VM.
    ///
    /// If `response` is true, the result is sent back from BeamNG.
    pub async fn queue_lua_command(
        &mut self,
        chunk: &str,
        response: bool,
    ) -> Result<Option<rmpv::Value>> {
        let conn = self.bng.conn()?;
        let resp = conn
            .request(
                "QueueLuaCommandGE",
                &[
                    ("chunk", rmpv::Value::from(chunk)),
                    ("resp", rmpv::Value::from(response)),
                ],
            )
            .await?;
        Ok(resp.get("resp").cloned())
    }

    /// Return to the main menu, closing any loaded scenario.
    pub async fn return_to_main_menu(&mut self) -> Result<()> {
        self.bng
            .conn()?
            .ack("StopScenario", "ScenarioStopped", &[])
            .await
    }

    /// Quit the simulator.
    pub async fn quit_beamng(&mut self) -> Result<()> {
        self.bng.conn()?.ack("Quit", "Quit", &[]).await
    }
}
