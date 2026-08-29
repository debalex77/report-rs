use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::widget::{
    Space, button, column, combo_box, container, mouse_area, opaque, row, scrollable, stack, svg,
    text, text_editor, text_input,
};
use iced::{
    Background, Color, Element, Fill, Point, Rectangle, Renderer, Size, Task, Theme, keyboard,
};

use report_core::common;
use report_core::font::FontSpec;
use report_core::font_resolver::SystemFontResolver;
use report_core::model::{
    Band, BandKind, Color as ReportColor, HorizontalAlign, ImageFit, ImageItem, Item, LayoutItem,
    Margins, Mm, Orientation, Padding, Page, PageSize, RectangleItem, Report, TextItem,
    VerticalAlign,
};

mod settings;
use settings::{DesignerSettings, MarginField, page_font_family};

// CSS pixels per millimeter at the standard 96 DPI screen scale.
const BASE_SCALE: f32 = 96.0 / 25.4;
const PAGE_MARGIN: f32 = 112.0;
const RULER_SIZE: f32 = 24.0;
const RULER_GAP: f32 = 4.0;
const BAND_BADGE_WIDTH: f32 = 74.0;
const DEFAULT_INSPECTOR_WIDTH: f32 = 360.0;
const MIN_INSPECTOR_WIDTH: f32 = 280.0;
const MAX_INSPECTOR_WIDTH: f32 = 560.0;
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

const MAX_SELECTION_DEPTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Selection {
    band: usize,
    path: [usize; MAX_SELECTION_DEPTH],
    depth: usize,
}

impl Selection {
    const fn top_level(band: usize, item: usize) -> Self {
        let mut path = [0; MAX_SELECTION_DEPTH];
        path[0] = item;
        Self {
            band,
            path,
            depth: 1,
        }
    }

    const fn top_index(self) -> usize {
        self.path[0]
    }

    const fn is_top_level(self) -> bool {
        self.depth == 1
    }

    fn indices(&self) -> &[usize] {
        &self.path[..self.depth]
    }

    fn descendants(&self) -> &[usize] {
        &self.path[1..self.depth]
    }

    fn push(mut self, index: usize) -> Option<Self> {
        if self.depth >= MAX_SELECTION_DEPTH {
            return None;
        }
        self.path[self.depth] = index;
        self.depth += 1;
        Some(self)
    }
}

#[derive(Debug, Clone, Copy)]
enum PropertyGroup {
    General,
    Geometry,
    TextValue,
    Font,
    TextColor,
    Alignment,
}

struct CollapsedGroups {
    general: bool,
    geometry: bool,
    text_value: bool,
    font: bool,
    text_color: bool,
    alignment: bool,
}

impl Default for CollapsedGroups {
    fn default() -> Self {
        Self {
            general: false,
            geometry: false,
            text_value: false,
            font: true,
            text_color: true,
            alignment: true,
        }
    }
}

impl CollapsedGroups {
    fn toggle(&mut self, group: PropertyGroup) {
        let value = match group {
            PropertyGroup::General => &mut self.general,
            PropertyGroup::Geometry => &mut self.geometry,
            PropertyGroup::TextValue => &mut self.text_value,
            PropertyGroup::Font => &mut self.font,
            PropertyGroup::TextColor => &mut self.text_color,
            PropertyGroup::Alignment => &mut self.alignment,
        };
        *value = !*value;
    }

