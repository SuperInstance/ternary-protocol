# ternary-protocol

Wire protocol for communication between ternary agents — message passing, serialization, and synchronization.

Pure Rust, no unsafe, no external dependencies.

---

## Why This Exists

If agents are going to coordinate — pass tasks, share state, negotiate — they need a language. Not human language (that's what A2A is for), but a *wire language*. A way for one agent to say "here's my state vector" to another agent in a format both can parse without an LLM in the middle.

This protocol encodes everything as sequences of **trits** (ternary digits: -1, 0, +1). Why ternary? Because the agents in the SuperInstance ecosystem speak ternary natively. Their state, their relationships, their strategies — all expressed as {-1, 0, +1} vectors. Encoding the wire protocol in the same formalism means no translation layer between "what the agent thinks" and "what goes on the wire."

The protocol is the *nervous system* of the ternary agent fleet. Agents don't send paragraphs. They send structured trit sequences that another agent can route, sync, and verify without needing to *understand* the content — just the structure.

---

> ⛏️ **DEEP CUT: Trit Encoding Is Not About Compression**
> 
> Packing two trits per byte (4+4 nibble scheme) looks like an optimization. It's not. The real reason is **alignment**: when every message is a sequence of trits, and every trit is the same size on the wire, the routing layer doesn't need to care about message content. It can forward, batch, and synchronize purely on structure.
> 
> The 4+4 nibble scheme is also carefully chosen so that trit boundaries are always byte-aligned for odd-length sequences (the padding trit goes in the lower nibble of the last byte). This means a receiver can always find the next message boundary by looking at the byte count — no variable-length decoding needed.
> 
> The cost is wasted bits (4 bits per trit, but only 2 bits are needed for {-1,0,+1}). That's 50% overhead. It's worth it because the routing layer stays stupid-simple. Solid-state physics trits (spin states, charge states, flux quanta) also naturally map to the {-1,0,+1} encoding with zero translation.
> 
> In other words: the protocol wastes bytes to save CPU cycles, which is the correct trade for an agent fleet where CPU is scarce and bandwidth is abundant. The opposite of the internet's tradeoff, and intentionally so.

---

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

Synchronization handles agent join/leave, state reconciliation, and clock drift.

**Agent Registration Message** (sent on join):
```
[8 bytes: agent ID (u64 BE)]
[2 bytes: listening port (u16 BE)]
[4 bytes: protocol version (u32 BE)]
```

**State Sync Request**:
```
[8 bytes: requesting agent ID (u64 BE)]
[8 bytes: last known state timestamp (u64 BE)]
[2 bytes: max payload size (u16 BE)]
```

**State Sync Response**:
```
[8 bytes: responding agent ID (u64 BE)]
[8 bytes: current state timestamp (u64 BE)]
[2 bytes: flags (u16 BE)] — bit 0: more data, bit 1: error
[...state payload bytes]
```

**Heartbeat** (periodic, no response expected):
```
[4 bytes: agent ID (u32 BE)]
[4 bytes: uptime seconds (u32 BE)]
```

## Design Decisions

1. **No LLM in the hot path** — trit parsing is bit-twiddling, not inference. The protocol is designed so that a simple C or Rust receiver can parse and route messages without *any* AI component.

2. **Fixed-size headers** — every message header is exactly 32 bytes (4× u64). This means a receiver can always find the header boundary regardless of payload size.

3. **Big-endian wire format** — network byte order for compatibility across architectures. The performance cost on little-endian machines is negligible at agent-scale message rates.

4. **Zero = broadcast** — reusing the reserved receiver ID 0 as a broadcast signal eliminates the need for a separate broadcast header or routing table check.

5. **No encryption in-band** — the protocol assumes a transport layer handles encryption (TLS, Noise, or physical isolation). The protocol itself is plaintext for simplicity and debuggability.

License: MIT
