use super::*;

impl DesignerCanvas<'_> {
    pub(super) fn resize_hit_test(&self, position: Point) -> Option<(Selection, ResizeHandle)> {
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

    pub(super) fn band_resize_hit_test(&self, position: Point) -> Option<usize> {
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

    pub(super) fn layout_divider_hit_test(
        &self,
        position: Point,
    ) -> Option<(Selection, usize, bool)> {
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

    pub(super) fn selected_item(&self, selection: Selection) -> Option<(&Item, f32, f32)> {
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

    pub(super) fn hit_test(&self, position: Point) -> Option<Selection> {
        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;
        let mut candidates = Vec::new();

        for (band_index, band) in self.page.bands.iter().enumerate() {
            for (item_index, item) in band.items.iter().enumerate() {
                let parent = Selection::top_level(band_index, item_index);
                if let Some(selection) =
                    hit_test_item(item, content_x, band_y, self.scale, position, parent, false)
                {
                    candidates.push(selection);
                }
            }
            band_y += band.height.0 * self.scale;
        }

        candidates.pop()
    }

    pub(super) fn band_hit_test(&self, position: Point) -> Option<usize> {
        let content_x = PAGE_MARGIN + self.page.margins.left.0 * self.scale;
        let content_width = self.page.printable_width().0 * self.scale;
        let mut band_y = PAGE_MARGIN + self.page.margins.top.0 * self.scale;
        for (index, band) in self.page.bands.iter().enumerate() {
            let band_height = band.height.0 * self.scale;
            let bounds = Rectangle::new(
                Point::new(content_x, band_y),
                Size::new(content_width, band_height),
            );
            let badge_bounds = Rectangle::new(
                Point::new(
                    PAGE_MARGIN - RULER_SIZE - RULER_GAP - BAND_BADGE_WIDTH - 8.0,
                    band_y,
                ),
                Size::new(BAND_BADGE_WIDTH, band_height),
            );
            if bounds.contains(position) || badge_bounds.contains(position) {
                return Some(index);
            }
            band_y += band_height;
        }
        None
    }
}

pub(crate) fn hit_test_item(
    item: &Item,
    offset_x: f32,
    offset_y: f32,
    scale: f32,
    position: Point,
    selection: Selection,
    nested_in_horizontal: bool,
) -> Option<Selection> {
    let rect = item_bounds(item, offset_x, offset_y, scale)?;
    if layout_label_bounds(item, rect, nested_in_horizontal)
        .is_some_and(|label| label.contains(position))
    {
        return Some(selection);
    }
    if let Some(layout) = item_layout(item) {
        let children_are_in_horizontal = matches!(item, Item::HorizontalLayout(_));
        for (index, child) in layout.items.iter().enumerate().rev() {
            let Some(child_selection) = selection.push(index) else {
                continue;
            };
            if let Some(hit) = hit_test_item(
                child,
                rect.x,
                rect.y,
                scale,
                position,
                child_selection,
                children_are_in_horizontal,
            ) {
                return Some(hit);
            }
        }
    }
    rect.contains(position).then_some(selection)
}