    fn is_collapsed(&self, group: PropertyGroup) -> bool {
        match group {
            PropertyGroup::General => self.general,
            PropertyGroup::Geometry => self.geometry,
            PropertyGroup::TextValue => self.text_value,
            PropertyGroup::Font => self.font,
            PropertyGroup::TextColor => self.text_color,
            PropertyGroup::Alignment => self.alignment,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeometryField {
    X,
    Y,
    Width,
    Height,
    X1,
    Y1,
    X2,
    Y2,
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
}

impl TextInputs {
    fn sync(&mut self, item: &Item) {
        if let Item::Text(item) = item {
            self.text = text_editor::Content::with_text(&item.text);
            self.font_size = format_pt(item.font_size);
            self.font_family.clone_from(&item.font_family);
            self.text_color = format_report_color(item.text_color);
        } else {
            *self = Self::default();
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
    LineStart,
    LineEnd,
}

#[derive(Debug, Clone, Copy)]
enum DragOperation {
    Move(Selection),
    Resize(Selection, ResizeHandle),
    ResizeBand(usize),
    ResizeLayoutDivider(Selection, usize, bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMenu {
    File,
    Edit,
    Info,
}

#[derive(Debug, Clone, Copy)]
enum DesignerTool {
    ReportHeader,
    DataBand,
    ReportFooter,
    Text,
    Image,
    Shape,
    HorizontalLayout,
    VerticalLayout,
    Delete,
}

#[derive(Debug, Clone)]
enum Message {
    Reload,
    Load,
    Save,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    MoveItem {
        selection: Selection,
        dx: f32,
        dy: f32,
    },
    ResizeItem {
        selection: Selection,
        handle: ResizeHandle,
        dx: f32,
        dy: f32,
    },
    ResizeBand {
        band: usize,
        dy: f32,
    },
    ResizeLayoutDivider {
        selection: Selection,
        divider: usize,
        horizontal: bool,
        delta: f32,
    },
    GeometryChanged(GeometryField, String),
    GeometryStep(GeometryField, f32),
    TextEdited(text_editor::Action),
    FontSizeChanged(String),
    HorizontalAlignChanged(HorizontalAlign),
    VerticalAlignChanged(VerticalAlign),
    FontFamilyChanged(String),
    BoldChanged(bool),
    ItalicChanged(bool),
    TextColorChanged(String),
    TextColorSelected(ReportColor),
    ToggleGroup(PropertyGroup),
    FontLoaded,
    Undo,
    Redo,
    BeginCanvasInteraction {
        selection: Option<Selection>,
        band: Option<usize>,
        additive: bool,
    },
    EndCanvasInteraction,
    ToggleProperties,
    ResizeProperties(f32),
    ToggleGuides,
    DismissError,
    OpenSettings,
    CloseSettings,
    SettingsOrientationChanged(Orientation),
    SettingsMarginChanged(MarginField, String),
    SettingsMarginStep(MarginField, f32),
    SettingsFontChanged(String),
    ApplySettings,
    ToggleMenu(AppMenu),
    CloseMenu,
    OpenAbout,
    CloseAbout,
    ToggleToolbox,
    UseTool(DesignerTool),
    NewReport,
    ToggleRecentReports,
    OpenRecentReport(PathBuf),
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
    font_families: combo_box::State<String>,
    font_resolver: SystemFontResolver,
    font_names: HashMap<String, &'static str>,
    collapsed_groups: CollapsedGroups,
    undo_stack: Vec<Report>,
    redo_stack: Vec<Report>,
    canvas_interaction_active: bool,
    properties_visible: bool,
    properties_width: f32,
    guides_visible: bool,
    error_message: Option<String>,
    settings: Option<DesignerSettings>,
    open_menu: Option<AppMenu>,
    about_visible: bool,
    toolbox_visible: bool,
    recent_reports: Vec<PathBuf>,
    recent_reports_expanded: bool,
    new_report_confirmation_pending: bool,
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
            font_families,
            font_resolver,
            font_names,
            collapsed_groups: CollapsedGroups::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            canvas_interaction_active: false,
            properties_visible: true,
            properties_width: DEFAULT_INSPECTOR_WIDTH,
            guides_visible: true,
            error_message: None,
            settings: None,
            open_menu: None,
            about_visible: false,
            toolbox_visible: true,
            recent_reports,
            recent_reports_expanded: false,
            new_report_confirmation_pending: false,
        }
    }
}

impl DesignerApp {
    fn update(&mut self, message: Message) -> Task<Message> {
        if !matches!(
            &message,
            Message::ToggleMenu(_) | Message::CloseMenu | Message::ToggleRecentReports
        ) {
            self.open_menu = None;
            self.recent_reports_expanded = false;
        }
        match message {
            Message::Load => match select_report_file() {
                Ok(Some(path)) => self.open_report(path),
                Ok(None) => self.status = "Load cancelled".to_string(),
                Err(error) => self.set_error(format!("File dialog failed: {error}")),
            },
            Message::Reload => {
                if let Some(path) = self.path.clone() {
                    self.open_report(path);
                }
            }
            Message::Save => {
                let Some(path) = self.path.as_ref() else {
                    self.set_error("Load a JSON file before saving");
                    return Task::none();
                };
                match self.report.save_to_file(path.to_string_lossy().as_ref()) {
                    Ok(()) => {
                        self.dirty = false;
                        self.error_message = None;
                        self.status = format!("Saved {}", path.display());
                    }
                    Err(error) => self.set_error(format!("Save failed: {error}")),
                }
            }
            Message::ZoomIn => self.zoom = (self.zoom + 0.1).min(2.0),
            Message::ZoomOut => self.zoom = (self.zoom - 0.1).max(0.5),
            Message::ZoomReset => self.zoom = 1.0,
            Message::BeginCanvasInteraction {
                selection,
                band,
                additive,
            } => {
                if additive {
                    if let Some(selection) = selection {
                        if let Some(index) = self
                            .selected_items
                            .iter()
                            .position(|selected| *selected == selection)
                        {
                            self.selected_items.remove(index);
                            self.selection = self.selected_items.last().copied();
                        } else {
                            self.selected_items.push(selection);
                            self.selection = Some(selection);
                        }
                    }
                } else {
                    self.selection = selection;
                    self.selected_items = selection.into_iter().collect();
                }
                self.active_band = self.selection.map(|selection| selection.band).or(band);
                if let Some(selection) = self.selection {
                    self.sync_geometry_inputs(selection);
                    self.canvas_interaction_active = false;
                    return self.load_selected_font();
                }
            }
            Message::EndCanvasInteraction => self.canvas_interaction_active = false,
            Message::MoveItem { selection, dx, dy } => {
                if !self.canvas_interaction_active {
                    self.record_undo();
                    self.canvas_interaction_active = true;
                }
                let mut changed = false;
                if let Some(page) = self.report.pages.first_mut() {
                    let band_width = page.printable_width().0;
                    if let Some(band) = page.bands.get_mut(selection.band) {
                        let band_height = band.height.0;
                        if let Some(item) = band.items.get_mut(selection.top_index()) {
                            move_item(item, dx, dy, band_width, band_height);
                            self.selection = Some(selection);
                            self.dirty = true;
                            self.status = "Unsaved changes".to_string();
                            changed = true;
                        }
                    }
                }
                if changed {
                    self.sync_geometry_inputs(selection);
                }
            }
            Message::ResizeItem {
                selection,
                handle,
                dx,
                dy,
            } => {
                if !self.canvas_interaction_active {
                    self.record_undo();
                    self.canvas_interaction_active = true;
                }
                let mut changed = false;
                if let Some(page) = self.report.pages.first_mut() {
                    let band_width = page.printable_width().0;
                    if let Some(band) = page.bands.get_mut(selection.band) {
                        let band_height = band.height.0;
                        if let Some(item) = band.items.get_mut(selection.top_index()) {
                            resize_item(item, handle, dx, dy, band_width, band_height);
                            self.selection = Some(selection);
                            self.dirty = true;
                            self.status = "Unsaved changes".to_string();
                            changed = true;
                        }
                    }
                }
                if changed {
                    self.sync_geometry_inputs(selection);
                }
            }
            Message::ResizeBand { band, dy } => {
                if !self.canvas_interaction_active {
                    self.record_undo();
                    self.canvas_interaction_active = true;
                }
                if let Some(page) = self.report.pages.first_mut()
                    && resize_band_height(page, band, dy)
                {
                    self.active_band = Some(band);
                    self.selection = None;
                    self.selected_items.clear();
                    self.mark_dirty();
                }
            }
            Message::ResizeLayoutDivider {
                selection,
                divider,
                horizontal,
                delta,
            } => {
                if !self.canvas_interaction_active {
                    self.record_undo();
                    self.canvas_interaction_active = true;
                }
                let changed = item_at_selection_mut(&mut self.report, selection)
                    .is_some_and(|item| resize_layout_divider(item, divider, horizontal, delta));
                if changed {
                    self.selection = Some(selection);
                    self.mark_dirty();
                }
            }
            Message::GeometryChanged(field, value) => {
                self.geometry_inputs.set(field, value.clone());
                if let (Some(selection), Ok(value)) = (self.selection, value.parse::<f32>())
                    && value.is_finite()
                {
                    self.record_undo();
                    if !self.set_geometry(selection, field, value) {
                        return Task::none();
                    }
                    if let Some(actual) = self.geometry_value(selection, field)
                        && (actual - value).abs() > f32::EPSILON
                    {
                        self.geometry_inputs.set(field, format_mm(actual));
                    }
                    self.dirty = true;
                    self.status = "Unsaved changes".to_string();
                }
            }
            Message::GeometryStep(field, delta) => {
                if let Some(selection) = self.selection
                    && let Some(value) = self.geometry_value(selection, field)
                {
                    self.record_undo();
                    if self.set_geometry(selection, field, value + delta) {
                        self.sync_geometry_inputs(selection);
                        self.mark_dirty();
                    }
                }
            }
            Message::TextEdited(action) => {
                self.record_undo();
                self.text_inputs.text.perform(action);
                let value = self.text_inputs.text.text();
                if self.update_selected_text(|item| item.text = value) {
                    self.mark_dirty();
                }
            }
            Message::FontSizeChanged(value) => {
                self.text_inputs.font_size.clone_from(&value);
                if let Ok(font_size) = value.parse::<f32>()
                    && font_size.is_finite()
                {
                    let font_size = font_size.clamp(1.0, 200.0);
                    self.record_undo();
                    if self.update_selected_text(|item| item.font_size = font_size) {
                        if (font_size - value.parse::<f32>().unwrap_or(font_size)).abs()
                            > f32::EPSILON
                        {
                            self.text_inputs.font_size = format_pt(font_size);
                        }
                        self.mark_dirty();
                    }
                }
            }
            Message::HorizontalAlignChanged(alignment) => {
                self.record_undo();
                if self.update_selected_text(|item| item.horizontal_align = alignment) {
                    self.mark_dirty();
                }
            }
            Message::VerticalAlignChanged(alignment) => {
                self.record_undo();
                if self.update_selected_text(|item| item.vertical_align = alignment) {
                    self.mark_dirty();
                }
            }
            Message::FontFamilyChanged(value) => {
                self.text_inputs.font_family.clone_from(&value);
                self.record_undo();
                if self.update_selected_text(|item| item.font_family = value) {
                    self.mark_dirty();
                    return self.load_selected_font();
                }
            }
            Message::BoldChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|item| item.bold = value) {
                    self.mark_dirty();
                    return self.load_selected_font();
                }
            }
            Message::ItalicChanged(value) => {
                self.record_undo();
                if self.update_selected_text(|item| item.italic = value) {
                    self.mark_dirty();
                    return self.load_selected_font();
                }
            }
            Message::TextColorChanged(value) => {
                self.text_inputs.text_color.clone_from(&value);
                if let Some(color) = parse_report_color(&value) {
                    self.record_undo();
                    if !self.update_selected_text(|item| item.text_color = color) {
                        return Task::none();
                    }
                    self.mark_dirty();
                }
            }
            Message::TextColorSelected(color) => {
                self.text_inputs.text_color = format_report_color(color);
                self.record_undo();
                if self.update_selected_text(|item| item.text_color = color) {
                    self.mark_dirty();
                }
            }
            Message::ToggleGroup(group) => self.collapsed_groups.toggle(group),
            Message::FontLoaded => {}
            Message::Undo => self.undo(),
            Message::Redo => self.redo(),
            Message::ToggleProperties => self.properties_visible = !self.properties_visible,
            Message::ResizeProperties(dx) => {
                self.properties_width =
                    (self.properties_width - dx).clamp(MIN_INSPECTOR_WIDTH, MAX_INSPECTOR_WIDTH);
            }
            Message::ToggleGuides => self.guides_visible = !self.guides_visible,
            Message::DismissError => self.error_message = None,
            Message::OpenSettings => {
                if let Some(page) = self.report.pages.first() {
                    self.settings = Some(DesignerSettings::from_page(page, page_font_family(page)));
                }
            }
            Message::CloseSettings => self.settings = None,
            Message::SettingsOrientationChanged(orientation) => {
                if let Some(settings) = &mut self.settings {
                    settings.orientation = orientation;
                }
            }
            Message::SettingsMarginChanged(field, value) => {
                if let Some(settings) = &mut self.settings {
                    settings.set_margin(field, value);
                }
            }
            Message::SettingsMarginStep(field, delta) => {
                if let Some(settings) = &mut self.settings {
                    settings.step_margin(field, delta);
                }
            }
            Message::SettingsFontChanged(value) => {
                if let Some(settings) = &mut self.settings {
                    settings.font_family = value;
                }
            }
            Message::ApplySettings => {
                let Some(settings) = self.settings.clone() else {
                    return Task::none();
                };
                self.record_undo();
                let result = self
                    .report
                    .pages
                    .first_mut()
                    .ok_or_else(|| "The report does not contain any pages".to_string())
                    .and_then(|page| settings.apply(page));
                match result {
                    Ok(()) => {
                        self.settings = None;
                        self.error_message = None;
                        if let Some(selection) = self.selection {
                            self.sync_geometry_inputs(selection);
                        }
                        self.mark_dirty();
                        return self.load_font_family(&settings.font_family);
                    }
                    Err(error) => {
                        self.undo_stack.pop();
                        self.set_error(error);
                    }
                }
            }
            Message::ToggleMenu(menu) => {
                self.open_menu = (self.open_menu != Some(menu)).then_some(menu);
                if self.open_menu != Some(AppMenu::File) {
                    self.recent_reports_expanded = false;
                }
            }
            Message::CloseMenu => self.open_menu = None,
            Message::OpenAbout => {
                self.settings = None;
                self.about_visible = true;
            }
            Message::CloseAbout => self.about_visible = false,
            Message::ToggleToolbox => self.toolbox_visible = !self.toolbox_visible,
            Message::UseTool(tool) => self.use_tool(tool),
            Message::NewReport => self.new_report(),
            Message::ToggleRecentReports => {
                self.recent_reports_expanded = !self.recent_reports_expanded;
            }
            Message::OpenRecentReport(path) => self.open_report(path),
        }

        Task::none()
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.status = "Unsaved changes".to_string();
    }

    fn new_report(&mut self) {
        if self.dirty && !self.new_report_confirmation_pending {
            self.new_report_confirmation_pending = true;
            self.set_error("Unsaved changes: choose New report again to discard them");
            return;
        }
        self.new_report_confirmation_pending = false;
        self.report = blank_report();
        self.path = None;
        self.selection = None;
        self.selected_items.clear();
        self.active_band = None;
        self.geometry_inputs = GeometryInputs::default();
        self.text_inputs = TextInputs::default();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.error_message = None;
        self.status = "New blank report".to_string();
    }

    fn remember_recent_report(&mut self, path: &PathBuf) {
        self.recent_reports.retain(|recent| recent != path);
        self.recent_reports.insert(0, path.clone());
        self.recent_reports.truncate(8);
    }

    fn use_tool(&mut self, tool: DesignerTool) {
        match tool {
            DesignerTool::ReportHeader => self.add_band(BandKind::ReportHeader),
            DesignerTool::DataBand => self.add_band(BandKind::Data {
                source: "data".to_string(),
            }),
            DesignerTool::ReportFooter => self.add_band(BandKind::ReportFooter),
            DesignerTool::Text => {
                let font_family = self
                    .report
                    .pages
                    .first()
                    .map(page_font_family)
                    .unwrap_or_else(report_core::model::default_font_family);
                self.add_item(new_text_item(font_family));
            }
            DesignerTool::Image => self.add_item(new_image_item()),
            DesignerTool::Shape => self.add_item(new_shape_item()),
            DesignerTool::HorizontalLayout => self.create_layout_from_selection(true),
            DesignerTool::VerticalLayout => self.create_layout_from_selection(false),
            DesignerTool::Delete => self.delete_selected_item(),
        }
    }

    fn add_band(&mut self, kind: BandKind) {
        let Some(page) = self.report.pages.first() else {
            self.set_error("The report does not contain any pages");
            return;
        };
        if page
            .bands
            .iter()
            .any(|band| same_band_kind(&band.kind, &kind))
        {
            self.set_error(format!("{} already exists", band_name(&kind)));
            return;
        }
        self.record_undo();
        let page = &mut self.report.pages[0];
        page.bands.push(Band {
            kind,
            height: Mm(30.0),
            items: Vec::new(),
        });
        self.selection = None;
        self.selected_items.clear();
        self.active_band = Some(page.bands.len() - 1);
        self.mark_dirty();
    }

    fn add_item(&mut self, mut item: Item) {
        let Some(band_index) = self
            .selection
            .map(|selection| selection.band)
            .or(self.active_band)
        else {
            self.set_error("Select a report band or an item inside the target band first");
            return;
        };
        let (band_width, band_height) = match self.report.pages.first() {
            Some(page) => (page.printable_width().0, page.bands[band_index].height.0),
            None => return,
        };
        assign_unique_item_name_in_report(&mut item, &self.report);
        apply_generated_text(&mut item);
        self.record_undo();
        let Some(band) = self
            .report
            .pages
            .first_mut()
            .and_then(|page| page.bands.get_mut(band_index))
        else {
            self.undo_stack.pop();
            self.set_error("The selected band is no longer available");
            return;
        };
        let Some((x, y)) = find_free_item_position(&item, &band.items, band_width, band_height)
        else {
            self.undo_stack.pop();
            self.set_error("No free space is available in the selected band");
            return;
        };
        set_item_origin(&mut item, x, y);
        band.items.push(item);
        self.selection = Some(Selection::top_level(band_index, band.items.len() - 1));
        self.selected_items = vec![self.selection.unwrap()];
        self.active_band = Some(band_index);
        self.sync_geometry_inputs(self.selection.unwrap());
        self.mark_dirty();
    }

    fn delete_selected_item(&mut self) {
        let Some(selection) = self.selection else {
            self.set_error("Select an item to delete");
            return;
        };
        self.record_undo();
        let Some(band) = self
            .report
            .pages
            .first_mut()
            .and_then(|page| page.bands.get_mut(selection.band))
        else {
            self.undo_stack.pop();
            return;
        };
        if selection.top_index() >= band.items.len() {
            self.undo_stack.pop();
            return;
        }
        if !remove_item_at_path(&mut band.items, selection.indices()) {
            self.undo_stack.pop();
            return;
        }
        self.selection = None;
        self.selected_items.clear();
        self.active_band = Some(selection.band);
        self.geometry_inputs = GeometryInputs::default();
        self.text_inputs = TextInputs::default();
        self.mark_dirty();
    }

    fn create_layout_from_selection(&mut self, horizontal: bool) {
        if self.selected_items.len() < 2 {
            self.set_error("Select at least two items with Ctrl + click");
            return;
        }
        if self
            .selected_items
            .iter()
            .any(|selection| !selection.is_top_level())
        {
            self.set_error("Nested layout selection is not supported");
            return;
        }
        let band_index = self.selected_items[0].band;
        if self
            .selected_items
            .iter()
            .any(|selection| selection.band != band_index)
        {
            self.set_error("All layout items must belong to the same report band");
            return;
        }
        let mut indices: Vec<usize> = self
            .selected_items
            .iter()
            .map(|item| item.top_index())
            .collect();
        indices.sort_unstable();
        indices.dedup();
        let Some(band) = self
            .report
            .pages
            .first()
            .and_then(|page| page.bands.get(band_index))
        else {
            return;
        };
        if indices.iter().any(|index| *index >= band.items.len()) {
            self.set_error("The selection is no longer valid");
            return;
        }
        let used_names = collect_report_item_names(&self.report);
        self.record_undo();
        let band = &mut self.report.pages[0].bands[band_index];
        let selected: Vec<Item> = indices
            .iter()
            .map(|index| band.items[*index].clone())
            .collect();
        let origin_x = selected
            .iter()
            .map(|item| normalized_geometry(item).0)
            .fold(f32::INFINITY, f32::min);
        let origin_y = selected
            .iter()
            .map(|item| normalized_geometry(item).1)
            .fold(f32::INFINITY, f32::min);
        let (mut children, retained_name) = flatten_matching_layouts(selected, horizontal);
        let (width, height) = if horizontal {
            (
                children
                    .iter()
                    .map(|item| normalized_geometry(item).2)
                    .sum(),
                children
                    .iter()
                    .map(|item| normalized_geometry(item).3)
                    .fold(0.0_f32, f32::max),
            )
        } else {
            (
                children
                    .iter()
                    .map(|item| normalized_geometry(item).2)
                    .fold(0.0_f32, f32::max),
                children
                    .iter()
                    .map(|item| normalized_geometry(item).3)
                    .sum(),
            )
        };
        arrange_layout_children(&mut children, horizontal, width, height);
        let mut layout = LayoutItem {
            name: retained_name.unwrap_or_default(),
            x: Mm(origin_x),
            y: Mm(origin_y),
            width: Mm(width),
            height: Mm(height),
            items: children,
        };
        if layout.name.is_empty() {
            let mut candidate = if horizontal {
                Item::HorizontalLayout(layout.clone())
            } else {
                Item::VerticalLayout(layout.clone())
            };
            assign_unique_item_name_from_used(&mut candidate, &used_names);
            layout.name = item_name(&candidate).to_string();
        }
        let layout_item = if horizontal {
            Item::HorizontalLayout(layout)
        } else {
            Item::VerticalLayout(layout)
        };

        let insert_at = indices[0];
        for index in indices.into_iter().rev() {
            band.items.remove(index);
        }
        band.items.insert(insert_at, layout_item);
        let selection = Selection::top_level(band_index, insert_at);
        self.selection = Some(selection);
        self.selected_items = vec![selection];
        self.active_band = Some(band_index);
        self.sync_geometry_inputs(selection);
        self.mark_dirty();
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
    }

    fn open_report(&mut self, path: PathBuf) {
        match Report::from_file(path.to_string_lossy().as_ref()) {
            Ok(mut report) => {
                ensure_unique_item_names(&mut report);
                self.report = report;
                self.path = Some(path.clone());
                self.selection = None;
                self.selected_items.clear();
                self.active_band = None;
                self.geometry_inputs = GeometryInputs::default();
                self.text_inputs = TextInputs::default();
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.dirty = false;
                self.error_message = None;
                self.settings = None;
                self.new_report_confirmation_pending = false;
                self.status = format!("Loaded {}", path.display());
                self.remember_recent_report(&path);
            }
            Err(error) => self.set_error(format!("Load failed: {error}")),
        }
    }

    fn record_undo(&mut self) {
        const HISTORY_LIMIT: usize = 100;
        if self.undo_stack.len() == HISTORY_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.report.clone());
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        let Some(report) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack
            .push(std::mem::replace(&mut self.report, report));
        self.sync_after_history("Undo");
    }

    fn redo(&mut self) {
        let Some(report) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack
            .push(std::mem::replace(&mut self.report, report));
        self.sync_after_history("Redo");
    }

    fn sync_after_history(&mut self, action: &str) {
        self.selected_items
            .retain(|selection| report_contains_selection(&self.report, *selection));
        if let Some(selection) = self
            .selection
            .filter(|selection| report_contains_selection(&self.report, *selection))
        {
            self.active_band = Some(selection.band);
            self.sync_geometry_inputs(selection);
        } else {
            self.selection = None;
            self.geometry_inputs = GeometryInputs::default();
            self.text_inputs = TextInputs::default();
            if self
                .active_band
                .is_some_and(|band| !report_contains_band(&self.report, band))
            {
                self.active_band = None;
            }
        }
        self.dirty = true;
        self.status = format!("{action}: unsaved changes");
    }

    fn load_selected_font(&self) -> Task<Message> {
        let Some(selection) = self.selection else {
            return Task::none();
        };
        let Some(Item::Text(item)) = item_at_selection(&self.report, selection) else {
            return Task::none();
        };
        let Some(font) = self.font_resolver.resolve(&item.font_spec()) else {
            return Task::none();
        };

        iced::font::load(font.data).map(|_| Message::FontLoaded)
    }

    fn load_font_family(&self, family: &str) -> Task<Message> {
        let spec = FontSpec {
            family: family.to_string(),
            ..FontSpec::default()
        };
        let Some(font) = self.font_resolver.resolve(&spec) else {
            return Task::none();
        };

        iced::font::load(font.data).map(|_| Message::FontLoaded)
    }

    fn update_selected_text(
        &mut self,
        update: impl FnOnce(&mut report_core::model::TextItem),
    ) -> bool {
        let Some(selection) = self.selection else {
            return false;
        };
        let Some(Item::Text(item)) = item_at_selection_mut(&mut self.report, selection) else {
            return false;
        };
        update(item);
        true
    }

    fn sync_geometry_inputs(&mut self, selection: Selection) {
        if let Some(item) = item_at_selection(&self.report, selection) {
            self.geometry_inputs.sync(item);
            self.text_inputs.sync(item);
        }
    }

    fn set_geometry(&mut self, selection: Selection, field: GeometryField, value: f32) -> bool {
        if !selection.is_top_level() {
            return false;
        }
        let Some(page) = self.report.pages.first_mut() else {
            return false;
        };
        let band_width = page.printable_width().0;
        let Some(band) = page.bands.get_mut(selection.band) else {
            return false;
        };
        let band_height = band.height.0;
        let Some(item) = band.items.get_mut(selection.top_index()) else {
            return false;
        };

        set_item_geometry(item, field, value, band_width, band_height)
    }

    fn geometry_value(&self, selection: Selection, field: GeometryField) -> Option<f32> {
        let item = item_at_selection(&self.report, selection)?;
        item_geometry_value(item, field)
    }

    fn inspector(&self) -> Element<'_, Message> {
        let status = if self.dirty {
            format!("● {}", self.status)
        } else {
            self.status.clone()
        };
        let mut content = column![text("Properties").size(20), text(status).size(11)].spacing(7);

        if let Some((band, item)) = self.selection.and_then(|selection| {
            let band = self.report.pages.first()?.bands.get(selection.band)?;
            let item = item_at_selection(&self.report, selection)?;
            Some((band, item))
        }) {
            content = content.push(property_group_header(
                "General",
                PropertyGroup::General,
                self.collapsed_groups.is_collapsed(PropertyGroup::General),
            ));
            if !self.collapsed_groups.is_collapsed(PropertyGroup::General) {
                content = content
                    .push(text(format!("Band: {}", band_name(&band.kind))).size(13))
                    .push(text(format!("Item: {}", item_type_name(item))).size(13))
                    .push(text(format!("Name: {}", item_name(item))).size(13));
            }

            content = content.push(property_group_header(
                "Geometry",
                PropertyGroup::Geometry,
                self.collapsed_groups.is_collapsed(PropertyGroup::Geometry),
            ));
            if !self.collapsed_groups.is_collapsed(PropertyGroup::Geometry) {
                if self
                    .selection
                    .is_some_and(|selection| !selection.is_top_level())
                {
                    content = content.push(
                        text("Geometry is controlled by the parent layout.")
                            .size(12)
                            .color(Color::from_rgb8(150, 155, 165)),
                    );
                } else {
                    for (label, field) in geometry_field_specs(item) {
                        content = content.push(
                            row![
                                text(label).size(12).width(52),
                                spin_button("−", Message::GeometryStep(field, -0.10)),
                                text_input("mm", self.geometry_inputs.value(field))
                                    .width(90)
                                    .size(13)
                                    .padding(6)
                                    .on_input(move |value| Message::GeometryChanged(field, value)),
                                spin_button("+", Message::GeometryStep(field, 0.10)),
                            ]
                            .spacing(5)
                            .align_y(iced::Alignment::Center),
                        );
                    }
                }
            }

            if let Item::Text(text_item) = item {
                content = content.push(property_group_header(
                    "Text / Value",
                    PropertyGroup::TextValue,
                    self.collapsed_groups.is_collapsed(PropertyGroup::TextValue),
                ));
                if !self.collapsed_groups.is_collapsed(PropertyGroup::TextValue) {
                    content = content.push(
                        text_editor(&self.text_inputs.text)
                            .placeholder("Text / Value")
                            .size(13)
                            .padding(8)
                            .height(110)
                            .on_action(Message::TextEdited),
                    );
                }

                content = content.push(property_group_header(
                    "Font Family",
                    PropertyGroup::Font,
                    self.collapsed_groups.is_collapsed(PropertyGroup::Font),
                ));
                if !self.collapsed_groups.is_collapsed(PropertyGroup::Font) {
                    content = content
                        .push(text("Font size (pt)").size(12))
                        .push(
                            text_input("pt", &self.text_inputs.font_size)
                                .size(13)
                                .padding(6)
                                .on_input(Message::FontSizeChanged),
                        )
                        .push(
                            combo_box(
                                &self.font_families,
                                "Font family",
                                Some(&self.text_inputs.font_family),
                                Message::FontFamilyChanged,
                            )
                            .size(13)
                            .padding(6)
                            .on_input(Message::FontFamilyChanged),
                        )
                        .push(
                            row![
                                alignment_button(
                                    include_bytes!("../../../assets/format-text-bold-symbolic.svg"),
                                    text_item.bold,
                                )
                                .on_press(Message::BoldChanged(!text_item.bold)),
                                alignment_button(
                                    include_bytes!(
                                        "../../../assets/format-text-italic-symbolic.svg"
                                    ),
                                    text_item.italic,
                                )
                                .on_press(Message::ItalicChanged(!text_item.italic)),
                            ]
                            .spacing(4),
                        );
                }

                content = content.push(property_group_header(
                    "Text Color",
                    PropertyGroup::TextColor,
                    self.collapsed_groups.is_collapsed(PropertyGroup::TextColor),
                ));

                let mut palette_top = row![].spacing(2);
                let mut palette_bottom = row![].spacing(2);
                for (index, color) in TEXT_COLOR_PALETTE.into_iter().enumerate() {
                    let color_button = button(text(""))
                        .width(28)
                        .height(24)
                        .padding(0)
                        .style(color_swatch_style(color, color == text_item.text_color))
                        .on_press(Message::TextColorSelected(color));
                    if index < 5 {
                        palette_top = palette_top.push(color_button);
                    } else {
                        palette_bottom = palette_bottom.push(color_button);
                    }
                }

                if !self.collapsed_groups.is_collapsed(PropertyGroup::TextColor) {
                    content = content
                        .push(
                            text_input("#RRGGBB", &self.text_inputs.text_color)
                                .size(13)
                                .padding(6)
                                .on_input(Message::TextColorChanged),
                        )
                        .push(column![palette_top, palette_bottom].spacing(4))
                        .push(text("Custom color").size(12))
                        .push(
                            container(
                                Canvas::new(ColorWheel {
                                    selected: text_item.text_color,
                                })
                                .width(170)
                                .height(170),
                            )
                            .width(Fill)
                            .center_x(Fill),
                        );
                }

                content = content.push(property_group_header(
                    "Alignment",
                    PropertyGroup::Alignment,
                    self.collapsed_groups.is_collapsed(PropertyGroup::Alignment),
                ));
                if !self.collapsed_groups.is_collapsed(PropertyGroup::Alignment) {
                    content = content
                        .push(text("Horizontal").size(12))
                        .push(
                            row![
                                alignment_button(
                                    include_bytes!(
                                        "../../../assets/format-justify-left-symbolic.svg"
                                    ),
                                    matches!(text_item.horizontal_align, HorizontalAlign::Left),
                                )
                                .on_press(Message::HorizontalAlignChanged(HorizontalAlign::Left)),
                                alignment_button(
                                    include_bytes!(
                                        "../../../assets/format-justify-center-symbolic.svg"
                                    ),
                                    matches!(text_item.horizontal_align, HorizontalAlign::Center),
                                )
                                .on_press(Message::HorizontalAlignChanged(HorizontalAlign::Center)),
                                alignment_button(
                                    include_bytes!(
                                        "../../../assets/format-justify-right-symbolic.svg"
                                    ),
                                    matches!(text_item.horizontal_align, HorizontalAlign::Right),
                                )
                                .on_press(Message::HorizontalAlignChanged(HorizontalAlign::Right)),
                            ]
                            .spacing(4),
                        )
                        .push(text("Vertical").size(12))
                        .push(
                            row![
                                alignment_button(
                                    include_bytes!("../../../assets/go-top-symbolic.svg"),
                                    matches!(text_item.vertical_align, VerticalAlign::Top),
                                )
                                .on_press(Message::VerticalAlignChanged(VerticalAlign::Top)),
                                alignment_button(
                                    include_bytes!(
                                        "../../../assets/format-align-vertical-center-symbolic.svg"
                                    ),
                                    matches!(text_item.vertical_align, VerticalAlign::Center),
                                )
                                .on_press(Message::VerticalAlignChanged(VerticalAlign::Center)),
                                alignment_button(
                                    include_bytes!("../../../assets/go-bottom-symbolic.svg"),
                                    matches!(text_item.vertical_align, VerticalAlign::Bottom),
                                )
                                .on_press(Message::VerticalAlignChanged(VerticalAlign::Bottom)),
                            ]
                            .spacing(4),
                        );
                }
            }
        } else if let Some(band_index) = self.active_band {
            if let Some(band) = self.report.pages[0].bands.get(band_index) {
                content = content
                    .push(text(format!("Band: {}", band_name(&band.kind))).size(13))
                    .push(text("Choose an item tool to insert it in this band.").size(12));
            }
        } else {
            content = content.push(text("Click an item on the page to select it."));
        }

        container(scrollable(content.padding(12)).height(Fill))
            .width(self.properties_width)
            .height(Fill)
            .into()
    }

    fn settings_dialog(&self) -> Element<'_, Message> {
        let Some(settings) = &self.settings else {
            return Space::new().into();
        };
        let orientation = row![
            button(text("Portrait").size(13))
                .style(if matches!(settings.orientation, Orientation::Portrait) {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SettingsOrientationChanged(Orientation::Portrait)),
            button(text("Landscape").size(13))
                .style(if matches!(settings.orientation, Orientation::Landscape) {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SettingsOrientationChanged(Orientation::Landscape)),
        ]
        .spacing(5);

        let mut content = column![
            row![
                text("Designer settings").size(20),
                Space::new().width(Fill),
                button(container(text("×").size(14)).center(Fill))
                    .width(28)
                    .height(28)
                    .padding(0)
                    .style(common::style_button(4.0))
                    .on_press(Message::CloseSettings),
            ]
            .align_y(iced::Alignment::Center),
            text("Orientation").size(13),
            orientation,
            text("Margins (mm)").size(13),
        ]
        .spacing(8);

        for (label, field) in [
            ("Left", MarginField::Left),
            ("Top", MarginField::Top),
            ("Right", MarginField::Right),
            ("Bottom", MarginField::Bottom),
        ] {
            content = content.push(
                row![
                    text(label).size(12).width(52),
                    spin_button("−", Message::SettingsMarginStep(field, -0.10)),
                    text_input("mm", settings.margin(field))
                        .width(90)
                        .size(13)
                        .padding(6)
                        .on_input(move |value| Message::SettingsMarginChanged(field, value)),
                    spin_button("+", Message::SettingsMarginStep(field, 0.10)),
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center),
            );
        }

        content = content
            .push(text("Default font family").size(13))
            .push(
                combo_box(
                    &self.font_families,
                    "Font family",
                    Some(&settings.font_family),
                    Message::SettingsFontChanged,
                )
                .size(13)
                .padding(6)
                .on_input(Message::SettingsFontChanged),
            )
            .push(
                row![
                    button(text("Cancel").size(13))
                        .style(common::style_button(5.0))
                        .on_press(Message::CloseSettings),
                    button(text("Apply").size(13))
                        .style(button::primary)
                        .on_press(Message::ApplySettings),
                ]
                .spacing(6),
            );

        container(scrollable(content.padding(18)).height(460))
            .width(430)
            .style(|theme: &Theme| {
                let mut dialog_background = theme.palette().background;
                dialog_background.a = 1.0;
                container::Style {
                    background: Some(Background::Color(dialog_background)),
                    border: iced::Border {
                        color: theme.extended_palette().background.strong.text,
                        width: 1.0,
                        radius: iced::border::radius(14),
                    },
                    text_color: Some(theme.palette().text),
                    shadow: iced::Shadow {
                        color: Color::from_rgba8(0, 0, 0, 0.38),
                        offset: iced::Vector::new(0.0, 8.0),
                        blur_radius: 24.0,
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn about_dialog(&self) -> Element<'_, Message> {
        dialog_container(
            column![
                row![
                    text(concat!(
                        "Designer (report-rs v",
                        env!("CARGO_PKG_VERSION"),
                        ")"
                    ))
                    .size(20),
                    Space::new().width(Fill),
                    button(container(text("×").size(14)).center(Fill))
                        .width(28)
                        .height(28)
                        .padding(0)
                        .style(common::style_button(4.0))
                        .on_press(Message::CloseAbout),
                ]
                .align_y(iced::Alignment::Center),
                text("Visual designer for creating and editing JSON report templates.").size(13),
                button(text("Close").size(13))
                    .style(button::primary)
                    .on_press(Message::CloseAbout),
            ]
            .spacing(12)
            .padding(18),
            430.0,
        )
    }

    fn menu_popup(&self, menu: AppMenu) -> Element<'_, Message> {
        let actions: Element<'_, Message> = match menu {
            AppMenu::File => {
                let mut actions = column![
                    popup_menu_action("New report", Some(Message::NewReport)),
                    popup_menu_action("Load", Some(Message::Load)),
                    popup_menu_action("Save", self.path.is_some().then_some(Message::Save)),
                    popup_menu_action("Reload", self.path.is_some().then_some(Message::Reload)),
                    popup_menu_separator(),
                    popup_menu_action(
                        if self.recent_reports_expanded {
                            "Recent reports  ▾"
                        } else {
                            "Recent reports  ▸"
                        },
                        (!self.recent_reports.is_empty()).then_some(Message::ToggleRecentReports),
                    ),
                ];
                if self.recent_reports_expanded {
                    for path in &self.recent_reports {
                        actions = actions.push(popup_menu_action_owned(
                            format!("    {}", truncate(&path.display().to_string(), 34)),
                            Some(Message::OpenRecentReport(path.clone())),
                        ));
                    }
                }
                actions.spacing(2).width(230).into()
            }
            AppMenu::Edit => column![
                popup_menu_action(
                    "Undo",
                    (!self.undo_stack.is_empty()).then_some(Message::Undo),
                ),
                popup_menu_action(
                    "Redo",
                    (!self.redo_stack.is_empty()).then_some(Message::Redo),
                ),
                popup_menu_separator(),
                popup_menu_action("Designer settings", Some(Message::OpenSettings)),
            ]
            .spacing(2)
            .width(180)
            .into(),
            AppMenu::Info => column![popup_menu_action("About", Some(Message::OpenAbout))]
                .spacing(2)
                .width(180)
                .into(),
        };

        let left = match menu {
            AppMenu::File => 8.0,
            AppMenu::Edit => 57.0,
            AppMenu::Info => 107.0,
        };
        let popup: Element<'_, Message> = container(opaque(
            container(actions)
                .padding(5)
                .width(if menu == AppMenu::File { 240 } else { 190 })
                .style(popup_menu_style),
        ))
        .padding(iced::Padding {
            top: 34.0,
            right: 0.0,
            bottom: 0.0,
            left,
        })
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Top)
        .width(Fill)
        .height(Fill)
        .into();

        stack![
            mouse_area(Space::new().width(Fill).height(Fill)).on_press(Message::CloseMenu),
            popup
        ]
        .into()
    }

    fn toolbox(&self) -> Element<'_, Message> {
        let content = column![
            text("Report bands").size(12),
            toolbox_button(
                include_bytes!("../../../assets/report-band-symbolic.svg"),
                "ReportHeader",
                DesignerTool::ReportHeader,
            ),
            toolbox_button(
                include_bytes!("../../../assets/report-band-symbolic.svg"),
                "DataBand",
                DesignerTool::DataBand,
            ),
            toolbox_button(
                include_bytes!("../../../assets/report-band-symbolic.svg"),
                "ReportFooter",
                DesignerTool::ReportFooter,
            ),
            toolbox_separator(),
            text("Items").size(12),
            toolbox_button(
                include_bytes!("../../../assets/text-item-symbolic.svg"),
                "TextItem",
                DesignerTool::Text,
            ),
            toolbox_button(
                include_bytes!("../../../assets/image-item-symbolic.svg"),
                "ImageItem",
                DesignerTool::Image,
            ),
            toolbox_button(
                include_bytes!("../../../assets/shape-item-symbolic.svg"),
                "ShapeItem",
                DesignerTool::Shape,
            ),
            toolbox_separator(),
            text("Layouts").size(12),
            toolbox_button(
                include_bytes!("../../../assets/horizontal-layout-symbolic.svg"),
                "HorizontalLayout",
                DesignerTool::HorizontalLayout,
            ),
            toolbox_button(
                include_bytes!("../../../assets/vertical-layout-symbolic.svg"),
                "VerticalLayout",
                DesignerTool::VerticalLayout,
            ),
            Space::new().height(Fill),
            toolbox_separator(),
            toolbox_button(
                include_bytes!("../../../assets/delete-item-symbolic.svg"),
                "Delete item",
                DesignerTool::Delete,
            ),
        ]
        .spacing(5)
        .padding(8);

        container(content)
            .width(190)
            .height(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(theme.palette().background)),
                border: iced::Border {
                    color: theme.extended_palette().background.strong.color,
                    width: 0.0,
                    radius: iced::border::radius(0),
                },
                ..Default::default()
            })
            .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let Some(page) = self.report.pages.first() else {
            return container(text("The report does not contain any pages"))
                .center(Fill)
                .into();
        };

        let scale = BASE_SCALE * self.zoom;
        let canvas_width = page.width().0 * scale + PAGE_MARGIN * 2.0;
        let canvas_height = page.height().0 * scale + PAGE_MARGIN * 2.0;
        let canvas = Canvas::new(DesignerCanvas {
            page,
            selection: self.selection,
            selected_items: &self.selected_items,
            active_band: self.active_band,
            scale,
            font_names: &self.font_names,
            guides_visible: self.guides_visible,
        })
        .width(canvas_width)
        .height(canvas_height);
        let workspace = scrollable(container(canvas).center_x(Fill))
            .width(Fill)
            .height(Fill);
        let menu_bar = row![
            menu_button("File", AppMenu::File, self.open_menu),
            menu_button("Edit", AppMenu::Edit, self.open_menu),
            menu_button("Info", AppMenu::Info, self.open_menu),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center);

        let toolbar = row![
            button(text("Load").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press(Message::Load),
            button(text("Save").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press_maybe(self.path.is_some().then_some(Message::Save)),
            button(text("Reload").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press_maybe(self.path.is_some().then_some(Message::Reload)),
            toolbar_separator(),
            button(text("Undo").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press_maybe((!self.undo_stack.is_empty()).then_some(Message::Undo)),
            button(text("Redo").size(13))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press_maybe((!self.redo_stack.is_empty()).then_some(Message::Redo)),
            toolbar_separator(),
            alignment_button(
                include_bytes!("../../../assets/edit-select-all-symbolic.svg"),
                self.guides_visible,
            )
            .on_press(Message::ToggleGuides),
            alignment_button(
                include_bytes!("../../../assets/preferences-system-symbolic.svg"),
                true,
            )
            .on_press(Message::OpenSettings),
            button(container(text("−").size(14)).center(Fill))
                .width(30)
                .height(30)
                .padding(0)
                .style(common::style_button(8.0))
                .on_press(Message::ZoomOut),
            button(text(format!("{}%", (self.zoom * 100.0).round() as u32)))
                .height(30)
                .padding([5, 8])
                .style(common::style_button(6.0))
                .on_press(Message::ZoomReset),
            button(container(text("+").size(14)).center(Fill))
                .width(30)
                .height(30)
                .padding(0)
                .style(common::style_button(6.0))
                .on_press(Message::ZoomIn),
            text(
                self.path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Untitled report".to_string())
            )
            .size(13)
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let work_area: Element<'_, Message> = if self.properties_visible {
            row![
                if self.toolbox_visible {
                    self.toolbox()
                } else {
                    Space::new().into()
                },
                workspace,
                Canvas::new(PropertiesResizer)
                    .width(RESIZER_WIDTH)
                    .height(Fill),
                self.inspector()
            ]
            .height(Fill)
            .into()
        } else {
            row![
                if self.toolbox_visible {
                    self.toolbox()
                } else {
                    Space::new().into()
                },
                workspace
            ]
            .height(Fill)
            .into()
        };
        let status = if self.dirty {
            format!("● {}", self.status)
        } else {
            self.status.clone()
        };
        let status_bar = row![
            status_icon_button(
                include_bytes!("../../../assets/toolbox-symbolic.svg"),
                self.toolbox_visible,
            )
            .on_press(Message::ToggleToolbox),
            text(status).size(12),
            Space::new().width(Fill),
            status_icon_button(
                include_bytes!("../../../assets/sidebar-show-right-symbolic.svg"),
                self.properties_visible,
            )
            .on_press(Message::ToggleProperties),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let mut layout = column![container(menu_bar).padding([3, 8]).width(Fill)];
        layout = layout
            .push(container(toolbar).padding(10).width(Fill))
            .push(work_area);
        if let Some(error) = &self.error_message {
            let error_bar = row![
                text(format!("⚠  {error}")).size(12),
                Space::new().width(Fill),
                button(container(text("×").size(14)).center(Fill))
                    .width(24)
                    .height(22)
                    .padding(0)
                    .style(common::style_button(4.0))
                    .on_press(Message::DismissError),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);
            layout = layout.push(
                container(error_bar)
                    .padding([4, 8])
                    .width(Fill)
                    .style(container::danger),
            );
        }

        let base: Element<'_, Message> = layout
            .push(container(status_bar).padding([2, 8]).width(Fill))
            .into();

        let base = if let Some(menu) = self.open_menu {
            stack![base, self.menu_popup(menu)].into()
        } else {
            base
        };

        if self.settings.is_some() {
            let modal = opaque(
                container(self.settings_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.58))),
                        ..Default::default()
                    }),
            );
            stack![base, modal].into()
        } else if self.about_visible {
            let modal = opaque(
                container(self.about_dialog())
                    .center(Fill)
                    .width(Fill)
                    .height(Fill)
                    .style(modal_backdrop_style),
            );
            stack![base, modal].into()
        } else {
            base
        }
    }
}

