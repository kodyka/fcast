# Android Sender Migration (`senders/android/src/migration`) — Step-by-Step Explanation

This document explains how the migrated Android sender flow works, in practical order, from an incoming command to live media routing.

## 1) Entry points and module layout

The migration lives under:

- `senders/android/src/migration/protocol.rs` — wire protocol types (commands/results/info)
- `senders/android/src/migration/messages.rs` — internal message payload structs/enums
- `senders/android/src/migration/runtime.rs` — JSON + HTTP command ingress, runtime lifecycle
- `senders/android/src/migration/node_manager.rs` — command dispatch and graph bookkeeping
- `senders/android/src/migration/media_bridge.rs` — producer→consumer media fan-out bridge
- `senders/android/src/migration/nodes/` — node implementations:
  - `source.rs`
  - `destination.rs`
  - `mixer.rs`
  - `video_generator.rs`
  - `control.rs` (control-point evaluation logic)

## 2) Command ingress (runtime)

1. Runtime receives command input either:
   - as raw JSON command payload, or
   - as HTTP `POST /command` body.
2. JSON is parsed into protocol types (`Command` or `ControllerMessage`).
3. Runtime forwards the parsed command to the graph manager.
4. Runtime wraps manager output back into protocol response JSON (`ServerMessage`).

Health check and routing behavior:

- `GET /health` returns status JSON.
- Unknown route => `404`.
- Unsupported method on known route => `405`.

## 3) Command dispatch (node manager)

1. `NodeManager` validates command parameters.
2. It routes by command variant:
   - `CreateSource`
   - `CreateDestination`
   - `CreateMixer`
   - `CreateVideoGenerator`
   - `Connect` / `Disconnect`
   - `Start` / `Reschedule`
   - `AddControlPoint` / `RemoveControlPoint`
   - `GetInfo`
   - `Remove`
3. On success, manager mutates:
   - node registry,
   - link registry,
   - media-bridge map.
4. It returns `CommandResult::{Success,Info,Error}`.

## 4) Node creation

When creating nodes, manager verifies:

- unique IDs,
- media capability flags (audio/video),
- type-specific configuration validity.

Then each node instance is stored in manager state.

## 5) Connect/disconnect flow

### Connect

1. Resolve producer/source node by `src_id`.
2. Resolve consumer/sink node by `sink_id`.
3. Validate media compatibility (audio/video).
4. Register link metadata (`link_id`, endpoints, config).
5. Update node-level slot/link bookkeeping.
6. Ensure/attach corresponding `StreamBridge` paths used for frame transport.

### Disconnect

1. Lookup `link_id`.
2. Remove link from manager registry.
3. Remove slot/link references from both endpoint nodes.
4. Prune bridge consumers and clean bridge state if no longer used.

## 6) Scheduling and state transitions

Each node supports scheduling with `cue_time` and optional `end_time`.

Typical transition model:

- `Initial` → `Starting` (preroll window)
- `Starting` → `Started` (at cue)
- `Started` → `Stopped` (at end or explicit stop)

Node-specific pipeline stage enums mirror these transitions (e.g., idle/prerolling/playing).

## 7) Control points

Control points are time-stamped property updates.

1. Manager accepts add/remove control-point commands.
2. Target can be mixer-global properties or mixer-slot properties.
3. `nodes/control.rs` evaluates points at query time:
   - `Set`: latest point at/before timestamp wins.
   - `Interpolate`: numeric interpolation when possible.
4. Evaluated values are applied back to active settings/slot settings.

## 8) Media bridge behavior

`StreamBridge` handles runtime producer→many-consumer fan-out:

1. Producer sink attaches to bridge.
2. Consumers (`appsrc`) attach/detach by link ID.
3. Caps are cached/applied so late consumers can align format.
4. Clearing bridge removes consumer state and sink attachment.

## 9) Info/reporting path

`GetInfo` collects per-node protocol-compatible snapshots (`NodeInfo`) including:

- node state,
- timing (`cue_time`, `end_time`),
- slot/link associations,
- mixer settings/control maps where relevant.

`VideoGeneratorNode` reports via a source-compatible shape for protocol compatibility.

## 10) Error handling model

Errors are returned as protocol `CommandResult::Error` strings when:

- IDs are missing/duplicated,
- links or nodes are absent,
- media capabilities are incompatible,
- control-point targets/properties are invalid,
- scheduling preconditions fail.

Runtime wraps malformed JSON/request failures into deterministic JSON error envelopes.

## 11) Practical end-to-end example

1. `CreateSource(id=s1)`
2. `CreateMixer(id=m1)`
3. `CreateDestination(id=d1)`
4. `Connect(link=l1, src=s1, sink=m1)`
5. `Connect(link=l2, src=m1, sink=d1)`
6. `AddControlPoint(controllee=m1, property=width, ...)`
7. `Start(id=d1)` then `Start(id=s1)` or schedule with cue/end times
8. `GetInfo()` to inspect graph state

That sequence exercises protocol parsing, manager dispatch, node/link bookkeeping, control-point evaluation, scheduling, and response serialization.
