## v0.2.2

Follow-up to the 0.2.1 review fixes.

### Fixed
- **CLI output no longer panics when piped.** `println!` aborts the process with
  `failed printing to stdout: The pipe is being closed (os error 232)` when the reader
  closes early (PowerShell, `| more`, CI). Output now goes through macros that ignore
  write errors, so `caps-switch --status | Out-Null` works.
- CI smoke test now captures CLI output safely (`Start-Process -Wait -RedirectStandardOutput`).

### Notes
- Everything from v0.2.1 is included: typed registry settings, tray icon survival after
  Explorer restart, modifier handling, edge-triggered CapsLock guard, DPI awareness,
  `--version`, 7 unit tests, `fmt`/`clippy -D warnings`/`test` in CI.

### Install
```powershell
caps-switch.exe --install              # normal privileges
caps-switch.exe --install --elevated   # highest privileges (works in elevated windows too)
```
