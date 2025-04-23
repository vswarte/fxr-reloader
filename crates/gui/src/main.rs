use std::path::PathBuf;

use eframe::{egui::{self, Align, IconData, Layout, RichText, Style, Vec2, Visuals}, icon_data::from_png_bytes};

mod game;

const WINDOW_SIZE: Vec2 = Vec2::new(300.0, 450.0);

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("fxr-reloader")
            .with_title(get_project_title())
            .with_inner_size(WINDOW_SIZE)
            .with_taskbar(true)
            .with_resizable(false)
            .with_icon(from_png_bytes(include_bytes!("icon.png")).unwrap()),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        get_project_title().as_str(),
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(FxrReloaderApp::new(cc)))
        }),
    ).expect("Could not run egui app");
}

#[derive(Default)]
struct FxrReloaderApp {
    selected_process: Option<game::GameProcess>,
    selected_files: Vec<PathBuf>,
    log_entries: Vec<String>,
}

impl FxrReloaderApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_style(Style {
            visuals: Visuals::dark(),
            ..Style::default()
        });

        Self {
            selected_process: game::get_running_games().first().cloned(),
            ..Self::default()
        }
    }

    fn reload_selected_fxrs(&mut self) {
        let result = game::call_fxr_patch(
            self.selected_process.as_ref().unwrap().pid,
            &self.selected_files
        );

        match result {
            Ok(_) => self.log_entries.push(String::from("Reloaded FXR")),
            Err(e) => self.log_entries.push(format!("Failed to reload FXR: {e}")),
        }
    }
}

impl eframe::App for FxrReloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.image(egui::include_image!("logo.gif"));
                ui.separator();

                egui::ComboBox::from_label("Game process")
                    .selected_text(
                        self.selected_process
                            .as_ref()
                            .map(format_process_label)
                            .unwrap_or(String::from("No process selected")),
                    )
                    .show_ui(ui, |ui| {
                        game::get_running_games().iter().for_each(|e| {
                            ui.selectable_value(
                                &mut self.selected_process,
                                Some(e.clone()),
                                format_process_label(e),
                            );
                        })
                    });

                ui.spacing();

                ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
                    if ui.add_enabled(
                        self.selected_process.is_some(),
                        egui::Button::new("Patch FXR")
                    ).clicked() {
                        if let Some(fxrs) = prompt_fxr_files() {
                            self.selected_files = fxrs.iter()
                                .map(|f| f.to_path_buf())
                                .collect();

                            self.reload_selected_fxrs();
                        }
                    }

                    if ui.add_enabled(
                        self.selected_process.is_some() && !self.selected_files.is_empty(),
                        egui::Button::new("Reload last reloaded FXRs")
                    ).clicked() {
                        self.reload_selected_fxrs();
                    }
                });

                ui.spacing();

                ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut self.log_entries.join("\n"))
                        .interactive(false),
                );
            });
    }
}

fn format_process_label(process: &game::GameProcess) -> String {
    format!("{} ({})", process.name, process.pid.as_u32())
}

fn get_project_title() -> String {
    format!("FXR reloader v{}", env!("CARGO_PKG_VERSION"))
}

fn prompt_fxr_files() -> Option<Vec<PathBuf>> {
    rfd::FileDialog::new()
        .add_filter("FXR Files", &["fxr"])
        .pick_files()
}
