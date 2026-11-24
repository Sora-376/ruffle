use crate::avm1::{Activation, ActivationIdentifier, NativeObject, Object, Value};
use crate::context::UpdateContext;
use crate::display_object::TDisplayObject;
use ruffle_common::avm_string::AvmString;

use egui::{Grid, TextEdit, Ui, Window};

#[derive(Debug, PartialEq, Default, Clone)]
enum Panel {
    #[default]
    Favorite,
    All,
}

#[derive(Debug, Clone, PartialEq)]
enum WatcherScope {
    Root,
    Global,
}

#[derive(Debug, Clone, PartialEq)]
enum ParsedValue {
    Number(f64),
    Bool(bool),
    String(String),
}

#[derive(Debug)]
struct WatcherItem {
    scope: WatcherScope,
    name: String,
    value_display: String,
}

#[derive(Debug, PartialEq)]
struct FavoriteEntry {
    scope: WatcherScope,
    name: String,
    edit_buffer: String,
    is_frozen: bool,
    frozen_value: Option<ParsedValue>,
}

impl FavoriteEntry {
    fn new(scope: WatcherScope, name: String) -> Self {
        Self {
            scope,
            name,
            edit_buffer: String::from(""),
            is_frozen: false,
            frozen_value: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct WatcherWindow {
    open_panel: Panel,
    search: String,
    favorites: Vec<FavoriteEntry>,
    all_cache: Vec<WatcherItem>,
}

impl WatcherWindow {
    pub fn show(&mut self, egui_ctx: &egui::Context, context: &mut UpdateContext) -> bool {
        let mut keep_open: bool = true;

        Window::new("Watcher")
            .open(&mut keep_open)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.open_panel, Panel::Favorite, "Favorite");
                    ui.selectable_value(&mut self.open_panel, Panel::All, "All");
                });

                ui.separator();

                match self.open_panel {
                    Panel::Favorite => self.show_favorite(ui, context),
                    Panel::All => self.show_all(ui, context),
                }
            });

        keep_open
    }

    fn show_favorite(&mut self, ui: &mut Ui, context: &mut UpdateContext) {
        let mut activation =
            Activation::from_stub(context, ActivationIdentifier::root("[Watcher]"));
        let mut pending_remove: Option<usize> = None;

        egui::ScrollArea::vertical()
            .id_salt("watcher_favorite_scroll")
            .show(ui, |ui| {
                Grid::new(ui.id().with("watcher_favorite_grid"))
                    .num_columns(4)
                    .show(ui, |ui| {
                        for (i, entry) in self.favorites.iter_mut().enumerate() {
                            let name = AvmString::new_utf8(activation.gc(), entry.name.as_str());

                            let owner = match entry.scope {
                                WatcherScope::Global => Some(activation.global_object()),
                                WatcherScope::Root => activation
                                    .context
                                    .stage
                                    .root_clip()
                                    .and_then(|c| c.object1()),
                            };

                            let Some(owner) = owner else {
                                ui.label(format!("{} (Object is not exist)", entry.name));
                                ui.end_row();
                                continue;
                            };

                            if entry.is_frozen {
                                if let Some(frozen) = &entry.frozen_value {
                                    let value = Self::parsed_to_avm1_value(frozen, &mut activation);
                                    let _ = owner.set(name, value, &mut activation);
                                }
                            }

                            let scope_str = match entry.scope {
                                WatcherScope::Root => "_root",
                                WatcherScope::Global => "_global",
                            };

                            ui.label(format!("{scope_str}.{}", entry.name));

                            if let Ok(current) = owner.get(name, &mut activation) {
                                let display = Self::format_value_safely(&current);
                                ui.label(display.clone());

                                ui.text_edit_singleline(&mut entry.edit_buffer);
                                if ui.button("Save").clicked() {
                                    let parsed = Self::parse_edit_buffer(&entry.edit_buffer);
                                    let value =
                                        Self::parsed_to_avm1_value(&parsed, &mut activation);
                                    let _ = owner.set(name, value, &mut activation);
                                }

                                let freeze_resp = ui.checkbox(&mut entry.is_frozen, "Frozen");
                                if freeze_resp.changed() && entry.is_frozen {
                                    entry.frozen_value = Some(Self::parse_edit_buffer(&display));
                                }
                            } else {
                                ui.label("Null");
                            }

                            if ui.button("Remove").clicked() {
                                pending_remove = Some(i);
                            }
                            ui.end_row();
                        }
                    });
            });

        if let Some(i) = pending_remove {
            self.favorites.remove(i);
        }
    }

    fn is_already_favorite(&self, scope: &WatcherScope, name: &str) -> bool {
        self.favorites
            .iter()
            .any(|e| e.scope == *scope && e.name == name)
    }

    fn parse_edit_buffer(input: &str) -> ParsedValue {
        if let Ok(n) = input.parse::<f64>() {
            ParsedValue::Number(n)
        } else if input.eq_ignore_ascii_case("true") {
            ParsedValue::Bool(true)
        } else if input.eq_ignore_ascii_case("false") {
            ParsedValue::Bool(false)
        } else {
            ParsedValue::String(input.to_string())
        }
    }

    fn parsed_to_avm1_value<'gc>(
        parsed: &ParsedValue,
        activation: &mut Activation<'_, 'gc>,
    ) -> Value<'gc> {
        match parsed {
            ParsedValue::Number(n) => Value::Number(*n),
            ParsedValue::Bool(b) => Value::Bool(*b),
            ParsedValue::String(s) => {
                Value::String(AvmString::new_utf8(activation.gc(), s.as_str()))
            }
        }
    }

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

        ui.label(format!("Item: {}", self.all_cache.len()));

        ui.separator();

        let mut pending_favorite: Option<usize> = None;

        egui::ScrollArea::vertical()
            .id_salt("watcher_all_scroll")
            .show(ui, |ui| {
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
            });

        if let Some(i) = pending_favorite {
            let item = &self.all_cache[i];
            if !self.is_already_favorite(&item.scope, &item.name) {
                self.favorites
                    .push(FavoriteEntry::new(item.scope.clone(), item.name.clone()));
            }
        }
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
