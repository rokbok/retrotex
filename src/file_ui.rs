use eframe::egui;

use crate::storage::FileRegistry;

pub(crate) fn show_file_list_panel(
    ctx: &egui::Context,
    file_registry: &FileRegistry,
    active_file_id: &mut u128,
) {
    egui::SidePanel::left("file_list_panel")
        .default_width(220.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Files");
            ui.separator();

            let files = file_registry.files_sorted();

            for (id, name) in &files {
                let selected = *id == *active_file_id;
                if ui.selectable_label(selected, name).clicked() {
                    *active_file_id = *id;
                }
            }
        });
}
