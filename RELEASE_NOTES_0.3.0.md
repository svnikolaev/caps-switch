## v0.3.0

### Added
- **Custom tray icon**: a rounded key with two opposing arrows (switch), drawn in
  16/24/32/48/64/128/256 px and embedded into the executable as `assets/icon.ico`.
  The icon is loaded at runtime with `CreateIconFromResourceEx` (falls back to the
  stock Windows icon if anything goes wrong).
- `assets/preview-256.png` — preview for documentation.
- `tools/make-icon.ps1` — the icon generator (System.Drawing, no external tools),
  so the icon can be regenerated or tweaked without design software.
- Unit test that validates the embedded ICO structure.

### Notes
- The executable itself still has no icon resource / version info in Explorer
  properties — that needs a build script (planned for 0.4.0).
- Everything from 0.2.1/0.2.2 is included (typed settings, tray survival, modifier
  handling, `EPIPE` fix, CI: fmt/clippy/test).

### Install
```powershell
caps-switch.exe --install              # normal privileges
caps-switch.exe --install --elevated   # highest privileges (works in elevated windows too)
```
