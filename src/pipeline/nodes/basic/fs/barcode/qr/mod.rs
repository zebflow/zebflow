//! QR Code (ISO/IEC 18004): a 2D code for any text (a URL, a certificate
//! number). The encoder is ours: `encode` builds the symbol, `reed_solomon`
//! its error correction, `penalty` picks the mask, `tables` holds the
//! standard's capacities. `fs.barcode.render --symbology qr` draws it.

pub mod encode;
mod penalty;
pub mod reed_solomon;
pub mod tables;

#[cfg(test)]
mod tests;
