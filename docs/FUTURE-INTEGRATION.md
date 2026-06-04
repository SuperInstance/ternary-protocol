# Future Integration: ternary-protocol

## Current State

ternary-protocol defines the wire format for ternary agent communication. It provides `Trit` (base unit), `Payload` (message content), `TernaryMessage` with `MessageId`, `MessageBus` with `RoutingMode` (unicast/broadcast/multicast), `SyncProtocol` with `Diff` and `DiffOp` for state synchronization, `Handshake` with `Capability` negotiation and `HandshakeState`, and `ProtocolVersion` for version compatibility. Pure Rust, no unsafe, no external dependencies.

## Integration Opportunities

### I2I Bottle Protocol Bridge (Primary Integration)

The ROOM-AS-CODESPACE-ARCHITECTURE.md proposes layering I2I semantic messages on top of ternary-protocol binary signaling:

- **I2I TELL** → `TernaryMessage` with `Signal` (+1, promoting information)
- **I2I ASK** → `TernaryMessage` with `Signal` (+1, requesting response)
- **I2I ALERT/WARN** → `TernaryMessage` with `Suppress` (-1, escalation)
- **I2I HEARTBEAT** → `TernaryMessage` with `Silence` (0, still alive)
- **I2I COMPLETE** → `TernaryMessage` with `Signal` (+1, task done with artifacts)

The mapping: `MessageBus` handles real-time signaling (microseconds) between agents in the same Codespace or local network. When agents are across organizations or sleeping, I2I over git commits provides async delivery. The agent shouldn't know which transport is used — `MessageBus::send()` abstracts the transport layer.

### SyncProtocol → PLATO Tile Sync

`SyncProtocol` with `Diff` and `DiffOp` is exactly what PLATO needs for tile synchronization between rooms. When an agent generates tiles in a CodespaceRoom, `SyncProtocol` computes the diff against the PLATO tile store and sends only changes. `DiffOp::Insert` = new tile, `DiffOp::Update` = modified tile, `DiffOp::Delete` = pruned tile. The `Handshake` phase negotiates which tile domains each room supports.

### Handshake → construct-core Capability Negotiation

`Handshake` with `Capability` enum maps to construct-core's hardware tiers:

- `Capability` set { "read", "query" } → Layer 0 (ESP32, bare metal)
- `Capability` set { "read", "query", "write", "compute" } → Layer 1 (Pi, sync)
- `Capability` set { "read", "query", "write", "compute", "network", "async" } → Layer 2 (DGX, async)

When two rooms connect, `Handshake` negotiates which capabilities both support. The minimum capability set determines the protocol features used. This prevents a CodespaceRoom from sending async requests to a BareRoom that can't handle them.

### ternary-steganography → Enriched Protocol Messages

Every `TernaryMessage` can carry hidden data via `ternary-steganography`'s embedding techniques. `Payload` becomes both visible carrier and hidden channel. Visible data = fleet coordination messages. Hidden data = skill version, trust score, tamper-evident checksum. `SpreadSpectrum` steganography makes protocol messages tamper-evident: any modification destroys the hidden checksum.

## Potential in Mature Systems

ternary-protocol becomes the universal communication layer of the ecosystem. Every room-to-room message, every ensign request, every fleet heartbeat rides on ternary-protocol. The `RoutingMode` determines delivery: `Unicast` for room-to-room, `Broadcast` for fleet-wide alerts, `Multicast` for domain-specific coordination (all engine-monitor ensigns). `ProtocolVersion` ensures backward compatibility as the protocol evolves. The binary wire format enables ESP32-to-DGX communication without protocol translation.

## Cross-Pollination Ideas

- **ternary-game-theory → MessageBus routing**: Use Nash equilibrium finding to route messages. Instead of static routing, agents "play" a routing game where the equilibrium determines optimal message paths.
- **ternary-consensus → SyncProtocol conflict resolution**: When `DiffOp` conflicts arise (two rooms modify the same tile), `ternary-consensus`'s PBFT protocol resolves the conflict.
- **ternary-locks → Handshake access control**: `Lock` patterns from `ternary-locks` become capability gates in `Handshake`. A `LockComposition::And` = both capabilities required, `LockComposition::Or` = either sufficient.

## Dependencies for Next Steps

1. I2I message type mapping layer on top of `TernaryMessage`
2. `SyncProtocol` → PLATO tile store adapter
3. `Handshake` → construct-core `CapabilityMatrix` integration
4. Transport abstraction: TCP/UDP for local, git commits for async
5. ESP32-compatible encoding (no_std, fixed-size buffers)
