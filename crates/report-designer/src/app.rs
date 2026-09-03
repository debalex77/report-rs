use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::widget::{
    Space, button, checkbox, combo_box, container, mouse_area, opaque, pick_list, progress_bar,
    row, rule, scrollable, stack, svg, text, text_editor, text_input, toggler, tooltip,
};
use iced::{
    Background, Color, Element, Fill, Point, Rectangle, Renderer, Size, Task, Theme, keyboard,
};

use report_core::common;
use report_core::datasource::{
    DataProvider, SqliteDataProvider, Value, apply_query_transformations,
};
use report_core::font::FontSpec;
use report_core::font::resolver::SystemFontResolver;
use report_core::image::layout::calculate_image_placement;
use report_core::model::{
    Band, BandKind, Border, Color as ReportColor, DataConnection, DataQuery, DataSourceDefinition,
    FilterOperator, HorizontalAlign, ImageFit, ImageItem, ImageSourceType, Item, LayoutItem,
    Margins, Mm, Orientation, Padding, Page, PageSize, QueryFilter, QuerySort, QuerySource,
    RectangleItem, Report, ReportParameter, ReportParameterType, SortDirection, TextItem,
    ValueFormat, ValueType, VerticalAlign,
};

#[cfg(test)]
#[path = "tests.rs"]
mod app_tests;
#[path = "clipboard.rs"]
mod clipboard;
#[path = "data_sources.rs"]
mod data_sources;
#[path = "canvas.rs"]
mod designer_canvas;
#[path = "document.rs"]
mod document;
#[path = "app/document_actions.rs"]
mod document_actions;
#[path = "files.rs"]
mod files;
#[path = "history.rs"]
mod history;
#[path = "images.rs"]
mod images;
#[path = "inspector/mod.rs"]
mod inspector;
#[path = "menus.rs"]
mod menus;
#[path = "message.rs"]
mod message;
#[path = "app/properties.rs"]
mod properties;
#[path = "app/resources.rs"]
mod resources;
#[path = "settings.rs"]
mod settings;
#[path = "shortcuts.rs"]
mod shortcuts;
#[path = "state.rs"]
mod state;
#[path = "table_templates.rs"]
mod table_templates;
#[path = "app/toolbar.rs"]
mod toolbar;
#[path = "app/ui_helpers.rs"]
mod ui_helpers;
#[path = "app/update.rs"]
mod update;
#[path = "app/view.rs"]
mod view;
use data_sources::{
    DataQueryEditor, DataQueryTab, DataSourceEditor, QueryTextTarget, load_query_rules_preview,
    parse_filter_operator, save_data_query, save_data_source,
};
use document::*;
use files::{
    ensure_json_extension, launch_preview, report_directory, select_image_file, select_report_file,
    select_report_save_file, select_sqlite_file,
};
use images::{DesignerImage, load_designer_images};
use message::Message;
use settings::{DesignerSettings, MarginField, page_font_family};
use shortcuts::keyboard_shortcuts;
use state::{
    AppMenu, BorderSide, CollapsedGroups, DesignerTool, DragOperation, GeometryField, PaddingField,
    PendingLayoutMove, PropertyGroup, ResizeHandle, Selection, SidebarTab, StructureDropTarget,
};
use table_templates::{TableTemplate, load_table_templates, save_table_templates};
use ui_helpers::*;

// CSS pixels per millimeter at the standard 96 DPI screen scale.
const BASE_SCALE: f32 = 96.0 / 25.4;
const PAGE_MARGIN: f32 = 112.0;
const RULER_SIZE: f32 = 24.0;
const RULER_GAP: f32 = 4.0;
const BAND_BADGE_WIDTH: f32 = 74.0;
const DEFAULT_INSPECTOR_WIDTH: f32 = 330.0;
const MIN_INSPECTOR_WIDTH: f32 = 250.0;
const MAX_INSPECTOR_WIDTH: f32 = 500.0;
const RESIZER_WIDTH: f32 = 7.0;
const HANDLE_SIZE: f32 = 8.0;
const MIN_ITEM_SIZE: f32 = 1.0;
const TEXT_COLOR_PALETTE: [ReportColor; 10] = [
    ReportColor::rgb(0, 0, 0),
    ReportColor::rgb(90, 95, 105),
    ReportColor::rgb(255, 255, 255),
    ReportColor::rgb(210, 55, 55),
    ReportColor::rgb(225, 125, 35),
    ReportColor::rgb(225, 190, 35),
    ReportColor::rgb(45, 150, 80),
    ReportColor::rgb(30, 140, 155),
    ReportColor::rgb(45, 105, 200),
    ReportColor::rgb(135, 75, 175),
];

