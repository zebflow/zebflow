//! `fs.*` — project file storage (ZebFS): `fs.save`, `fs.list`, `fs.head`,
//! `fs.get`, `fs.put`, `fs.delete`, `fs.copy`, `fs.move`, `fs.mkdir`,
//! `fs.compress`, `fs.decompress`; and by format, `fs.pdf.convert`,
//! `fs.image.thumbnail`, `fs.image.chromakey`, `fs.svg.convert` (`fs/<format>/<verb>.rs`).
//!
//! Every node that stores a file answers it as a bare FileRef — the eleven
//! contract fields and nothing else (`crate::pipeline::nodes::shared::file_ref`).

use crate::pipeline::NodeDefinition;

pub mod compress;
pub mod decompress;
pub mod image;
pub mod object;
pub mod pdf;
pub mod save;
pub mod svg;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        compress::definition(),
        decompress::definition(),
        object::list_definition(),
        object::head_definition(),
        object::get_definition(),
        object::put_definition(),
        object::delete_definition(),
        object::copy_definition(),
        object::move_definition(),
        object::mkdir_definition(),
        pdf::convert::definition(),
        save::definition(),
        image::chromakey::definition(),
        image::thumbnail::definition(),
        svg::convert::definition(),
    ]
}
