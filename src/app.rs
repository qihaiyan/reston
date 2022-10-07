use std::collections::BTreeMap;

use eframe::egui;
use egui::{
    style::Margin, CollapsingHeader, Frame, ScrollArea, SidePanel, TextBuffer, TextStyle,
    TopBottomPanel, Ui, WidgetText,
};
use egui_dock::{DockArea, TabViewer};
use poll_promise::Promise;

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

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
}

impl Default for Method {
    fn default() -> Self {
        Self::Get
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum ScrollDemo {
    ScrollTo,
    ManyLines,
    LargeCanvas,
}

impl Default for ScrollDemo {
    fn default() -> Self {
        Self::ScrollTo
    }
}

#[derive(Debug, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct ApiCollection {
    name: String,
    buffers: BTreeMap<String, String>,
}

impl ApiCollection {
    pub fn new(name: String, buffers: BTreeMap<String, String>) -> Self {
        Self { name, buffers }
    }
}

// #[derive(Debug, PartialEq)]
// struct Location {
//     name: String,
//     url: String,
// }

// impl Location {
//     pub fn new(name: String, url: String) -> Self {
//         Self { name, url }
//     }
// }

// impl Default for Location {
//     fn default() -> Self {
//         Self {
//             url: "".into(),
//             name: "".into(),
//         }
//     }
// }

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct MyContext {
    // #[serde(skip)]
    buffers: BTreeMap<String, String>,
    name: String,
    // url: String,
    #[serde(skip)]
    method: Method,
    #[serde(skip)]
    demo: ScrollDemo,
    #[serde(skip)]
    promise: Option<Promise<ehttp::Result<Resource>>>,
}

impl MyContext {
    pub fn new(name: String, buffers: BTreeMap<String, String>) -> Self {
        Self {
            buffers,
            name,
            // url,
            method: Method::Get,
            demo: ScrollDemo::ScrollTo,
            promise: Default::default(),
        }
    }
}

impl TabViewer for MyContext {
    type Tab = String;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        Frame::none()
            .inner_margin(Margin::same(2.0))
            .show(ui, |ui| {
                // let url = {
                //     match self.buffers.get_mut(tab) {
                //         Some(x) => x,
                //         None => {
                //             self.buffers.insert("".to_owned(), "".to_owned());
                //             return "";
                //         }
                //     }
                // };
                let mut url;
                if let Some(u) = self.buffers.get_mut(tab) {
                    url = u;
                } else {
                    self.buffers.insert("".to_owned(), "".to_owned());
                    url = self.buffers.get_mut(tab).unwrap()
                }

                let trigger_fetch = ui_url(ui, &mut self.method, &mut url);

                if trigger_fetch {
                    let ctx = ui.ctx().clone();
                    let (sender, promise) = Promise::new();
                    let request = ehttp::Request::get(&url);
                    ehttp::fetch(request, move |response| {
                        ctx.request_repaint(); // wake up UI thread
                        let resource =
                            response.map(|response| Resource::from_response(&ctx, response));
                        sender.send(resource);
                    });
                    self.promise = Some(promise);
                }

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.demo, ScrollDemo::ScrollTo, "Parameters");
                    ui.selectable_value(&mut self.demo, ScrollDemo::ManyLines, "Body");
                    ui.selectable_value(&mut self.demo, ScrollDemo::LargeCanvas, "Headers");
                });

                match self.demo {
                    ScrollDemo::ScrollTo => {
                        ui.label("Query Parameters");
                        egui::Grid::new("response_headers")
                            .num_columns(2)
                            .striped(true)
                            // .spacing(egui::vec2(ui.spacing().item_spacing.x * 2.0, 0.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(egui::TextEdit::singleline(&mut "".to_owned()));
                                    ui.add(egui::TextEdit::singleline(&mut "".to_owned()));
                                });
                                ui.end_row();
                            });
                    }
                    ScrollDemo::ManyLines => {
                        huge_content_lines(ui);
                    }
                    ScrollDemo::LargeCanvas => {
                        huge_content_lines(ui);
                    }
                }

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

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        egui::WidgetText::from(&*tab)
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct HttpApp {
    api_collection: Vec<ApiCollection>,
    search: String,
    #[serde(skip)]
    method: Method,
    #[serde(skip)]
    demo: ScrollDemo,
    tree: egui_dock::Tree<String>,
    context: MyContext,

    #[serde(skip)]
    promise: Option<Promise<ehttp::Result<Resource>>>,
}

