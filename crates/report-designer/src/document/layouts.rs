use super::*;

pub(crate) fn reorder_item_same_parent(
    report: &mut Report,
    source: Selection,
    target: Selection,
) -> Option<Selection> {
    if source.band != target.band
        || source.parent_indices() != target.parent_indices()
        || source == target
    {
        return None;
    }
    let nested = !source.parent_indices().is_empty();
    let items = items_at_parent_mut(report, source)?;
    let source_index = source.item_index();
    let target_index = target.item_index();
    if source_index >= items.len() || target_index >= items.len() {
        return None;
    }
    let slots = nested.then(|| items.iter().map(normalized_geometry).collect::<Vec<_>>());
    let moved = items.remove(source_index);
    let target_index = target_index.min(items.len());
    items.insert(target_index, moved);
    if let Some(slots) = slots {
        for (item, (x, y, width, height)) in items.iter_mut().zip(slots) {
            let (_, _, old_width, old_height) = normalized_geometry(item);
            set_item_frame(item, x, y, width, height);
            scale_layout_contents(item, old_width, old_height, width, height);
        }
    }
    Some(source.with_item_index(target_index))
}

pub(crate) fn reorder_items_same_parent(
    report: &mut Report,
    sources: &[Selection],
    target: Selection,
) -> Option<Vec<Selection>> {
    let first = *sources.first()?;
    if sources.len() < 2
        || sources.iter().any(|source| {
            source.band != target.band || source.parent_indices() != target.parent_indices()
        })
        || sources.contains(&target)
    {
        return None;
    }
    let mut indices = sources
        .iter()
        .map(|source| source.item_index())
        .collect::<Vec<_>>();
    indices.sort_unstable();
    indices.dedup();
    if indices.len() != sources.len() {
        return None;
    }
    let nested = !first.parent_indices().is_empty();
    let items = items_at_parent_mut(report, first)?;
    if target.item_index() >= items.len()
        || indices.last().is_some_and(|index| *index >= items.len())
    {
        return None;
    }
    let slots = nested.then(|| items.iter().map(normalized_geometry).collect::<Vec<_>>());
    let mut moved = Vec::with_capacity(indices.len());
    for &index in indices.iter().rev() {
        moved.push(items.remove(index));
    }
    moved.reverse();
    let insert_at = target.item_index()
        - indices
            .iter()
            .filter(|index| **index < target.item_index())
            .count();
    for (offset, item) in moved.into_iter().enumerate() {
        items.insert(insert_at + offset, item);
    }
    if let Some(slots) = slots {
        for (item, (x, y, width, height)) in items.iter_mut().zip(slots) {
            let (_, _, old_width, old_height) = normalized_geometry(item);
            set_item_frame(item, x, y, width, height);
            scale_layout_contents(item, old_width, old_height, width, height);
        }
    }
    Some(
        (0..indices.len())
            .map(|offset| first.with_item_index(insert_at + offset))
            .collect(),
    )
}

pub(crate) fn move_items_to_band(
    report: &mut Report,
    sources: &[Selection],
    target_band: usize,
) -> Option<Vec<Selection>> {
    let first = *sources.first()?;
    if sources.len() < 2
        || sources.iter().any(|source| {
            source.band != first.band || source.parent_indices() != first.parent_indices()
        })
        || report.pages.first()?.bands.get(target_band).is_none()
    {
        return None;
    }
    let mut indices = sources
        .iter()
        .map(|source| source.item_index())
        .collect::<Vec<_>>();
    indices.sort_unstable();
    indices.dedup();
    let moved = {
        let items = items_at_parent_mut(report, first)?;
        if indices.len() != sources.len()
            || indices.last().is_some_and(|index| *index >= items.len())
        {
            return None;
        }
        let mut moved = Vec::with_capacity(indices.len());
        for &index in indices.iter().rev() {
            moved.push(items.remove(index));
        }
        moved.reverse();
        moved
    };
    let target = &mut report.pages.first_mut()?.bands[target_band].items;
    let insert_at = target.len();
    target.extend(moved);
    grow_band_to_fit_items(report, target_band);
    Some(
        (0..indices.len())
            .map(|offset| Selection::top_level(target_band, insert_at + offset))
            .collect(),
    )
}