struct DesignerCanvas<'a> {
    page: &'a Page,
    selection: Option<Selection>,
    selected_items: &'a [Selection],
    active_band: Option<usize>,
    scale: f32,
    font_names: &'a HashMap<String, &'static str>,
    guides_visible: bool,
}

struct ColorWheel {
    selected: ReportColor,
}

struct PropertiesResizer;

#[derive(Default)]
struct ResizerState {
    dragging: bool,
    last_x: Option<f32>,
}

impl canvas::Program<Message> for PropertiesResizer {
    type State = ResizerState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                state.dragging = true;
                state.last_x = cursor.position().map(|position| position.x);
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                let x = cursor.position()?.x;
                let previous = state.last_x.replace(x)?;
                Some(canvas::Action::publish(Message::ResizeProperties(x - previous)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.dragging =>
            {
                state.dragging = false;
                state.last_x = None;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill(
            &Path::rectangle(
                Point::new(bounds.width / 2.0 - 1.0, 0.0),
                Size::new(2.0, bounds.height),
            ),
            Color::from_rgb8(95, 100, 110),
        );
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.dragging || cursor.is_over(bounds) {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::default()
        }
    }
}

impl canvas::Program<Message> for ColorWheel {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let position = cursor.position_in(bounds)?;
            let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
            let dx = position.x - center.x;
            let dy = position.y - center.y;
            let radius = bounds.width.min(bounds.height) / 2.0 - 8.0;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance <= radius {
                let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                let saturation = (distance / radius).clamp(0.0, 1.0);
                return Some(
                    canvas::Action::publish(Message::TextColorSelected(hsv_to_report_color(
                        hue, saturation, 1.0,
                    )))
                    .and_capture(),
                );
            }
        }

        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 8.0;
        let rings = 14;
        let slices = 72;

        for ring in 0..rings {
            let inner = radius * ring as f32 / rings as f32;
            let outer = radius * (ring + 1) as f32 / rings as f32;
            let saturation = (ring as f32 + 0.5) / rings as f32;
            for slice in 0..slices {
                let a0 = std::f32::consts::TAU * slice as f32 / slices as f32;
                let a1 = std::f32::consts::TAU * (slice + 1) as f32 / slices as f32;
                let path = Path::new(|builder| {
                    builder.move_to(polar_point(center, inner, a0));
                    builder.line_to(polar_point(center, outer, a0));
                    builder.line_to(polar_point(center, outer, a1));
                    builder.line_to(polar_point(center, inner, a1));
                    builder.close();
                });
                let hue = 360.0 * (slice as f32 + 0.5) / slices as f32;
                frame.fill(
                    &path,
                    report_color_to_iced(hsv_to_report_color(hue, saturation, 1.0)),
                );
            }
        }

        frame.stroke(
            &Path::circle(center, radius),
            canvas::Stroke {
                width: 1.0,
                style: canvas::Style::Solid(Color::from_rgb8(90, 95, 105)),
                ..Default::default()
            },
        );

        let (hue, saturation, _) = report_color_to_hsv(self.selected);
        let angle = hue.to_radians();
        let marker = polar_point(center, radius * saturation, angle);
        frame.fill(
            &Path::circle(marker, 5.0),
            report_color_to_iced(self.selected),
        );
        frame.stroke(
            &Path::circle(marker, 6.0),
            canvas::Stroke {
                width: 2.0,
                style: canvas::Style::Solid(Color::from_rgb8(35, 38, 45)),
                ..Default::default()
            },
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        cursor
            .position_in(bounds)
            .map(|_| mouse::Interaction::Pointer)
            .unwrap_or_default()
    }
}

impl canvas::Program<Message> for DesignerCanvas<'_> {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                let (selection, band, operation) = if let Some((selection, divider, horizontal)) =
                    self.layout_divider_hit_test(position)
                {
                    (
                        Some(selection),
                        Some(selection.band),
                        Some(DragOperation::ResizeLayoutDivider(
                            selection, divider, horizontal,
                        )),
                    )
                } else if let Some(band) = self.band_resize_hit_test(position) {
                    (None, Some(band), Some(DragOperation::ResizeBand(band)))
                } else if let Some((selection, handle)) = self.resize_hit_test(position) {
                    (
                        Some(selection),
                        Some(selection.band),
                        Some(DragOperation::Resize(selection, handle)),
                    )
                } else {
                    let selection = self.hit_test(position);
                    let band = selection
                        .map(|selection| selection.band)
                        .or_else(|| self.band_hit_test(position));
                    let operation = selection
                        .filter(|selection| selection.is_top_level())
                        .map(DragOperation::Move);
                    (selection, band, operation)
                };
                state.dragging = operation;
                state.last_position = operation.map(|_| position);

