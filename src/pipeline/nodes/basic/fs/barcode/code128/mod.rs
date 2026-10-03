//! Code 128 (ISO/IEC 15417): a linear barcode for printable ASCII text (a
//! ticket, a certificate or order number). The encoder is ours (`encode`);
//! `fs.barcode.render --symbology code128` draws it.

pub mod encode;
