# ternary-protocol

**Wire protocol for communication between ternary agents — serialization, routing, synchronization, and capability negotiation.**

## Background

Every distributed system needs a wire protocol — a shared language for encoding, transporting, and decoding messages between nodes. Protocols like HTTP, gRPC (Protocol Buffers), and MQTT define how data is serialized, how messages are routed, and how connections are established. The SuperInstance ecosystem uses **balanced ternary** (−1, 0, +1) as its fundamental data representation, requiring a purpose-built protocol that operates on trits rather than bits.

`ternary-protocol` defines the complete wire format for inter-agent communication: trit encoding, ternary payloads, structured messages, message bus routing, state synchronization via diffs, capability-based handshakes, and version negotiation. Pure Rust, no unsafe code, no external dependencies.

## How It Works

### Trit Encoding (`trit.rs`)

The atomic unit is a `Trit`: `Neg` (−1), `Zero` (0), or `Pos` (+1). Two trits pack into a single byte using nibble encoding — upper nibble for the first trit, lower nibble for the second. This gives a 50% density improvement over naive one-trit-per-byte encoding, though it remains less dense than binary (log₂3 ≈ 1.585 bits per trit).

### Payload Serialization (`payload.rs`)

`Payload` is a sequence of trits with compact binary serialization:

```
[4 bytes: trit count (big-endian)] [packed trit pairs]
```

Odd-length payloads pad the final byte with `Zero`. This format is self-delimiting — the length prefix enables parsing without external framing.

### Messages (`message.rs`)

`TernaryMessage` is a structured envelope:

```
[8B id] [8B sender] [8B receiver] [8B timestamp] [payload bytes]
```

Receiver `0` denotes broadcast. Messages are `Eq + Hash` for deduplication and `Clone` for fan-out. `MessageId` provides a monotonic generator for sequential IDs.

### Message Bus (`bus.rs`)

`MessageBus` is an in-process routing layer with three modes:

- **Unicast** — deliver to a specific agent
- **Multicast** — deliver to a group (topic-like)
- **Broadcast** — deliver to all registered agents

Agents register with `register(id)`, join multicast groups with `join_group(group, agent)`, and receive messages via per-agent inboxes. Errors (`AgentNotFound`, `NoSubscribers`, `QueueFull`) surface routing failures.

### State Synchronization (`sync.rs`)

`SyncProtocol` computes diffs between `Payload` instances, producing `DiffOp` sequences:

- **Insert** — add trits at a position
- **Remove** — delete trits at a position
- **Replace** — overwrite trits at a position

Diffs serialize to a compact binary format and can be applied to reconstruct the target payload. This enables incremental state sync — similar to rsync's delta-transfer algorithm but for ternary data.

### Handshake (`handshake.rs`)

Before communicating, agents perform a capability handshake:

1. **Hello** — exchange agent IDs, capabilities (name + version), and protocol version ranges
2. **Negotiation** — intersect capabilities, select the highest compatible protocol version
3. **Completion** — both agents have a shared set of negotiated capabilities

Handshake states: `Idle → HelloSent → Completed` (or `Rejected`). Errors include `VersionMismatch` and `NoCommonCapabilities`.

### Version Negotiation (`version.rs`)

`ProtocolVersion` follows semver: `major.minor.patch`. Compatibility is defined as same major version. The `negotiate()` method selects the highest compatible version from a set of peer versions.

## Experimental Results

Each module has dedicated tests validating:

- **Trit encoding** — `from_i8` roundtrips, invalid values rejected, pack/unpack symmetry
- **Payload serialization** — empty, single-trit, multi-trit, odd-length padding
- **Message wire format** — unicast and broadcast construction, byte serialization
- **Bus routing** — unicast delivery, multicast fan-out, broadcast, error cases
- **Diff computation** — insert, remove, replace operations, apply correctness
- **Handshake** — successful negotiation, version mismatch rejection, no-common-capabilities
- **Version** — compatibility checks, negotiation across multiple peer versions

## Impact

`ternary-protocol` is the lingua franca of the SuperInstance ecosystem. Every other crate either produces or consumes protocol messages. The design choices — ternary-native encoding, capability negotiation, diff-based sync — reflect the ecosystem's core thesis that balanced ternary enables richer semantics than binary.

The diff-based synchronization is particularly relevant for fleet management: when rooms diverge, only the delta needs to be transmitted, reducing bandwidth usage and enabling fast convergence. This mirrors how version control systems (Git, Mercurial) synchronize state, but adapted for ternary data structures.

## Use Cases

1. **Inter-node fleet communication** — Rooms exchange `TernaryMessage` instances over TCP, using the wire format for serialization and the bus for routing. Handshakes ensure capability compatibility before data exchange.

2. **State replication** — Rooms synchronize shared state using `SyncProtocol` diffs. When a room reconnects after a network partition, it requests a diff from the current state rather than a full snapshot, minimizing transfer size.

3. **Version-aware agent networks** — Agents running different protocol versions negotiate the highest compatible version during handshake, enabling rolling upgrades without fleet-wide downtime.

4. **Multicast event distribution** — The message bus's multicast groups enable topic-based event routing: agents subscribe to relevant groups and receive only matching messages, similar to MQTT topics.

5. **Ternary data interchange** — Systems that natively operate on trits (ternary processors, ternary neural networks) can use the payload format directly without binary ↔ ternary conversion overhead.

## Open Questions

- **Encryption and authentication:** The protocol currently has no security layer. Should TLS or a custom ternary encryption scheme be integrated, or should security be handled at a different layer (e.g., WireGuard tunnels)?
- **Streaming and backpressure:** The current message bus is push-based with bounded queues. For high-throughput scenarios, should the protocol support reactive-streams-style backpressure signaling?
- **Compression:** Trit packing achieves ~1.585 bits per trit. Could domain-specific compression (e.g., run-length encoding for sparse ternary data) further reduce wire size?

## Connection to Oxide Stack

`ternary-protocol` is the universal connector:

- **`ternary-channel`** — channels transport protocol messages
- **`ternary-event`** — events are serialized as protocol payloads for cross-node delivery
- **`ternary-command`** — commands are encoded as protocol messages for remote dispatch
- **`ternary-blockchain`** — blocks and transactions use ternary hashing and Merkle trees
- **`ternary-zkp`** — proof transcripts could be serialized as protocol payloads

The protocol version (`0.1.0`) and capability negotiation ensure forward compatibility as the ecosystem evolves, enabling heterogeneous fleets where not all agents run the same software version.
