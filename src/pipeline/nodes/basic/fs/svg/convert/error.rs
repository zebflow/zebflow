//! Why an SVG was not converted. `kind` picks the node's error code.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertError {
    pub kind: ConvertErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertErrorKind {
    /// The SVG itself: it does not parse, names a picture outside the
    /// project, or a picture is over its cap — the author's to fix.
    Source,
    /// A font family the SVG names is neither the bundled default nor a
    /// face the project has.
    Font,
    /// resvg or the encoder failed.
    Raster,
}

impl ConvertError {
    pub fn source(message: impl Into<String>) -> Self {
        Self { kind: ConvertErrorKind::Source, message: message.into() }
    }
    pub fn font(message: impl Into<String>) -> Self {
        Self { kind: ConvertErrorKind::Font, message: message.into() }
    }
    pub fn raster(message: impl Into<String>) -> Self {
        Self { kind: ConvertErrorKind::Raster, message: message.into() }
    }
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
