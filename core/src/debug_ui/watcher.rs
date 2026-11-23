use crate::avm1::{Activation, ActivationIdentifier, NativeObject, Object, Value};
use crate::context::UpdateContext;
use crate::display_object::TDisplayObject;

use egui::{Grid, TextEdit, Ui, Window};

#[derive(Debug, Eq, PartialEq, Hash, Default, Copy, Clone)]
enum Panel {
    #[default]
    Favorite,
    All,
}

#[derive(Debug, Clone)]
enum WatcherScope {
    Root,
    Global,
}

#[derive(Debug)]
struct WatcherItem {
    scope: WatcherScope,
    name: String,
    value_display: String,
}

#[derive(Debug, Default)]
pub struct WatcherWindow {
    open_panel: Panel,
    search: String,
    // favorites: Vec<WatcherItem>,
    all_cache: Vec<WatcherItem>,
    last_scan: Option<std::time::Instant>,
}

impl WatcherWindow {
    pub fn show(&mut self, egui_ctx: &egui::Context, context: &mut UpdateContext) -> bool {
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

    fn show_all(&mut self, ui: &mut Ui, context: &mut UpdateContext) {
        ui.horizontal(|ui| {
            TextEdit::singleline(&mut self.search)
                .hint_text("Search")
                .show(ui);
            if ui.button("Refersh").clicked() {
                self.refresh_all(context);
            }
        });

        ui.separator();

        if let Some(t) = self.last_scan {
            ui.label(format!(
                "Last Scan: {:.1}s ago / {} item in total",
                t.elapsed().as_secs_f32(),
                self.all_cache.len()
            ));
            ui.separator();
        }

        let mut pending_favorite: Option<usize> = None;

        Grid::new(ui.id().with("watcher_all_grid"))
            .num_columns(3)
            .show(ui, |ui| {
                for (i, item) in self.all_cache.iter().enumerate() {
                    if !self.search.is_empty() && !item.name.contains(&self.search) {
                        continue;
                    }
                    let scope_str = match item.scope {
                        WatcherScope::Root => "_root",
                        WatcherScope::Global => "_global",
                    };
                    ui.label(format!("{scope_str}.{}", item.name));
                    ui.label(&item.value_display);
                    if ui.button("Favorite").clicked() {
                        pending_favorite = Some(i);
                    }
                    ui.end_row();
                }
            });

        // if let Some(i) = pending_favorite {
        //     let item = &self.all_cache[i];
        //     self.favorites.push();
        // }
    }

    fn refresh_all(&mut self, context: &mut UpdateContext) {
        let mut activation =
            Activation::from_stub(context, ActivationIdentifier::root("[Watcher]"));
        let mut result = Vec::new();

        let global_obj = activation.global_object();
        Self::scan_object(
            &mut activation,
            WatcherScope::Global,
            global_obj,
            &mut result,
        );

        if let Some(root_obj) = activation
            .context
            .stage
            .root_clip()
            .and_then(|c| c.object1())
        {
            Self::scan_object(&mut activation, WatcherScope::Root, root_obj, &mut result);
        }

        self.all_cache = result;
        self.last_scan = Some(std::time::Instant::now());
    }

    fn scan_object<'gc>(
        activation: &mut Activation<'_, 'gc>,
        scope: WatcherScope,
        object: Object<'gc>,
        out: &mut Vec<WatcherItem>,
    ) {
        for key in object.get_keys(activation, false) {
            let name = key.to_string();

            if let Ok(value) = object.get(key, activation) {
                match &value {
                    Value::MovieClip(_) => continue,
                    Value::Object(obj) if matches!(obj.native(), NativeObject::Function(_)) => {
                        continue;
                    }
                    _ => {}
                }

                out.push(WatcherItem {
                    scope: scope.clone(),
                    name,
                    value_display: Self::format_value_safely(&value),
                });
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
