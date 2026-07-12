# Contract-2 Security Acceptance Matrix

This document maps the single-project hardened contract to executable tests.
It is the acceptance index for `br-hardened-papercuts-fork-x30.11`; the ADRs
remain normative when a test name and prose disagree.

Multi-project inventory, aliases, and digest behavior are deliberately absent.
They remain deferred until single-project pilot evidence exists.

## Safe runner

Run the dedicated real-binary acceptance suite with:

```bash
scripts/security-acceptance.sh
```

The runner records only its run ID, tool versions, commit, command, test names,
and result under `target/security-acceptance/<run-id>/`. It replaces local repo
and Cargo target paths in captured output. Fixtures are synthetic and failure
messages identify fixture IDs or test names, never the matched value.

The runner is the focused evidence surface, not a replacement for the full
Cargo, Clippy, formatting, UBS, Gitleaks, and journal-doctor gates.

## Contract mapping

| Contract surface | Unit or black-box proof |
|---|---|
| Static schema, error and exit dictionaries, canonical records, v0.1 field tolerance | `schema_contract2_is_static_exact_and_runtime_sourced`, `error_envelope_matrix`, `contract2_private_and_committed_records_are_exact_and_share_the_same_id` |
| Profile, target, content-policy and read-only precedence | `contract2_policy_and_target_precedence_are_explicit`, `read_only_is_monotonic_and_precedes_clock_storage_and_stdin`, `sensitive_policy_floor_and_override_gate_are_centralized` |
| Command-relevant environment and lazy clock | `commands_read_only_the_environment_that_can_affect_them`, `schema_and_read_commands_ignore_unrelated_hostile_environment` |
| First-run private storage, committed HOME fallback, missing HOME, no-write reads and dry runs | `private_default_uses_common_git_storage_with_user_only_permissions`, `storage_fallback_and_no_side_effect_matrix_uses_disposable_roots`, `mutation_dry_runs_do_not_write` |
| Legacy-only, dual-journal, migration and rollback selection | `private_non_git_and_legacy_migration_states_are_explicit`, `private_projection_redacts_legacy_records_without_rewriting_source_bytes` |
| Permissions, final/parent symlinks and no fallback | `insecure_implicit_private_permissions_block_mutation_and_doctor_reports`, `private_profile_rejects_final_and_implicit_directory_symlinks`, `private_explicit_parent_symlink_is_allowed_without_path_echo` |
| Ordinary Git, linked worktree, submodule, bare/non-Git and no Git executable | `discovery_precedence_virtual_empty_and_valid_git_root`, `linked_worktrees_share_one_private_journal`, `submodule_private_identity_is_distinct_and_committed_symlink_is_legacy_compatible`, `bare_repository_uses_private_non_git_semantics`, `relative_gitdir_resolution_works_without_git_on_path` |
| Strict gitdir/commondir grammar, missing structures, nested invalid marker and traversal | `repository_metadata_grammar_refuses_malformed_nearest_markers`, `malformed_nearest_git_marker_never_falls_back_to_outer_repository`, `metadata_paths_preserve_symlink_parent_traversal_until_canonicalization` |
| Non-UTF-8 and path projection | `non_utf8_paths_are_omitted_in_private_and_labeled_in_committed`, `symlink_git_marker_and_non_utf8_gitdir_behave_deterministically`, `private_lifecycle_never_projects_storage_fragments` |
| Every high- and medium-risk content category; actual and dry-run refusal; no echo/no write | `real_binary_refuses_every_catalog_category_without_echo_or_write`, `catalog_variants_cover_every_version_one_family` |
| Text, tag, explicit/environment agent, resolution note and cross-field AWS pair | `real_binary_scans_every_persisted_field_and_cross_field_pairs`, `sensitive_preflight_covers_tags_agents_and_resolution_notes`, `cross_field_aws_pair_and_field_order_are_stable` |
| Balanced/strict floor and two-part exact override | `override_gate_is_exact_and_audited_by_the_real_binary`, `override_must_exactly_cover_refusing_categories` |
| False positives, placeholders, variable references, encoded/partial documented misses and all dry-run decisions | `documented_false_positive_controls_and_dry_run_modes_are_real_binary_clean`, `controls_and_documented_misses_remain_allowed` |
| UTF-8, CRLF/LF, exact field/count bounds and one-over limits | `invalid_utf8_and_parser_values_are_redacted_and_write_nothing`, `sensitive_preflight_enforces_bounded_stdin_fields_and_notes`, `byte_and_count_bounds_are_exact` |
| Contract-1/2 mixed data, duplicate/conflict/orphan/unknown/malformed/torn records and stable warnings | `mixed_journal_is_read_only_deterministic_and_safely_diagnosed`, `doctor_reports_all_core_findings_and_recomputed_ids`, `doctor_finding_counts_match_fold_bytes_warning_counts` |
| Distinct/identical add races, resolve race, lock timeout and append tear healing | `eight_way_distinct_add_race_loses_no_lines`, `eight_way_identical_add_race_appends_once`, `eight_way_resolve_race_appends_once`, `lock_timeout_is_retryable_exit_75`, `torn_tail_self_heals_on_add` |
| Parser/config redaction and policy-preserving suggested fixes | `rejected_values_never_echo_and_suggested_fixes_never_weaken_policy`, `invalid_utf8_and_parser_values_are_redacted_and_write_nothing`, `private_errors_use_opaque_locations_and_policy_metadata` |

## Evidence interpretation

- A refusal must leave a missing target and parent missing, or preserve the
  exact bytes of an existing journal.
- Private operations scan stdout, stderr, and journal bytes for synthetic path
  fragments across add, duplicate, dry-run, list, resolve, already-resolved,
  doctor, and error paths.
- Tests that intentionally accept an override verify the persisted
  `content_policy` audit. The environment gate alone and the category flag
  alone both remain insufficient.
- The focused runner uses one test thread for readable deterministic evidence.
  The full suite continues to exercise parallel-process races independently.
