use eframe::egui;
use egui::collapsing_header::CollapsingState;
use egui::{
    lerp, style::Margin, Color32, Frame, ScrollArea, SidePanel, TopBottomPanel, Ui, WidgetText,
};
use egui_dock::{DockArea, NodeIndex, StyleBuilder, TabViewer};
use font_kit::{
    family_name::FamilyName,
    properties::{Properties, Weight},
    source::SystemSource,
};
use serde_json::Value;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, io::Read, sync::mpsc, thread};

use ureq::{OrAnyStatus, Response, Transport};
use uuid::Uuid;

use crate::{syntax_highlighting, uri};
pub type Result<T> = std::result::Result<T, Transport>;

#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct Resource {
    /// HTTP response
    url: String,
    body: String,
    headers: Vec<(String, String)>,
    length: usize,
    content_type: String,
    status: usize,
    status_text: String,
    elapsed: u128,
    // If set, the response was text with some supported syntax highlighting (e.g. ".rs" or ".md").
    // colored_text: Option<ColoredText>,
}

impl Resource {
    fn from_response(response: Result<Response>, elapsed: u128) -> Option<Self> {
        if let Ok(response) = response {
            let url = response.get_url().to_string();
            let status = response.status().into();
            let status_text = response.status_text().to_string();
            let mut length = response
                .header("Content-Length")
                .unwrap_or_else(|| "0")
                .parse()
                .unwrap();
            let content_type = response.content_type().to_string();

            let mut headers = Vec::new();
            for key in response.headers_names() {
                headers.push((key.to_string(), response.header(&key).unwrap().to_string()));
            }

            let body = response.into_string().unwrap_or_default().to_string();
            let body_len = body.len();
            if length == 0 {
                length = body_len;
            }
            return Some(Self {
                url,
                body,
                headers,
                length,
                content_type,
                status,
                status_text,
                elapsed,
            });
        } else {
            return None;
        }
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
    Options,
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
            Method::Options => "OPTIONS".to_owned(),
            Method::Head => "HEAD".to_owned(),
        }
    }
}