pub(crate) fn move_items_into_layout(
    report: &mut Report,
    sources: &[Selection],
    target_layout: Selection,
) -> Option<Vec<Selection>> {
    let first = *sources.first()?;
    if sources.len() < 2
        || sources.iter().any(|source| {
            source.band != first.band
                || source.parent_indices() != first.parent_indices()
                || *source == target_layout
                || source.is_ancestor_of(target_layout)
        })
        || item_at_selection(report, target_layout)
            .and_then(item_layout)
            .is_none()
    {
        return None;
    }
    let mut indices = sources
        .iter()
        .map(|source| source.item_index())
        .collect::<Vec<_>>();
    indices.sort_unstable();
    indices.dedup();
    let mut adjusted_target = target_layout;
    for &index in indices.iter().rev() {
        adjusted_target = adjusted_target.adjusted_after_removal(first.with_item_index(index))?;
    }
    let moved = {
        let items = items_at_parent_mut(report, first)?;
        if indices.len() != sources.len()
            || indices.last().is_some_and(|index| *index >= items.len())
        {
            return None;
        }
        let mut moved = Vec::with_capacity(indices.len());
        for &index in indices.iter().rev() {
            moved.push(items.remove(index));
        }
        moved.reverse();
        moved
    };
    let target = item_at_selection_mut(report, adjusted_target)?;
    let horizontal = matches!(target, Item::HorizontalLayout(_));
    let layout = item_layout_mut(target)?;
    let insert_at = layout.items.len();
    layout.items.extend(moved);
    arrange_layout_children(
        &mut layout.items,
        horizontal,
        layout.width.0,
        layout.height.0,
    );
    Some(
        (0..indices.len())
            .filter_map(|offset| adjusted_target.push(insert_at + offset))
            .collect(),
    )
}

pub(crate) fn move_item_to_band(
    report: &mut Report,
    source: Selection,
    target_band: usize,
) -> Option<Selection> {
    let page = report.pages.first()?;
    if source.band >= page.bands.len() || target_band >= page.bands.len() {
        return None;
    }
    let item = {
        let items = items_at_parent_mut(report, source)?;
        if source.item_index() >= items.len() {
            return None;
        }
        items.remove(source.item_index())
    };
    let target_items = &mut report.pages.first_mut()?.bands[target_band].items;
    let target_index = target_items.len();
    target_items.push(item);
    grow_band_to_fit_items(report, target_band);
    Some(Selection::top_level(target_band, target_index))
}

pub(crate) fn move_item_into_layout(
    report: &mut Report,
    source: Selection,
    target_layout: Selection,
) -> Option<Selection> {
    if source == target_layout || source.is_ancestor_of(target_layout) {
        return None;
    }
    if item_at_selection(report, target_layout)
        .and_then(item_layout)
        .is_none()
    {
        return None;
    }
    let adjusted_target = target_layout.adjusted_after_removal(source)?;
    let item = {
        let items = items_at_parent_mut(report, source)?;
        if source.item_index() >= items.len() {
            return None;
        }
        items.remove(source.item_index())
    };
    let target = item_at_selection_mut(report, adjusted_target)?;
    let horizontal = matches!(target, Item::HorizontalLayout(_));
    let layout = item_layout_mut(target)?;
    let index = layout.items.len();
    layout.items.push(item);
    arrange_layout_children(
        &mut layout.items,
        horizontal,
        layout.width.0,
        layout.height.0,
    );
    adjusted_target.push(index)
}

