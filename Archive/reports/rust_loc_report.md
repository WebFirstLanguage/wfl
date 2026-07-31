# Rust Code Line Count Report
*Generated on: 2026-07-04 01:44:09*


## Overall Statistics
Total files processed: 93
Total lines: 58944
Code lines: 48441 (82.2%)
Comment lines: 4693 (8.0%)
Blank lines: 5810 (9.9%)

## Lines by Directory
Directory                                Total      Code       Comments   Blank     
| Directory | Total | Code | Comments | Blank |
| --- | --- | --- | --- | --- |
| src\interpreter | 11900 | 9942 | 839 | 1119 |
| src\parser\stmt | 6903 | 5747 | 449 | 707 |
| src\stdlib | 6197 | 4575 | 763 | 859 |
| src\parser | 5544 | 4499 | 607 | 438 |
| src\analyzer | 5471 | 4672 | 286 | 513 |
| src | 4560 | 3790 | 253 | 517 |
| src\typechecker | 4369 | 3928 | 216 | 225 |
| src\transpiler | 3116 | 2724 | 149 | 243 |
| src\pattern | 2448 | 1510 | 644 | 294 |
| src\lexer | 2109 | 1696 | 193 | 220 |
| src\parser\expr | 2052 | 1703 | 182 | 167 |
| src\wfl_config | 1444 | 1237 | 36 | 171 |
| src\fixer | 1334 | 1146 | 39 | 149 |
| src\diagnostics | 833 | 707 | 33 | 93 |
| src\linter | 622 | 529 | 2 | 91 |
| src\bin | 42 | 36 | 2 | 4 |