impl Method {
    fn from_text(method: String) -> Method {
        if method.to_uppercase() == "GET" {
            return Method::Get;
        } else if method.to_uppercase() == "POST" {
            return Method::Post;
        } else if method.to_uppercase() == "PUT" {
            return Method::Post;
        } else if method.to_uppercase() == "PATCH" {
            return Method::Patch;
        } else if method.to_uppercase() == "DELETE" {
            return Method::Delete;
        } else if method.to_uppercase() == "HEAD" {
            return Method::Head;
        } else {
            return Method::Get;
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

#[derive(Debug, PartialEq, Default, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct ApiCollection {
    name: String,
    buffers: BTreeMap<String, Location>,
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
    response: Option<Resource>,
}

#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Directory {
    id: String,
    name: String,
    parent: String,
    leaf: bool,
    locations: Vec<String>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Postman {
    info: PostmanInfo,
    item: Vec<PostmanItem>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PostmanInfo {
    _postman_id: String,
    name: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PostmanItem {
    id: String,
    name: String,
    request: PostmanRequest,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PostmanRequest {
    method: String,
    header: Vec<PostmanHeader>,
    body: PostmanBody,
    url: PostmanUrl,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PostmanHeader {
    key: String,
    value: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PostmanUrl {
    raw: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PostmanBody {
    urlencoded: Vec<PostmanForm>,
    raw: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PostmanForm {
    key: String,
    value: String,
}

#[derive(Clone)]
struct Color {
    color: Color32,
    name: String,
}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

enum RunState {
    Running,
}

struct MyContext<'a> {
    api_collection: &'a mut ApiCollection,
    reqest_editor: &'a mut RequestEditor,
    sender: &'a mpsc::Sender<Resource>,
    receiver: &'a mpsc::Receiver<Resource>,
    added_nodes: &'a mut Vec<Location>,
    run_state: &'a mut Vec<RunState>,
}

impl TabViewer for MyContext<'_> {
    type Tab = String;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        Frame::none()
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                let mut add_location = false;
                let location = self.api_collection.buffers.get_mut(tab).unwrap();

                let trigger_fetch = ui_url(ui, location);

                if trigger_fetch {
                    self.run_state.push(RunState::Running);

                    let mut request = ureq::request(&location.method.to_text(), &location.url)
                        .timeout(Duration::from_secs(10));

                    let headers = location.header.iter().filter(|e| (e.0.is_empty() == false));
                    for e in headers {
                        request = request.set(&e.0, &e.1);
                    }

                    let sender = self.sender.clone();
                    let resource_location = location.clone();
                    let ctx = ui.ctx().clone();
                    thread::spawn(move || {
                        let start = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        let response = match resource_location.method {
                            Method::Get => {
                                // let params = resource_location
                                //     .params
                                //     .iter()
                                //     .filter(|e| (e.0.is_empty() == false));
                                // for e in params {
                                //     request = request.query(&e.0, &e.1);
                                // }
                                request.call().or_any_status()
                            }
                            Method::Post => match resource_location.content_type {
                                ContentType::Json => request
                                    .set("Content-Type", "application/json")
                                    .send_string(&resource_location.body)
                                    .or_any_status(),
                                ContentType::FormUrlEncoded => {
                                    let params = resource_location
                                        .params
                                        .iter()
                                        .filter(|e| (e.0.is_empty() == false));
                                    for e in params {
                                        request = request.query(&e.0, &e.1);
                                    }
                                    let from_param: Vec<(&str, &str)> = resource_location
                                        .form_params
                                        .as_slice()
                                        .into_iter()
                                        .map(|f| (f.0.as_str(), f.1.as_str()))
                                        .collect();
                                    request.send_form(&from_param[..]).or_any_status()
                                }
                                _ => request.call().or_any_status(),
                            },
                            _ => request.call().or_any_status(),
                        };
                        let stop = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        let resource = Resource::from_response(response, stop - start);
                        if let Some(resource) = resource {
                            sender.send(resource).unwrap();
                            ctx.request_repaint();
                        }
                    });
                }

                if self.run_state.len() > 0 {
                    ui.spinner();
                }

                match self.receiver.try_recv() {
                    Ok(resource) => {
                        location.response = Some(resource);
                        self.run_state.clear();
                    }
                    Err(_) => {}
                }

                ui.horizontal(|ui| {
                    ui.selectable_value(self.reqest_editor, RequestEditor::Params, "Params");
                    ui.selectable_value(self.reqest_editor, RequestEditor::Body, "Body");
                    ui.selectable_value(self.reqest_editor, RequestEditor::Headers, "Headers");
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
                                    if ui.text_edit_singleline(&mut location.params[i].0).changed()
                                    {
                                        let urlstr = location.url.clone();
                                        let url = uri(&urlstr);
                                        if url.is_ok() {
                                            let mut url = url.ok().unwrap().1;
                                            if url.query.is_some() {
                                                url.query.as_mut().unwrap().clear();
                                                location.params.iter().for_each(|p| {
                                                    if p.0.len() > 0 {
                                                        url.query
                                                            .as_mut()
                                                            .unwrap()
                                                            .push((&p.0, &p.1));
                                                    }
                                                });
                                            } else {
                                                url.query = Some(vec![]);
                                                location.params.iter().for_each(|p| {
                                                    if p.0.len() > 0 {
                                                        url.query
                                                            .as_mut()
                                                            .unwrap()
                                                            .push((&p.0, &p.1));
                                                    }
                                                });
                                            }
                                            location.url = url.to_string();
                                        }
                                    };
                                    if ui.text_edit_singleline(&mut location.params[i].1).changed()
                                    {
                                        let urlstr = location.url.clone();
                                        let url = uri(&urlstr);
                                        if url.is_ok() {
                                            let mut url = url.ok().unwrap().1;
                                            if url.query.is_some() {
                                                url.query.as_mut().unwrap().clear();
                                                location.params.iter().for_each(|p| {
                                                    if p.0.len() > 0 {
                                                        url.query
                                                            .as_mut()
                                                            .unwrap()
                                                            .push((&p.0, &p.1));
                                                    }
                                                });
                                            } else {
                                                url.query = Some(vec![]);
                                                location.params.iter().for_each(|p| {
                                                    if p.0.len() > 0 {
                                                        url.query
                                                            .as_mut()
                                                            .unwrap()
                                                            .push((&p.0, &p.1));
                                                    }
                                                });
                                            }
                                            location.url = url.to_string();
                                        }
                                    };
                                    // ui.text_edit_singleline(&mut location.params[i].0);
                                    // ui.text_edit_singleline(&mut location.params[i].1);
                                    if ui.button("del").clicked() {
                                        location.params.remove(i);
                                        let urlstr = location.url.clone();
                                        let url = uri(&urlstr);
                                        if url.is_ok() {
                                            let mut url = url.ok().unwrap().1;
                                            if url.query.is_some() {
                                                url.query.as_mut().unwrap().clear();
                                                location.params.iter().for_each(|p| {
                                                    if p.0.len() > 0 {
                                                        url.query
                                                            .as_mut()
                                                            .unwrap()
                                                            .push((&p.0, &p.1));
                                                    }
                                                });
                                            } else {
                                                url.query = Some(vec![]);
                                                location.params.iter().for_each(|p| {
                                                    if p.0.len() > 0 {
                                                        url.query
                                                            .as_mut()
                                                            .unwrap()
                                                            .push((&p.0, &p.1));
                                                    }
                                                });
                                            }
                                            location.url = url.to_string();
                                        }
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
                            ScrollArea::vertical()
                                .id_source("source")
                                .max_height(200.0)
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut location.body)
                                            .code_editor()
                                            .lock_focus(true)
                                            .desired_width(f32::INFINITY),
                                    );
                                });
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

                ui_resource(ui, &location.response);
            });
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        let mut location = Location::default();
        let name = &self
            .api_collection
            .buffers
            .get_mut(tab)
            .unwrap_or_else(|| &mut location)
            .name;
        egui::WidgetText::from(name)
    }

    fn on_add(&mut self, _node: NodeIndex) {
        let id = Uuid::new_v4().to_string();
        let location: Location = Location {
            id: id.clone(),
            name: ("Item get".into()),
            url: ("https://httpbin.org/get".into()),
            params: (Vec::new()),
            body: ("".into()),
            header: (vec![("".to_owned(), "".to_owned())]),
            content_type: ContentType::Json,
            form_params: Vec::new(),
            method: Method::Get,
            response: Default::default(),
        };
        self.api_collection.buffers.insert(id, location.clone());
        self.added_nodes.push(location);
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct HttpApp {
    darkmode: bool,
    directory: BTreeMap<String, Directory>,
    search: String,
    tree: egui_dock::Tree<String>,
    api_collection: ApiCollection,
    reqest_editor: RequestEditor,
    #[serde(skip)]
    sender: mpsc::Sender<Resource>,
    #[serde(skip)]
    receiver: mpsc::Receiver<Resource>,
    // context: MyContext<'a>,
    picked_path: Option<String>,
    #[serde(skip)]
    show_confirmation_dialog: bool,
    #[serde(skip)]
    dir_rename: String,
    #[serde(skip)]
    items: Vec<Color>,
    #[serde(skip)]
    preview: Option<Vec<Color>>,
    #[serde(skip)]
    run_state: Vec<RunState>,
}

impl Default for HttpApp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            darkmode: true,
            search: "".to_owned(),
            directory: BTreeMap::default(),
            tree: Default::default(),
            api_collection: Default::default(),
            reqest_editor: Default::default(),
            sender,
            receiver,
            // context: MyContext::default(),
            picked_path: Default::default(),
            show_confirmation_dialog: false,
            dir_rename: Default::default(),
            items: vec![
                Color {
                    name: "Panic Purple".to_string(),
                    color: egui::hex_color!("642CA9"),
                },
                Color {
                    name: "Generic Green".to_string(),
                    color: egui::hex_color!("2A9D8F"),
                },
                Color {
                    name: "Ownership Orange*".to_string(),
                    color: egui::hex_color!("E9C46A"),
                },
            ],
            preview: None,
            run_state: Default::default(),
        }
    }
}

impl HttpApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&_cc.egui_ctx);
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
                vertex_gradient(
                    ui,
                    Default::default(),
                    &Gradient(
                        self.preview
                            .as_ref()
                            .unwrap_or(self.items.as_ref())
                            .iter()
                            .map(|c| c.color)
                            .collect(),
                    ),
                );

