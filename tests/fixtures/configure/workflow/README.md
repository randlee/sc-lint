# Workflow transformer fixtures

`test_workflow_transformer.py` creates approved-plan scenarios in isolated
temporary repositories so every case crosses the CLI digest and transaction
boundary. Rust unit tests supply malformed-YAML and injected-rollback fixtures,
because they exercise the private `ManagedArtifact` extension seam.

The fixture cases cover deterministic creation and reapply, user-owned and
near-match workflows, target changes after planning, and rollback of a real
workflow artifact followed by a synthetic second artifact.
