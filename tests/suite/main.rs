//! Single-binary integration suite (issue #693).
//!
//! Cargo treats each `tests/*.rs` file as its own binary. Files moved under
//! `tests/suite/` become modules of this one target instead.

#[path = "../common/mod.rs"]
mod common;

mod action_return_type_residuals_test;
mod ambiguous_write_analyzer_test;
mod ambiguous_write_branch_typecheck_test;
mod analyzer_include_expect_test;
mod analyzer_unused_statement_uses_test;
mod colon_consumption_test;
mod constant_mutation_analyzer_test;
mod container_ast_corruption_test;
mod container_parsing_fixes;
mod contains_unification_test;
mod crypto_async_test;
mod crypto_kdf_test;
mod crypto_seal_test;
mod crypto_test;
mod database_analyzer_test;
mod database_parser_test;
mod diagnostics_fixtures_test;
mod export_constant_mutability_test;
mod export_statement_test;
mod http_request_parser_test;
mod include_change_variable_test;
mod include_diamond_test;
mod include_preserves_constness_test;
mod include_statement_test;
mod main_loop_parser_test;
mod nested_count_loops_test;
mod nothing_reassign_widen_test;
mod open_file_local_type_test;
mod parse_multipart_test;
mod password_hashing_test;
mod pattern_doctest_fixes_test;
mod random_functions_test;
mod recursive_action_return_type_test;
mod respond_statement_parser_test;
mod sha256_hmac_test;
mod static_container_member_test;
mod test_framework_counting_test;
mod time_stdlib_test;
mod toml_test;
mod transaction_analyzer_walk_test;
mod transaction_completion_type_test;
mod typechecker_alias_provenance_residual_test;
mod typechecker_builtin_contract_test;
mod typechecker_container_contract_test;
mod typechecker_definite_binding_test;
mod typechecker_expression_coverage_test;
mod typechecker_gradual_any_test;
mod typechecker_legacy_list_property_test;
mod typechecker_list_alias_depth_test;
mod typechecker_loop_runtime_parity_test;
mod typechecker_repeat_until_backedge_test;
mod typechecker_response_contract_test;
mod typechecker_reuse_test;
mod typechecker_runtime_binding_test;
mod typechecker_statement_completion_parity_test;
mod typechecker_statement_operand_contract_test;
mod typechecker_try_finally_join_test;
mod typechecker_websocket_binding_scope_test;
mod value_cycle_safety;
mod value_equality;
mod web_server_tls_parser_test;
mod wflhash_hardened_security_test;
mod wflhash_security_test;