/// Parses either a decimal count (`2`) or a zero mask (`00`).
/// An empty value means that the default numeric representation is used.
fn parse_decimal_places(value: &str) -> Option<Option<u8>> {
    let value = value.trim();
    if value.is_empty() {
        return Some(None);
    }
    if value.bytes().all(|byte| byte == b'0') {
        return u8::try_from(value.len()).ok().map(Some);
    }
    value.parse::<u8>().ok().map(Some)
}

#[derive(Default)]
struct GeometryInputs {
    x: String,
    y: String,
    width: String,
    height: String,
}

#[derive(Default)]
struct TextInputs {
    text: text_editor::Content,
    font_size: String,
    font_family: String,
    text_color: String,
    padding_left: String,
    padding_top: String,
    padding_right: String,
    padding_bottom: String,
    background: String,
    border_width: String,
    decimal_places: String,
    date_pattern: String,
    value_prefix: String,
    value_suffix: String,
}

#[derive(Default)]
struct ShapeInputs {
    border_width: String,
}

#[derive(Default)]
struct BandInputs {
    height: String,
    data_source: String,
    group_field: String,
}

struct QueryFieldPicker {
    query_name: String,
    fields: Vec<String>,
}

struct QueryRulesEditor {
    source_index: usize,
    query_index: usize,
    query_name: String,
    fields: Vec<String>,
    filters: Vec<QueryFilter>,
    sorts: Vec<QuerySort>,
    preview: Option<QueryRulesPreview>,
}

struct QueryRulesPreview {
    total_rows: usize,
    filtered_rows: usize,
    fields: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Clone)]
struct DataFieldDrag {
    query: String,
    fields: Vec<String>,
}

struct PendingDataFieldDrop {
    band: usize,
    query: String,
    columns: Vec<TableColumnSpec>,
    center_table: bool,
    include_row_number: bool,
    groups: Vec<TableGroupSpec>,
    template_name: String,
}

impl BandInputs {
    fn sync(&mut self, band: &Band) {
        self.height = format_mm(band.height.0);
        self.data_source = match &band.kind {
            BandKind::Data { source }
            | BandKind::DataHeader { source, .. }
            | BandKind::GroupHeader { source, .. }
            | BandKind::GroupFooter { source, .. } => source.clone(),
            _ => String::new(),
        };
        self.group_field = match &band.kind {
            BandKind::GroupHeader { field, .. } | BandKind::GroupFooter { field, .. } => {
                field.clone()
            }
            _ => String::new(),
        };
    }
}

impl ShapeInputs {
    fn sync(&mut self, item: &Item) {
        if let Item::Rectangle(item) = item {
            self.border_width = format_mm(item.border_width.0);
        } else {
            *self = Self::default();
        }
    }
}

impl TextInputs {
    fn sync(&mut self, item: &Item) {
        if let Some(item) = first_text_item(item) {
            self.text = text_editor::Content::with_text(&item.text);
            self.font_size = format_pt(item.font_size);
            self.font_family.clone_from(&item.font_family);
            self.text_color = format_report_color(item.text_color);
            self.padding_left = format_mm(item.padding.left.0);
            self.padding_top = format_mm(item.padding.top.0);
            self.padding_right = format_mm(item.padding.right.0);
            self.padding_bottom = format_mm(item.padding.bottom.0);
            self.background = item.background.map(format_report_color).unwrap_or_default();
            self.border_width = item
                .border
                .as_ref()
                .map(|border| format_mm(border.width))
                .unwrap_or_else(|| format_mm(0.5));
            self.decimal_places = item
                .value_format
                .decimal_places
                .map(|value| value.to_string())
                .unwrap_or_default();
            self.date_pattern = item.value_format.date_pattern.clone().unwrap_or_default();
            self.value_prefix.clone_from(&item.value_format.prefix);
            self.value_suffix.clone_from(&item.value_format.suffix);
        } else {
            *self = Self::default();
        }
    }

    fn padding(&self, field: PaddingField) -> &str {
        match field {
            PaddingField::Left => &self.padding_left,
            PaddingField::Top => &self.padding_top,
            PaddingField::Right => &self.padding_right,
            PaddingField::Bottom => &self.padding_bottom,
        }
    }