pub(crate) fn move_item_before(
    report: &mut Report,
    source: Selection,
    target: Selection,
) -> Option<Selection> {
    if source == target || source.is_ancestor_of(target) {
        return None;
    }
    let adjusted_target = target.adjusted_after_removal(source)?;
    let item = {
        let items = items_at_parent_mut(report, source)?;
        if source.item_index() >= items.len() {
            return None;
        }
        items.remove(source.item_index())
    };
    let index = adjusted_target.item_index();
    if let Some(parent) = adjusted_target.parent() {
        let parent_item = item_at_selection_mut(report, parent)?;
        let horizontal = matches!(parent_item, Item::HorizontalLayout(_));
        let layout = item_layout_mut(parent_item)?;
        if index > layout.items.len() {
            return None;
        }
        layout.items.insert(index, item);
        arrange_layout_children(
            &mut layout.items,
            horizontal,
            layout.width.0,
            layout.height.0,
        );
    } else {
        let items = &mut report.pages.first_mut()?.bands[adjusted_target.band].items;
        if index > items.len() {
            return None;
        }
        items.insert(index, item);
        grow_band_to_fit_items(report, adjusted_target.band);
    }
    Some(adjusted_target)
}

pub(crate) fn grow_band_to_fit_items(report: &mut Report, band_index: usize) -> bool {
    let Some(band) = report
        .pages
        .first_mut()
        .and_then(|page| page.bands.get_mut(band_index))
    else {
        return false;
    };
    let required_height = band
        .items
        .iter()
        .map(|item| {
            let (_, y, _, height) = normalized_geometry(item);
            y + height
        })
        .fold(0.0_f32, f32::max);
    if required_height > band.height.0 {
        band.height = Mm(required_height);
        true
    } else {
        false
    }
}

pub(crate) fn dismantle_layout(
    report: &mut Report,
    selection: Selection,
) -> Option<Vec<Selection>> {
    let items = items_at_parent_mut(report, selection)?;
    let index = selection.item_index();
    let item = items.get(index)?;
    let (layout_x, layout_y, children) = match item {
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
            (layout.x.0, layout.y.0, layout.items.clone())
        }
        _ => return None,
    };
    items.remove(index);
    let child_count = children.len();
    for (offset, mut child) in children.into_iter().enumerate() {
        translate_item(&mut child, layout_x, layout_y);
        items.insert(index + offset, child);
    }
    Some(
        (0..child_count)
            .map(|offset| selection.with_item_index(index + offset))
            .collect(),
    )
}

fn translate_item(item: &mut Item, dx: f32, dy: f32) {
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

fn items_at_parent_mut(report: &mut Report, selection: Selection) -> Option<&mut Vec<Item>> {
    let mut items = &mut report
        .pages
        .first_mut()?
        .bands
        .get_mut(selection.band)?
        .items;
    for &index in selection.parent_indices() {
        items = &mut item_layout_mut(items.get_mut(index)?)?.items;
    }
    Some(items)
}

pub(crate) fn equalize_layout_children(item: &mut Item) -> bool {
    let (layout, horizontal) = match item {
        Item::HorizontalLayout(layout) => (layout, true),
        Item::VerticalLayout(layout) => (layout, false),
        _ => return false,
    };
    if layout.items.is_empty() {
        return false;
    }
    arrange_layout_children(
        &mut layout.items,
        horizontal,
        layout.width.0,
        layout.height.0,
    );
    true
}

pub(crate) fn arrange_layout_children(
    items: &mut [Item],
    horizontal: bool,
    width: f32,
    height: f32,
) {
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

pub(crate) fn scale_layout_contents(
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

pub(crate) fn flatten_matching_layouts(
    selected: Vec<Item>,
    horizontal: bool,
) -> (Vec<Item>, Option<String>) {
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

pub(crate) fn resize_layout_divider(
    item: &mut Item,
    divider: usize,
    horizontal: bool,
    delta: f32,
) -> bool {
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

pub(crate) fn set_item_frame(item: &mut Item, x: f32, y: f32, width: f32, height: f32) {
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
