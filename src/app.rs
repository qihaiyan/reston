use std::collections::BTreeMap;

use eframe::egui;
use egui::{
    style::Margin, Frame, ScrollArea, SidePanel, TextStyle, TopBottomPanel, Ui, WidgetText,
};
use egui_dock::{DockArea, TabViewer};
use uuid::Uuid;

#[derive(Debug, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct Resource {
    /// HTTP response
    url: String,
    body: String,
    headers: Vec<(String, String)>,
    length: String,
    content_type: String,
    status: usize,
    status_text: String,
    // If set, the response was text with some supported syntax highlighting (e.g. ".rs" or ".md").
    // colored_text: Option<ColoredText>,
}

impl Resource {
    fn from_response(response: Option<ureq::Response>) -> Option<Self> {
        if let Some(response) = response {
            let url = response.get_url().to_string();
            let status = response.status().into();
            let status_text = response.status_text().to_string();
            let length = response.header("Content-Length").unwrap().to_string();
            let content_type = response.content_type().to_string();

            let mut headers = Vec::new();
            for key in response.headers_names() {
                headers.push((key.to_string(), response.header(&key).unwrap().to_string()));
            }

            let body = response.into_string().unwrap().to_string();

            return Some(Self {
                url,
                body,
                headers,
                length,
                content_type,
                status,
                status_text,
            });
        }
        return None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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

impl Method {
    fn to_text(self: &Self) -> String {
        match self {
            Method::Get => "GET".to_owned(),
            Method::Post => "POST".to_owned(),
            Method::Put => "PUT".to_owned(),
            Method::Patch => "PATCH".to_owned(),
            Method::Delete => "DELETE".to_owned(),
            Method::Head => "HEAD".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum ContentType {
    Json,
    FormUrlEncoded,
    FormData,
}

impl Default for ContentType {
    fn default() -> Self {
        Self::Json
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum RequestEditor {
    Params,
    Body,
    Headers,
}

impl Default for RequestEditor {
    fn default() -> Self {
        Self::Params
    }
}

#[derive(Debug, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct ApiCollection {
    name: String,
    buffers: BTreeMap<String, Location>,
}

impl ApiCollection {
    pub fn new(name: String, buffers: BTreeMap<String, Location>) -> Self {
        Self { name, buffers }
    }
}

#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Location {
    id: String,
    name: String,
    url: String,
    method: Method,
    params: Vec<(String, String)>,
    body: String,
    form_params: Vec<(String, String)>,
    header: Vec<(String, String)>,
    content_type: ContentType,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct MyContext {
    buffers: BTreeMap<String, Location>,
    name: String,
    resource: Option<Resource>,
    reqest_editor: RequestEditor,
}

impl MyContext {
    pub fn new(name: String, buffers: BTreeMap<String, Location>) -> Self {
        Self {
            buffers,
            name,
            reqest_editor: RequestEditor::Params,
            resource: Default::default(),
        }
    }
}

impl TabViewer for MyContext {
    type Tab = String;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        Frame::none()
            .inner_margin(Margin::same(2.0))
            .show(ui, |ui| {
                let mut add_location = false;
                let location;
                if let Some(u) = self.buffers.get_mut(tab) {
                    location = u;
                } else {
                    self.buffers.insert("".to_owned(), Location::default());
                    location = self.buffers.get_mut("").unwrap()
                }

                let trigger_fetch = ui_url(ui, &mut location.method, &mut location.url);

                if trigger_fetch {
                    let mut request = ureq::request(&location.method.to_text(), &location.url);

                    let headers = location.header.iter().filter(|e| (e.0.is_empty() == false));
                    for e in headers {
                        request = request.set(&e.0, &e.1);
                    }

                    self.resource = Resource::from_response(match location.method {
                        Method::Get => {
                            let params =
                                location.params.iter().filter(|e| (e.0.is_empty() == false));
                            for e in params {
                                request = request.query(&e.0, &e.1);
                            }
                            request.call().ok()
                        }
                        Method::Post => match location.content_type {
                            ContentType::Json => request.send_string(&location.body).ok(),
                            ContentType::FormUrlEncoded => {
                                let params =
                                    location.params.iter().filter(|e| (e.0.is_empty() == false));
                                for e in params {
                                    request = request.query(&e.0, &e.1);
                                }
                                let from_param: Vec<(&str, &str)> = location
                                    .form_params
                                    .as_slice()
                                    .into_iter()
                                    .map(|f| (f.0.as_str(), f.1.as_str()))
                                    .collect();
                                request.send_form(&from_param[..]).ok()
                            }
                            _ => request.call().ok(),
                        },
                        _ => request.call().ok(),
                    });
                }

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.reqest_editor, RequestEditor::Params, "Params");
                    ui.selectable_value(&mut self.reqest_editor, RequestEditor::Body, "Body");
                    ui.selectable_value(&mut self.reqest_editor, RequestEditor::Headers, "Headers");
                });

                match self.reqest_editor {
                    RequestEditor::Params => {
                        ui.horizontal(|ui| {
                            ui.label("Query Params");
                            if ui.button("add").clicked() {
                                add_location = true;
                                location.params.push(("".to_owned(), "".to_owned()));
                                ui.end_row();
                            }
                        });
                        egui::Grid::new("query_params")
                            .num_columns(3)
                            .min_col_width(300.0)
                            .spacing(egui::vec2(
                                ui.spacing().item_spacing.x * 0.5,
                                ui.spacing().item_spacing.x * 0.5,
                            ))
                            .show(ui, |ui| {
                                // ui.horizontal(|ui| {
                                if location.params.is_empty() {
                                    location.params.push(("".to_owned(), "".to_owned()));
                                    // });
                                    ui.end_row();
                                }

                                let mut i = 0 as usize;
                                while i < location.params.len() {
                                    ui.add(egui::TextEdit::singleline(&mut location.params[i].0));
                                    ui.add(egui::TextEdit::singleline(&mut location.params[i].1));
                                    if ui.button("del").clicked() {
                                        location.params.remove(i);
                                    }
                                    i = i + 1;
                                    ui.end_row();
                                }
                            });
                    }
                    RequestEditor::Body => {
                        ui.horizontal(|ui| {
                            ui.radio_value(
                                &mut location.content_type,
                                ContentType::Json,
                                "application/json",
                            );
                            ui.radio_value(
                                &mut location.content_type,
                                ContentType::FormData,
                                "form-data",
                            );
                            ui.radio_value(
                                &mut location.content_type,
                                ContentType::FormUrlEncoded,
                                "x-www-form-url-encoded",
                            );
                        });
                        if location.content_type == ContentType::Json {
                            ui.add(
                                egui::TextEdit::multiline(&mut location.body)
                                    .font(egui::TextStyle::Monospace) // for cursor height
                                    .code_editor()
                                    .desired_rows(10)
                                    .lock_focus(true)
                                    .desired_width(f32::INFINITY),
                            );
                        } else {
                            ui.horizontal(|ui| {
                                ui.label("Request Body");
                                if ui.button("add").clicked() {
                                    add_location = true;
                                    location.form_params.push(("".to_owned(), "".to_owned()));
                                    ui.end_row();
                                }
                            });
                            egui::Grid::new("request_body")
                                .num_columns(3)
                                .min_col_width(300.0)
                                .spacing(egui::vec2(
                                    ui.spacing().item_spacing.x * 0.5,
                                    ui.spacing().item_spacing.x * 0.5,
                                ))
                                .show(ui, |ui| {
                                    // ui.horizontal(|ui| {
                                    if location.form_params.is_empty() {
                                        location.form_params.push(("".to_owned(), "".to_owned()));
                                        // });
                                        ui.end_row();
                                    }

                                    let mut i = 0 as usize;
                                    while i < location.form_params.len() {
                                        ui.add(egui::TextEdit::singleline(
                                            &mut location.form_params[i].0,
                                        ));
                                        ui.add(egui::TextEdit::singleline(
                                            &mut location.form_params[i].1,
                                        ));
                                        if ui.button("del").clicked() {
                                            location.form_params.remove(i);
                                        }
                                        i = i + 1;
                                        ui.end_row();
                                    }
                                });
                        }
                    }
                    RequestEditor::Headers => {
                        ui.horizontal(|ui| {
                            ui.label("Headers");
                            if ui.button("add").clicked() {
                                add_location = true;
                                location.header.push(("".to_owned(), "".to_owned()));
                                ui.end_row();
                            }
                        });
                        egui::Grid::new("query_headers")
                            .num_columns(3)
                            .min_col_width(300.0)
                            .spacing(egui::vec2(
                                ui.spacing().item_spacing.x * 0.5,
                                ui.spacing().item_spacing.x * 0.5,
                            ))
                            .show(ui, |ui| {
                                // ui.horizontal(|ui| {
                                if location.header.is_empty() {
                                    location.header.push(("".to_owned(), "".to_owned()));
                                    // });
                                    ui.end_row();
                                }

                                let mut i = 0 as usize;
                                while i < location.header.len() {
                                    ui.add(egui::TextEdit::singleline(&mut location.header[i].0));
                                    ui.add(egui::TextEdit::singleline(&mut location.header[i].1));
                                    if ui.button("del").clicked() {
                                        location.header.remove(i);
                                    }
                                    i = i + 1;
                                    ui.end_row();
                                }
                            });
                    }
                }

                if let Some(resource) = &self.resource {
                    ui_resource(ui, resource);
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
    demo: RequestEditor,
    tree: egui_dock::Tree<String>,
    context: MyContext,
}

impl Default for HttpApp {
    fn default() -> Self {
        let mut buffers: BTreeMap<String, Location> = BTreeMap::default();
        let location1: Location = Location {
            id: Uuid::new_v4().to_string(),
            name: ("Item get".into()),
            url: ("https://httpbin.org/get".into()),
            params: (Vec::new()),
            body: ("".into()),
            header: (vec![("".to_owned(), "".to_owned())]),
            content_type: ContentType::Json,
            form_params: Vec::new(),
            method: Method::Get,
        };
        let location2: Location = Location {
            id: Uuid::new_v4().to_string(),
            name: ("Item anything".into()),
            url: ("https://httpbin.org/anything".into()),
            params: (Vec::new()),
            body: ("".into()),
            header: (vec![("".to_owned(), "".to_owned())]),
            content_type: ContentType::Json,
            form_params: Vec::new(),
            method: Method::Get,
        };
        buffers.insert("Item get".into(), location1);
        buffers.insert("Item anything".into(), location2);
        let context = MyContext::new("Simple Demo".to_owned(), buffers.clone());
        let api_collection = ApiCollection::new("Widgets 1".to_owned(), buffers);
        Self {
            search: "".to_owned(),
            api_collection: vec![api_collection],
            method: Method::Get,
            demo: RequestEditor::Params,
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
                                let location1: Location = Location {
                                    id: Uuid::new_v4().to_string(),
                                    name: ("Item get".into()),
                                    url: ("https://httpbin.org/get".into()),
                                    params: (Vec::new()),
                                    body: ("".into()),
                                    header: (vec![("".to_owned(), "".to_owned())]),
                                    content_type: ContentType::Json,
                                    form_params: Vec::new(),
                                    method: Method::Get,
                                };
                                ac.buffers.insert("new request".to_owned(), location1);
                            };
                            ui.collapsing(ac.name.clone(), |ui| {
                                let mut localtion_del = "".to_owned();
                                for (name, location) in &ac.buffers {
                                    let tab_location = self.tree.find_tab(name);
                                    let is_open = tab_location.is_some();
                                    ui.horizontal(|ui| {
                                        if ui.selectable_label(is_open, name.clone()).clicked() {
                                            if let Some((node_index, tab_index)) = tab_location {
                                                self.tree.set_active_tab(node_index, tab_index);
                                            } else {
                                                self.tree.push_to_focused_leaf(name.clone());
                                            }
                                        }
                                        if ui.button("del").clicked() {
                                            localtion_del = location.to_owned().url;
                                        };
                                    });
                                }
                                ac.buffers.retain(|_, v| v.url != localtion_del)
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
    ui.monospace(format!("url:          {}", resource.url));
    ui.monospace(format!(
        "status:       {} ({})",
        resource.status, resource.status_text
    ));
    ui.monospace(format!("content-type: {:?}", resource.content_type));
    ui.monospace(format!("size:         {:.1} kB", resource.length));

    ui.separator();

    let colored_text = syntax_highlighting(&resource.body);

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            egui::CollapsingHeader::new("Response headers")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("response_headers")
                        .spacing(egui::vec2(ui.spacing().item_spacing.x * 2.0, 0.0))
                        .show(ui, |ui| {
                            for (key, value) in &resource.headers {
                                ui.label(key);
                                ui.label(value);
                                ui.end_row();
                            }
                        })
                });

            ui.separator();

            let tooltip = "Click to copy the response body";
            if ui.button("📋").on_hover_text(tooltip).clicked() {
                ui.output().copied_text = resource.body.clone();
            }
            ui.separator();

            if let Some(colored_text) = colored_text {
                colored_text.ui(ui);
            } else if let Some(text) = Some(&resource.body) {
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
fn syntax_highlighting(_: &str) -> Option<ColoredText> {
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
