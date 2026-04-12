use eframe::egui;

use crate::prelude::*;
use crate::FileNameDialogMode;
use crate::UiData;

pub(crate) fn show_file_list_panel(
    ctx: &egui::Context,
    files: &[(FileId, String)],
    active_file_id: &FileId,
    ui_data: &mut UiData,
) -> Option<FileId> {
    let mut ret = None;
    egui::SidePanel::left("file_list_panel")
        .default_width(220.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Files");
            ui.separator();

            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    for (id, name) in files {
                        let selected = *id == *active_file_id;
                        if ui.selectable_label(selected, name).clicked() {
                            ret = Some(*id);
                        }
                    }
                    if ui
                        .add_sized([ui.available_width(), ui.spacing().interact_size.y], egui::Button::new("New"))
                        .clicked()
                    {
                        ui_data.file_name_dialog = Some(FileNameDialogMode::Create);
                        ui_data.file_name_dialog_just_opened = true;
                    }
                });
        });
    ret
}