                Some(
                    canvas::Action::publish(Message::BeginCanvasInteraction {
                        selection,
                        band,
                        additive: state.modifiers.control(),
                    })
                    .and_capture(),
                )
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let operation = state.dragging?;
                let position = cursor.position_in(bounds)?;
                let previous = state.last_position.replace(position)?;
                let dx = (position.x - previous.x) / self.scale;
                let dy = (position.y - previous.y) / self.scale;
                let message = match operation {
                    DragOperation::Move(selection) => Message::MoveItem { selection, dx, dy },
                    DragOperation::Resize(selection, handle) => Message::ResizeItem {
                        selection,
                        handle,
                        dx,
                        dy,
                    },
                    DragOperation::ResizeBand(band) => Message::ResizeBand { band, dy },
                    DragOperation::ResizeLayoutDivider(selection, divider, horizontal) => {
                        Message::ResizeLayoutDivider {
                            selection,
                            divider,
                            horizontal,
                            delta: if horizontal { dx } else { dy },
                        }
                    }
                };

                Some(canvas::Action::publish(message).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging.take().is_some() {
                    state.last_position = None;
                    Some(canvas::Action::publish(Message::EndCanvasInteraction).and_capture())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let page_origin = Point::new(PAGE_MARGIN, PAGE_MARGIN);
        let page_size = Size::new(
            self.page.width().0 * self.scale,
            self.page.height().0 * self.scale,
        );
        let page_rect = Path::rectangle(page_origin, page_size);

        frame.fill(&page_rect, Color::WHITE);
        frame.stroke(
            &page_rect,
            canvas::Stroke {
                width: 1.0,
                style: canvas::Style::Solid(Color::from_rgb8(90, 95, 105)),
                ..Default::default()
            },
        );
        if self.guides_visible {
            draw_page_grid(&mut frame, self.page, page_origin, self.scale);

            let selected_bounds = self
                .selection
                .and_then(|selection| selection_bounds(self.page, selection, self.scale));
            draw_rulers(
                &mut frame,
                self.page,
                page_origin,
                page_size,
                self.scale,
                selected_bounds,
            );
        }

        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let content_width = self.page.printable_width().0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;

        for (band_index, band) in self.page.bands.iter().enumerate() {
            let band_height = band.height.0 * self.scale;
            let (band_fill, band_border) = band_colors(&band.kind);
            let band_rect = Path::rectangle(
                Point::new(content_x, band_y),
                Size::new(content_width, band_height),
            );
            frame.fill(&band_rect, band_fill);
            frame.stroke(
                &band_rect,
                canvas::Stroke {
                    width: if self.active_band == Some(band_index) {
                        2.0
                    } else {
                        1.0
                    },
                    style: canvas::Style::Solid(band_border),
                    ..Default::default()
                },
            );
            frame.fill(
                &Path::rectangle(Point::new(content_x, band_y), Size::new(3.0, band_height)),
                band_border,
            );
            frame.fill_text(canvas::Text {
                content: band_name(&band.kind).to_string(),
                position: Point::new(content_x + content_width - 5.0, band_y + 10.0),
                color: Color {
                    a: 0.55,
                    ..band_border
                },
                size: iced::Pixels(10.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                ..Default::default()
            });
            let badge = Path::rectangle(
                Point::new(
                    PAGE_MARGIN - RULER_SIZE - RULER_GAP - BAND_BADGE_WIDTH - 8.0,
                    band_y,
                ),
                Size::new(BAND_BADGE_WIDTH, band_height),
            );
            frame.fill(&badge, band_border);
            frame.fill_text(canvas::Text {
                content: band_name(&band.kind).to_string(),
                position: Point::new(
                    PAGE_MARGIN - RULER_SIZE - RULER_GAP - BAND_BADGE_WIDTH - 3.0,
                    band_y + band_height / 2.0,
                ),
                color: Color::WHITE,
                size: iced::Pixels(10.0),
                align_y: iced::alignment::Vertical::Center,
                ..Default::default()
            });

            for (item_index, item) in band.items.iter().enumerate() {
                let top_level = Selection::top_level(band_index, item_index);
                let selected_path = self
                    .selected_items
                    .iter()
                    .find(|selection| {
                        selection.band == band_index && selection.top_index() == item_index
                    })
                    .map(Selection::descendants);
                draw_item(
                    &mut frame,
                    item,
                    content_x,
                    band_y,
                    self.scale,
                    self.font_names,
                    self.selected_items.contains(&top_level),
                    selected_path,
                    true,
                );
            }

            if self.active_band == Some(band_index) {
                let handle = Path::rounded_rectangle(
                    Point::new(
                        content_x + content_width / 2.0 - 14.0,
                        band_y + band_height - 3.0,
                    ),
                    Size::new(28.0, 6.0),
                    iced::border::radius(3),
                );
                frame.fill(&handle, Color::from_rgb8(225, 80, 55));
            }

            band_y += band_height;
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(position) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };
        if self.band_resize_hit_test(position).is_some() {
            mouse::Interaction::ResizingVertically
        } else if let Some((_, _, horizontal)) = self.layout_divider_hit_test(position) {
            if horizontal {
                mouse::Interaction::ResizingHorizontally
            } else {
                mouse::Interaction::ResizingVertically
            }
        } else if self.resize_hit_test(position).is_some()
            || self.hit_test(position).is_some()
            || self.band_hit_test(position).is_some()
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

#[derive(Default)]
struct CanvasState {
    dragging: Option<DragOperation>,
    last_position: Option<Point>,
    modifiers: keyboard::Modifiers,
}

impl DesignerCanvas<'_> {
    fn resize_hit_test(&self, position: Point) -> Option<(Selection, ResizeHandle)> {
        let selection = self.selection?;
        if !selection.is_top_level() {
            return None;
        }
        let (item, offset_x, offset_y) = self.selected_item(selection)?;

        resize_handle_points(item, offset_x, offset_y, self.scale)
            .into_iter()
            .find(|(point, _)| handle_bounds(*point).contains(position))
            .map(|(_, handle)| (selection, handle))
    }

    fn band_resize_hit_test(&self, position: Point) -> Option<usize> {
        let band_index = self.active_band?;
        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let content_width = self.page.printable_width().0 * self.scale;
        let mut bottom = PAGE_MARGIN + self.page.margins.top.0 * self.scale;
        for (index, band) in self.page.bands.iter().enumerate() {
            bottom += band.height.0 * self.scale;
            if index == band_index {
                let handle = Rectangle::new(
                    Point::new(content_x + content_width / 2.0 - 18.0, bottom - 6.0),
                    Size::new(36.0, 12.0),
                );
                return handle.contains(position).then_some(index);
            }
        }
        None
    }

    fn layout_divider_hit_test(&self, position: Point) -> Option<(Selection, usize, bool)> {
        let selection = self.selection?;
        let (item, offset_x, offset_y) = self.selected_item(selection)?;
        let (layout, horizontal) = match item {
            Item::HorizontalLayout(layout) => (layout, true),
            Item::VerticalLayout(layout) => (layout, false),
            _ => return None,
        };
        if layout.items.len() < 2 {
            return None;
        }
        let layout_x = offset_x + layout.x.0 * self.scale;
        let layout_y = offset_y + layout.y.0 * self.scale;
        for divider in 0..layout.items.len() - 1 {
            let child = normalized_geometry(&layout.items[divider]);
            let handle = if horizontal {
                Rectangle::new(
                    Point::new(layout_x + (child.0 + child.2) * self.scale - 5.0, layout_y),
                    Size::new(10.0, layout.height.0 * self.scale),
                )
            } else {
                Rectangle::new(
                    Point::new(layout_x, layout_y + (child.1 + child.3) * self.scale - 5.0),
                    Size::new(layout.width.0 * self.scale, 10.0),
                )
            };
            if handle.contains(position) {
                return Some((selection, divider, horizontal));
            }
        }
        None
    }

    fn selected_item(&self, selection: Selection) -> Option<(&Item, f32, f32)> {
        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;

        for (band_index, band) in self.page.bands.iter().enumerate() {
            if band_index == selection.band {
                let mut item = band.items.get(selection.top_index())?;
                let mut offset_x = content_x;
                let mut offset_y = band_y;
                for &index in selection.descendants() {
                    let layout = item_layout(item)?;
                    offset_x += layout.x.0 * self.scale;
                    offset_y += layout.y.0 * self.scale;
                    item = layout.items.get(index)?;
                }
                return Some((item, offset_x, offset_y));
            }
            band_y += band.height.0 * self.scale;
        }

        None
    }

    fn hit_test(&self, position: Point) -> Option<Selection> {
        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;
        let mut candidates = Vec::new();

        for (band_index, band) in self.page.bands.iter().enumerate() {
            for (item_index, item) in band.items.iter().enumerate() {
                let parent = Selection::top_level(band_index, item_index);
                if let Some(selection) =
                    hit_test_item(item, content_x, band_y, self.scale, position, parent)
                {
                    candidates.push(selection);
                }
            }
            band_y += band.height.0 * self.scale;
        }

        candidates.pop()
    }

    fn band_hit_test(&self, position: Point) -> Option<usize> {
        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let content_width = self.page.printable_width().0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;
        for (index, band) in self.page.bands.iter().enumerate() {
            let bounds = Rectangle::new(
                Point::new(content_x, band_y),
                Size::new(content_width, band.height.0 * self.scale),
            );
            if bounds.contains(position) {
                return Some(index);
            }
            band_y += band.height.0 * self.scale;
        }
        None
    }
}

fn hit_test_item(
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    position: Point,
    selection: Selection,
) -> Option<Selection> {
    let rect = item_bounds(item, offset_x, offset_y, scale)?;
    if layout_label_bounds(item, rect).is_some_and(|label| label.contains(position)) {
        return Some(selection);
    }
    if !rect.contains(position) {
        return None;
    }
    if let Some(layout) = item_layout(item) {
        for (index, child) in layout.items.iter().enumerate().rev() {
            let Some(child_selection) = selection.push(index) else {
                continue;
            };
            if let Some(hit) =
                hit_test_item(child, rect.x, rect.y, scale, position, child_selection)
            {
                return Some(hit);
            }
        }
    }
    Some(selection)
}

fn draw_item(
    frame: &mut Frame<Renderer>,
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    font_names: &HashMap<String, &'static str>,
    selected: bool,
    selected_path: Option<&[usize]>,
    draw_handles: bool,
) {
    let color = if selected {
        Color::from_rgb8(225, 80, 55)
    } else {
        Color::from_rgb8(30, 140, 155)
    };
    let stroke = canvas::Stroke {
        width: if selected { 2.0 } else { 1.0 },
        style: canvas::Style::Solid(color),
        ..Default::default()
    };

    match item {
        Item::Line(line) => frame.stroke(
            &Path::line(
                Point::new(offset_x + line.x1.0 * scale, offset_y + line.y1.0 * scale),
                Point::new(offset_x + line.x2.0 * scale, offset_y + line.y2.0 * scale),
            ),
            stroke,
        ),
        Item::Text(text_item) => {
            if let Some(rect) = item_bounds(item, offset_x, offset_y, scale) {
                let path = Path::rectangle(rect.position(), rect.size());
                frame.fill(&path, Color::from_rgba8(30, 140, 155, 0.06));
                frame.stroke(&path, stroke);

                if rect.width >= 50.0 && rect.height >= 14.0 {
                    let (text_x, align_x) = match text_item.horizontal_align {
                        HorizontalAlign::Left => {
                            (rect.x + 4.0, iced::alignment::Horizontal::Left.into())
                        }
                        HorizontalAlign::Center => (
                            rect.x + rect.width / 2.0,
                            iced::alignment::Horizontal::Center.into(),
                        ),
                        HorizontalAlign::Right => (
                            rect.x + rect.width - 4.0,
                            iced::alignment::Horizontal::Right.into(),
                        ),
                    };
                    let (text_y, align_y) = match text_item.vertical_align {
                        VerticalAlign::Top => (rect.y + 2.0, iced::alignment::Vertical::Top),
                        VerticalAlign::Center => (
                            rect.y + rect.height / 2.0,
                            iced::alignment::Vertical::Center,
                        ),
                        VerticalAlign::Bottom => (
                            rect.y + rect.height - 2.0,
                            iced::alignment::Vertical::Bottom,
                        ),
                    };
                    frame.fill_text(canvas::Text {
                        content: truncate(&text_item.text, 34),
                        position: Point::new(text_x, text_y),
                        max_width: (rect.width - 8.0).max(0.0),
                        color: report_color_to_iced(text_item.text_color),
                        size: iced::Pixels(
                            text_item.font_size * (96.0 / 72.0) * (scale / BASE_SCALE),
                        ),
                        font: designer_font(text_item, font_names),
                        align_x,
                        align_y,
                        ..Default::default()
                    });
                }
            }
        }
        Item::Rectangle(_)
        | Item::Image(_)
        | Item::HorizontalLayout(_)
        | Item::VerticalLayout(_) => {
            if let Some(rect) = item_bounds(item, offset_x, offset_y, scale) {
                frame.stroke(&Path::rectangle(rect.position(), rect.size()), stroke);
                if let Item::HorizontalLayout(_) | Item::VerticalLayout(_) = item {
                    if let Some(label_bounds) = layout_label_bounds(item, rect) {
                        let label_background = if selected {
                            Color::from_rgba8(225, 80, 55, 0.24)
                        } else {
                            Color::from_rgba8(30, 140, 155, 0.16)
                        };
                        let label_path = Path::rounded_rectangle(
                            label_bounds.position(),
                            label_bounds.size(),
                            iced::border::radius(4),
                        );
                        frame.fill(&label_path, label_background);
                        frame.stroke(&label_path, stroke);
                        frame.fill_text(canvas::Text {
                            content: item_name(item).to_string(),
                            position: Point::new(label_bounds.x + 6.0, label_bounds.y + 9.0),
                            color,
                            size: iced::Pixels(10.0),
                            align_y: iced::alignment::Vertical::Center,
                            ..Default::default()
                        });
                    }
                    let children = match item {
                        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
                            &layout.items
                        }
                        _ => unreachable!(),
                    };
                    for (index, child) in children.iter().enumerate() {
                        draw_item(
                            frame,
                            child,
                            rect.x,
                            rect.y,
                            scale,
                            font_names,
                            selected_path.is_some_and(|path| {
                                path.first() == Some(&index) && path.len() == 1
                            }),
                            selected_path.and_then(|path| selected_descendant_path(path, index)),
                            false,
                        );
                    }
                    if selected {
                        draw_layout_dividers(frame, item, rect, scale);
                    }
                }
            }
        }
    }

    if selected && draw_handles {
        draw_resize_handles(frame, item, offset_x, offset_y, scale);
    }
}

fn selected_descendant_path(path: &[usize], child_index: usize) -> Option<&[usize]> {
    match path {
        [selected_child, descendants @ ..]
            if *selected_child == child_index && !descendants.is_empty() =>
        {
            Some(descendants)
        }
        _ => None,
    }
}

fn draw_layout_dividers(frame: &mut Frame<Renderer>, item: &Item, bounds: Rectangle, scale: f32) {
    let (layout, horizontal) = match item {
        Item::HorizontalLayout(layout) => (layout, true),
        Item::VerticalLayout(layout) => (layout, false),
        _ => return,
    };
    for child in layout
        .items
        .iter()
        .take(layout.items.len().saturating_sub(1))
    {
        let child = normalized_geometry(child);
        let center = if horizontal {
            Point::new(
                bounds.x + (child.0 + child.2) * scale,
                bounds.y + bounds.height / 2.0,
            )
        } else {
            Point::new(
                bounds.x + bounds.width / 2.0,
                bounds.y + (child.1 + child.3) * scale,
            )
        };
        let size = if horizontal {
            Size::new(6.0, 24.0_f32.min(bounds.height))
        } else {
            Size::new(24.0_f32.min(bounds.width), 6.0)
        };
        frame.fill(
            &Path::rounded_rectangle(
                Point::new(center.x - size.width / 2.0, center.y - size.height / 2.0),
                size,
                iced::border::radius(3),
            ),
            Color::from_rgb8(225, 80, 55),
        );
    }
}

fn draw_resize_handles(
    frame: &mut Frame<Renderer>,
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
) {
    for (point, _) in resize_handle_points(item, offset_x, offset_y, scale) {
        let bounds = handle_bounds(point);
        let path = Path::rectangle(bounds.position(), bounds.size());
        frame.fill(&path, Color::WHITE);
        frame.stroke(
            &path,
            canvas::Stroke {
                width: 1.5,
                style: canvas::Style::Solid(Color::from_rgb8(225, 80, 55)),
                ..Default::default()
            },
        );
    }
}

fn resize_handle_points(
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
) -> Vec<(Point, ResizeHandle)> {
    match item {
        Item::Line(line) => vec![
            (
                Point::new(offset_x + line.x1.0 * scale, offset_y + line.y1.0 * scale),
                ResizeHandle::LineStart,
            ),
            (
                Point::new(offset_x + line.x2.0 * scale, offset_y + line.y2.0 * scale),
                ResizeHandle::LineEnd,
            ),
        ],
        _ => item_bounds(item, offset_x, offset_y, scale)
            .map(|bounds| {
                vec![
                    (bounds.position(), ResizeHandle::TopLeft),
                    (
                        Point::new(bounds.x + bounds.width / 2.0, bounds.y),
                        ResizeHandle::Top,
                    ),
                    (
                        Point::new(bounds.x + bounds.width, bounds.y),
                        ResizeHandle::TopRight,
                    ),
                    (
                        Point::new(bounds.x, bounds.y + bounds.height / 2.0),
                        ResizeHandle::Left,
                    ),
                    (
                        Point::new(bounds.x + bounds.width, bounds.y + bounds.height / 2.0),
                        ResizeHandle::Right,
                    ),
                    (
                        Point::new(bounds.x, bounds.y + bounds.height),
                        ResizeHandle::BottomLeft,
                    ),
                    (
                        Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height),
                        ResizeHandle::Bottom,
                    ),
                    (
                        Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
                        ResizeHandle::BottomRight,
                    ),
                ]
            })
            .unwrap_or_default(),
    }
}

fn handle_bounds(center: Point) -> Rectangle {
    Rectangle::new(
        Point::new(center.x - HANDLE_SIZE / 2.0, center.y - HANDLE_SIZE / 2.0),
        Size::new(HANDLE_SIZE, HANDLE_SIZE),
    )
}

fn draw_page_grid(frame: &mut Frame<Renderer>, page: &Page, page_origin: Point, scale: f32) {
    let page_width = page.width().0 * scale;
    let page_height = page.height().0 * scale;

    for millimeter in 0..=page.width().0.floor() as u32 {
        let x = page_origin.x + millimeter as f32 * scale;
        frame.stroke(
            &Path::line(
                Point::new(x, page_origin.y),
                Point::new(x, page_origin.y + page_height),
            ),
            grid_stroke(millimeter),
        );
    }

    for millimeter in 0..=page.height().0.floor() as u32 {
        let y = page_origin.y + millimeter as f32 * scale;
        frame.stroke(
            &Path::line(
                Point::new(page_origin.x, y),
                Point::new(page_origin.x + page_width, y),
            ),
            grid_stroke(millimeter),
        );
    }
}

fn grid_stroke(millimeter: u32) -> canvas::Stroke<'static> {
    let (width, color) = if millimeter % 10 == 0 {
        (1.0, Color::from_rgba8(70, 85, 100, 0.18))
    } else if millimeter % 5 == 0 {
        (1.0, Color::from_rgba8(70, 85, 100, 0.10))
    } else {
        (0.5, Color::from_rgba8(70, 85, 100, 0.05))
    };