                let layout = egui::Layout::top_down(egui::Align::Center).with_main_justify(true);
                ui.allocate_ui_with_layout(ui.available_size(), layout, |ui| {
                    ui.add(egui::Hyperlink::from_label_and_url(
                        egui::RichText::new("Feedback").text_style(egui::TextStyle::Monospace),
                        "https://github.com/qihaiyan/reston",
                    ));
                });
            });

        SidePanel::left("left_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // egui::widgets::global_dark_light_mode_switch(ui);
                        // if self.darkmode {
                        //     if ui
                        //         .button("☀ Light")
                        //         .on_hover_text("Switch to light mode")
                        //         .clicked()
                        //     {
                        //         ui.ctx().set_visuals(egui::Visuals::light());
                        //         self.darkmode = true;
                        //     }
                        // } else {
                        //     if ui
                        //         .button("🌙 Dark")
                        //         .on_hover_text("Switch to dark mode")
                        //         .clicked()
                        //     {
                        //         ui.ctx().set_visuals(egui::Visuals::dark());
                        //         self.darkmode = false;
                        //     }
                        // }
                        ui.label("search:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() {
                            let mut dir_node = Directory::default();
                            dir_node.id = Uuid::new_v4().to_string();
                            dir_node.name = format!("new {}", self.directory.len());
                            self.directory.insert(dir_node.id.clone(), dir_node);
                        }
                        if ui.button("Import").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                let fpath = path.display().to_string();
                                let fname = std::path::Path::new(&fpath);
                                let zipfile = std::fs::File::open(fname).unwrap();

                                let mut archive = zip::ZipArchive::new(zipfile).unwrap();

                                for i in 0..archive.len() - 1 {
                                    let mut file = archive.by_index(i).unwrap();
                                    let mut contents = String::new();
                                    file.read_to_string(&mut contents).unwrap();
                                    let p: Postman = serde_json::from_str(&contents).unwrap();
                                    let mut items: Vec<String> = Vec::new();
                                    for item in p.item.into_iter() {
                                        items.push(item.id.clone());

                                        let location: Location = Location {
                                            id: item.id.clone(),
                                            name: (item.name.clone()),
                                            url: (item.request.url.raw.clone()),
                                            params: (Vec::new()),
                                            body: (item.request.body.raw),
                                            header: (item
                                                .request
                                                .header
                                                .into_iter()
                                                .map(|i| (i.key, i.value))
                                                .collect()),
                                            content_type: ContentType::Json,
                                            form_params: item
                                                .request
                                                .body
                                                .urlencoded
                                                .into_iter()
                                                .map(|f| (f.key, f.value))
                                                .collect(),
                                            method: Method::from_text(item.request.method),
                                            response: Default::default(),
                                        };
                                        self.api_collection
                                            .buffers
                                            .insert(item.id.clone(), location.clone());
                                    }
                                    let mut dir_node = Directory::default();
                                    dir_node.id = p.info._postman_id.clone();
                                    dir_node.name = p.info.name;
                                    dir_node.locations.append(&mut items);
                                    self.directory.insert(p.info._postman_id.clone(), dir_node);
                                }
                            }
                        }
                    });

                    let mut dir_del = "".to_owned();
                    let ac = self.api_collection.clone();
                    for dir in self.directory.iter_mut().filter(|x| {
                        ac.buffers
                            .iter()
                            .any(|d| x.1.locations.contains(&d.0) && d.1.url.contains(&self.search))
                    })
                    // .get(x.1.locations)
                    // .unwrap()
                    // .url
                    // .contains(&self.search))
                    {
                        ui.horizontal(|ui| {
                            if ui.button("add").clicked() {
                                let id = Uuid::new_v4().to_string();
                                let location: Location = Location {
                                    id: id.clone(),
                                    name: ("Item get".into()),
                                    url: ("https://httpbin.org/get".into()),
                                    params: (Vec::new()),
                                    body: ("".into()),
                                    header: (vec![("".to_owned(), "".to_owned())]),
                                    content_type: ContentType::Json,
                                    form_params: Vec::new(),
                                    method: Method::Get,
                                    response: Default::default(),
                                };
                                dir.1.locations.push(id.clone());
                                self.api_collection.buffers.insert(id, location.clone());
                            };
                            if ui.button("del").clicked() {
                                dir_del = dir.0.clone();
                            };
                            if ui.button("rename").clicked() {
                                self.dir_rename = dir.0.clone();
                                self.show_confirmation_dialog = true;
                            };
                            ui.vertical(|ui| {
                                let mut collapsing_state = CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    ui.make_persistent_id(dir.0.clone()),
                                    false,
                                );
                                if !self.search.is_empty() {
                                    collapsing_state.set_open(true);
                                }
                                collapsing_state
                                    .show_header(ui, |ui| ui.label(dir.1.name.clone()))
                                    // .default_open(!self.search.is_empty())
                                    .body(|ui| {
                                        // ui.collapsing(dir.1.name.clone(), |ui| {
                                        let mut localtion_del = "".to_owned();
                                        for id in dir.1.locations.clone().into_iter().filter(|x| {
                                            self.api_collection
                                                .buffers
                                                .get(x)
                                                .unwrap()
                                                .url
                                                .contains(&self.search)
                                        }) {
                                            let tab_location = self.tree.find_tab(&id);
                                            let is_open = tab_location.is_some();
                                            ui.horizontal(|ui| {
                                                // if self
                                                //     .api_collection
                                                //     .buffers
                                                //     .get(&id)
                                                //     .unwrap()
                                                //     .url
                                                //     .contains(&self.search)
                                                // {
                                                let name = self
                                                    .api_collection
                                                    .buffers
                                                    .get(&id)
                                                    .unwrap()
                                                    .name
                                                    .clone();
                                                if ui.selectable_label(is_open, name).clicked() {
                                                    if let Some((node_index, tab_index)) =
                                                        tab_location
                                                    {
                                                        self.tree
                                                            .set_active_tab(node_index, tab_index);
                                                    } else {
                                                        self.tree.push_to_focused_leaf(id.clone());
                                                    }
                                                }
                                                if ui.button("del").clicked() {
                                                    localtion_del = id.to_owned();
                                                };
                                                // }
                                            });
                                        }
                                        dir.1.locations.retain(|v| v != &localtion_del)
                                    })
                            });
                        });
                    }
                    self.directory.retain(|v, _| v != &dir_del);
                    if self.show_confirmation_dialog {
                        egui::Window::new("")
                            .collapsible(false)
                            .resizable(false)
                            .show(ctx, |ui| {
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(
                                        &mut self.directory.get_mut(&self.dir_rename).unwrap().name,
                                    );
                                    if ui.button("Ok").clicked() {
                                        self.show_confirmation_dialog = false;
                                        self.dir_rename = Default::default();
                                    }
                                });
                            });
                    }
                });
            });

        let mut added_nodes = Vec::new();
        DockArea::new(&mut self.tree)
            .style(
                StyleBuilder::from_egui(ctx.style().as_ref())
                    .with_tab_bar_height(40.0)
                    .show_add_buttons(true)
                    .build(),
            )
            .show(
                ctx,
                &mut MyContext {
                    api_collection: &mut self.api_collection,
                    // name: String,
                    reqest_editor: &mut self.reqest_editor,
                    // #[serde(skip)]
                    sender: &self.sender,
                    // #[serde(skip)]
                    receiver: &self.receiver,
                    // #[serde(skip)]
                    added_nodes: &mut added_nodes,
                    run_state: &mut self.run_state,
                },
            );
        added_nodes.drain(..).for_each(|node| {
            // self.tree.set_focused_node(node);
            self.tree.push_to_focused_leaf(node.id);
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut *self
    }
}

