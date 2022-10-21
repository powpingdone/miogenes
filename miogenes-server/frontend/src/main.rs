// DUMMY MAIN, PLEASE IGNORE. RUST-ANALYZER DOES NOT LIKE WASM
#[cfg(not(target_arch = "wasm32"))]
fn main() { 
    let n = eframe::NativeOptions::default();
    eframe::run_native(
        "main",
        n,
        Box::new(|cc| Box::new(experience::Application::new(cc))),
    );
}

#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        "main",
        web_options,
        Box::new(|cc| Box::new(experience::Application::new(cc))),
    )
    .expect("failed to start eframe");
}

mod experience {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Default)]
    pub enum Page {
        #[default]
        Page,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(default)]
    pub struct Application {
        page: Page,
    }

    impl Default for Application {
        fn default() -> Self {
            Self {
                page: Page::default(),
            }
        }
    }

    impl Application {
        /// Called once before the first frame.
        pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
            // This is also where you can customized the look at feel of egui using
            // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

            // Load previous app state (if any).
            // Note that you must enable the `persistence` feature for this to work.
            if let Some(storage) = cc.storage {
                return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            }

            Default::default()
        }
    }

    impl eframe::App for Application {
        fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
            todo!()
        }
    }
}
