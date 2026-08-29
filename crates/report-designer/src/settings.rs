use report_core::model::{Item, Margins, Mm, Orientation, Page};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarginField {
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct DesignerSettings {
    pub orientation: Orientation,
    pub left: String,
    pub top: String,
    pub right: String,
    pub bottom: String,
    pub font_family: String,
}

impl DesignerSettings {
    pub fn from_page(page: &Page, font_family: String) -> Self {
        Self {
            orientation: page.orientation,
            left: format_mm(page.margins.left.0),
            top: format_mm(page.margins.top.0),
            right: format_mm(page.margins.right.0),
            bottom: format_mm(page.margins.bottom.0),
            font_family,
        }
    }

    pub fn margin(&self, field: MarginField) -> &str {
        match field {
            MarginField::Left => &self.left,
            MarginField::Top => &self.top,
            MarginField::Right => &self.right,
            MarginField::Bottom => &self.bottom,
        }
    }

    pub fn set_margin(&mut self, field: MarginField, value: String) {
        match field {
            MarginField::Left => self.left = value,
            MarginField::Top => self.top = value,
            MarginField::Right => self.right = value,
            MarginField::Bottom => self.bottom = value,
        }
    }

    pub fn step_margin(&mut self, field: MarginField, delta: f32) {
        let value = self
            .margin(field)
            .parse::<f32>()
            .unwrap_or_default()
            .max(0.0);
        self.set_margin(field, format_mm((value + delta).max(0.0)));
    }

    pub fn apply(&self, page: &mut Page) -> Result<(), String> {
        let margins = Margins {
            left: Mm(parse_margin("Left", &self.left)?),
            top: Mm(parse_margin("Top", &self.top)?),
            right: Mm(parse_margin("Right", &self.right)?),
            bottom: Mm(parse_margin("Bottom", &self.bottom)?),
        };
        let (width, height) = page.size.oriented_dimensions(self.orientation);
        if margins.left.0 + margins.right.0 >= width.0 {
            return Err("Left and right margins must leave printable page width".to_string());
        }
        if margins.top.0 + margins.bottom.0 >= height.0 {
            return Err("Top and bottom margins must leave printable page height".to_string());
        }
        if self.font_family.trim().is_empty() {
            return Err("Font family cannot be empty".to_string());
        }

        page.orientation = self.orientation;
        page.margins = margins;
        for band in &mut page.bands {
            for item in &mut band.items {
                apply_font_family(item, &self.font_family);
            }
        }
        Ok(())
    }
}

pub fn page_font_family(page: &Page) -> String {
    page.bands
        .iter()
        .flat_map(|band| &band.items)
        .find_map(find_font_family)
        .unwrap_or_else(report_core::model::default_font_family)
}

fn apply_font_family(item: &mut Item, font_family: &str) {
    match item {
        Item::Text(text) => text.font_family = font_family.to_string(),
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
            for child in &mut layout.items {
                apply_font_family(child, font_family);
            }
        }
        _ => {}
    }
}

fn find_font_family(item: &Item) -> Option<String> {
    match item {
        Item::Text(text) => Some(text.font_family.clone()),
        Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
            layout.items.iter().find_map(find_font_family)
        }
        _ => None,
    }
}

fn parse_margin(label: &str, value: &str) -> Result<f32, String> {
    let value = value
        .parse::<f32>()
        .map_err(|_| format!("{label} margin must be a number"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{label} margin must be zero or greater"));
    }
    Ok(value)
}

fn format_mm(value: f32) -> String {
    format!("{value:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use report_core::model::{Band, BandKind, PageSize};

    fn page() -> Page {
        Page {
            size: PageSize::A4,
            orientation: Orientation::Portrait,
            margins: Margins {
                left: Mm(10.0),
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
            },
            bands: vec![Band {
                kind: BandKind::PageHeader,
                height: Mm(20.0),
                items: Vec::new(),
            }],
        }
    }

    #[test]
    fn applies_orientation_and_margins() {
        let mut page = page();
        let mut settings = DesignerSettings::from_page(&page, "DejaVu Sans".to_string());
        settings.orientation = Orientation::Landscape;
        settings.left = "12.50".to_string();

        settings.apply(&mut page).unwrap();

        assert!(matches!(page.orientation, Orientation::Landscape));
        assert_eq!(page.margins.left, Mm(12.5));
    }

    #[test]
    fn rejects_margins_without_printable_space() {
        let mut page = page();
        let mut settings = DesignerSettings::from_page(&page, "DejaVu Sans".to_string());
        settings.left = "110".to_string();
        settings.right = "110".to_string();

        assert!(settings.apply(&mut page).is_err());
    }
}
