# Current development library

This branch provides read-only workflow primitives; it has no FlowGuard CLI, enforced gate or production approval provider. Build in the coordinated sibling checkout layout with Rust 1.90+ (tested using 1.99.0):

```sh
cargo test --locked
```

`input_limits::AllowedRoot` bounds access to a caller-authorized immutable checkout. `stage::discover` inventories the `flowguard.docs/v1` ten-document layout. `dependencies::build_graph` validates explicit stage dependencies. `baseline::resolve_baseline` verifies structural exact references without authenticating approvals. `obligations::freeze` preserves required providers and coverage before collection.

`context::bind` consumes actual GitGuard candidate observations and returns an advisory GE RunBinding, rejecting ambiguous identities, nonexistent objects and dirty source drift. `specguard_adapter::import_fixture` consumes only explicitly fixture-labelled native SpecGuard exports. `approvals::ApprovalProvider` is a controller-side integration port with no production implementation. Transition and action results do not grant execution authority.

See [implementation ledger](implementation-progress.md), [schema decisions](adr/domain-schema.md), [approval boundary](adr/approval-port.md), and [verified legacy survey](adr/legacy-compatibility.md). The original architecture documents describe the broader target; this library implements only the recorded slice.

The next local gate slice adds `gate::prepare_gate` and a closed `projection` mapping through actual GuardEngine APIs. It freezes required scopes before collecting evidence, produces an independent report plus versioned advisory GateDecision, and refreshes retained specialist authority at consumption. See [gate contract and limits](adr/gate-contract.md). No production provider or FG-GATE capability is implied.
