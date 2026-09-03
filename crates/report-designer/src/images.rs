use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use iced::widget::image::Handle;
use report_core::image::loader::load_image;
use report_core::model::{ImageSourceType, Item, Report};

#[derive(Clone)]
pub(crate) struct DesignerImage {
    pub(crate) handle: Handle,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

pub(crate) fn load_designer_images(
    report: &Report,
    report_dir: PathBuf,
) -> HashMap<String, DesignerImage> {
    let mut sources = HashSet::new();
    for page in &report.pages {
        for band in &page.bands {
            collect_image_sources(&band.items, &mut sources);
        }
    }
    sources
        .into_iter()
        .filter_map(|source| {
            let path = PathBuf::from(&source);
            let resolved = if path.is_absolute() {
                path
            } else {
                report_dir.join(path)
            };
            load_image(&resolved).ok().map(|image| {
                (
                    source,
                    DesignerImage {
                        handle: Handle::from_rgba(image.width, image.height, image.rgba),
                        width: image.width,
                        height: image.height,
                    },
                )
            })
        })
        .collect()
}

fn collect_image_sources(items: &[Item], sources: &mut HashSet<String>) {
    for item in items {
        match item {
            Item::Image(image)
                if image.source_type == ImageSourceType::File && !image.source.is_empty() =>
            {
                sources.insert(image.source.clone());
            }
            Item::HorizontalLayout(layout) | Item::VerticalLayout(layout) => {
                collect_image_sources(&layout.items, sources);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use report_core::model::{ImageFit, ImageItem, ImageSourceType, LayoutItem, Mm, QuerySource};

    #[test]
    fn collects_sources_from_nested_layouts() {
        let items = vec![Item::HorizontalLayout(LayoutItem {
            name: "layout".to_string(),
            x: Mm(0.0),
            y: Mm(0.0),
            width: Mm(10.0),
            height: Mm(10.0),
            items: vec![Item::Image(ImageItem {
                name: "image".to_string(),
                x: Mm(0.0),
                y: Mm(0.0),
                width: Mm(10.0),
                height: Mm(10.0),
                source: "logo.png".to_string(),
                source_type: ImageSourceType::File,
                query_source: QuerySource::Main,
                field: None,
                fit: ImageFit::Contain,
            })],
        })];
        let mut sources = HashSet::new();

        collect_image_sources(&items, &mut sources);

        assert!(sources.contains("logo.png"));
    }
}
