# Moteur — points différés (revue du 2026-11-05)

Branche feat/moteur du dépôt logiciel/. Points mineurs relevés pendant les relectures et laissés pour plus tard (la revue finale les a classés « peut attendre »).

- Task 1: minor (deferred): name() panics for InputId>28; encode_* unchecked profile range (add debug_assert); rustfmt not run
- Task 2: minor (deferred): no test for duplicate usages / same-report up+down ordering / short report inputs
- Task 3: minor (deferred): RED skipped; untested branches (expired window in down, up-fires-Hold w/o tick, 250ms boundary); tick output in HashMap order
- Task 4: minor (deferred): save_default_if_missing non-atomic write; config_path falls back to relative path; ConfigError lacks source(); no deny_unknown_fields; boundary ±100 untested
- Task 5: minor (deferred): send_text sends 
- Task 6: minor (deferred): CoInitializeEx count never balanced (use thread-local init once on worker thread); CoInitializeEx errors all discarded; command stdin/stderr inherited
- Task 7: minor (deferred): macro Do steps SendInput on macro thread can interleave with worker; timing not hot-reloaded; toggle state survives reload/device_lost; unbounded thread spawn; ProfileAnnounce Up / Toggle->Macro route untested
- Task 8: minor (deferred, final review should triage): unaligned RAWINPUTHEADER ref (read_unaligned); matches() reads hid union w/o dwType check + caches transient failure; WM_INPUT missing DefWindowProcW; GetMessageW -1 exits silently; tracing_appender panics if log dir fails + no level filter; logs dir path duplicated vs paths.rs; arg errors invisible in release; simulate test claims logging check it does not do, writes real %APPDATA% logs
- Task 9: minor (deferred): watcher Err events dropped unlogged; no retry on transient sharing-violation read; no rename/replace-save test; temp dirs not cleaned
- Task 10: minor (deferred): unrelated rustfmt reflow of codes.rs mixed into commit (behavior-neutral, clears T1 fmt minor); magic 2 vs ERROR_FILE_NOT_FOUND; is_autostart true for stale path; exit(0) skips tray drop (ghost icon); CreateMutexW failure unlogged; autostart test touches real HKCU Run
- Task 11: minor (deferred): e2e cleanup not on failure path (Drop guard); negative asserts wait only 500ms; README example JSON not covered by a test; README should say --simulate skips autostart/single-instance
