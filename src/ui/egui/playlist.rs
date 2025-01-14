use egui::{Label, Ui};
use egui_extras::{Column, TableBuilder};

use crate::core::{feusic::loader::MusicLoader, player::controller::FeusicPlayerController};

pub(super) fn render<M: MusicLoader>(ui: &mut Ui, player: &FeusicPlayerController<M>) {
    TableBuilder::new(ui)
        .column(Column::auto())
        .column(Column::remainder())
        .sense(egui::Sense::click())
        .body(|body| {
            let feusic_index = player.feusic_index();
            let feusic_names_ref = player.feusic_names();
            let feusic_names = feusic_names_ref.get();

            body.rows(18.0, feusic_names.len(), |mut row| {
                let index = row.index();
                row.set_selected(index == feusic_index);

                row.col(|ui| {
                    ui.add(Label::new((index + 1).to_string()).selectable(false));
                });

                row.col(|ui| {
                    ui.add(Label::new(feusic_names[index].clone()).selectable(false));
                });

                if row.response().double_clicked() {
                    player.play_index(index);
                }
            });
        });
}
