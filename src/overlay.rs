use crate::cycle_state::CycleState;
use crate::x11_manager::{EveWindow, X11Manager};
use eframe::egui;
use std::sync::{Arc, Mutex};
use x11rb::connection::Connection;

pub struct OverlayApp {
    x11: Arc<X11Manager>,
    state: Arc<Mutex<CycleState>>,
    config: crate::config::Config,
}

impl OverlayApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        x11: Arc<X11Manager>,
        state: Arc<Mutex<CycleState>>,
        config: crate::config::Config,
    ) -> Self {
        Self {
            x11,
            state,
            config,
        }
    }
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Request repaint for smooth updates
        ctx.request_repaint();

        // Get active window
        let active_window = self.x11.get_active_window().unwrap_or(0);

        // Update windows list and sync state
        if let Ok(windows) = self.x11.get_eve_windows() {
            let mut state = self.state.lock().unwrap();
            state.update_windows(windows);
            state.sync_with_active(active_window);
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200))
                    .rounding(5.0)
                    .inner_margin(10.0),
            )
            .show(ctx, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(0, 255, 0),
                        egui::RichText::new("=== EVE MULTIBOX ===").strong(),
                    );
                });

                ui.add_space(5.0);

                // Status
                ui.horizontal(|ui| {
                    ui.label("Daemon:");
                    let daemon_running = std::path::Path::new("/tmp/eve-multibox.sock").exists();
                    if daemon_running {
                        ui.colored_label(egui::Color32::from_rgb(0, 255, 0), "[*] Running");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(255, 0, 0), "[X] Stopped");
                    }
                });

                ui.add_space(5.0);

                // Restack button
                if ui.button("[R] Restack Windows").clicked() {
                    let x11_clone = Arc::clone(&self.x11);
                    let config = self.config.clone();
                    std::thread::spawn(move || {
                        if let Ok(windows) = x11_clone.get_eve_windows() {
                            let _ = x11_clone.stack_windows(
                                &windows,
                                config.eve_x(),
                                config.eve_y(),
                                config.eve_width,
                                config.eve_height_adjusted(),
                            );
                        }
                    });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(5.0);

                // Window list
                let state = self.state.lock().unwrap();
                let windows = state.get_windows();
                let current_index = state.get_current_index();

                ui.label(format!("Clients: {}", windows.len()));
                ui.add_space(5.0);

                for (i, window) in windows.iter().enumerate() {
                    let text = if i == current_index {
                        format!("> [{}] {}", i + 1, &window.title[..window.title.len().min(20)])
                    } else {
                        format!("  [{}] {}", i + 1, &window.title[..window.title.len().min(20)])
                    };

                    ui.monospace(text);
                }

                if windows.is_empty() {
                    ui.colored_label(egui::Color32::GRAY, "No EVE clients detected");
                    ui.add_space(5.0);
                    ui.label("Launch EVE clients to begin");
                }
            });
    }
}

pub fn run_overlay(
    x11: Arc<X11Manager>,
    state: Arc<Mutex<CycleState>>,
    overlay_x: f32,
    overlay_y: f32,
    config: crate::config::Config,
) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([280.0, 450.0])
            .with_position([overlay_x, overlay_y])
            .with_decorations(false)
            .with_always_on_top()
            .with_transparent(true)
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "EVE Clients",
        options,
        Box::new(move |cc| {
            // Set X11 window properties after window is created
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(300));
                // Use wmctrl to set always on top
                let _ = std::process::Command::new("wmctrl")
                    .args(&["-r", "EVE Clients", "-b", "add,above"])
                    .output();
            });
            Ok(Box::new(OverlayApp::new(cc, x11, state, config)))
        }),
    )
}
