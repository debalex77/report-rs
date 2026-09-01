use crate::font::FontSpec;
use cosmic_text::fontdb::{Database, Family, Query, Stretch, Style, Weight};
use std::collections::BTreeSet;

/// Physical font selected for a FontSpec.
#[derive(Debug, Clone)]
pub struct ResolvedFont {
    pub data: Vec<u8>,
    pub face_index: u32,
}

/// Resolves logical report fonts to fonts installed in the system.
pub struct SystemFontResolver {
    db: Database,
}

impl SystemFontResolver {
    /// Creates a resolver and loads fonts installed in the operating system.
    pub fn new() -> Self {
        let mut db = Database::new();
        db.load_system_fonts();

        Self { db }
    }

    /// Resolves a FontSpec to the matching physical font data.
    pub fn resolve(&self, font: &FontSpec) -> Option<ResolvedFont> {
        let families = [Family::Name(font.family.as_str())];

        let weight = if font.bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        };

        let style = if font.italic {
            Style::Italic
        } else {
            Style::Normal
        };

        let id = self.db.query(&Query {
            families: &families,
            weight,
            stretch: Stretch::Normal,
            style,
        })?;

        let mut resolved = None;

        self.db.with_face_data(id, |data, face_index| {
            resolved = Some(ResolvedFont {
                data: data.to_vec(),
                face_index,
            });
        });

        resolved
    }

    /// Returns the unique font family names installed in the operating system.
    pub fn families(&self) -> Vec<String> {
        self.db
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

impl Default for SystemFontResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_dejavu_sans() {
        let resolver = SystemFontResolver::new();

        let font = FontSpec {
            family: "DejaVu Sans".to_string(),
            size: 12.0,
            bold: false,
            italic: false,
        };

        let resolved = resolver
            .resolve(&font)
            .expect("DejaVu Sans should be installed");

        assert!(!resolved.data.is_empty());
    }

    #[test]
    fn resolve_font_variants() {
        let resolver = SystemFontResolver::new();

        for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
            let font = FontSpec {
                family: "DejaVu Sans".to_string(),
                size: 12.0,
                bold,
                italic,
            };

            let resolved = resolver
                .resolve(&font)
                .expect("Font variant should be resolved");

            assert!(!resolved.data.is_empty());
        }
    }

    #[test]
    fn list_system_font_families() {
        let families = SystemFontResolver::new().families();

        assert!(families.iter().any(|family| family == "DejaVu Sans"));
    }
}
