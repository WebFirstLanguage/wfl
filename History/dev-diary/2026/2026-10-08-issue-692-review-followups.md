# Issue #692 review follow-ups

PR #754 landed the three harness fixes. Review of that PR asked for three
honesties that do not change the original assertions.

The vacuous `file_io_execution_tempdir_removes_fixtures_on_panic` test is
gone. It never called `execute_wfl_code`, would have passed on `main`
before the tempfile migration, and deleted `test_exec_basic.txt` from the
working directory first, which would hide a real leak. A meaningful
panic-path leak test of `TempDir` is not feasible here: cleanup-on-panic
is the tempdir's job, and the leak we can actually catch is a relative
`at "name"` that the rewrite set forgot. `rewrite_fixture_paths` now
rejects a listed name missing from the source and a relative `at`
operand that is not listed. The directory-listing test passes an empty
rewrite set because its WFL source only names `"."`.

`wait_until_process_stops` is gone. `kill_process` already removes the
handle and awaits `terminate_foreground_child` (`start_kill` + `wait`),
so the first `is_process_running` poll was already false. The kill tests
still assert the process is running before kill and not running after.

The `wfl_release_exe` documentation is back on `wfl_release_exe`;
`require_existing_release_binary` keeps its one-line summary.
