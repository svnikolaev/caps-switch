## v0.2.0

### New
- **Switch method selector** in the tray menu: `Alt+Shift` (default), `Ctrl+Shift`, `Win+Space`.
- **Interface language**: Russian and English, detected from the system locale and switchable
  in the tray menu (Auto / Русский / English).
- **Restart** item in the tray menu — brings the app back if Windows killed it.
- Settings (enabled, switch method, Shift+Caps mode, language) are persisted in
  `HKCU\Software\caps-switch` and survive restarts.

### Fixed
- Tray icon is restored after Explorer restarts (`TaskbarCreated`).
- Key auto-repeat no longer triggers repeated layout switches while CapsLock is held.
- Settings are written asynchronously, so the tray menu never blocks.

### Install
```powershell
caps-switch.exe --install              # normal privileges
caps-switch.exe --install --elevated   # highest privileges (works in elevated windows too)
```