    canvas::Stroke {
        width,
        style: canvas::Style::Solid(color),
        ..Default::default()
    }
}

fn draw_rulers(
    frame: &mut Frame<Renderer>,
    page: &Page,
    page_origin: Point,
    page_size: Size,
    scale: f32,
    selected_bounds: Option<Rectangle>,
) {
    let ruler_fill = Color::from_rgb8(224, 226, 230);
    let ruler_line = Color::from_rgb8(75, 80, 88);
    let horizontal_y = page_origin.y - RULER_GAP - RULER_SIZE;
    let vertical_x = page_origin.x - RULER_GAP - RULER_SIZE;

    frame.fill(
        &Path::rectangle(
            Point::new(page_origin.x, horizontal_y),
            Size::new(page_size.width, RULER_SIZE),
        ),
        ruler_fill,
    );
    frame.fill(
        &Path::rectangle(
            Point::new(vertical_x, page_origin.y),
            Size::new(RULER_SIZE, page_size.height),
        ),
        ruler_fill,
    );

    if let Some(bounds) = selected_bounds {
        let shadow = Color::from_rgba8(225, 80, 55, 0.32);
        frame.fill(
            &Path::rectangle(
                Point::new(bounds.x, horizontal_y),
                Size::new(bounds.width, RULER_SIZE),
            ),
            shadow,
        );
        frame.fill(
            &Path::rectangle(
                Point::new(vertical_x, bounds.y),
                Size::new(RULER_SIZE, bounds.height),
            ),
            shadow,
        );
    }

    let tick_stroke = canvas::Stroke {
        width: 1.0,
        style: canvas::Style::Solid(ruler_line),
        ..Default::default()
    };

    for millimeter in 0..=page.width().0.floor() as u32 {
        let x = page_origin.x + millimeter as f32 * scale;
        let tick_height = ruler_tick_length(millimeter);
        frame.stroke(
            &Path::line(
                Point::new(x, page_origin.y - RULER_GAP),
                Point::new(x, page_origin.y - RULER_GAP - tick_height),
            ),
            tick_stroke,
        );
        if millimeter % 10 == 0 {
            frame.fill_text(canvas::Text {
                content: millimeter.to_string(),
                position: Point::new(x + 2.0, horizontal_y + 2.0),
                color: ruler_line,
                size: iced::Pixels(9.0),
                ..Default::default()
            });
        }
    }

    for millimeter in 0..=page.height().0.floor() as u32 {
        let y = page_origin.y + millimeter as f32 * scale;
        let tick_width = ruler_tick_length(millimeter);
        frame.stroke(
            &Path::line(
                Point::new(page_origin.x - RULER_GAP, y),
                Point::new(page_origin.x - RULER_GAP - tick_width, y),
            ),
            tick_stroke,
        );
        if millimeter % 10 == 0 {
            frame.fill_text(canvas::Text {
                content: millimeter.to_string(),
                position: Point::new(vertical_x + 2.0, y + 3.0),
                color: ruler_line,
                size: iced::Pixels(9.0),
                ..Default::default()
            });
        }
    }
}

fn selection_bounds(page: &Page, selection: Selection, scale: f32) -> Option<Rectangle> {
    let content_x = PAGE_MARGIN + page.margins.left.0 * scale;
    let mut band_y = PAGE_MARGIN + page.margins.top.0 * scale;

    for (band_index, band) in page.bands.iter().enumerate() {
        if band_index == selection.band {
            let mut item = band.items.get(selection.top_index())?;
            let mut offset_x = content_x;
            let mut offset_y = band_y;
            for &index in selection.descendants() {
                let layout = item_layout(item)?;
                offset_x += layout.x.0 * scale;
                offset_y += layout.y.0 * scale;
                item = layout.items.get(index)?;
            }
            return item_bounds(item, offset_x, offset_y, scale);
        }
        band_y += band.height.0 * scale;
    }

    None
}

fn ruler_tick_length(millimeter: u32) -> f32 {
    if millimeter % 10 == 0 {
        10.0
    } else if millimeter % 5 == 0 {
        7.0
    } else {
        4.0
    }
}

