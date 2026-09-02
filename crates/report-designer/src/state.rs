const MAX_SELECTION_DEPTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Selection {
    pub(crate) band: usize,
    path: [usize; MAX_SELECTION_DEPTH],
    depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarTab {
    Properties,
    Structure,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructureDropTarget {
    Band(usize),
    Item(Selection),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PendingLayoutMove {
    pub(crate) source: Selection,
    pub(crate) layout: Selection,
    pub(crate) target: StructureDropTarget,
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

    pub(crate) fn parent_indices(&self) -> &[usize] {
        &self.path[..self.depth.saturating_sub(1)]
    }

    pub(crate) fn parent(mut self) -> Option<Self> {
        if self.depth <= 1 {
            return None;
        }
        self.depth -= 1;
        Some(self)
    }

    pub(crate) const fn item_index(self) -> usize {
        self.path[self.depth - 1]
    }

    pub(crate) fn with_item_index(mut self, index: usize) -> Self {
        self.path[self.depth - 1] = index;
        self
    }

    pub(crate) fn is_ancestor_of(self, other: Self) -> bool {
        self.band == other.band
            && self.depth < other.depth
            && other.indices().starts_with(self.indices())
    }

    pub(crate) fn adjusted_after_removal(mut self, removed: Self) -> Option<Self> {
        if self.band != removed.band {
            return Some(self);
        }
        let parent_depth = removed.depth - 1;
        if self.depth <= parent_depth
            || self.indices()[..parent_depth] != removed.indices()[..parent_depth]
        {
            return Some(self);
        }
        let index = self.path[parent_depth];
        let removed_index = removed.item_index();
        if index == removed_index {
            return None;
        }
        if index > removed_index {
            self.path[parent_depth] -= 1;
        }
        Some(self)
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
    Band,
    Geometry,
    TextValue,
    ValueFormat,
    Font,
    TextColor,
    Alignment,
    Appearance,
    Shape,
    Layout,
    Image,
}

pub(crate) struct CollapsedGroups {
    general: bool,
    band: bool,
    geometry: bool,
    text_value: bool,
    value_format: bool,
    font: bool,
    text_color: bool,
    alignment: bool,
    appearance: bool,
    shape: bool,
    layout: bool,
    image: bool,
}

impl Default for CollapsedGroups {
    fn default() -> Self {
        Self {
            general: false,
            band: false,
            geometry: false,
            text_value: false,
            value_format: true,
            font: true,
            text_color: true,
            alignment: true,
            appearance: true,
            shape: false,
            layout: false,
            image: false,
        }
    }
}

impl CollapsedGroups {
    pub(crate) fn toggle(&mut self, group: PropertyGroup) {
        let value = match group {
            PropertyGroup::General => &mut self.general,
            PropertyGroup::Band => &mut self.band,
            PropertyGroup::Geometry => &mut self.geometry,
            PropertyGroup::TextValue => &mut self.text_value,
            PropertyGroup::ValueFormat => &mut self.value_format,
            PropertyGroup::Font => &mut self.font,
            PropertyGroup::TextColor => &mut self.text_color,
            PropertyGroup::Alignment => &mut self.alignment,
            PropertyGroup::Appearance => &mut self.appearance,
            PropertyGroup::Shape => &mut self.shape,
            PropertyGroup::Layout => &mut self.layout,
            PropertyGroup::Image => &mut self.image,
        };
        *value = !*value;
    }

    pub(crate) fn is_collapsed(&self, group: PropertyGroup) -> bool {
        match group {
            PropertyGroup::General => self.general,
            PropertyGroup::Band => self.band,
            PropertyGroup::Geometry => self.geometry,
            PropertyGroup::TextValue => self.text_value,
            PropertyGroup::ValueFormat => self.value_format,
            PropertyGroup::Font => self.font,
            PropertyGroup::TextColor => self.text_color,
            PropertyGroup::Alignment => self.alignment,
            PropertyGroup::Appearance => self.appearance,
            PropertyGroup::Shape => self.shape,
            PropertyGroup::Layout => self.layout,
            PropertyGroup::Image => self.image,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PaddingField {
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BorderSide {
    Left,
    Top,
    Right,
    Bottom,
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
    DataHeader,
    DataBand,
    ReportFooter,
    Text,
    Image,
    Shape,
    HorizontalLayout,
    VerticalLayout,
    Delete,
}
