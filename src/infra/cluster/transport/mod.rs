//! Master/worker control transport.
//!
//! This layer is separate from the application `StateBus`.
//!
//! - `transport/` here is for secure control-plane communication
//! - `io/state/` is for project/runtime state propagation

pub mod interface;
pub mod message;
pub mod stream;

pub use interface::{ControlTransport, ControlTransportError, DynControlTransport};
pub use message::{ControlRequest, ControlResponse};
pub use stream::InProcessControlTransport;