fn item_bounds(item: &Item, offset_x: f32, offset_y: f32, scale: f32) -> Option<Rectangle> {
    let rect = match item {
        Item::Text(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::Rectangle(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::Image(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => Rectangle::new(
            Point::new(offset_x + item.x.0 * scale, offset_y + item.y.0 * scale),
            Size::new(item.width.0 * scale, item.height.0 * scale),
        ),
        Item::Line(item) => {
            let x1 = offset_x + item.x1.0 * scale;
            let y1 = offset_y + item.y1.0 * scale;
            let x2 = offset_x + item.x2.0 * scale;
            let y2 = offset_y + item.y2.0 * scale;
            Rectangle::new(
                Point::new(x1.min(x2) - 4.0, y1.min(y2) - 4.0),
                Size::new((x2 - x1).abs() + 8.0, (y2 - y1).abs() + 8.0),
            )
        }
    };

    Some(rect)
}

fn band_name(kind: &BandKind) -> &'static str {
    match kind {
        BandKind::ReportHeader => "ReportHeader",
        BandKind::PageHeader => "PageHeader",
        BandKind::Data { .. } => "DataBand",
        BandKind::PageFooter => "PageFooter",
        BandKind::ReportFooter => "ReportFooter",
    }
}

fn band_colors(kind: &BandKind) -> (Color, Color) {
    match kind {
        BandKind::ReportHeader => (
            Color::from_rgba8(224, 238, 250, 0.30),
            Color::from_rgb8(52, 115, 165),
        ),
        BandKind::PageHeader => (
            Color::from_rgba8(224, 242, 238, 0.30),
            Color::from_rgb8(35, 135, 120),
        ),
        BandKind::Data { .. } => (
            Color::from_rgba8(250, 239, 218, 0.30),
            Color::from_rgb8(185, 120, 35),
        ),
        BandKind::PageFooter => (
            Color::from_rgba8(236, 231, 248, 0.30),
            Color::from_rgb8(115, 85, 165),
        ),
        BandKind::ReportFooter => (
            Color::from_rgba8(245, 230, 235, 0.30),
            Color::from_rgb8(165, 75, 105),
        ),
    }
}

fn move_item(item: &mut Item, dx: f32, dy: f32, band_width: f32, band_height: f32) {
    let (min_x, max_x, min_y, max_y) = match item {
        Item::Text(item) => (
            item.x.0,
            item.x.0 + item.width.0,
            item.y.0,
            item.y.0 + item.height.0,
        ),
        Item::Rectangle(item) => (
            item.x.0,
            item.x.0 + item.width.0,
            item.y.0,
            item.y.0 + item.height.0,
        ),
        Item::Image(item) => (
            item.x.0,
            item.x.0 + item.width.0,
            item.y.0,
            item.y.0 + item.height.0,
        ),
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => (
            item.x.0,
            item.x.0 + item.width.0,
            item.y.0,
            item.y.0 + item.height.0,
        ),
        Item::Line(item) => (
            item.x1.0.min(item.x2.0),
            item.x1.0.max(item.x2.0),
            item.y1.0.min(item.y2.0),
            item.y1.0.max(item.y2.0),
        ),
    };
    let (dx, dy) = constrained_delta(min_x, max_x, min_y, max_y, dx, dy, band_width, band_height);

    match item {
        Item::Text(item) => {
            item.x.0 += dx;
            item.y.0 += dy;
        }
        Item::Rectangle(item) => {
            item.x.0 += dx;
            item.y.0 += dy;
        }
        Item::Image(item) => {
            item.x.0 += dx;
            item.y.0 += dy;
        }
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => {
            item.x.0 += dx;
            item.y.0 += dy;
        }
        Item::Line(item) => {
            item.x1.0 += dx;
            item.y1.0 += dy;
            item.x2.0 += dx;
            item.y2.0 += dy;
        }
    }
}

fn resize_item(
    item: &mut Item,
    handle: ResizeHandle,
    dx: f32,
    dy: f32,
    band_width: f32,
    band_height: f32,
) {
    match item {
        Item::Text(item) => resize_rectangle(
            &mut item.x.0,
            &mut item.y.0,
            &mut item.width.0,
            &mut item.height.0,
            handle,
            dx,
            dy,
            band_width,
            band_height,
        ),
        Item::Rectangle(item) => resize_rectangle(
            &mut item.x.0,
            &mut item.y.0,
            &mut item.width.0,
            &mut item.height.0,
            handle,
            dx,
            dy,
            band_width,
            band_height,
        ),
        Item::Image(item) => resize_rectangle(
            &mut item.x.0,
            &mut item.y.0,
            &mut item.width.0,
            &mut item.height.0,
            handle,
            dx,
            dy,
            band_width,
            band_height,
        ),
        Item::HorizontalLayout(item) => {
            resize_layout_trailing_edge(item, true, handle, dx, dy, band_width, band_height)
        }
        Item::VerticalLayout(item) => {
            resize_layout_trailing_edge(item, false, handle, dx, dy, band_width, band_height)
        }
        Item::Line(item) => match handle {
            ResizeHandle::LineStart => {
                item.x1.0 = (item.x1.0 + dx).clamp(0.0, band_width);
                item.y1.0 = (item.y1.0 + dy).clamp(0.0, band_height);
            }
            ResizeHandle::LineEnd => {
                item.x2.0 = (item.x2.0 + dx).clamp(0.0, band_width);
                item.y2.0 = (item.y2.0 + dy).clamp(0.0, band_height);
            }
            _ => {}
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn resize_layout_trailing_edge(
    layout: &mut LayoutItem,
    horizontal: bool,
    handle: ResizeHandle,
    dx: f32,
    dy: f32,
    band_width: f32,
    band_height: f32,
) {
    let old_width = layout.width.0;
    let old_height = layout.height.0;
    resize_rectangle(
        &mut layout.x.0,
        &mut layout.y.0,
        &mut layout.width.0,
        &mut layout.height.0,
        handle,
        dx,
        dy,
        band_width,
        band_height,
    );
    if horizontal
        && matches!(
            handle,
            ResizeHandle::TopRight | ResizeHandle::Right | ResizeHandle::BottomRight
        )
    {
        let Some(last) = layout.items.last_mut() else {
            return;
        };
        let geometry = normalized_geometry(last);
        let requested_delta = layout.width.0 - old_width;
        let actual_delta = requested_delta.max(MIN_ITEM_SIZE - geometry.2);
        layout.width.0 = old_width + actual_delta;
        set_item_frame(
            last,
            geometry.0,
            geometry.1,
            geometry.2 + actual_delta,
            geometry.3,
        );
        scale_layout_contents(
            last,
            geometry.2,
            geometry.3,
            geometry.2 + actual_delta,
            geometry.3,
        );
    } else if !horizontal
        && matches!(
            handle,
            ResizeHandle::BottomLeft | ResizeHandle::Bottom | ResizeHandle::BottomRight
        )
    {
        let Some(last) = layout.items.last_mut() else {
            return;
        };
        let geometry = normalized_geometry(last);
        let requested_delta = layout.height.0 - old_height;
        let actual_delta = requested_delta.max(MIN_ITEM_SIZE - geometry.3);
        layout.height.0 = old_height + actual_delta;
        set_item_frame(
            last,
            geometry.0,
            geometry.1,
            geometry.2,
            geometry.3 + actual_delta,
        );
        scale_layout_contents(
            last,
            geometry.2,
            geometry.3,
            geometry.2,
            geometry.3 + actual_delta,
        );
    }

    if horizontal
        && matches!(
            handle,
            ResizeHandle::TopLeft
                | ResizeHandle::Top
                | ResizeHandle::TopRight
                | ResizeHandle::BottomLeft
                | ResizeHandle::Bottom
                | ResizeHandle::BottomRight
        )
    {
        for child in &mut layout.items {
            let geometry = normalized_geometry(child);
            set_item_frame(child, geometry.0, 0.0, geometry.2, layout.height.0);
            scale_layout_contents(child, geometry.2, geometry.3, geometry.2, layout.height.0);
        }
    } else if !horizontal
        && matches!(
            handle,
            ResizeHandle::TopLeft
                | ResizeHandle::Left
                | ResizeHandle::BottomLeft
                | ResizeHandle::TopRight
                | ResizeHandle::Right
                | ResizeHandle::BottomRight
        )
    {
        for child in &mut layout.items {
            let geometry = normalized_geometry(child);
            set_item_frame(child, 0.0, geometry.1, layout.width.0, geometry.3);
            scale_layout_contents(child, geometry.2, geometry.3, layout.width.0, geometry.3);
        }
    }
}

fn reflow_layout(item: &mut Item) {
    match item {
        Item::HorizontalLayout(layout) => {
            arrange_layout_children(&mut layout.items, true, layout.width.0, layout.height.0);
        }
        Item::VerticalLayout(layout) => {
            arrange_layout_children(&mut layout.items, false, layout.width.0, layout.height.0);
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn resize_rectangle(
    x: &mut f32,
    y: &mut f32,
    width: &mut f32,
    height: &mut f32,
    handle: ResizeHandle,
    dx: f32,
    dy: f32,
    band_width: f32,
    band_height: f32,
) {
    let right = *x + *width;
    let bottom = *y + *height;

    match handle {
        ResizeHandle::TopLeft | ResizeHandle::Left | ResizeHandle::BottomLeft => {
            let new_x = (*x + dx).clamp(0.0, right - MIN_ITEM_SIZE);
            *x = new_x;
            *width = right - new_x;
        }
        ResizeHandle::TopRight | ResizeHandle::Right | ResizeHandle::BottomRight => {
            *width = (right + dx).clamp(*x + MIN_ITEM_SIZE, band_width) - *x;
        }
        _ => {}
    }

    match handle {
        ResizeHandle::TopLeft | ResizeHandle::Top | ResizeHandle::TopRight => {
            let new_y = (*y + dy).clamp(0.0, bottom - MIN_ITEM_SIZE);
            *y = new_y;
            *height = bottom - new_y;
        }
        ResizeHandle::BottomLeft | ResizeHandle::Bottom | ResizeHandle::BottomRight => {
            *height = (bottom + dy).clamp(*y + MIN_ITEM_SIZE, band_height) - *y;
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn constrained_delta(
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    dx: f32,
    dy: f32,
    band_width: f32,
    band_height: f32,
) -> (f32, f32) {
    let item_width = max_x - min_x;
    let item_height = max_y - min_y;
    let target_x = (min_x + dx).clamp(0.0, (band_width - item_width).max(0.0));
    let target_y = (min_y + dy).clamp(0.0, (band_height - item_height).max(0.0));

    (target_x - min_x, target_y - min_y)
}

fn item_name(item: &Item) -> &str {
    match item {
        Item::Text(item) => non_empty_name(&item.name, "TextItem"),
        Item::Line(item) => non_empty_name(&item.name, "LineItem"),
        Item::Rectangle(item) => non_empty_name(&item.name, "ShapeItem"),
        Item::Image(item) => non_empty_name(&item.name, "ImageItem"),
        Item::HorizontalLayout(item) => non_empty_name(&item.name, "HorizontalLayout"),
        Item::VerticalLayout(item) => non_empty_name(&item.name, "VerticalLayout"),
    }
}

fn item_type_name(item: &Item) -> &'static str {
    match item {
        Item::Text(_) => "TextItem",
        Item::Line(_) => "LineItem",
        Item::Rectangle(_) => "ShapeItem",
        Item::Image(_) => "ImageItem",
        Item::HorizontalLayout(_) => "HorizontalLayout",
        Item::VerticalLayout(_) => "VerticalLayout",
    }
}

fn non_empty_name<'a>(name: &'a str, fallback: &'static str) -> &'a str {
    if name.is_empty() { fallback } else { name }
}

fn geometry_field_specs(item: &Item) -> [(&'static str, GeometryField); 4] {
    match item {
        Item::Line(_) => [
            ("X1", GeometryField::X1),
            ("Y1", GeometryField::Y1),
            ("X2", GeometryField::X2),
            ("Y2", GeometryField::Y2),
        ],
        _ => [
            ("X", GeometryField::X),
            ("Y", GeometryField::Y),
            ("Width", GeometryField::Width),
            ("Height", GeometryField::Height),
        ],
    }
}

fn geometry_values(item: &Item) -> (f32, f32, f32, f32) {
    match item {
        Item::Text(item) => (item.x.0, item.y.0, item.width.0, item.height.0),
        Item::Rectangle(item) => (item.x.0, item.y.0, item.width.0, item.height.0),
        Item::Image(item) => (item.x.0, item.y.0, item.width.0, item.height.0),
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => {
            (item.x.0, item.y.0, item.width.0, item.height.0)
        }
        Item::Line(item) => (item.x1.0, item.y1.0, item.x2.0, item.y2.0),
    }
}

fn item_geometry_value(item: &Item, field: GeometryField) -> Option<f32> {
    match (item, field) {
        (Item::Text(item), GeometryField::X) => Some(item.x.0),
        (Item::Text(item), GeometryField::Y) => Some(item.y.0),
        (Item::Text(item), GeometryField::Width) => Some(item.width.0),
        (Item::Text(item), GeometryField::Height) => Some(item.height.0),
        (Item::Rectangle(item), GeometryField::X) => Some(item.x.0),
        (Item::Rectangle(item), GeometryField::Y) => Some(item.y.0),
        (Item::Rectangle(item), GeometryField::Width) => Some(item.width.0),
        (Item::Rectangle(item), GeometryField::Height) => Some(item.height.0),
        (Item::Image(item), GeometryField::X) => Some(item.x.0),
        (Item::Image(item), GeometryField::Y) => Some(item.y.0),
        (Item::Image(item), GeometryField::Width) => Some(item.width.0),
        (Item::Image(item), GeometryField::Height) => Some(item.height.0),
        (Item::HorizontalLayout(item), GeometryField::X)
        | (Item::VerticalLayout(item), GeometryField::X) => Some(item.x.0),
        (Item::HorizontalLayout(item), GeometryField::Y)
        | (Item::VerticalLayout(item), GeometryField::Y) => Some(item.y.0),
        (Item::HorizontalLayout(item), GeometryField::Width)
        | (Item::VerticalLayout(item), GeometryField::Width) => Some(item.width.0),
        (Item::HorizontalLayout(item), GeometryField::Height)
        | (Item::VerticalLayout(item), GeometryField::Height) => Some(item.height.0),
        (Item::Line(item), GeometryField::X1) => Some(item.x1.0),
        (Item::Line(item), GeometryField::Y1) => Some(item.y1.0),
        (Item::Line(item), GeometryField::X2) => Some(item.x2.0),
        (Item::Line(item), GeometryField::Y2) => Some(item.y2.0),
        _ => None,
    }
}

fn format_mm(value: f32) -> String {
    format!("{value:.2}")
}

fn format_pt(value: f32) -> String {
    format!("{value:.1}")
}

fn alignment_button(icon: &'static [u8], selected: bool) -> iced::widget::Button<'static, Message> {
    button(
        svg(svg::Handle::from_memory(icon))
            .width(16)
            .height(16)
            .style(move |theme: &Theme, _status: svg::Status| svg::Style {
                color: Some(if selected {
                    Color::WHITE
                } else {
                    theme.palette().text
                }),
            }),
    )
    .width(36)
    .height(30)
    .style(move |theme, status| {
        let mut style = if selected {
            button::primary(theme, status)
        } else {
            button::secondary(theme, status)
        };
        style.border.radius = iced::border::radius(5);
        style
    })
}

fn status_icon_button(
    icon: &'static [u8],
    selected: bool,
) -> iced::widget::Button<'static, Message> {
    button(
        svg(svg::Handle::from_memory(icon))
            .width(14)
            .height(14)
            .style(move |theme: &Theme, _status: svg::Status| svg::Style {
                color: Some(if selected {
                    Color::WHITE
                } else {
                    theme.palette().text
                }),
            }),
    )
    .width(30)
    .height(24)
    .padding(0)
    .style(move |theme, status| {
        let mut style = if selected {
            button::primary(theme, status)
        } else {
            button::secondary(theme, status)
        };
        style.border.radius = iced::border::radius(4);
        style
    })
}

fn toolbar_separator() -> Element<'static, Message> {
    text("│")
        .size(18)
        .color(Color::from_rgba8(150, 155, 165, 0.55))
        .into()
}

fn menu_button(
    label: &'static str,
    menu: AppMenu,
    open_menu: Option<AppMenu>,
) -> iced::widget::Button<'static, Message> {
    button(text(label).size(13))
        .height(28)
        .padding([4, 10])
        .style(if open_menu == Some(menu) {
            button::primary
        } else {
            button::text
        })
        .on_press(Message::ToggleMenu(menu))
}

fn popup_menu_action(
    label: &'static str,
    message: Option<Message>,
) -> iced::widget::Button<'static, Message> {
    button(text(label).size(12))
        .width(Fill)
        .height(28)
        .padding([5, 10])
        .style(button::text)
        .on_press_maybe(message)
}

fn popup_menu_action_owned(
    label: String,
    message: Option<Message>,
) -> iced::widget::Button<'static, Message> {
    button(text(label).size(12))
        .width(Fill)
        .height(28)
        .padding([5, 10])
        .style(button::text)
        .on_press_maybe(message)
}

fn popup_menu_separator() -> Element<'static, Message> {
    container(Space::new().height(1))
        .width(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color,
            )),
            ..Default::default()
        })
        .into()
}

fn popup_menu_style(theme: &Theme) -> container::Style {
    let mut background = theme.palette().background;
    background.a = 1.0;
    container::Style {
        background: Some(Background::Color(background)),
        border: iced::Border {
            color: theme.extended_palette().background.strong.color,
            width: 1.0,
            radius: iced::border::radius(6),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.35),
            offset: iced::Vector::new(0.0, 5.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}

fn toolbox_button(
    icon: &'static [u8],
    label: &'static str,
    tool: DesignerTool,
) -> iced::widget::Button<'static, Message> {
    button(
        row![
            svg(svg::Handle::from_memory(icon))
                .width(16)
                .height(16)
                .style(|theme: &Theme, _status: svg::Status| svg::Style {
                    color: Some(theme.palette().text),
                }),
            text(label).size(12),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .height(30)
    .padding([5, 8])
    .style(button::secondary)
    .on_press(Message::UseTool(tool))
}

fn toolbox_separator() -> Element<'static, Message> {
    container(Space::new().height(1))
        .width(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color,
            )),
            ..Default::default()
        })
        .into()
}

fn dialog_container<'a>(
    content: impl Into<Element<'a, Message>>,
    width: f32,
) -> Element<'a, Message> {
    container(content).width(width).style(dialog_style).into()
}

fn dialog_style(theme: &Theme) -> container::Style {
    let mut dialog_background = theme.palette().background;
    dialog_background.a = 1.0;
    container::Style {
        background: Some(Background::Color(dialog_background)),
        border: iced::Border {
            color: theme.extended_palette().background.strong.text,
            width: 1.0,
            radius: iced::border::radius(14),
        },
        text_color: Some(theme.palette().text),
        shadow: iced::Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.38),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}

fn modal_backdrop_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.58))),
        ..Default::default()
    }
}

fn spin_button(label: &'static str, message: Message) -> iced::widget::Button<'static, Message> {
    button(container(text(label).size(14)).center(Fill))
        .width(28)
        .height(28)
        .padding(0)
        .style(common::style_button(4.0))
        .on_press(message)
}

fn property_group_header(
    label: &'static str,
    group: PropertyGroup,
    collapsed: bool,
) -> Element<'static, Message> {
    let marker = if collapsed { "▶" } else { "▼" };
    button(text(format!("{marker}  {label}")).size(14))
        .width(Fill)
        .padding([5, 8])
        .on_press(Message::ToggleGroup(group))
        .into()
}

fn color_swatch_style(
    color: ReportColor,
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, _status| button::Style {
        background: Some(Background::Color(report_color_to_iced(color))),
        border: iced::Border {
            color: if selected {
                Color::from_rgb8(225, 80, 55)
            } else {
                Color::from_rgb8(95, 100, 110)
            },
            width: if selected { 3.0 } else { 1.0 },
            radius: iced::border::radius(4),
        },
        ..Default::default()
    }
}

fn polar_point(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

fn hsv_to_report_color(hue: f32, saturation: f32, value: f32) -> ReportColor {
    let chroma = value * saturation;
    let hue_segment = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (hue_segment.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hue_segment as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = value - chroma;

    ReportColor {
        r: ((r + m) * 255.0).round() as u8,
        g: ((g + m) * 255.0).round() as u8,
        b: ((b + m) * 255.0).round() as u8,
        a: 255,
    }
}

fn report_color_to_hsv(color: ReportColor) -> (f32, f32, f32) {
    let r = color.r as f32 / 255.0;
    let g = color.g as f32 / 255.0;
    let b = color.b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };

    (hue, saturation, max)
}

fn format_report_color(color: ReportColor) -> String {
    if color.a == 255 {
        format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
    } else {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            color.r, color.g, color.b, color.a
        )
    }
}

fn parse_report_color(value: &str) -> Option<ReportColor> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if value.len() != 6 && value.len() != 8 {
        return None;
    }

    Some(ReportColor {
        r: u8::from_str_radix(&value[0..2], 16).ok()?,
        g: u8::from_str_radix(&value[2..4], 16).ok()?,
        b: u8::from_str_radix(&value[4..6], 16).ok()?,
        a: if value.len() == 8 {
            u8::from_str_radix(&value[6..8], 16).ok()?
        } else {
            255
        },
    })
}

fn report_color_to_iced(color: ReportColor) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a as f32 / 255.0)
}

fn designer_font(
    item: &report_core::model::TextItem,
    font_names: &HashMap<String, &'static str>,
) -> iced::Font {
    let family_name = item.font_family.to_ascii_lowercase();
    let family = if let Some(name) = font_names.get(&item.font_family) {
        iced::font::Family::Name(name)
    } else if family_name.contains("mono") {
        iced::font::Family::Monospace
    } else if family_name.contains("serif") && !family_name.contains("sans") {
        iced::font::Family::Serif
    } else {
        iced::font::Family::SansSerif
    };

    iced::Font {
        family,
        weight: if item.bold {
            iced::font::Weight::Bold
        } else {
            iced::font::Weight::Normal
        },
        style: if item.italic {
            iced::font::Style::Italic
        } else {
            iced::font::Style::Normal
        },
        ..iced::Font::DEFAULT
    }
}

fn set_item_geometry(
    item: &mut Item,
    field: GeometryField,
    value: f32,
    band_width: f32,
    band_height: f32,
) -> bool {
    match item {
        Item::Text(item) => {
            return set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
        }
        Item::HorizontalLayout(item) => {
            if field == GeometryField::Width {
                let delta = value - item.width.0;
                resize_layout_trailing_edge(
                    item,
                    true,
                    ResizeHandle::Right,
                    delta,
                    0.0,
                    band_width,
                    band_height,
                );
                return delta.abs() > f32::EPSILON;
            }
            if field == GeometryField::Height {
                let delta = value - item.height.0;
                resize_layout_trailing_edge(
                    item,
                    true,
                    ResizeHandle::Bottom,
                    0.0,
                    delta,
                    band_width,
                    band_height,
                );
                return delta.abs() > f32::EPSILON;
            }
            let changed = set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
            return changed;
        }
        Item::VerticalLayout(item) => {
            if field == GeometryField::Width {
                let delta = value - item.width.0;
                resize_layout_trailing_edge(
                    item,
                    false,
                    ResizeHandle::Right,
                    delta,
                    0.0,
                    band_width,
                    band_height,
                );
                return delta.abs() > f32::EPSILON;
            }
            if field == GeometryField::Height {
                let delta = value - item.height.0;
                resize_layout_trailing_edge(
                    item,
                    false,
                    ResizeHandle::Bottom,
                    0.0,
                    delta,
                    band_width,
                    band_height,
                );
                return delta.abs() > f32::EPSILON;
            }
            let changed = set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
            return changed;
        }
        Item::Rectangle(item) => {
            return set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
        }
        Item::Image(item) => {
            return set_rectangle_geometry(
                &mut item.x.0,
                &mut item.y.0,
                &mut item.width.0,
                &mut item.height.0,
                field,
                value,
                band_width,
                band_height,
            );
        }
        Item::Line(item) => match field {
            GeometryField::X1 => item.x1.0 = value.clamp(0.0, band_width),
            GeometryField::Y1 => item.y1.0 = value.clamp(0.0, band_height),
            GeometryField::X2 => item.x2.0 = value.clamp(0.0, band_width),
            GeometryField::Y2 => item.y2.0 = value.clamp(0.0, band_height),
            _ => return false,
        },
    }

    true
}

