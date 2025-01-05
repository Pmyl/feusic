use std::{error::Error, time::Duration};

use imgui::{Condition, Ui};

use crate::core::player::PhasicPlayerController;

pub(super) fn render(ui: &Ui, player: &PhasicPlayerController) -> Result<(), Box<dyn Error>> {
    ui.window("Hello world")
        .size([300.0, 100.0], Condition::FirstUseEver)
        .resizable(false)
        .build(|| {
            ui.separator();
            ui.text("Controls");
            ui.separator();
            let pressed = ui.button("Next");
            if pressed {
                player.next();
            }
            let pressed = ui.button("Crossfade");
            if pressed {
                player.crossfade(Duration::from_millis(1000));
            }
        });

    Ok(())
}