    fn set_padding(&mut self, field: PaddingField, value: String) {
        match field {
            PaddingField::Left => self.padding_left = value,
            PaddingField::Top => self.padding_top = value,
            PaddingField::Right => self.padding_right = value,
            PaddingField::Bottom => self.padding_bottom = value,
        }
    }
}

impl GeometryInputs {
    fn sync(&mut self, item: &Item) {
        let (x, y, width, height) = geometry_values(item);
        self.x = format_mm(x);
        self.y = format_mm(y);
        self.width = format_mm(width);
        self.height = format_mm(height);
    }

    fn value(&self, field: GeometryField) -> &str {
        match field {
            GeometryField::X | GeometryField::X1 => &self.x,
            GeometryField::Y | GeometryField::Y1 => &self.y,
            GeometryField::Width | GeometryField::X2 => &self.width,
            GeometryField::Height | GeometryField::Y2 => &self.height,
        }
    }

    fn set(&mut self, field: GeometryField, value: String) {
        match field {
            GeometryField::X | GeometryField::X1 => self.x = value,
            GeometryField::Y | GeometryField::Y1 => self.y = value,
            GeometryField::Width | GeometryField::X2 => self.width = value,
            GeometryField::Height | GeometryField::Y2 => self.height = value,
        }
    }
}

struct DesignerApp {
    path: Option<PathBuf>,
    report: Report,
    selection: Option<Selection>,
    selected_items: Vec<Selection>,
    active_band: Option<usize>,
    status: String,
    zoom: f32,
    dirty: bool,
    geometry_inputs: GeometryInputs,
    text_inputs: TextInputs,
    shape_inputs: ShapeInputs,
    band_inputs: BandInputs,
    font_families: combo_box::State<String>,
    font_resolver: SystemFontResolver,
    font_names: HashMap<String, &'static str>,
    collapsed_groups: CollapsedGroups,
    undo_stack: Vec<Report>,
    redo_stack: Vec<Report>,
    canvas_interaction_active: bool,
    properties_visible: bool,
    properties_width: f32,
    sidebar_tab: SidebarTab,
    collapsed_structure_layouts: HashSet<Selection>,
    structure_drag: Option<Selection>,
    structure_drop_target: Option<StructureDropTarget>,
    pending_layout_move: Option<PendingLayoutMove>,
    structure_rename: Option<Selection>,
    structure_name_input: String,
    structure_selection_anchor: Option<Selection>,
    keyboard_modifiers: keyboard::Modifiers,
    guides_visible: bool,
    error_message: Option<String>,
    auto_close_messages: bool,
    settings: Option<DesignerSettings>,
    data_source_editor: Option<DataSourceEditor>,
    data_query_editor: Option<DataQueryEditor>,
    query_text_menu: Option<QueryTextTarget>,
    query_text_menu_position: Option<Point>,
    query_name_input_id: iced::widget::Id,
    item_name_input_id: iced::widget::Id,
    date_pattern_input_id: iced::widget::Id,
    value_prefix_input_id: iced::widget::Id,
    value_suffix_input_id: iced::widget::Id,
    text_color_input_id: iced::widget::Id,
    query_rules_editor: Option<QueryRulesEditor>,
    query_fields: HashMap<String, Vec<String>>,
    query_field_types: HashMap<(String, String), ValueType>,
    expanded_data_queries: HashSet<String>,
    selected_data_fields: HashSet<(String, String)>,
    open_data_templates: Option<String>,
    data_templates_position: Option<Point>,
    cursor_position: Point,
    data_field_drag: Option<DataFieldDrag>,
    pending_data_field_drop: Option<PendingDataFieldDrop>,
    table_templates: Vec<TableTemplate>,
    query_field_picker: Option<QueryFieldPicker>,
    function_picker_visible: bool,
    preview_loading: bool,
    preview_progress: f32,
    preview_stage: String,
    preview_ready_path: Option<PathBuf>,
    preview_started_at: Option<std::time::Instant>,
    open_menu: Option<AppMenu>,
    about_visible: bool,
    toolbox_visible: bool,
    recent_reports: Vec<PathBuf>,
    recent_reports_expanded: bool,
    new_report_confirmation_pending: bool,
    clipboard_item: Option<Item>,
    context_menu_position: Option<Point>,
    images: HashMap<String, DesignerImage>,
}

