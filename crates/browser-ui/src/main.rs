use browser_core::BrowserState;
use resource_manager::ResourceManager;
use eframe::egui::{self, Color32, RichText};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::collections::HashMap;
use std::time::Duration;

const MAX_PREVIEWS: usize = 3;

struct BrowserApp {
    browser: BrowserState,
    address: String,
    status: String,
    dark: bool,
    resources: ResourceManager,
    preview_executable: Option<PathBuf>,
    previews: HashMap<u64, Child>,
}

impl Default for BrowserApp {
    fn default() -> Self {
        let mut browser = BrowserState::new();
        let initial = browser.open_tab();
        let mut resources = ResourceManager::default();
        resources.register(initial);
        resources.activate(initial);
        Self { browser, address: String::new(), status: "Servo preview available after building the engine".into(), dark: true, resources, preview_executable: find_preview(), previews: HashMap::new() }
    }
}

impl BrowserApp {
    fn sync_address(&mut self) {
        self.address = self.browser.active().and_then(|t| t.url.as_ref()).map(|u| u.to_string()).unwrap_or_default();
    }

    fn submit(&mut self) {
        let result = BrowserState::parse_address(&self.address).map_err(str::to_owned)
            .and_then(|url| {
                self.open_preview(url.as_str())?;
                self.browser.commit(url).map_err(str::to_owned)?;
                Ok(())
            });
        self.status = match result { Ok(()) => { self.sync_address(); "Renderer started".into() }, Err(error) => error };
    }

    fn history(&mut self, forward: bool) {
        let Some(url) = self.browser.history_target(forward).cloned() else { return };
        match self.open_preview(url.as_str()) {
            Ok(()) => { self.browser.move_history(forward); self.sync_address(); self.status = "Renderer started".into(); }
            Err(error) => self.status = error,
        }
    }

    fn stop_preview(&mut self, id: u64) -> Result<(), String> {
        if let Some(child) = self.previews.get_mut(&id) {
            match child.try_wait() {
                Ok(Some(_)) => {},
                Ok(None) => {
                    child.kill().map_err(|e| e.to_string())?;
                    child.wait().map_err(|e| e.to_string())?;
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        self.previews.remove(&id);
        Ok(())
    }

    fn reap_previews(&mut self) {
        self.previews.retain(|_, child| !matches!(child.try_wait(), Ok(Some(_))));
    }

    fn open_preview(&mut self, url: &str) -> Result<(), String> {
        let Some(id) = self.browser.active_id() else { return Err("No active tab".into()); };
        if self.preview_executable.as_ref().map_or(true, |p| !p.is_file()) {
            self.preview_executable = find_preview();
        }
        let Some(executable) = self.preview_executable.clone() else {
            return Err("Servo executable missing. Run build-all.bat first.".into());
        };
        self.reap_previews();
        if !self.previews.contains_key(&id) && self.previews.len() >= MAX_PREVIEWS {
            return Err("Renderer limit reached. Close another preview first.".into());
        }
        if let Err(error) = self.stop_preview(id) {
            return Err(format!("Could not stop previous renderer: {error}"));
        }
        match Command::new(executable).arg(url).stdin(Stdio::null()).spawn() {
            Ok(child) => { self.previews.insert(id, child); Ok(()) }
            Err(error) => Err(format!("Could not start Servo: {error}")),
        }
    }
}

impl Drop for BrowserApp {
    fn drop(&mut self) {
        for child in self.previews.values_mut() {
            if matches!(child.try_wait(), Ok(None)) && child.kill().is_ok() {
                let _ = child.wait();
            }
        }
    }
}

impl eframe::App for BrowserApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.reap_previews();
        if !self.previews.is_empty() { ctx.request_repaint_after(Duration::from_secs(2)); }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Lutra").strong().size(18.0));
                if ui.button("+").clicked() { let id = self.browser.open_tab(); self.resources.register(id); self.resources.activate(id); self.sync_address(); }
                if ui.add_enabled(self.browser.history_target(false).is_some(), egui::Button::new("‹")).clicked() { self.history(false); }
                if ui.add_enabled(self.browser.history_target(true).is_some(), egui::Button::new("›")).clicked() { self.history(true); }
                let available = (ui.available_width() - 110.0).max(80.0);
                let response = ui.add_sized([available, 30.0], egui::TextEdit::singleline(&mut self.address).hint_text("Search or enter address"));
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { self.submit(); }
                if ui.button("Go").clicked() { self.submit(); }
                if ui.button(if self.dark { "Light" } else { "Dark" }).clicked() {
                    self.dark = !self.dark;
                    ctx.set_visuals(if self.dark { egui::Visuals::dark() } else { egui::Visuals::light() });
                }
            });
            ui.add_space(6.0);
        });
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            let mut select = None;
            let mut close = None;
            ui.horizontal_wrapped(|ui| {
                for tab in self.browser.tabs() {
                    if ui.selectable_label(self.browser.active_id() == Some(tab.id), &tab.title).clicked() { select = Some(tab.id); }
                    if ui.small_button("×").clicked() { close = Some(tab.id); }
                }
            });
            if let Some(id) = select {
                self.browser.select_tab(id);
                self.resources.activate(id);
                self.sync_address();
            }
            if let Some(id) = close {
                match self.stop_preview(id) {
                    Err(error) => self.status = error,
                    Ok(()) => {
                        self.browser.close_tab(id);
                        self.resources.remove(id);
                        if self.browser.active_id().is_none() {
                            let id = self.browser.open_tab();
                            self.resources.register(id);
                        }
                        if let Some(active) = self.browser.active_id() { self.resources.activate(active); }
                        self.sync_address();
                    }
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.22);
                ui.heading(RichText::new("Lutra").size(36.0));
                ui.label(RichText::new("Privacy by design. Speed by default.").color(Color32::GRAY));
                ui.add_space(24.0);
                ui.label("Native Rust shell · Servo rendering opens in a separate window");
                ui.label(format!("Renderer processes: {} / {}", self.previews.len(), MAX_PREVIEWS));
                if let Some(id) = self.browser.active_id() {
                    if self.previews.contains_key(&id) && ui.button("Stop renderer").clicked() {
                        self.status = match self.stop_preview(id) {
                            Ok(()) => "Renderer stopped. Open the address again to reload.".into(),
                            Err(error) => error,
                        };
                    }
                }
                ui.add_space(12.0);
                ui.label(RichText::new(&self.status).small().color(Color32::GRAY));
            });
        });
    }
}

fn find_preview() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    let directory = current.parent()?;
    let candidates = [directory.join("servo-preview.exe"), directory.join("servo-preview")];
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn main() -> eframe::Result<()> {
    let icon = image::load_from_memory(include_bytes!("../../../assets/lutra.png")).ok().map(|image| {
        let rgba = image.into_rgba8();
        egui::IconData { width: rgba.width(), height: rgba.height(), rgba: rgba.into_raw() }
    });
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Lutra")
        .with_inner_size([1180.0, 760.0])
        .with_min_inner_size([700.0, 480.0]);
    if let Some(icon) = icon { viewport = viewport.with_icon(icon); }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Lutra", options, Box::new(|cc| {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Ok(Box::new(BrowserApp::default()))
    }))
}
