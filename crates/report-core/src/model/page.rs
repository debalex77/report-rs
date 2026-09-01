use super::*;

/// Physical paper size used by a report page.
///
/// Built-in and custom dimensions are stored in millimeters and describe the
/// portrait orientation; landscape orientation swaps the two axes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PageSize {
    A4,
    A5,
    Letter,
    Custom { width: Mm, height: Mm },
}

impl PageSize {
    /// Returns the base width and height before orientation is applied.
    pub fn dimensions(&self) -> (Mm, Mm) {
        match self {
            PageSize::A4 => (Mm(210.0), Mm(297.0)),
            PageSize::A5 => (Mm(148.0), Mm(210.0)),
            PageSize::Letter => (Mm(215.9), Mm(279.4)),
            PageSize::Custom { width, height } => (*width, *height),
        }
    }

    /// Returns dimensions with portrait or landscape orientation applied.
    pub fn oriented_dimensions(&self, orientation: Orientation) -> (Mm, Mm) {
        let (width, height) = self.dimensions();

        match orientation {
            Orientation::Portrait => (width, height),
            Orientation::Landscape => (height, width),
        }
    }
}

/// Orientation applied to the base dimensions of a [`PageSize`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Non-printable spacing around the page content, in millimeters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Margins {
    pub left: Mm,
    pub top: Mm,
    pub right: Mm,
    pub bottom: Mm,
}

/// Declarative definition of one logical report page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub size: PageSize,
    pub orientation: Orientation,
    pub margins: Margins,
    pub bands: Vec<Band>,
}

impl Page {
    /// Returns the oriented physical page dimensions.
    pub fn dimensions(&self) -> (Mm, Mm) {
        self.size.oriented_dimensions(self.orientation)
    }

    /// Returns the oriented physical page width.
    pub fn width(&self) -> Mm {
        let (width, _) = self.dimensions();
        width
    }

    /// Returns the oriented physical page height.
    pub fn height(&self) -> Mm {
        let (_, height) = self.dimensions();
        height
    }

    /// Returns the width remaining after left and right margins are removed.
    pub fn printable_width(&self) -> Mm {
        self.width() - self.margins.left - self.margins.right
    }

    /// Returns the height remaining after top and bottom margins are removed.
    pub fn printable_height(&self) -> Mm {
        self.height() - self.margins.top - self.margins.bottom
    }
}
