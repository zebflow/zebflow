//! `fs.*` — project file storage (ZebFS): `fs.file.put` (`put.rs`), `fs.folder.list`, `fs.file.head`,
//! `fs.file.get`, `fs.file.delete`, `fs.file.copy`, `fs.file.move`, `fs.folder.create`,
//! `fs.archive.create`, `fs.archive.extract`; and by format, `fs.pdf.convert`,
//! `fs.image.thumbnail`, `fs.image.chromakey`, `fs.image.render` (`fs/<format>/<verb>.rs`);
//! and codes a scanner reads, `fs.barcode.qr`, `fs.barcode.code128` (`fs/barcode/<symbology>/`).
//!
//! Every node that stores a file answers it as a bare FileRef — the eleven
//! contract fields and nothing else (`crate::pipeline::nodes::shared::file_ref`).

use crate::pipeline::NodeDefinition;

pub mod barcode;
pub mod compress;
pub mod decompress;
pub mod image;
pub mod object;
pub mod pdf;
pub mod put;
pub mod svg;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        barcode::code128::definition(),
        barcode::qr::definition(),
        compress::definition(),
        decompress::definition(),
        object::list_definition(),
        object::head_definition(),
        object::get_definition(),
        object::delete_definition(),
        object::copy_definition(),
        object::move_definition(),
        object::mkdir_definition(),
        pdf::convert::definition(),
        put::definition(),
        image::chromakey::definition(),
        image::thumbnail::definition(),
        svg::convert::definition(),
    ]
}
