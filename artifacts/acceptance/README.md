# MindScape Acceptance Builds

This is the only local handoff location for executable acceptance builds.

- `versions/<build-id>/mindscape-desktop.exe` is an immutable acceptance executable.
- `versions/<build-id>/manifest.json` records source, time, SHA-256, and dirty-tree state.
- `versions/<build-id>/SHA256SUMS.txt` allows the executable to be verified.
- `LATEST.txt` points to the newest published build.

The generated files are intentionally ignored by Git. Product source, tests, rules, and
sanitized evidence belong in Git; compiled executables do not.

Publish a new build from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/publish-acceptance.ps1 -Label <task-id>
```

Do not hand an executable to an acceptor from `desktop/src-tauri/target/`, and do not
create alternate directories such as `target-fixed`, `target-new`, or `target-final`.
Acceptance publishing always runs a fresh Tauri release build; an existing Rust debug
executable cannot be imported or promoted into this directory.
The complete policy is in `docs/engineering/acceptance-build-policy.md`.
