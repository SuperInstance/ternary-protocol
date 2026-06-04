//! # ternary-protocol
//!
//! Wire protocol for communication between ternary agents — message passing,
//! serialization, and synchronization. Pure Rust, no unsafe, no external deps.

mod trit;
mod payload;
mod message;
mod bus;
mod sync;
mod handshake;
mod version;

pub use trit::Trit;
pub use payload::Payload;
pub use message::{TernaryMessage, MessageId};
pub use bus::{MessageBus, RoutingMode, BusError};
pub use sync::{SyncProtocol, Diff, DiffOp, SyncError};
pub use handshake::{Handshake, Capability, HandshakeError, HandshakeState};
pub use version::{ProtocolVersion, VersionError};

/// Re-export of core protocol types for convenience.
pub mod prelude {
    pub use crate::trit::Trit;
    pub use crate::payload::Payload;
    pub use crate::message::{TernaryMessage, MessageId};
    pub use crate::bus::{MessageBus, RoutingMode};
    pub use crate::sync::{SyncProtocol, Diff};
    pub use crate::handshake::{Handshake, Capability, HandshakeState};
    pub use crate::version::ProtocolVersion;
}
