mod state;
mod rt;

// DUMMY MAIN, PLEASE IGNORE. RUST-ANALYZER DOES NOT LIKE WASM
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eframe::run_native(
        "main",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(state::Application::new(cc))),
    );
}

// actual main here
#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        "main",
        web_options,
        Box::new(|cc| Box::new(state::Application::new(cc))),
    )
    .expect("failed to start eframe");
}

