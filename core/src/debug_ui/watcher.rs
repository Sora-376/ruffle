use crate::context::UpdateContext;
use crate::avm1::{Activation, ActivationIdentifier, NativeObject, Value};
use crate::display_object::TDisplayObject;

use egui::{Grid, TextEdit, Ui, Window};

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
        context: &mut UpdateContext,
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
                Panel::All => self.show_all(ui, context),
            }
        });

        keep_open
    }

    fn show_favorite(&mut self) {}

    fn show_all(
        &mut self,
        ui: &mut Ui,
        context: &mut UpdateContext,
    ) {
        TextEdit::singleline(&mut self.search)
        .hint_text("Search")
        .show(ui);

        ui.separator();

        let mut activation = Activation::from_stub(context, ActivationIdentifier::root("[Watcher]"));
        let global_object = activation.global_object();
        let root_obj = activation.context.stage.root_clip().and_then(|c| c.object1());

        Grid::new(ui.id().with("watcher_all_grid"))
        .num_columns(3)
        .show(ui, |ui| {
            Self::render_scope_rows(ui, &mut activation, "_global", global_object, &self.search);
            if let Some(root_obj, ) = root_obj {
                Self::render_scope_rows(ui, &mut activation, "_root", root_obj, &self.search);
            }
        });
    }

    fn render_scope_rows<'gc>(
        ui: &mut Ui,
        activation: &mut Activation<'_, 'gc>,
        scope_label: &str,
        object: crate::avm1::Object<'gc>,
        search: &str,
    ) {
        for key in object.get_keys(activation, false) {
            let name = key.to_string();
            if !search.is_empty() && !name.contains(search) { continue; }

            if let Ok(value) = object.get(key, activation) {
                match &value {
                    Value::MovieClip(_) => continue,
                    Value::Object(obj) if matches!(obj.native(), NativeObject::Function(_)) => continue,
                    _ => {}
                }

                let value_str = Self::format_value_safely(&value);

                ui.label(format!("{scope_label}.{name}"));
                ui.label(value_str);
                if ui.button("favorite").clicked() {
                    println!("favorite {scope_label}.{name}");
                }
                ui.end_row();
            }
        }
    }

    fn format_value_safely(value: &Value) -> String {
        match value {
            Value::Bool(b) => b.to_string(),
            Value::String(s) => s.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Object(_) => "[Object]".to_string(),
            Value::MovieClip(mcr) => mcr.path().to_string(),
            Value::Null => "null".to_string(),
            Value::Undefined => "undefined".to_string(),
        }
    }
}