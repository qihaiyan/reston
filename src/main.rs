#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{Button, CollapsingHeader, Label, Slider, ScrollArea};
use poll_promise::Promise;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "✨ Orient",
        options,
        Box::new(|_cc| Box::new(HttpApp::default())),
    );
}

struct Resource {
    /// HTTP response
    response: ehttp::Response,

    text: Option<String>,

    /// If set, the response was text with some supported syntax highlighting (e.g. ".rs" or ".md").
    colored_text: Option<ColoredText>,
}

impl Resource {
    fn from_response(ctx: &egui::Context, response: ehttp::Response) -> Self {
        let content_type = response.content_type().unwrap_or_default();

        let text = response.text();
        let colored_text = text.and_then(|text| syntax_highlighting(ctx, &response, text));
        let text = text.map(|text| text.to_owned());

        Self {
            response,
            text,
            colored_text,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct HttpApp {
    url: String,
    items: Vec<Vec<String>>,
    cats: Vec<String>,
    search: String,

    #[cfg_attr(feature = "serde", serde(skip))]
    promise: Option<Promise<ehttp::Result<Resource>>>,
}

impl Default for HttpApp {
    fn default() -> Self {
        Self {
            url: "https://httpbin.org/get".to_owned(),
            items: vec![
                vec!["Item get", "https://httpbin.org/get"],
                vec!["Item anything", "https://httpbin.org/anything"],
                vec!["Item F", "Item G"],
            ]
            .into_iter()
            .map(|v| v.into_iter().map(ToString::to_string).collect())
            .collect(),
            promise: Default::default(),
            search: "".to_owned(),
            cats: vec!["Widgets 1".to_owned(), "Widgets 2".to_owned()],
        }
    }
}

impl eframe::App for HttpApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("http_bottom")
            .resizable(true)
            .show(ctx, |ui| {
                let layout = egui::Layout::top_down(egui::Align::Center).with_main_justify(true);
                ui.allocate_ui_with_layout(ui.available_size(), layout, |ui| {
                    ui.add(egui::Hyperlink::from_label_and_url(
                        egui::RichText::new("Context::set_fonts")
                            .text_style(egui::TextStyle::Monospace),
                        "https://docs.rs/egui/latest/egui/struct.Context.html#method.set_fonts",
                    ));
                });
            });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .show(ctx, |ui| {

                ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("search:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search)
                                .desired_width(f32::INFINITY),
                        );
                    });

                    if ui.button("Add").clicked() {
                        self.cats.push(format!("new {}", self.cats.len()));
                    }

                    for cat in &self.cats {
                        CollapsingHeader::new(cat)
                            .default_open(true)
                            .show(ui, |ui| {
                                for (col_idx, item) in self.items.clone().into_iter().enumerate() {
                                    if ui.button(item.get(0).unwrap()).clicked() {
                                        self.url = item.get(1).unwrap().to_string();
                                    }
                                }
                            });
                    }
                });
            });

        egui::TopBottomPanel::top("http_top")
            .resizable(true)
            .show(ctx, |ui| {
                let trigger_fetch = ui_url(ui, frame, &mut self.url);

                if trigger_fetch {
                    let ctx = ctx.clone();
                    let (sender, promise) = Promise::new();
                    let request = ehttp::Request::get(&self.url);
                    ehttp::fetch(request, move |response| {
                        ctx.request_repaint(); // wake up UI thread
                        let resource =
                            response.map(|response| Resource::from_response(&ctx, response));
                        sender.send(resource);
                    });
                    self.promise = Some(promise);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(promise) = &self.promise {
                if let Some(result) = promise.ready() {
                    match result {
                        Ok(resource) => {
                            ui_resource(ui, resource);
                        }
                        Err(error) => {
                            // This should only happen if the fetch API isn't available or something similar.
                            ui.colored_label(
                                ui.visuals().error_fg_color,
                                if error.is_empty() { "Error" } else { error },
                            );
                        }
                    }
                } else {
                    ui.spinner();
                }
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut *self
    }
}

fn ui_url(ui: &mut egui::Ui, frame: &mut eframe::Frame, url: &mut String) -> bool {
    let mut trigger_fetch = false;

    ui.horizontal(|ui| {
        ui.label("URL:");
        ui.add(egui::TextEdit::singleline(url).desired_width(f32::INFINITY));
    });

    if frame.is_web() {
        ui.label("HINT: paste the url of this page into the field above!");
    }

    ui.horizontal(|ui| {
        if ui.button("Go").clicked() {
            trigger_fetch = true;
        }
    });

    trigger_fetch
}

fn ui_resource(ui: &mut egui::Ui, resource: &Resource) {
    let Resource {
        response,
        text,
        colored_text,
    } = resource;

    ui.monospace(format!("url:          {}", response.url));
    ui.monospace(format!(
        "status:       {} ({})",
        response.status, response.status_text
    ));
    ui.monospace(format!(
        "content-type: {}",
        response.content_type().unwrap_or_default()
    ));
    ui.monospace(format!(
        "size:         {:.1} kB",
        response.bytes.len() as f32 / 1000.0
    ));

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            egui::CollapsingHeader::new("Response headers")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("response_headers")
                        .spacing(egui::vec2(ui.spacing().item_spacing.x * 2.0, 0.0))
                        .show(ui, |ui| {
                            for header in &response.headers {
                                ui.label(header.0);
                                ui.label(header.1);
                                ui.end_row();
                            }
                        })
                });

            ui.separator();

            if let Some(text) = &text {
                let tooltip = "Click to copy the response body";
                if ui.button("📋").on_hover_text(tooltip).clicked() {
                    ui.output().copied_text = text.clone();
                }
                ui.separator();
            }

            if let Some(colored_text) = colored_text {
                colored_text.ui(ui);
            } else if let Some(text) = &text {
                selectable_text(ui, text);
            } else {
                ui.monospace("[binary]");
            }
        });
}

fn selectable_text(ui: &mut egui::Ui, mut text: &str) {
    ui.add(
        egui::TextEdit::multiline(&mut text)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace),
    );
}

// ----------------------------------------------------------------------------
// Syntax highlighting:

#[cfg(feature = "syntect")]
fn syntax_highlighting(
    ctx: &egui::Context,
    response: &ehttp::Response,
    text: &str,
) -> Option<ColoredText> {
    let extension_and_rest: Vec<&str> = response.url.rsplitn(2, '.').collect();
    let extension = extension_and_rest.get(0)?;
    let theme = crate::syntax_highlighting::CodeTheme::from_style(&ctx.style());
    Some(ColoredText(crate::syntax_highlighting::highlight(
        ctx, &theme, text, extension,
    )))
}

#[cfg(not(feature = "syntect"))]
fn syntax_highlighting(_ctx: &egui::Context, _: &ehttp::Response, _: &str) -> Option<ColoredText> {
    None
}

struct ColoredText(egui::text::LayoutJob);

impl ColoredText {
    pub fn ui(&self, ui: &mut egui::Ui) {
        if true {
            // Selectable text:
            let mut layouter = |ui: &egui::Ui, _string: &str, wrap_width: f32| {
                let mut layout_job = self.0.clone();
                layout_job.wrap.max_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };

            let mut text = self.0.text.as_str();
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            );
        } else {
            let mut job = self.0.clone();
            job.wrap.max_width = ui.available_width();
            let galley = ui.fonts().layout_job(job);
            let (response, painter) = ui.allocate_painter(galley.size(), egui::Sense::hover());
            painter.add(egui::Shape::galley(response.rect.min, galley));
        }
    }
}