fn ui_url(ui: &mut egui::Ui, location: &mut Location) -> bool {
    let mut trigger_fetch = false;

    ui.text_edit_singleline(&mut location.name);
    ui.separator();

    ui.horizontal(|ui| {
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", location.method))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut location.method, Method::Get, "Get");
                ui.selectable_value(&mut location.method, Method::Post, "Post");
                ui.selectable_value(&mut location.method, Method::Put, "Put");
                ui.selectable_value(&mut location.method, Method::Patch, "Patch");
                ui.selectable_value(&mut location.method, Method::Delete, "Delete");
                ui.selectable_value(&mut location.method, Method::Options, "Options");
                ui.selectable_value(&mut location.method, Method::Head, "Head");
            });

        if ui
            .add(egui::TextEdit::singleline(&mut location.url).desired_width(800.0))
            .changed()
        {
            if location.url.ends_with("&") {
                if !location.params.contains(&("".to_string(), "".to_string())) {
                    location.params.push(("".to_string(), "".to_string()));
                }
            } else {
                let urlstr = location.url.clone();
                let url = uri(&urlstr);
                if url.is_ok() {
                    let url = url.unwrap().1;
                    location.params.drain(..);
                    url.query.unwrap_or_default().into_iter().for_each(|q| {
                        location
                            .params
                            .push((q.0.to_string().clone(), q.1.to_string().clone()));
                    })
                }
                // let req = ParsedRequest::new(req).unwrap();
                // let url = parse_url(&urlstr);
                // let url = URL::parse(&urlstr).unwrap();
            }
        }
        // let urlstr = location.url.clone();
        // let url = uri(&urlstr);
        // if url.is_ok() {
        //     let mut url = url.ok().unwrap().1;
        //     if url.query.is_some() {
        //         url.query.as_mut().unwrap().clear();
        //         location.params.iter().for_each(|p| {
        //             if p.0.len() > 0 {
        //                 url.query.as_mut().unwrap().push((&p.0, &p.1));
        //             }
        //         });
        //     } else {
        //         url.query = Some(vec![]);
        //         location.params.iter().for_each(|p| {
        //             if p.0.len() > 0 {
        //                 url.query.as_mut().unwrap().push((&p.0, &p.1));
        //             }
        //         });
        //     }
        //     if location.url.ends_with("&") {
        //         location.url = url.to_string() + "&";
        //     } else {
        //         location.url = url.to_string()
        //     }
        // }
        // let urlstr = location.url.clone();
        // let url = uri(&urlstr);
        // if url.is_ok() {
        //     let mut url = url.ok().unwrap().1;
        //     if url.query.is_some() {
        //         url.query.as_mut().unwrap().clear();
        //         location.params.iter().for_each(|p| {
        //             if p.0.len() > 0 {
        //                 url.query.as_mut().unwrap().push((&p.0, &p.1));
        //             }
        //         });
        //     }
        //     location.url = url.to_string();
        // }
        // let url = URL::parse(&location.url);
        // if url.is_ok() {
        //     let mut url = url.ok().unwrap();
        //     url.query_pairs_mut().clear();
        //     location.params.clone().into_iter().for_each(|p| {
        //         if p.0.len() > 0 {
        //             url.query_pairs_mut().append_pair(&p.0, &p.1);
        //         }
        //     });

        //     // if location.url.ends_with("&") {
        //     //     location.url = url_escape::decode(&url).to_string() + "&";
        //     // } else {
        //     //     location.url = url_escape::decode(&url).to_string();
        //     // }
        // }

        // let parsed = Url::parse(&location.url).unwrap();
        // let url: &str = &parsed[..url::Position::AfterPath];
        // let mut request = ureq::request("get", &urlstr);
        // for e in &location.params {
        //     request = request.query(&e.0, &e.1);
        // }
        // location.url = request.url().to_string();
        if ui.button("Go").clicked() {
            trigger_fetch = true;
        }
    });

    trigger_fetch
}

