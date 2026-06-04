# ternary-protocol

Wire protocol for communication between ternary agents — message passing, serialization, and synchronization.

Pure Rust, no unsafe, no external dependencies.

## Protocol Specification

### Overview

This crate defines the wire protocol used by ternary agents to communicate. All data is encoded as sequences of **trits** (ternary digits: -1, 0, +1), serialized in a compact binary format.

### Trit Encoding

A **trit** is a ternary digit with three possible values:

| Value | Name     | Meaning   |
|-------|----------|-----------|
| -1    | `Neg`    | Negative  |
| 0     | `Zero`   | Zero      |
| +1    | `Pos`    | Positive  |

**Wire encoding**: Two trits are packed into a single byte using a 4+4 nibble scheme:
- Upper nibble = `trit_value + 1` (maps -1→0, 0→1, 1→2)
- Lower nibble = `trit_value + 1`

### Payload

A `Payload` is a variable-length sequence of trits.

**Wire format**:
```
[4 bytes: trit count (u32 BE)] [packed trit pairs...]
```

If the trit count is odd, the last byte contains a padding trit (Zero) in the lower nibble.

### TernaryMessage

A structured message between agents.

**Wire format**:
```
[8 bytes: message ID (u64 BE)]
[8 bytes: sender agent ID (u64 BE)]
[8 bytes: receiver agent ID (u64 BE)]  (0 = broadcast)
[8 bytes: timestamp (u64 BE)]
[...payload bytes]
```

### Message Routing (MessageBus)

Three routing modes:

- **Unicast**: Direct message to a specific agent (receiver = agent ID)
- **Broadcast**: Message to all registered agents (receiver = 0)
- **Multicast**: Message to a group of agents (receiver = group ID)

### Sync Protocol

Differential synchronization — only send changes, not full state.

**Diff operations**:

| Op | Code | Fields |
|----|------|--------|
| Insert | 0x00 | position (u64 BE), trit count (u32 BE), packed trits |
| Remove | 0x01 | position (u64 BE), count (u32 BE) |
| Replace | 0x02 | position (u64 BE), trit count (u32 BE), packed trits |

**Wire format for a Diff**:
```
[4 bytes: op count (u32 BE)]
[...ops]
```

### Handshake

Agents perform a handshake before collaborating:

1. **Hello**: Initiator sends agent ID, version range (min/max), and capability list
2. **Ack**: Responder checks version compatibility and negotiates shared capabilities

Capabilities are matched by name; the minimum version of matching capabilities is used.

**Capability encoding in hello payload**: Each byte is encoded as 5 balanced-ternary trits.

### Protocol Version

Semver-style: `(major.minor.patch)`. Compatible when major versions match.

**Wire format**: 12 bytes (three u32 big-endian values).

## Usage

```rust
use ternary_protocol::prelude::*;

// Create a payload
let payload = Payload::from_trits(&[Trit::Pos, Trit::Neg, Trit::Zero]);

// Create a message
let msg = TernaryMessage::new(
    MessageId(1), 100, 200, payload, 1234567890
);

// Serialize
let bytes = msg.to_bytes();
let decoded = TernaryMessage::from_bytes(&bytes).unwrap();

// Message bus
let mut bus = MessageBus::new();
bus.register(100);
bus.register(200);
bus.send(msg, RoutingMode::Unicast).unwrap();

// Sync protocol
let old = Payload::from_trits(&[Trit::Pos, Trit::Pos]);
let new = Payload::from_trits(&[Trit::Pos, Trit::Neg, Trit::Zero]);
let diff = SyncProtocol::diff(&old, &new);
let applied = SyncProtocol::apply(&old, &diff).unwrap();
assert_eq!(applied, new);

// Handshake
let mut alice = Handshake::new(1, vec![Capability::new("search", 2)], (1, 3));
let mut bob = Handshake::new(2, vec![Capability::new("search", 3)], (2, 4));
let hello = alice.create_hello();
let negotiated = bob.respond_to_hello(&hello).unwrap();
```

## License

MIT
