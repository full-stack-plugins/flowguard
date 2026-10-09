# Native specialist baseline coordination

Status: implemented for independent review; local fixture profile, no production issuer.

The same candidate has different domain evidence scopes. GitGuard binds a full Git snapshot; ArchGuard Cargo declarations bind the manifests they analyze; CodeGuard Ruff F401 binds the selected Python file and its qualified configuration; TestGuard binds a frozen test plan and requirement baseline. Their domain policy digests are independently protected. Replacing native source digests or removing TestGuard's baseline would misrepresent actual producer output.

`flowguard.specialist-sources/v1alpha1` retains its exact serialized fields, digest and behavior: only source digest is coordinated and all other binding fields must match the controller. Consequently actual TestGuard evidence with a baseline cannot pass a baseline-free v1 gate.

Additive `v1alpha2` requires an explicit baseline choice for every scoped obligation. `absent` requires native baseline absence. `frozen_workflow_baseline` requires the exact baseline in the independently frozen workflow obligations. There is no arbitrary producer baseline override. Unknown versions, omitted choices, explicit null, foreign baseline and unexpected fields fail validation. The mapping is supplied before evidence collection; native observations never fill it in.

The profile includes the complete frozen obligation digest, controller context and workflow baseline. Its digest enters gate required coverage and complete work identity. A changed mapping cannot publish an old result into a new reservation. Existing outer composite obligation scopes map to exact native coverage without modifying native contracts, facts, reports or decisions. All other binding fields remain exact.

The actual cross-provider fixture freezes protected contexts and contracts before collecting completed evidence. CG's existing bound-error preparation records required scopes without supplying qualification. The later native run has its own observed run ID. Both inputs and outputs are retained. AG and GG candidates share repository, commit, base, task, requirements and paths, while preserving independent domain policies; each candidate is validated through its own producer API.

This coordination is an explicit controller policy, not authentication. Fixture issuer records are digest-indexed and are not a production service. Missing providers, weak/error evidence and unavailable/revoked issuers remain closed. No remote write authority is enabled.
