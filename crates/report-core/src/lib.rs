pub mod common;
pub mod datasource;
pub mod expressions;
pub mod font;
pub mod image;
pub mod layout;
pub mod model;

/// Compatibility path for the original flat module layout.
pub mod font_measurer {
    pub use crate::font::measurer::*;
}

/// Compatibility path for the original flat module layout.
pub mod font_resolver {
    pub use crate::font::resolver::*;
}

/// Compatibility path for the original flat module layout.
pub mod image_layout {
    pub use crate::image::layout::*;
}

/// Compatibility path for the original flat module layout.
pub mod image_loader {
    pub use crate::image::loader::*;
}

/// Compatibility path for the original flat module layout.
pub mod text_layout {
    pub use crate::layout::text::*;
}

pub use model::{Band, BandKind, Mm, Page, Report};
