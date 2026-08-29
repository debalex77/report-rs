use super::*;

#[cfg(test)]
pub(crate) fn assign_unique_item_name(item: &mut Item, siblings: &[Item]) {
    let used = siblings
        .iter()
        .map(|item| item_name_storage(item).clone())
        .collect();
    assign_unique_item_name_from_used(item, &used);
}

pub(crate) fn assign_unique_item_name_in_report(item: &mut Item, report: &Report) {
    let used = collect_report_item_names(report);
    assign_unique_item_name_from_used(item, &used);
}

pub(crate) fn assign_unique_item_name_from_used(item: &mut Item, used: &HashSet<String>) {
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

pub(crate) fn apply_generated_text(item: &mut Item) {
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

pub(crate) fn apply_pasted_text(item: &mut Item) {
    match item {
        Item::Text(text) => {
            if let Some(index) = text.name.strip_prefix("itemText")
                && !index.is_empty()
                && index.chars().all(|character| character.is_ascii_digit())
            {
                text.text = format!("text{index}");
            }
        }
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
            for child in &mut layout.items {
                apply_pasted_text(child);
            }
        }
        _ => {}
    }
}

pub(crate) fn ensure_unique_item_names(report: &mut Report) {
    let mut used = HashSet::new();
    let mut counters: HashMap<&'static str, usize> = HashMap::new();
    for page in &mut report.pages {
        for band in &mut page.bands {
            ensure_unique_names_in_items(&mut band.items, &mut used, &mut counters);
        }
    }
}

pub(crate) fn ensure_unique_names_in_items(
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

pub(crate) fn collect_report_item_names(report: &Report) -> HashSet<String> {
    let mut names = HashSet::new();
    for page in &report.pages {
        for band in &page.bands {
            collect_item_names(&band.items, &mut names);
        }
    }
    names
}

pub(crate) fn collect_item_names(items: &[Item], names: &mut HashSet<String>) {
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

pub(crate) fn item_name_prefix(item: &Item) -> &'static str {
    match item {
        Item::Text(_) => "itemText",
        Item::Image(_) => "itemImage",
        Item::Rectangle(_) => "itemShape",
        Item::Line(_) => "itemLine",
        Item::HorizontalLayout(_) => "horizontalLayout",
        Item::VerticalLayout(_) => "verticalLayout",
    }
}

pub(crate) fn item_name_storage(item: &Item) -> &String {
    match item {
        Item::Text(item) => &item.name,
        Item::Line(item) => &item.name,
        Item::Rectangle(item) => &item.name,
        Item::Image(item) => &item.name,
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => &item.name,
    }
}

pub(crate) fn item_name_mut(item: &mut Item) -> &mut String {
    match item {
        Item::Text(item) => &mut item.name,
        Item::Line(item) => &mut item.name,
        Item::Rectangle(item) => &mut item.name,
        Item::Image(item) => &mut item.name,
        Item::HorizontalLayout(item) | Item::VerticalLayout(item) => &mut item.name,
    }
}