#[allow(clippy::too_many_arguments)]
fn set_rectangle_geometry(
    x: &mut f32,
    y: &mut f32,
    width: &mut f32,
    height: &mut f32,
    field: GeometryField,
    value: f32,
    band_width: f32,
    band_height: f32,
) -> bool {
    match field {
        GeometryField::X => *x = value.clamp(0.0, (band_width - *width).max(0.0)),
        GeometryField::Y => *y = value.clamp(0.0, (band_height - *height).max(0.0)),
        GeometryField::Width => {
            *width = value.clamp(MIN_ITEM_SIZE, (band_width - *x).max(MIN_ITEM_SIZE))
        }
        GeometryField::Height => {
            *height = value.clamp(MIN_ITEM_SIZE, (band_height - *y).max(MIN_ITEM_SIZE))
        }
        _ => return false,
    }

    true
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{shortened}…")
    } else {
        shortened
    }
}

fn blank_report() -> Report {
    Report {
        name: "Untitled report".to_string(),
        pages: vec![Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,
            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },
            bands: Vec::new(),
        }],
    }
}

fn same_band_kind(left: &BandKind, right: &BandKind) -> bool {
    matches!(
        (left, right),
        (BandKind::ReportHeader, BandKind::ReportHeader)
            | (BandKind::PageHeader, BandKind::PageHeader)
            | (BandKind::Data { .. }, BandKind::Data { .. })
            | (BandKind::PageFooter, BandKind::PageFooter)
            | (BandKind::ReportFooter, BandKind::ReportFooter)
    )
}

fn report_contains_band(report: &Report, band: usize) -> bool {
    report
        .pages
        .first()
        .is_some_and(|page| band < page.bands.len())
}

fn report_contains_selection(report: &Report, selection: Selection) -> bool {
    item_at_selection(report, selection).is_some()
}

fn item_layout(item: &Item) -> Option<&report_core::model::LayoutItem> {
    match item {
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => Some(layout),
        _ => None,
    }
}

fn item_layout_mut(item: &mut Item) -> Option<&mut report_core::model::LayoutItem> {
    match item {
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => Some(layout),
        _ => None,
    }
}

fn item_at_selection(report: &Report, selection: Selection) -> Option<&Item> {
    let mut item = report
        .pages
        .first()?
        .bands
        .get(selection.band)?
        .items
        .get(selection.top_index())?;
    for &index in selection.descendants() {
        item = item_layout(item)?.items.get(index)?;
    }
    Some(item)
}

fn item_at_selection_mut(report: &mut Report, selection: Selection) -> Option<&mut Item> {
    let item = report
        .pages
        .first_mut()?
        .bands
        .get_mut(selection.band)?
        .items
        .get_mut(selection.top_index())?;
    item_at_descendant_mut(item, selection.descendants())
}

fn item_at_descendant_mut<'a>(item: &'a mut Item, path: &[usize]) -> Option<&'a mut Item> {
    let Some((&index, rest)) = path.split_first() else {
        return Some(item);
    };
    let child = item_layout_mut(item)?.items.get_mut(index)?;
    item_at_descendant_mut(child, rest)
}

fn remove_item_at_path(items: &mut Vec<Item>, path: &[usize]) -> bool {
    let Some((&index, rest)) = path.split_first() else {
        return false;
    };
    if rest.is_empty() {
        if index >= items.len() {
            return false;
        }
        items.remove(index);
        return true;
    }
    let Some(parent) = items.get_mut(index) else {
        return false;
    };
    let Some(layout) = item_layout_mut(parent) else {
        return false;
    };
    if !remove_item_at_path(&mut layout.items, rest) {
        return false;
    }
    reflow_layout(parent);
    true
}

fn resize_band_height(page: &mut Page, band_index: usize, dy: f32) -> bool {
    if band_index >= page.bands.len() || !dy.is_finite() {
        return false;
    }
    let other_height: f32 = page
        .bands
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != band_index)
        .map(|(_, band)| band.height.0)
        .sum();
    let item_height = page.bands[band_index]
        .items
        .iter()
        .map(|item| {
            let (_, y, _, height) = geometry_values(item);
            if matches!(item, Item::Line(_)) {
                y.max(height)
            } else {
                y + height
            }
        })
        .fold(5.0_f32, f32::max);
    let max_height = (page.printable_height().0 - other_height).max(item_height);
    let old_height = page.bands[band_index].height.0;
    let new_height = (old_height + dy).clamp(item_height, max_height);
    page.bands[band_index].height = Mm(new_height);
    (new_height - old_height).abs() > f32::EPSILON
}

fn new_text_item(font_family: String) -> Item {
    Item::Text(TextItem {
        name: String::new(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(50.0),
        height: Mm(10.0),
        text: "Text".to_string(),
        font_size: 12.0,
        font_family,
        bold: false,
        italic: false,
        text_color: ReportColor::BLACK,
        horizontal_align: HorizontalAlign::Left,
        vertical_align: VerticalAlign::Center,
        word_wrap: false,
        auto_height: false,
        padding: Padding::default(),
        background: None,
        border: None,
    })
}

fn new_image_item() -> Item {
    Item::Image(ImageItem {
        name: String::new(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(50.0),
        height: Mm(20.0),
        source: String::new(),
        fit: ImageFit::Contain,
    })
}

fn new_shape_item() -> Item {
    Item::Rectangle(RectangleItem {
        name: String::new(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(40.0),
        height: Mm(20.0),
        border_width: Mm(0.5),
    })
}

#[cfg(test)]
fn new_layout_item(horizontal: bool) -> Item {
    let layout = LayoutItem {
        name: String::new(),
        x: Mm(0.0),
        y: Mm(0.0),
        width: Mm(60.0),
        height: Mm(20.0),
        items: Vec::new(),
    };
    if horizontal {
        Item::HorizontalLayout(layout)
    } else {
        Item::VerticalLayout(layout)
    }
}

fn arrange_layout_children(items: &mut [Item], horizontal: bool, width: f32, height: f32) {
    if items.is_empty() {
        return;
    }
    let count = items.len() as f32;
    for (index, item) in items.iter_mut().enumerate() {
        let previous = normalized_geometry(item);
        if horizontal {
            let child_width = width / count;
            set_item_frame(item, index as f32 * child_width, 0.0, child_width, height);
            scale_layout_contents(item, previous.2, previous.3, child_width, height);
        } else {
            let child_height = height / count;
            set_item_frame(item, 0.0, index as f32 * child_height, width, child_height);
            scale_layout_contents(item, previous.2, previous.3, width, child_height);
        }
    }
}

fn scale_layout_contents(
    item: &mut Item,
    old_width: f32,
    old_height: f32,
    new_width: f32,
    new_height: f32,
) {
    let Some(layout) = item_layout_mut(item) else {
        return;
    };
    let scale_x = if old_width > 0.0 {
        new_width / old_width
    } else {
        1.0
    };
    let scale_y = if old_height > 0.0 {
        new_height / old_height
    } else {
        1.0
    };
    for child in &mut layout.items {
        let geometry = normalized_geometry(child);
        let child_width = geometry.2 * scale_x;
        let child_height = geometry.3 * scale_y;
        set_item_frame(
            child,
            geometry.0 * scale_x,
            geometry.1 * scale_y,
            child_width,
            child_height,
        );
        scale_layout_contents(child, geometry.2, geometry.3, child_width, child_height);
    }
}

fn flatten_matching_layouts(selected: Vec<Item>, horizontal: bool) -> (Vec<Item>, Option<String>) {
    let retained_name = selected.iter().find_map(|item| match (item, horizontal) {
        (Item::HorizontalLayout(layout), true) | (Item::VerticalLayout(layout), false) => {
            Some(layout.name.clone())
        }
        _ => None,
    });
    let mut children = Vec::new();
    for item in selected {
        match (item, horizontal) {
            (Item::HorizontalLayout(layout), true) | (Item::VerticalLayout(layout), false) => {
                children.extend(layout.items);
            }
            (item, _) => children.push(item),
        }
    }
    (children, retained_name)
}

fn resize_layout_divider(item: &mut Item, divider: usize, horizontal: bool, delta: f32) -> bool {
    const MIN_CHILD_SIZE: f32 = 1.0;
    let items = match (item, horizontal) {
        (Item::HorizontalLayout(layout), true) => &mut layout.items,
        (Item::VerticalLayout(layout), false) => &mut layout.items,
        _ => return false,
    };
    if divider + 1 >= items.len() || !delta.is_finite() {
        return false;
    }
    let (left_items, right_items) = items.split_at_mut(divider + 1);
    let before = &mut left_items[divider];
    let after = &mut right_items[0];
    let before_geometry = normalized_geometry(before);
    let after_geometry = normalized_geometry(after);
    let available_before = if horizontal {
        before_geometry.2
    } else {
        before_geometry.3
    };
    let available_after = if horizontal {
        after_geometry.2
    } else {
        after_geometry.3
    };
    let delta = delta.clamp(
        MIN_CHILD_SIZE - available_before,
        available_after - MIN_CHILD_SIZE,
    );
    if delta.abs() <= f32::EPSILON {
        return false;
    }
    if horizontal {
        set_item_frame(
            before,
            before_geometry.0,
            before_geometry.1,
            before_geometry.2 + delta,
            before_geometry.3,
        );
        set_item_frame(
            after,
            after_geometry.0 + delta,
            after_geometry.1,
            after_geometry.2 - delta,
            after_geometry.3,
        );
    } else {
        set_item_frame(
            before,
            before_geometry.0,
            before_geometry.1,
            before_geometry.2,
            before_geometry.3 + delta,
        );
        set_item_frame(
            after,
            after_geometry.0,
            after_geometry.1 + delta,
            after_geometry.2,
            after_geometry.3 - delta,
        );
    }
    reflow_layout(before);
    reflow_layout(after);
    true
}

fn set_item_frame(item: &mut Item, x: f32, y: f32, width: f32, height: f32) {
    match item {
        Item::Text(item) => {
            (item.x, item.y, item.width, item.height) = (Mm(x), Mm(y), Mm(width), Mm(height));
        }
        Item::Rectangle(item) => {
            (item.x, item.y, item.width, item.height) = (Mm(x), Mm(y), Mm(width), Mm(height));
        }
        Item::Image(item) => {
            (item.x, item.y, item.width, item.height) = (Mm(x), Mm(y), Mm(width), Mm(height));
        }
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => {
            (item.x, item.y, item.width, item.height) = (Mm(x), Mm(y), Mm(width), Mm(height));
        }
        Item::Line(item) => {
            (item.x1, item.y1, item.x2, item.y2) = (Mm(x), Mm(y), Mm(x + width), Mm(y + height));
        }
    }
}

#[cfg(test)]
fn assign_unique_item_name(item: &mut Item, siblings: &[Item]) {
    let used = siblings
        .iter()
        .map(|item| item_name_storage(item).clone())
        .collect();
    assign_unique_item_name_from_used(item, &used);
}

fn assign_unique_item_name_in_report(item: &mut Item, report: &Report) {
    let used = collect_report_item_names(report);
    assign_unique_item_name_from_used(item, &used);
}

fn assign_unique_item_name_from_used(item: &mut Item, used: &HashSet<String>) {
    let prefix = item_name_prefix(item);
    let mut index = 1;
    loop {
        let candidate = format!("{prefix}{index}");
        if !used.contains(&candidate) {
            *item_name_mut(item) = candidate;
            return;
        }
        index += 1;
    }
}

fn apply_generated_text(item: &mut Item) {
    let Item::Text(text) = item else {
        return;
    };
    if text.text != "Text" {
        return;
    }
    if let Some(index) = text.name.strip_prefix("itemText")
        && !index.is_empty()
        && index.chars().all(|character| character.is_ascii_digit())
    {
        text.text = format!("text{index}");
    }
}

fn ensure_unique_item_names(report: &mut Report) {
    let mut used = HashSet::new();
    let mut counters: HashMap<&'static str, usize> = HashMap::new();
    for page in &mut report.pages {
        for band in &mut page.bands {
            ensure_unique_names_in_items(&mut band.items, &mut used, &mut counters);
        }
    }
}

fn ensure_unique_names_in_items(
    items: &mut [Item],
    used: &mut HashSet<String>,
    counters: &mut HashMap<&'static str, usize>,
) {
    for item in items {
        let prefix = item_name_prefix(item);
        let current = item_name_storage(item).clone();
        if current.is_empty() || !used.insert(current) {
            let counter = counters.entry(prefix).or_insert(0);
            loop {
                *counter += 1;
                let candidate = format!("{prefix}{counter}");
                if used.insert(candidate.clone()) {
                    *item_name_mut(item) = candidate;
                    break;
                }
            }
        }
        match item {
            Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
                ensure_unique_names_in_items(&mut layout.items, used, counters);
            }
            _ => {}
        }
    }
}

fn collect_report_item_names(report: &Report) -> HashSet<String> {
    let mut names = HashSet::new();
    for page in &report.pages {
        for band in &page.bands {
            collect_item_names(&band.items, &mut names);
        }
    }
    names
}

fn collect_item_names(items: &[Item], names: &mut HashSet<String>) {
    for item in items {
        let name = item_name_storage(item);
        if !name.is_empty() {
            names.insert(name.clone());
        }
        if let Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) = item {
            collect_item_names(&layout.items, names);
        }
    }
}

fn item_name_prefix(item: &Item) -> &'static str {
    match item {
        Item::Text(_) => "itemText",
        Item::Image(_) => "itemImage",
        Item::Rectangle(_) => "itemShape",
        Item::Line(_) => "itemLine",
        Item::HorizontalLayout(_) => "horizontalLayout",
        Item::VerticalLayout(_) => "verticalLayout",
    }
}

fn item_name_storage(item: &Item) -> &String {
    match item {
        Item::Text(item) => &item.name,
        Item::Line(item) => &item.name,
        Item::Rectangle(item) => &item.name,
        Item::Image(item) => &item.name,
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => &item.name,
    }
}

fn item_name_mut(item: &mut Item) -> &mut String {
    match item {
        Item::Text(item) => &mut item.name,
        Item::Line(item) => &mut item.name,
        Item::Rectangle(item) => &mut item.name,
        Item::Image(item) => &mut item.name,
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => &mut item.name,
    }
}

fn find_free_item_position(
    item: &Item,
    siblings: &[Item],
    band_width: f32,
    band_height: f32,
) -> Option<(f32, f32)> {
    const STEP: f32 = 5.0;
    const GAP: f32 = 1.0;
    let (_, _, width, height) = normalized_geometry(item);
    if width > band_width || height > band_height {
        return None;
    }
    let rows = ((band_height - height) / STEP).floor() as usize;
    let columns = ((band_width - width) / STEP).floor() as usize;
    for row in 0..=rows {
        let y = row as f32 * STEP;
        for column in 0..=columns {
            let x = column as f32 * STEP;
            let candidate = (x, y, width, height);
            if siblings
                .iter()
                .all(|sibling| !rectangles_overlap(candidate, normalized_geometry(sibling), GAP))
            {
                return Some((x, y));
            }
        }
    }
    None
}

fn normalized_geometry(item: &Item) -> (f32, f32, f32, f32) {
    let (x, y, width, height) = geometry_values(item);
    if matches!(item, Item::Line(_)) {
        (
            x.min(width),
            y.min(height),
            (width - x).abs(),
            (height - y).abs(),
        )
    } else {
        (x, y, width, height)
    }
}

fn rectangles_overlap(left: (f32, f32, f32, f32), right: (f32, f32, f32, f32), gap: f32) -> bool {
    let (lx, ly, lw, lh) = left;
    let (rx, ry, rw, rh) = right;
    lx < rx + rw + gap && lx + lw + gap > rx && ly < ry + rh + gap && ly + lh + gap > ry
}

fn layout_label_bounds(item: &Item, bounds: Rectangle) -> Option<Rectangle> {
    let width = (item_name(item).chars().count() as f32 * 6.2 + 14.0).clamp(76.0, 180.0);
    match item {
        Item::HorizontalLayout(_) => Some(Rectangle::new(
            Point::new(bounds.x, bounds.y - 20.0),
            Size::new(width, 18.0),
        )),
        Item::VerticalLayout(_) => Some(Rectangle::new(
            Point::new(bounds.x + bounds.width + 2.0, bounds.y),
            Size::new(width, 18.0),
        )),
        _ => None,
    }
}

fn set_item_origin(item: &mut Item, x: f32, y: f32) {
    let (old_x, old_y, _, _) = normalized_geometry(item);
    move_item(item, x - old_x, y - old_y, f32::MAX, f32::MAX);
}

fn select_report_file() -> Result<Option<PathBuf>, String> {
    let output = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--title=Load report JSON",
            "--file-filter=Report JSON | *.json",
        ])
        .output()
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        return Ok(None);
    }

    let path = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let path = path.trim();
    Ok((!path.is_empty()).then(|| PathBuf::from(path)))
}

