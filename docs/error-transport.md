# Binding-aware failure transport

The local library/CLI uses GuardEngine TransportDiagnostic for failures before immutable candidate and required scope are resolved. CLI stdout is empty and stderr contains one static JSON diagnostic with exit 4. User arguments, paths and provider error text are not echoed. Unknown or non-UTF-8 arguments, ambiguous inputs, unrecognized request versions and missing/changed frozen pins cannot create a candidate envelope.

After preparation, PendingGate::finish_failure consumes the opaque attempt. Controller-observed crash, timeout or invalid evidence yields error/null; cancellation yields cancelled/null. The exact frozen required coverage remains, artifacts are absent, and encoding exits 4. ObservedFailure cannot be deserialized. The controller must actually observe the failure; this API does not run, kill, sandbox or supervise workers.

transport::encode_gate accepts only the gate-produced opaque result and revalidates the engine-backed envelope and available engine artifacts. Existing CLI JSON fields remain unchanged; completed ALLOW/BLOCK/REQUIRE_APPROVAL map to 0/2/3, errors and cancellation to 4. Missing evidence remains a real partial BLOCK. stdout contains one JSON result and never diagnostic text. The local_fixture label is descriptive, and execution_authorized remains false.

Maintained tests cover actual GE decisions/partial evidence, malformed specialist bytes, changed/unfrozen request pins, argument ambiguity, actual non-UTF-8 binary arguments, and actual controller-observed terminated/deadline-killed processes. These process tests demonstrate transport encoding, not product process isolation or production execution qualification.
