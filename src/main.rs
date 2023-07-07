use app::IsingApp;

mod app;
mod spin;
mod lattice;

fn main() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        min_window_size: Some(egui::vec2(550.0, 275.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Ising Model GUI",
        native_options,
        Box::new(|cc| Box::new(IsingApp::new(cc))),
    )
}