fn ui_resource(ui: &mut egui::Ui, resource: &Option<Resource>) {
    if let Some(resource) = resource {
        ui.monospace(format!("url:          {}", resource.url));
        ui.monospace(format!(
            "status:       {} ({})",
            resource.status, resource.status_text
        ));
        ui.monospace(format!("content-type: {}", resource.content_type));
        ui.monospace(format!(
            "size:         {:.1} kB",
            resource.length as f32 / 1000.0
        ));
        ui.monospace(format!("time:         {} ms", resource.elapsed));
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
                                for (key, value) in &resource.headers {
                                    ui.label(key);
                                    ui.label(value);
                                    ui.end_row();
                                }
                            })
                    });

                ui.separator();

                let mut body = resource.body.clone();
                if body.len() < 1 {
                    return;
                }
                let body1: Value = serde_json::from_str(&body).unwrap_or_default();
                body = serde_json::to_string_pretty(&body1).unwrap();
                let colored_text = syntax_highlighting(ui.ctx(), &body);

                let tooltip = "Click to copy the response body";
                if ui.button("📋").on_hover_text(tooltip).clicked() {
                    ui.output_mut(|u| u.copied_text = body.clone());
                }
                ui.separator();

                if let Some(colored_text) = colored_text {
                    colored_text.ui(ui);
                } else if let Some(text) = Some(&body) {
                    selectable_text(ui, text);
                } else {
                    ui.monospace("[binary]");
                }
            });
    }
}

