use egui_file_dialog::{DialogState, FileDialog};
use servo::{ContextMenuAction, ContextMenuItem, EmbedderControl, EmbedderControlId, RgbColor, SelectElementOptionOrOptgroup, SimpleDialog, WebView, WebViewId};
use url::Url;

pub enum ControlOutcome {
    Pending,
    Done,
    OpenLink(Url),
}

pub struct PageControl {
    owner: WebViewId,
    origin: String,
    control: Option<EmbedderControl>,
    selection: Vec<usize>,
    color: egui::Color32,
    file_dialog: Option<FileDialog>,
}

impl PageControl {
    pub fn new(view: WebView, control: EmbedderControl) -> Self {
        let selection = match &control {
            EmbedderControl::SelectElement(select) => select.selected_options(),
            _ => Vec::new(),
        };
        let color = match &control {
            EmbedderControl::ColorPicker(picker) => picker.current_color().map(|color| egui::Color32::from_rgb(color.red, color.green, color.blue)).unwrap_or(egui::Color32::BLACK),
            _ => egui::Color32::BLACK,
        };
        Self {
            owner: view.id(),
            origin: view.url().map(|url| url.origin().ascii_serialization()).unwrap_or_else(|| "Unknown origin".into()),
            control: Some(control),
            selection,
            color,
            file_dialog: None,
        }
    }

    pub fn matches(&self, owner: WebViewId, id: EmbedderControlId) -> bool {
        self.owner == owner && self.control.as_ref().is_some_and(|control| control.id() == id)
    }

    pub fn draw(&mut self, ctx: &egui::Context) -> ControlOutcome {
        let Some(control) = self.control.as_mut() else { return ControlOutcome::Done };
        if let EmbedderControl::FilePicker(picker) = control {
            if let Some(dialog) = self.file_dialog.as_mut() {
                let paths = match dialog.update(ctx).state() {
                    DialogState::Picked(path) => Some(vec![path.clone()]),
                    DialogState::PickedMultiple(paths) => Some(paths.clone()),
                    DialogState::Cancelled | DialogState::Closed => Some(Vec::new()),
                    DialogState::Open => None,
                };
                if let Some(paths) = paths {
                    if let Some(EmbedderControl::FilePicker(mut picker)) = self.control.take() {
                        if paths.is_empty() { picker.dismiss(); }
                        else { picker.select(&paths); picker.submit(); }
                    }
                    return ControlOutcome::Done;
                }
                return ControlOutcome::Pending;
            }
            let mut choose = false;
            let mut cancel = false;
            let origin = &self.origin;
            egui::Modal::new("lutra-file-confirm".into()).show(ctx, |ui| {
                ui.heading("Choose files for website");
                ui.label(origin);
                ui.label("Only files you explicitly select will be shared.");
                ui.horizontal(|ui| {
                    choose = ui.button("Choose files").clicked();
                    cancel = ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape));
                });
            });
            if cancel {
                if let Some(EmbedderControl::FilePicker(picker)) = self.control.take() { picker.dismiss(); }
                return ControlOutcome::Done;
            }
            if choose {
                let mut dialog = FileDialog::new();
                if picker.allow_select_multiple() { dialog.pick_multiple(); } else { dialog.pick_file(); }
                self.file_dialog = Some(dialog);
            }
            return ControlOutcome::Pending;
        }

        let mut accept = false;
        let mut cancel = false;
        let mut menu_action = None;
        let origin = &self.origin;
        let selection = &mut self.selection;
        let color = &mut self.color;
        egui::Modal::new("lutra-page-control".into()).show(ctx, |ui| {
            ui.heading("Website control");
            ui.label(origin);
            ui.separator();
            match control {
                EmbedderControl::SimpleDialog(dialog) => {
                    ui.label(dialog.message());
                    if let SimpleDialog::Prompt(prompt) = dialog {
                        let mut value = prompt.current_value().to_owned();
                        ui.text_edit_singleline(&mut value);
                        prompt.set_current_value(&value);
                    }
                    accept = ui.button("OK").clicked();
                    if !matches!(dialog, SimpleDialog::Alert(_)) { cancel = ui.button("Cancel").clicked(); }
                }
                EmbedderControl::SelectElement(select) => {
                    let multiple = select.allow_select_multiple();
                    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        let mut render = |ui: &mut egui::Ui, option: &servo::SelectElementOption| {
                            let selected = selection.contains(&option.id);
                            if ui.add_enabled(!option.is_disabled, egui::Button::new(&option.label).selected(selected)).clicked() {
                                if multiple {
                                    if selected { selection.retain(|id| *id != option.id); }
                                    else { selection.push(option.id); }
                                } else {
                                    selection.clear();
                                    selection.push(option.id);
                                    accept = true;
                                }
                            }
                        };
                        for option in select.options() {
                            match option {
                                SelectElementOptionOrOptgroup::Option(option) => render(ui, option),
                                SelectElementOptionOrOptgroup::Optgroup { label, options } => {
                                    ui.strong(label);
                                    for option in options { render(ui, option); }
                                }
                            }
                        }
                    });
                    if ui.button("Apply").clicked() { accept = true; }
                    cancel = ui.button("Cancel").clicked();
                }
                EmbedderControl::ColorPicker(_) => {
                    ui.color_edit_button_srgba(color);
                    accept = ui.button("Apply").clicked();
                    cancel = ui.button("Cancel").clicked();
                }
                EmbedderControl::ContextMenu(menu) => {
                    for item in menu.items() {
                        match item {
                            ContextMenuItem::Item { label, action, enabled } => {
                                if ui.add_enabled(*enabled, egui::Button::new(label)).clicked() { menu_action = Some(*action); }
                            }
                            ContextMenuItem::Separator => { ui.separator(); }
                        }
                    }
                    cancel = ui.button("Close menu").clicked();
                }
                _ => { cancel = true; }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) { cancel = true; }
        });

        if cancel {
            if let Some(control) = self.control.take() {
                match control {
                    EmbedderControl::SimpleDialog(dialog) => dialog.dismiss(),
                    EmbedderControl::ContextMenu(menu) => menu.dismiss(),
                    _ => {}
                }
            }
            return ControlOutcome::Done;
        }
        if let Some(action) = menu_action {
            if let Some(EmbedderControl::ContextMenu(menu)) = self.control.take() {
                let target = match action {
                    ContextMenuAction::OpenLinkInNewWebView => menu.element_info().link_url.clone(),
                    ContextMenuAction::OpenImageInNewView => menu.element_info().image_url.clone(),
                    _ => None,
                };
                if matches!(action, ContextMenuAction::OpenLinkInNewWebView | ContextMenuAction::OpenImageInNewView) {
                    menu.dismiss();
                    return target.map(ControlOutcome::OpenLink).unwrap_or(ControlOutcome::Done);
                }
                menu.select(action);
            }
            return ControlOutcome::Done;
        }
        if accept {
            if let Some(control) = self.control.take() {
                match control {
                    EmbedderControl::SimpleDialog(dialog) => dialog.confirm(),
                    EmbedderControl::SelectElement(mut select) => { select.select(self.selection.clone()); select.submit(); }
                    EmbedderControl::ColorPicker(mut picker) => {
                        picker.select(Some(RgbColor { red: self.color.r(), green: self.color.g(), blue: self.color.b() }));
                        picker.submit();
                    }
                    _ => {}
                }
            }
            return ControlOutcome::Done;
        }
        ControlOutcome::Pending
    }
}
