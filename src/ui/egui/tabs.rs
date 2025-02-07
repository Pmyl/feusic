
use egui::Ui;


use super::FeusicEguiScreen;

pub(super) fn render(ui: &mut Ui, screen: &FeusicEguiScreen) -> Option<FeusicEguiScreen> {
    let mut new_screen = None;

    ui.horizontal(|ui| {
        if ui
            .selectable_label(matches!(screen, FeusicEguiScreen::Main), "Player")
            .clicked()
        {
            new_screen = Some(FeusicEguiScreen::Main);
        }

        if ui
            .selectable_label(matches!(screen, FeusicEguiScreen::Youtube), "Youtube")
            .clicked()
        {
            new_screen = Some(FeusicEguiScreen::Youtube);
        }
    });

    new_screen
}