fn selectable_text(ui: &mut egui::Ui, mut text: &str) {
    ui.add(
        egui::TextEdit::multiline(&mut text)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace),
    );
}

fn syntax_highlighting(ctx: &egui::Context, text: &str) -> Option<ColoredText> {
    Some(ColoredText(syntax_highlighting::highlight(ctx, text)))
}

struct ColoredText(egui::text::LayoutJob);

impl ColoredText {
    pub fn ui(&self, ui: &mut egui::Ui) {
        if true {
            // Selectable text:
            let mut layouter = |ui: &egui::Ui, _string: &str, wrap_width: f32| {
                let mut job = self.0.clone();
                job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(job))
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
            let galley = ui.fonts(|f| f.layout_job(job));
            let (response, painter) = ui.allocate_painter(galley.size(), egui::Sense::hover());
            painter.add(egui::Shape::galley(response.rect.min, galley));
        }
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let source = SystemSource::new();
    let prop = if let Ok(font) = source.select_best_match(
        &[
            FamilyName::Title("微软雅黑".to_owned()),
            FamilyName::SansSerif,
        ],
        Properties::new().weight(Weight::NORMAL),
    ) {
        let font = match font.load() {
            Ok(font) => font,
            Err(_err) => {
                return;
            }
        };
        let Some(font_data) = font.copy_font_data() else {
            return;
        };
        let data = Box::leak((*font_data).clone().into_boxed_slice());
        data
    } else {
        return;
    };

    fonts
        .font_data
        .insert("prop".to_owned(), egui::FontData::from_static(prop));
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "prop".to_owned());

