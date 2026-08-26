# Phase 8: CLI Interface & Human Output Formatter — Validation Map

## Requirements Covered
- **CLI-01**: Core CLI commands: `gitctx index`, `search`, `evolution`, `show`, `impact`, `blame`, `doctor`.
- **CLI-02**: Structured `--json` output envelopes with schema version, config hash, warnings, and result citations.
- **CLI-03**: Rich `--format table` terminal rendering with chronological evolution timelines, diff stats, pager integration, and `NO_COLOR` support.
- **CLI-04**: `gitctx doctor` diagnostic tool checking index freshness, config hashes, vector consistency, and parse status distributions.

## Test Verification Matrix

| Requirement | Test Name | File | Assertion / Invariant Verified |
|:---|:---|:---|:---|
| **CLI-01** | `test_cli_parser_defaults_and_flags` | `crates/gitctx-cli/tests/cli_test.rs` | All 7 subcommands (`Index`, `Search`, `Evolution`, `Show`, `Impact`, `Blame`, `Doctor`) parse valid flags and options. |
| **CLI-02** | `test_response_envelope_serialization_roundtrip` | `crates/gitctx-cli/tests/cli_test.rs` | `ResponseEnvelope` enforces version `"v1"`, typed status (`success`/`error`), optional payload, and structured `ErrorDetail`. |
| **CLI-03** | `test_table_formatters_smoke` | `crates/gitctx-cli/tests/cli_test.rs` | Table formatters (`format_evolution`, `format_search`, `format_impact_single`, `format_impact_global`, `format_show`, `format_blame`) render properly via `comfy-table`. |
| **CLI-04** | `test_doctor_command_json_and_table` | `crates/gitctx-cli/tests/cli_test.rs` | `gitctx doctor` runs in both `Table` and `Json` modes, validating git discovery, SQLite health, schema version, and parse distributions. |

## Manual & Interactive Verification
- `gitctx doctor` executed directly in workspace root verifies repository discovery and index status.
- `gitctx doctor --format json` emits schema-compliant envelope.
