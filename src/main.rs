#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let icon_bytes = include_bytes!("../reston.png");
    let icon = image::load_from_memory(icon_bytes).unwrap().to_rgba8();
    let (icon_width, icon_height) = icon.dimensions();

    let native_options = eframe::NativeOptions {
        // initial_window_size: Some([1200.0, 800.0].into()),
        follow_system_theme: false,
        default_theme: eframe::Theme::Dark,
        viewport: egui::ViewportBuilder::default().with_icon(std::sync::Arc::new(egui::IconData {
            rgba: icon.into_raw(),
            width: icon_width,
            height: icon_height,
        })),
        #[cfg(target_os = "macos")]
        fullsize_content: reston::FULLSIZE_CONTENT,

        ..Default::default()
    };

    eframe::run_native(
        "Reston",
        native_options,
        Box::new(|cc| {
            let re_ui = reston::ReUi::load_and_apply(&cc.egui_ctx);
            Box::new(reston::HttpApp::new(re_ui, cc.storage))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        "the_canvas_id", // hardcode it
        web_options,
        Box::new(|cc| Box::new(eframe_template::TemplateApp::new(cc))),
    )
    .expect("failed to start eframe");
}