    let mono = if let Ok(font) = source.select_best_match(
        &[
            // FamilyName::Title("YaHei Consolas Hybrid".to_owned()),
            FamilyName::Title("微软雅黑".to_owned()),
            FamilyName::Title("Consolas".to_owned()),
            FamilyName::Monospace,
        ],
        Properties::new().weight(Weight::NORMAL),
    ) {
        let font = match font.load() {
            Ok(font) => font,
            Err(_err) => {
                return;
            }
        };
        let Some(font_data) = font.copy_font_data() else {
            return;
        };
        let data = Box::leak((*font_data).clone().into_boxed_slice());
        data
    } else {
        return;
    };

    fonts
        .font_data
        .insert("mono".to_owned(), egui::FontData::from_static(mono));

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "mono".to_owned());
    ctx.set_fonts(fonts);
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct Gradient(pub Vec<Color32>);

fn vertex_gradient(ui: &mut Ui, bg_fill: Color32, gradient: &Gradient) {
    use egui::epaint::*;

    let rect = ui.max_rect();

    if bg_fill != Default::default() {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(rect, bg_fill);
        ui.painter().add(Shape::mesh(mesh));
    }
    {
        let n = gradient.0.len();
        assert!(n >= 2);
        let mut mesh = Mesh::default();
        for (i, &color) in gradient.0.iter().enumerate() {
            let t = i as f32 / (n as f32 - 1.0);
            let y = lerp(rect.y_range(), t);
            mesh.colored_vertex(pos2(rect.left(), y), color);
            mesh.colored_vertex(pos2(rect.right(), y), color);
            if i < n - 1 {
                let i = i as u32;
                mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
                mesh.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
            }
        }
        ui.painter().add(Shape::mesh(mesh));
    };
}
