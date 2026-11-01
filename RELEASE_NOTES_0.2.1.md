## v0.2.1

Fixes from an independent code review.

### Fixed
- **Critical:** boolean settings were misread — `reg query` returns `0x1`/`0x0`, while the code
  compared with `"1"`. After changing anything in the tray menu the app disabled itself on the
  next start. Settings are now read through the Win32 Registry API with proper types.
- **Tray icon survives Explorer restart.** The window is now a hidden top-level window (message-only
  windows never receive the `TaskbarCreated` broadcast), the message is handled, and `NIM_ADD` is
  retried by a timer if Explorer was not ready at logon.
- **No more stuck modifiers.** If the user is already holding Shift/Ctrl/Alt, that modifier is not
  injected (previously the synthetic key-up could release it mid-typing).
- **No `reg.exe` in the hook/UI thread** — registry access is native and saving happens off-thread,
  so the low-level hook never risks the hook timeout.
- Layout fallback uses the foreground window's keyboard layout (thread id), not our own.
- Restart uses an exit event handshake instead of a racing flag; mutex is `Local\` (per session).
- `schtasks` path is quoted (works from paths with spaces); `--install --elevated` removes the
  duplicate Run entry.

### Added
- `--version`.
- Fatal errors show a message box instead of exiting silently.
- Per-monitor DPI awareness (tray menu positions correctly on mixed-DPI setups).
- 7 unit tests; CI now runs `cargo fmt --check`, `clippy -D warnings`, `cargo test` and a CLI smoke test.

### Install
```powershell
caps-switch.exe --install              # normal privileges
caps-switch.exe --install --elevated   # highest privileges (works in elevated windows too)
```
