## v0.4.1

### Fixed
- **"Check for updates" no longer freezes the app.** The network request ran on the same
  thread as the keyboard hook and the message loop: the window stopped responding for up
  to 20 seconds, and Windows could even drop the low-level hook on its timeout.
  The check now runs on a background thread; a tray balloon ("Checking for updates…")
  shows that work is in progress, and the result dialog appears when it finishes.
- Update installation requests the UI thread to exit via a posted message instead of
  `PostQuitMessage` from a worker thread (which only affects the calling thread).

### Notes
- No API changes; the CLI flag `--check-updates` works as before.

### Install
```powershell
caps-switch-setup-0.4.1.exe
```