impl Default for HttpApp {
    fn default() -> Self {
        let mut buffers: BTreeMap<String, String> = BTreeMap::default();
        buffers.insert("Item get".into(), "https://httpbin.org/get".into());
        buffers.insert(
            "Item anything".into(),
            "https://httpbin.org/anything".into(),
        );
        buffers.insert("Item F".into(), "Item G".into());
        let context = MyContext::new("Simple Demo".to_owned(), buffers.clone());
        let api_collection = ApiCollection::new("Widgets 1".to_owned(), buffers);
        Self {
            promise: Default::default(),
            search: "".to_owned(),
            api_collection: vec![api_collection],
            method: Method::Get,
            demo: ScrollDemo::ScrollTo,
            tree: Default::default(),
            context,
        }
    }
}

impl HttpApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = _cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for HttpApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::bottom("http_bottom")
            .resizable(false)
            .show(ctx, |ui| {
                let layout = egui::Layout::top_down(egui::Align::Center).with_main_justify(true);
                ui.allocate_ui_with_layout(ui.available_size(), layout, |ui| {
                    ui.add(egui::Hyperlink::from_label_and_url(
                        egui::RichText::new("Feedback").text_style(egui::TextStyle::Monospace),
                        "https://github.com/qihaiyan/orient",
                    ));
                });
            });

        SidePanel::left("left_panel")
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
                        self.api_collection.push(ApiCollection::new(
                            format!("new {}", self.api_collection.len()),
                            BTreeMap::new(),
                        ));
                    }

                    for ac in self.api_collection.iter_mut() {
                        ui.horizontal(|ui| {
                            if ui.button("add").clicked() {
                                ac.buffers.insert("a".to_owned(), "b".to_owned());
                            };
                            ui.collapsing(ac.name.clone(), |ui| {
                                for (name, _url) in &ac.buffers {
                                    let tab_location = self.tree.find_tab(name);
                                    let is_open = tab_location.is_some();
                                    if ui.selectable_label(is_open, name.clone()).clicked() {
                                        if let Some((node_index, tab_index)) = tab_location {
                                            self.tree.set_active_tab(node_index, tab_index);
                                        } else {
                                            self.tree.push_to_focused_leaf(name.clone());
                                        }
                                    }
                                }
                            });
                        });
                    }
                });
            });

        DockArea::new(&mut self.tree).show(ctx, &mut self.context);
    }

    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut *self
    }
}

fn ui_url(ui: &mut egui::Ui, method: &mut Method, url: &mut String) -> bool {
    let mut trigger_fetch = false;

    ui.horizontal(|ui| {
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", method))
            .show_ui(ui, |ui| {
                ui.selectable_value(method, Method::Get, "Get");
                ui.selectable_value(method, Method::Post, "Post");
                ui.selectable_value(method, Method::Put, "Put");
                ui.selectable_value(method, Method::Patch, "Patch");
                ui.selectable_value(method, Method::Delete, "Delete");
                ui.selectable_value(method, Method::Head, "Head");
            });

        ui.add(egui::TextEdit::singleline(url));

        if ui.button("Go").clicked() {
            trigger_fetch = true;
        }
    });

    trigger_fetch
}

fn huge_content_lines(ui: &mut egui::Ui) {
    ui.label(
        "A lot of rows, but only the visible ones are layed out, so performance is still good:",
    );
    ui.add_space(4.0);

    let text_style = TextStyle::Body;
    let row_height = ui.text_style_height(&text_style);
    let num_rows = 10_000;
    ScrollArea::vertical().auto_shrink([false; 2]).show_rows(
        ui,
        row_height,
        num_rows,
        |ui, row_range| {
            for row in row_range {
                let text = format!("This is row {}/{}", row + 1, num_rows);
                ui.label(text);
            }
        },
    );
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
