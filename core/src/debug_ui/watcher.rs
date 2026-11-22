// use crate::context::UpdateContext;
// use crate::debug_ui::Message;

use egui::{TextEdit, Ui, Window};

#[derive(Debug, Eq, PartialEq, Hash, Default, Copy, Clone)]
pub enum Panel {
    #[default]
    Favorite,
    All,
}

#[derive(Debug, Default)]
pub struct WatcherWindow {
    open_panel: Panel,
    search: String,
}

impl WatcherWindow {
    pub fn show(
        &mut self,
        egui_ctx: &egui::Context,
        // context: &mut UpdateContext,
        // messages: &mut Vec<Message>,
    ) -> bool {
        let mut keep_open: bool = true;

        Window::new("Watcher")
            .open(&mut keep_open)
            .scroll([true, true])
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.open_panel, Panel::Favorite, "Favorite");
                    ui.selectable_value(&mut self.open_panel, Panel::All, "All");
                });
                ui.separator();

                match self.open_panel {
                    Panel::Favorite => self.show_favorite(),
                    Panel::All => self.show_all(ui),
                }
            });

        keep_open
    }

    fn show_favorite(&mut self) {}
    fn show_all(
        &mut self,
        ui: &mut Ui,
        // context: &mut UpdateContext,
    ) {
        TextEdit::singleline(&mut self.search)
        .hint_text("Search for member or variable")
        .show(ui);
    }
}