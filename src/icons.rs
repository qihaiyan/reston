#[derive(Clone, Copy, Debug)]
pub struct Icon {
    /// Human readable unique id
    pub id: &'static str,

    pub png_bytes: &'static [u8],
}

impl Icon {
    pub const fn new(id: &'static str, png_bytes: &'static [u8]) -> Self {
        Self { id, png_bytes }
    }
}

pub const PLAY: Icon = Icon::new("play", include_bytes!("../resource/icons/play.png"));
pub const FOLLOW: Icon = Icon::new("follow", include_bytes!("../resource/icons/follow.png"));
pub const PAUSE: Icon = Icon::new("pause", include_bytes!("../resource/icons/pause.png"));
pub const ARROW_LEFT: Icon =
    Icon::new("arrow_left", include_bytes!("../resource/icons/arrow_left.png"));
pub const ARROW_RIGHT: Icon = Icon::new(
    "arrow_right",
    include_bytes!("../resource/icons/arrow_right.png"),
);
pub const LOOP: Icon = Icon::new("loop", include_bytes!("../resource/icons/loop.png"));

pub const RIGHT_PANEL_TOGGLE: Icon = Icon::new(
    "right_panel_toggle",
    include_bytes!("../resource/icons/right_panel_toggle.png"),
);
pub const BOTTOM_PANEL_TOGGLE: Icon = Icon::new(
    "bottom_panel_toggle",
    include_bytes!("../resource/icons/bottom_panel_toggle.png"),
);
pub const LEFT_PANEL_TOGGLE: Icon = Icon::new(
    "left_panel_toggle",
    include_bytes!("../resource/icons/left_panel_toggle.png"),
);

pub const MINIMIZE: Icon = Icon::new("minimize", include_bytes!("../resource/icons/minimize.png"));
pub const MAXIMIZE: Icon = Icon::new("maximize", include_bytes!("../resource/icons/maximize.png"));

pub const VISIBLE: Icon = Icon::new("visible", include_bytes!("../resource/icons/visible.png"));
pub const INVISIBLE: Icon = Icon::new("invisible", include_bytes!("../resource/icons/invisible.png"));

pub const ADD: Icon = Icon::new("add", include_bytes!("../resource/icons/add.png"));
pub const REMOVE: Icon = Icon::new("remove", include_bytes!("../resource/icons/remove.png"));

pub const RESET: Icon = Icon::new("reset", include_bytes!("../resource/icons/reset.png"));

pub const CLOSE: Icon = Icon::new("close", include_bytes!("../resource/icons/close.png"));

pub const SPACE_VIEW_TEXT: Icon = Icon::new(
    "spaceview_text",
    include_bytes!("../resource/icons/spaceview_text.png"),
);
pub const SPACE_VIEW_3D: Icon = Icon::new(
    "spaceview_3d",
    include_bytes!("../resource/icons/spaceview_3d.png"),
);
pub const SPACE_VIEW_CHART: Icon = Icon::new(
    "spaceview_chart",
    include_bytes!("../resource/icons/spaceview_chart.png"),
);
pub const SPACE_VIEW_SCATTERPLOT: Icon = Icon::new(
    "spaceview_scatterplot",
    include_bytes!("../resource/icons/spaceview_scatterplot.png"),
);
pub const SPACE_VIEW_RAW: Icon = Icon::new(
    "spaceview_raw",
    include_bytes!("../resource/icons/spaceview_raw.png"),
);
pub const SPACE_VIEW_TENSOR: Icon = Icon::new(
    "spaceview_tensor",
    include_bytes!("../resource/icons/spaceview_tensor.png"),
);
pub const SPACE_VIEW_HISTOGRAM: Icon = Icon::new(
    "spaceview_histogram",
    include_bytes!("../resource/icons/spaceview_histogram.png"),
);

pub const CONTAINER: Icon = Icon::new("container", include_bytes!("../resource/icons/container.png"));

pub const MENU: Icon = Icon::new("menu", include_bytes!("../resource/icons/menu.png"));