impl Default for DesignerApp {
    fn default() -> Self {
        let path = std::env::args().nth(1).map(PathBuf::from);
        let mut report = path
            .as_ref()
            .map(|path| {
                Report::from_file(path.to_string_lossy().as_ref())
                    .expect("Cannot load report definition")
            })
            .unwrap_or_else(blank_report);
        ensure_unique_item_names(&mut report);

        let font_resolver = SystemFontResolver::new();
        let families = font_resolver.families();
        let font_names = families
            .iter()
            .map(|family| {
                let name: &'static str = Box::leak(family.clone().into_boxed_str());
                (family.clone(), name)
            })
            .collect();
        let font_families = combo_box::State::new(families);
        let recent_reports = path.clone().into_iter().collect();

        let images = load_designer_images(&report, report_directory(path.as_deref()));

        Self {
            status: path
                .as_ref()
                .map(|path| format!("Loaded {}", path.display()))
                .unwrap_or_else(|| "New blank report".to_string()),
            path,
            report,
            selection: None,
            selected_items: Vec::new(),
            active_band: None,
            zoom: 1.0,
            dirty: false,
            geometry_inputs: GeometryInputs::default(),
            text_inputs: TextInputs::default(),
            shape_inputs: ShapeInputs::default(),
            band_inputs: BandInputs::default(),
            font_families,
            font_resolver,
            font_names,
            collapsed_groups: CollapsedGroups::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            canvas_interaction_active: false,
            properties_visible: true,
            properties_width: DEFAULT_INSPECTOR_WIDTH,
            sidebar_tab: SidebarTab::Properties,
            collapsed_structure_layouts: HashSet::new(),
            structure_drag: None,
            structure_drop_target: None,
            pending_layout_move: None,
            structure_rename: None,
            structure_name_input: String::new(),
            structure_selection_anchor: None,
            keyboard_modifiers: keyboard::Modifiers::default(),
            guides_visible: true,
            error_message: None,
            auto_close_messages: true,
            settings: None,
            data_source_editor: None,
            data_query_editor: None,
            query_text_menu: None,
            query_text_menu_position: None,
            query_name_input_id: iced::widget::Id::unique(),
            item_name_input_id: iced::widget::Id::unique(),
            date_pattern_input_id: iced::widget::Id::unique(),
            value_prefix_input_id: iced::widget::Id::unique(),
            value_suffix_input_id: iced::widget::Id::unique(),
            text_color_input_id: iced::widget::Id::unique(),
            query_rules_editor: None,
            query_fields: HashMap::new(),
            query_field_types: HashMap::new(),
            expanded_data_queries: HashSet::new(),
            selected_data_fields: HashSet::new(),
            open_data_templates: None,
            data_templates_position: None,
            cursor_position: Point::ORIGIN,
            data_field_drag: None,
            pending_data_field_drop: None,
            table_templates: load_table_templates().unwrap_or_default(),
            query_field_picker: None,
            function_picker_visible: false,
            preview_loading: false,
            preview_progress: 0.0,
            preview_stage: "Preparing preview".to_string(),
            preview_ready_path: None,
            preview_started_at: None,
            open_menu: None,
            about_visible: false,
            toolbox_visible: true,
            recent_reports,
            recent_reports_expanded: false,
            new_report_confirmation_pending: false,
            clipboard_item: None,
            context_menu_position: None,
            images,
        }
    }
}

impl DesignerApp {}

pub(crate) fn run() -> iced::Result {
    iced::application(DesignerApp::default, DesignerApp::update, DesignerApp::view)
        .title(concat!(
            "Designer (report-rs v",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
        .window_size(Size::new(1440.0, 850.0))
        .subscription(|app| {
            let timer = if app.auto_close_messages && app.error_message.is_some() {
                iced::time::every(std::time::Duration::from_secs(15)).map(|_| Message::DismissError)
            } else {
                iced::Subscription::none()
            };
            let preview_timer = if app.preview_loading {
                iced::time::every(std::time::Duration::from_millis(80))
                    .map(|_| Message::PreviewProgressTick)
            } else {
                iced::Subscription::none()
            };
            iced::Subscription::batch([keyboard_shortcuts(), timer, preview_timer])
        })
        .run()
}

use designer_canvas::{ColorTarget, ColorWheel, DesignerCanvas, PropertiesResizer};
#[cfg(test)]
use designer_canvas::{hit_test_item, selected_descendant_path};
