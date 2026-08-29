const MAX_SELECTION_DEPTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Selection {
    pub(crate) band: usize,
    path: [usize; MAX_SELECTION_DEPTH],
    depth: usize,
}

impl Selection {
    pub(crate) const fn top_level(band: usize, item: usize) -> Self {
        let mut path = [0; MAX_SELECTION_DEPTH];
        path[0] = item;
        Self {
            band,
            path,
            depth: 1,
        }
    }

    pub(crate) const fn top_index(self) -> usize {
        self.path[0]
    }

    pub(crate) const fn is_top_level(self) -> bool {
        self.depth == 1
    }

    pub(crate) fn indices(&self) -> &[usize] {
        &self.path[..self.depth]
    }

    pub(crate) fn descendants(&self) -> &[usize] {
        &self.path[1..self.depth]
    }

    pub(crate) fn push(mut self, index: usize) -> Option<Self> {
        if self.depth >= MAX_SELECTION_DEPTH {
            return None;
        }
        self.path[self.depth] = index;
        self.depth += 1;
        Some(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PropertyGroup {
    General,
    Geometry,
    TextValue,
    Font,
    TextColor,
    Alignment,
    Image,
}

pub(crate) struct CollapsedGroups {
    general: bool,
    geometry: bool,
    text_value: bool,
    font: bool,
    text_color: bool,
    alignment: bool,
    image: bool,
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
            image: false,
        }
    }
}

impl CollapsedGroups {
    pub(crate) fn toggle(&mut self, group: PropertyGroup) {
        let value = match group {
            PropertyGroup::General => &mut self.general,
            PropertyGroup::Geometry => &mut self.geometry,
            PropertyGroup::TextValue => &mut self.text_value,
            PropertyGroup::Font => &mut self.font,
            PropertyGroup::TextColor => &mut self.text_color,
            PropertyGroup::Alignment => &mut self.alignment,
            PropertyGroup::Image => &mut self.image,
        };
        *value = !*value;
    }

    pub(crate) fn is_collapsed(&self, group: PropertyGroup) -> bool {
        match group {
            PropertyGroup::General => self.general,
            PropertyGroup::Geometry => self.geometry,
            PropertyGroup::TextValue => self.text_value,
            PropertyGroup::Font => self.font,
            PropertyGroup::TextColor => self.text_color,
            PropertyGroup::Alignment => self.alignment,
            PropertyGroup::Image => self.image,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeometryField {
    X,
    Y,
    Width,
    Height,
    X1,
    Y1,
    X2,
    Y2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResizeHandle {
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
pub(crate) enum DragOperation {
    Move(Selection),
    Resize(Selection, ResizeHandle),
    ResizeBand(usize),
    ResizeLayoutDivider(Selection, usize, bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppMenu {
    File,
    Edit,
    Info,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DesignerTool {
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