## Lines by File
| File |  | Total | Code | Comment |
| Directory | Total | Code | Comments | Blank |
| --- | --- | --- | --- | --- |
| src\interpreter\mod.rs |  | 8686 | 7344 | 598 |
| src\typechecker\mod.rs |  | 4369 | 3928 | 216 |
| src\analyzer\mod.rs |  | 3260 | 2715 | 202 |
| src\transpiler\javascript.rs |  | 2206 | 1954 | 76 |
| src\analyzer\static_analyzer.rs |  | 1958 | 1758 | 76 |
| src\parser\tests.rs |  | 1817 | 1494 | 107 |
| src\main.rs |  | 1345 | 1153 | 64 |
| src\parser\expr\primary.rs |  | 1294 | 1104 | 98 |
| src\fixer\mod.rs |  | 1218 | 1060 | 31 |
| src\parser\stmt\patterns.rs |  | 1147 | 986 | 63 |
| src\config.rs |  | 1131 | 968 | 42 |
| src\stdlib\filesystem.rs |  | 1124 | 906 | 23 |
| src\parser\stmt\io.rs |  | 1057 | 920 | 58 |
| src\wfl_config\checker.rs |  | 1027 | 898 | 10 |
| src\parser\ast.rs |  | 1012 | 925 | 53 |
| src\pattern\compiler.rs |  | 961 | 578 | 256 |
| src\pattern\vm.rs |  | 880 | 649 | 123 |
| src\transpiler\runtime.rs |  | 807 | 702 | 51 |
| src\parser\mod_complete.rs |  | 804 | 722 | 19 |
| src\parser\stmt\control_flow.rs |  | 792 | 653 | 35 |
| src\stdlib\list.rs |  | 772 | 671 | 26 |
| src\parser\expr\binary.rs |  | 729 | 588 | 71 |
| src\diagnostics\mod.rs |  | 716 | 618 | 24 |
| src\parser\stmt\containers.rs |  | 705 | 611 | 28 |
| src\stdlib\text.rs |  | 691 | 519 | 74 |
| src\parser\mod.rs |  | 687 | 603 | 51 |
| src\lexer\token.rs |  | 666 | 606 | 24 |
| src\parser\cursor.rs |  | 662 | 277 | 325 |
| src\stdlib\helpers.rs |  | 641 | 232 | 389 |
| src\stdlib\crypto.rs |  | 626 | 445 | 91 |
| src\parser\stmt\actions.rs |  | 619 | 532 | 23 |
| src\lexer\tests.rs |  | 618 | 526 | 24 |
| src\interpreter\value.rs |  | 577 | 512 | 21 |
| src\linter\mod.rs |  | 561 | 477 | 2 |
| src\stdlib\time.rs |  | 542 | 415 | 41 |
| src\logging.rs |  | 510 | 432 | 12 |
| src\builtins.rs |  | 500 | 373 | 94 |
| src\stdlib\typechecker.rs |  | 500 | 351 | 22 |
| src\parser\stmt\processes.rs |  | 485 | 389 | 34 |
| src\debug_report.rs |  | 468 | 376 | 16 |
| src\parser\stmt\testing.rs |  | 468 | 374 | 47 |
| src\interpreter\command_sanitizer.rs |  | 452 | 368 | 29 |
| src\interpreter\database.rs |  | 448 | 378 | 40 |
| src\repl.rs |  | 410 | 348 | 0 |
| src\wfl_config\wizard.rs |  | 410 | 333 | 26 |
| src\parser\helpers.rs |  | 376 | 338 | 26 |
| src\interpreter\environment.rs |  | 346 | 251 | 45 |
| src\parser\stmt\collections.rs |  | 346 | 257 | 38 |
| src\lexer\mod.rs |  | 343 | 266 | 52 |
| src\stdlib\pattern.rs |  | 343 | 283 | 23 |
| src\parser\stmt\errors.rs |  | 339 | 304 | 12 |
| src\interpreter\tests.rs |  | 314 | 243 | 23 |
| src\pattern\instruction.rs |  | 301 | 193 | 68 |
| src\pattern\mod.rs |  | 274 | 67 | 194 |
| src\interpreter\assertion_helpers.rs |  | 271 | 259 | 6 |
| src\stdlib\random.rs |  | 264 | 207 | 14 |
| src\analyzer\tests.rs |  | 253 | 199 | 8 |
| src\parser\stmt\variables.rs |  | 253 | 209 | 18 |
| src\lexer\string_line_ending_tests.rs |  | 251 | 155 | 45 |
| src\parser\stmt\database.rs |  | 249 | 198 | 25 |
| src\parser\stmt\web.rs |  | 220 | 146 | 36 |
| src\stdlib\json.rs |  | 215 | 177 | 13 |
| src\interpreter\io_tests.rs |  | 191 | 151 | 0 |
| src\lexer\position_tests.rs |  | 189 | 123 | 29 |
| src\stdlib\math.rs |  | 186 | 161 | 1 |
| src\parser\container_ast.rs |  | 182 | 140 | 22 |
| src\interpreter\bounded_buffer.rs |  | 159 | 117 | 12 |
| src\stdlib\web.rs |  | 154 | 93 | 39 |
| src\parser\stmt\module.rs |  | 151 | 123 | 10 |
| src\interpreter\memory_tests.rs |  | 137 | 96 | 21 |
| src\interpreter\op_refactor_tests.rs |  | 123 | 75 | 20 |
| src\diagnostics\tests.rs |  | 117 | 89 | 9 |
| src\fixer\tests.rs |  | 116 | 86 | 8 |
| src\env_dump.rs |  | 111 | 75 | 18 |
| src\transpiler\mod.rs |  | 103 | 68 | 22 |
| src\lib.rs |  | 76 | 57 | 7 |
| src\parser\stmt\mod.rs |  | 72 | 45 | 22 |
| src\interpreter\io_capture.rs |  | 69 | 45 | 16 |
| src\interpreter\error.rs |  | 65 | 59 | 0 |
| src\stdlib\pattern_test.rs |  | 62 | 50 | 6 |
| src\linter\tests.rs |  | 61 | 52 | 0 |
| src\interpreter\op_refactor_error_tests. | rs | 53 | 35 | 8 |
| src\stdlib\core.rs |  | 47 | 37 | 1 |
| src\bin\cleanup_debug_files.rs |  | 42 | 36 | 2 |
| src\lexer\column_tests.rs |  | 42 | 20 | 19 |
| src\pattern\vm_test_lookahead.rs |  | 32 | 23 | 3 |
| src\stdlib\mod.rs |  | 30 | 28 | 0 |
| src\parser\expr\mod.rs |  | 29 | 11 | 13 |
| src\interpreter\control_flow.rs |  | 9 | 9 | 0 |
| src\repl_tests.rs |  | 8 | 7 | 0 |
| src\wfl_config\mod.rs |  | 7 | 6 | 0 |
| src\parser\container_parser.rs |  | 4 | 0 | 4 |
| src\version.rs |  | 1 | 1 | 0 |