fn main() -> iced::Result {
    iced::application(DesignerApp::default, DesignerApp::update, DesignerApp::view)
        .title(concat!(
            "Designer (report-rs v",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
        .window_size(Size::new(1440.0, 850.0))
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_delta_is_clamped_to_band_bounds() {
        let delta = constrained_delta(10.0, 30.0, 5.0, 15.0, -50.0, 100.0, 100.0, 40.0);

        assert_eq!(delta, (-10.0, 25.0));
    }

    #[test]
    fn resize_is_clamped_and_preserves_minimum_size() {
        let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

        resize_rectangle(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            ResizeHandle::BottomRight,
            100.0,
            100.0,
            40.0,
            20.0,
        );
        assert_eq!((x, y, width, height), (10.0, 5.0, 30.0, 15.0));

        resize_rectangle(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            ResizeHandle::TopLeft,
            100.0,
            100.0,
            40.0,
            20.0,
        );
        assert_eq!((x, y, width, height), (39.0, 19.0, 1.0, 1.0));
    }

    #[test]
    fn middle_handle_changes_only_height() {
        let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

        resize_rectangle(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            ResizeHandle::Bottom,
            12.0,
            3.0,
            100.0,
            40.0,
        );

        assert_eq!((x, y, width, height), (10.0, 5.0, 20.0, 13.0));
    }

    #[test]
    fn side_handle_changes_only_width() {
        let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

        resize_rectangle(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            ResizeHandle::Right,
            4.0,
            12.0,
            100.0,
            40.0,
        );

        assert_eq!((x, y, width, height), (10.0, 5.0, 24.0, 10.0));
    }

    #[test]
    fn property_geometry_is_clamped_to_band() {
        let (mut x, mut y, mut width, mut height) = (10.0, 5.0, 20.0, 10.0);

        assert!(set_rectangle_geometry(
            &mut x,
            &mut y,
            &mut width,
            &mut height,
            GeometryField::Width,
            200.0,
            100.0,
            40.0,
        ));

        assert_eq!((x, y, width, height), (10.0, 5.0, 90.0, 10.0));
    }

    #[test]
    fn text_color_hex_round_trip() {
        let color = ReportColor {
            r: 30,
            g: 140,
            b: 155,
            a: 128,
        };

        let encoded = format_report_color(color);

        assert_eq!(encoded, "#1E8C9B80");
        assert_eq!(parse_report_color(&encoded), Some(color));
        assert_eq!(parse_report_color("#not-a-color"), None);
    }

    #[test]
    fn hsv_color_conversion_uses_expected_primary_colors() {
        assert_eq!(
            hsv_to_report_color(0.0, 1.0, 1.0),
            ReportColor::rgb(255, 0, 0)
        );
        assert_eq!(
            hsv_to_report_color(120.0, 1.0, 1.0),
            ReportColor::rgb(0, 255, 0)
        );
        assert_eq!(
            hsv_to_report_color(240.0, 1.0, 1.0),
            ReportColor::rgb(0, 0, 255)
        );
    }

    #[test]
    fn property_groups_have_expected_initial_state() {
        let groups = CollapsedGroups::default();

        assert!(!groups.is_collapsed(PropertyGroup::General));
        assert!(!groups.is_collapsed(PropertyGroup::Geometry));
        assert!(!groups.is_collapsed(PropertyGroup::TextValue));
        assert!(groups.is_collapsed(PropertyGroup::Font));
        assert!(groups.is_collapsed(PropertyGroup::TextColor));
        assert!(groups.is_collapsed(PropertyGroup::Alignment));
    }

    #[test]
    fn millimeters_use_two_decimal_places() {
        assert_eq!(format_mm(0.0), "0.00");
        assert_eq!(format_mm(12.345), "12.35");
    }

    #[test]
    fn blank_report_contains_an_empty_a4_page() {
        let report = blank_report();

        assert_eq!(report.pages.len(), 1);
        assert!(report.pages[0].bands.is_empty());
        assert_eq!(report.pages[0].dimensions(), (Mm(210.0), Mm(297.0)));
    }

    #[test]
    fn band_resize_changes_only_height_and_stays_inside_printable_page() {
        let mut report = blank_report();
        report.pages[0].bands.push(Band {
            kind: BandKind::ReportHeader,
            height: Mm(30.0),
            items: Vec::new(),
        });

        assert!(resize_band_height(&mut report.pages[0], 0, 12.5));
        assert_eq!(report.pages[0].bands[0].height, Mm(42.5));

        resize_band_height(&mut report.pages[0], 0, 1_000.0);
        assert_eq!(
            report.pages[0].bands[0].height,
            report.pages[0].printable_height()
        );
    }

    #[test]
    fn history_selection_is_invalid_after_selected_item_disappears() {
        let mut report = blank_report();
        report.pages[0].bands.push(Band {
            kind: BandKind::ReportHeader,
            height: Mm(30.0),
            items: vec![new_text_item("DejaVu Sans".to_string())],
        });
        let selection = Selection::top_level(0, 0);
        assert!(report_contains_selection(&report, selection));

        report.pages[0].bands[0].items.clear();

        assert!(!report_contains_selection(&report, selection));
    }

    #[test]
    fn inserted_items_receive_sequential_unique_names() {
        let mut first = new_text_item("DejaVu Sans".to_string());
        assign_unique_item_name(&mut first, &[]);
        let mut second = new_text_item("DejaVu Sans".to_string());
        assign_unique_item_name(&mut second, &[first.clone()]);

        assert_eq!(item_name(&first), "itemText1");
        assert_eq!(item_name(&second), "itemText2");
    }

    #[test]
    fn inserted_item_moves_to_next_free_grid_position() {
        let mut occupied = new_text_item("DejaVu Sans".to_string());
        set_item_origin(&mut occupied, 0.0, 0.0);
        let candidate = new_text_item("DejaVu Sans".to_string());

        let position = find_free_item_position(&candidate, &[occupied], 190.0, 30.0);

        assert_eq!(position, Some((55.0, 0.0)));
    }

    #[test]
    fn layout_items_have_real_geometry_and_distinct_types() {
        let horizontal = new_layout_item(true);
        let vertical = new_layout_item(false);

        assert!(matches!(horizontal, Item::HorizontalLayout(_)));
        assert!(matches!(vertical, Item::VerticalLayout(_)));
        assert_eq!(geometry_values(&horizontal), (0.0, 0.0, 60.0, 20.0));
    }

    #[test]
    fn vertical_layout_distributes_two_items_equally() {
        let mut items = vec![
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
        ];

        arrange_layout_children(&mut items, false, 50.0, 20.0);

        assert_eq!(geometry_values(&items[0]), (0.0, 0.0, 50.0, 10.0));
        assert_eq!(geometry_values(&items[1]), (0.0, 10.0, 50.0, 10.0));
    }

    #[test]
    fn horizontal_layout_distributes_two_items_equally() {
        let mut items = vec![
            new_text_item("DejaVu Sans".to_string()),
            new_text_item("DejaVu Sans".to_string()),
        ];

        arrange_layout_children(&mut items, true, 100.0, 10.0);

        assert_eq!(geometry_values(&items[0]), (0.0, 0.0, 50.0, 10.0));
        assert_eq!(geometry_values(&items[1]), (50.0, 0.0, 50.0, 10.0));
    }

    #[test]
    fn horizontal_layout_divider_changes_only_adjacent_widths() {
        let mut layout = Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(100.0),
            height: Mm(10.0),
            items: vec![
                new_text_item("DejaVu Sans".to_string()),
                new_text_item("DejaVu Sans".to_string()),
            ],
        });
        reflow_layout(&mut layout);

        assert!(resize_layout_divider(&mut layout, 0, true, 10.0));

        let Item::HorizontalLayout(layout) = layout else {
            unreachable!();
        };
        assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 60.0, 10.0));
        assert_eq!(geometry_values(&layout.items[1]), (60.0, 0.0, 40.0, 10.0));
    }

    #[test]
    fn vertical_layout_divider_changes_only_adjacent_heights() {
        let mut layout = Item::VerticalLayout(LayoutItem {
            name: "verticalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(50.0),
            height: Mm(20.0),
            items: vec![
                new_text_item("DejaVu Sans".to_string()),
                new_text_item("DejaVu Sans".to_string()),
            ],
        });
        reflow_layout(&mut layout);

        assert!(resize_layout_divider(&mut layout, 0, false, 4.0));

        let Item::VerticalLayout(layout) = layout else {
            unreachable!();
        };
        assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 50.0, 14.0));
        assert_eq!(geometry_values(&layout.items[1]), (0.0, 14.0, 50.0, 6.0));
    }

    #[test]
    fn nested_selection_resolves_layout_child() {
        let mut report = blank_report();
        let child = new_text_item("DejaVu Sans".to_string());
        report.pages[0].bands.push(Band {
            kind: BandKind::ReportHeader,
            height: Mm(30.0),
            items: vec![Item::HorizontalLayout(LayoutItem {
                name: "horizontalLayout1".to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(50.0),
                height: Mm(10.0),
                items: vec![child],
            })],
        });
        let selection = Selection::top_level(0, 0).push(0).unwrap();

        assert!(matches!(
            item_at_selection(&report, selection),
            Some(Item::Text(_))
        ));
        assert!(report_contains_selection(&report, selection));
    }

    #[test]
    fn selection_reaches_text_inside_two_nested_layouts() {
        let mut report = blank_report();
        report.pages[0].bands.push(Band {
            kind: BandKind::ReportHeader,
            height: Mm(30.0),
            items: vec![Item::VerticalLayout(LayoutItem {
                name: "verticalLayout1".to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(50.0),
                height: Mm(20.0),
                items: vec![Item::HorizontalLayout(LayoutItem {
                    name: "horizontalLayout1".to_string(),
                    x: Mm(0.0),
                    y: Mm(0.0),
                    width: Mm(50.0),
                    height: Mm(10.0),
                    items: vec![new_text_item("DejaVu Sans".to_string())],
                })],
            })],
        });
        let selection = Selection::top_level(0, 0)
            .push(0)
            .and_then(|selection| selection.push(0))
            .unwrap();

        assert!(matches!(
            item_at_selection(&report, selection),
            Some(Item::Text(_))
        ));
        assert!(report_contains_selection(&report, selection));
    }

    #[test]
    fn empty_selection_path_has_no_descendant_and_does_not_panic() {
        assert_eq!(selected_descendant_path(&[], 0), None);
        assert_eq!(selected_descendant_path(&[0], 0), None);
        assert_eq!(selected_descendant_path(&[0, 2], 0), Some(&[2][..]));
        assert_eq!(selected_descendant_path(&[1, 2], 0), None);
    }

    #[test]
    fn vertical_layout_keeps_horizontal_layouts_nested() {
        let selected = vec![new_layout_item(true), new_layout_item(true)];

        let (children, retained_name) = flatten_matching_layouts(selected, false);

        assert_eq!(children.len(), 2);
        assert!(
            children
                .iter()
                .all(|item| matches!(item, Item::HorizontalLayout(_)))
        );
        assert!(retained_name.is_none());
    }

    #[test]
    fn nesting_layout_preserves_inner_divider_proportions() {
        let mut inner = Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(100.0),
            height: Mm(10.0),
            items: vec![
                new_text_item("DejaVu Sans".to_string()),
                new_text_item("DejaVu Sans".to_string()),
            ],
        });
        let Item::HorizontalLayout(layout) = &mut inner else {
            unreachable!();
        };
        set_item_frame(&mut layout.items[0], 0.0, 0.0, 30.0, 10.0);
        set_item_frame(&mut layout.items[1], 30.0, 0.0, 70.0, 10.0);

        arrange_layout_children(std::slice::from_mut(&mut inner), false, 200.0, 20.0);

        let Item::HorizontalLayout(layout) = inner else {
            unreachable!();
        };
        assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 60.0, 20.0));
        assert_eq!(geometry_values(&layout.items[1]), (60.0, 0.0, 140.0, 20.0));
    }

    #[test]
    fn layout_label_is_positioned_above_container() {
        let layout = new_layout_item(true);
        let bounds = Rectangle::new(Point::new(100.0, 80.0), Size::new(200.0, 40.0));

        let label = layout_label_bounds(&layout, bounds).unwrap();

        assert_eq!(label.x, bounds.x);
        assert_eq!(label.y + label.height, bounds.y - 2.0);
        assert!(!label.intersects(&bounds));
    }

    #[test]
    fn vertical_layout_label_is_positioned_right_of_container() {
        let layout = new_layout_item(false);
        let bounds = Rectangle::new(Point::new(100.0, 80.0), Size::new(200.0, 40.0));

        let label = layout_label_bounds(&layout, bounds).unwrap();

        assert_eq!(label.x, bounds.x + bounds.width + 2.0);
        assert_eq!(label.y, bounds.y);
        assert!(!label.intersects(&bounds));
    }

    #[test]
    fn matching_layout_is_flattened_when_adding_another_item() {
        let existing = Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(100.0),
            height: Mm(10.0),
            items: vec![
                new_text_item("DejaVu Sans".to_string()),
                new_text_item("DejaVu Sans".to_string()),
            ],
        });
        let text3 = new_text_item("DejaVu Sans".to_string());

        let (children, retained_name) = flatten_matching_layouts(vec![existing, text3], true);

        assert_eq!(children.len(), 3);
        assert_eq!(retained_name.as_deref(), Some("horizontalLayout1"));
        assert!(children.iter().all(|item| matches!(item, Item::Text(_))));
    }

    #[test]
    fn resizing_horizontal_layout_right_edge_resizes_only_last_child() {
        let mut layout = Item::HorizontalLayout(LayoutItem {
            name: "horizontalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(100.0),
            height: Mm(10.0),
            items: vec![
                new_text_item("DejaVu Sans".to_string()),
                new_text_item("DejaVu Sans".to_string()),
            ],
        });
        reflow_layout(&mut layout);
        resize_item(&mut layout, ResizeHandle::Right, 20.0, 0.0, 200.0, 50.0);

        let Item::HorizontalLayout(layout) = layout else {
            unreachable!();
        };
        assert_eq!(layout.width, Mm(120.0));
        assert_eq!(layout.height, Mm(10.0));
        assert_eq!(geometry_values(&layout.items[0]), (0.0, 0.0, 50.0, 10.0));
        assert_eq!(geometry_values(&layout.items[1]), (50.0, 0.0, 70.0, 10.0));
    }

    #[test]
    fn resizing_vertical_layout_right_edge_resizes_all_child_widths() {
        let mut layout = Item::VerticalLayout(LayoutItem {
            name: "verticalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(50.0),
            height: Mm(30.0),
            items: vec![
                new_text_item("DejaVu Sans".to_string()),
                new_text_item("DejaVu Sans".to_string()),
                new_text_item("DejaVu Sans".to_string()),
            ],
        });
        reflow_layout(&mut layout);

        resize_item(&mut layout, ResizeHandle::Right, 20.0, 0.0, 200.0, 100.0);

        let Item::VerticalLayout(layout) = layout else {
            unreachable!();
        };
        assert_eq!(layout.width, Mm(70.0));
        assert!(
            layout
                .items
                .iter()
                .all(|item| geometry_values(item).2 == 70.0)
        );
    }

    #[test]
    fn resizing_vertical_layout_bottom_expands_last_nested_layout_contents() {
        let horizontal_layout = |name: &str| {
            let mut item = Item::HorizontalLayout(LayoutItem {
                name: name.to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(100.0),
                height: Mm(10.0),
                items: vec![
                    new_text_item("DejaVu Sans".to_string()),
                    new_text_item("DejaVu Sans".to_string()),
                ],
            });
            reflow_layout(&mut item);
            item
        };
        let mut layout = Item::VerticalLayout(LayoutItem {
            name: "verticalLayout1".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(100.0),
            height: Mm(20.0),
            items: vec![
                horizontal_layout("horizontalLayout1"),
                horizontal_layout("horizontalLayout2"),
            ],
        });
        reflow_layout(&mut layout);

        resize_item(&mut layout, ResizeHandle::Bottom, 0.0, 20.0, 150.0, 100.0);

        let Item::VerticalLayout(layout) = layout else {
            unreachable!();
        };
        assert_eq!(layout.height, Mm(40.0));
        let Item::HorizontalLayout(last) = &layout.items[1] else {
            unreachable!();
        };
        assert_eq!(geometry_values(&layout.items[1]), (0.0, 10.0, 100.0, 30.0));
        assert!(
            last.items
                .iter()
                .all(|item| geometry_values(item).3 == 30.0)
        );
    }

    #[test]
    fn generated_text_matches_generated_item_name() {
        let mut item = new_text_item("DejaVu Sans".to_string());
        assign_unique_item_name(&mut item, &[]);

        apply_generated_text(&mut item);

        let Item::Text(text) = item else {
            unreachable!();
        };
        assert_eq!(text.name, "itemText1");
        assert_eq!(text.text, "text1");
    }

    #[test]
    fn generated_text_number_is_global_across_report_bands() {
        let mut report = blank_report();
        let mut first = new_text_item("DejaVu Sans".to_string());
        assign_unique_item_name_in_report(&mut first, &report);
        apply_generated_text(&mut first);
        report.pages[0].bands.push(Band {
            kind: BandKind::ReportHeader,
            height: Mm(20.0),
            items: vec![first],
        });
        report.pages[0].bands.push(Band {
            kind: BandKind::Data {
                source: "data".to_string(),
            },
            height: Mm(20.0),
            items: Vec::new(),
        });
        let mut second = new_text_item("DejaVu Sans".to_string());

        assign_unique_item_name_in_report(&mut second, &report);
        apply_generated_text(&mut second);

        let Item::Text(second) = second else {
            unreachable!();
        };
        assert_eq!(second.name, "itemText2");
        assert_eq!(second.text, "text2");
    }
}
