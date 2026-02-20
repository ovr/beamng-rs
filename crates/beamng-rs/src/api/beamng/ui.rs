use beamng_proto::Result;

use crate::beamng::BeamNg;

/// API for controlling the simulator's GUI.
pub struct UiApi<'a> {
    pub(crate) bng: &'a mut BeamNg,
}

impl UiApi<'_> {
    /// Display a toast message in the simulator UI.
    pub async fn display_message(&mut self, msg: &str) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "DisplayGuiMessage",
                "GuiMessageDisplayed",
                &[("message", rmpv::Value::from(msg))],
            )
            .await
    }

    /// Hide the HUD.
    pub async fn hide_hud(&mut self) -> Result<()> {
        self.bng.conn()?.send_raw("HideHUD", &[]).await?;
        Ok(())
    }

    /// Show the HUD.
    pub async fn show_hud(&mut self) -> Result<()> {
        self.bng.conn()?.send_raw("ShowHUD", &[]).await?;
        Ok(())
    }
}
