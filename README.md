# caps-switch

<img src="assets/preview-256.png" width="96" alt="caps-switch icon" align="right">

**CapsLock switches the keyboard layout** on Windows — instantly, without the language overlay.

```
CapsLock        → switch input language (Alt+Shift / Ctrl+Shift / Win+Space)
Shift+CapsLock  → normal CapsLock (toggles caps)
```

**Languages:** [English](README.md) · [Русский](README.ru.md)

## Features

- **Instant layout switch** — emulates the system hotkey instead of `Win+Space`, so no
  overlay animation and no delay.
- **CapsLock still works** — hold `Shift` and press CapsLock to toggle caps (optional).
- **Switch method** — `Alt+Shift` (default), `Ctrl+Shift` or `Win+Space`, chosen in the tray menu.
- **Interface language** — Russian and English, auto-detected, switchable in the tray.
- **Tray icon** — enable/disable, switch method, language, Shift+Caps mode, restart, exit.
- **Settings persist** in `HKCU\Software\caps-switch`.
- **Autostart** — user-level (Run) or a scheduled task with highest privileges, so it also
  works in elevated windows.
- **No runtime dependencies** — no .NET, no AutoHotkey; single ~370 KB exe.

## Install

### Option 1 — single-file installer (easiest)

Download **`caps-switch-setup-<version>.exe`** from the [latest release](../../releases/latest)
and run it. The wizard asks explicitly how to autostart:

- **Normal** — starts at logon without elevation (`HKCU\...\Run`);
- **Elevated** — a Task Scheduler job with highest privileges, so CapsLock also works
  while an administrator window is focused (one UAC prompt during installation).

Uninstall via **Settings → Apps → Installed apps → caps-switch**.

### Option 2 — one PowerShell line

```powershell
irm https://raw.githubusercontent.com/svnikolaev/caps-switch/master/install.ps1 | iex
```

With elevated autostart (UAC prompt):

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/svnikolaev/caps-switch/master/install.ps1))) -Elevated
```

Uninstall: `.\install.ps1 -Uninstall`

### Option 3 — manual

1. Download `caps-switch.exe`.
2. Put it in a permanent folder, e.g. `%LOCALAPPDATA%\Programs\caps-switch\`.
3. Run one of:

```powershell
caps-switch.exe --install              # autostart at logon, normal privileges
caps-switch.exe --install --elevated   # scheduled task, highest privileges
caps-switch.exe --status               # what is installed
caps-switch.exe --uninstall            # remove autostart entries
```

## Usage

Right-click the tray icon:

| Item | Meaning |
|---|---|
| Enable / Disable switching | temporarily restore stock CapsLock |
| Switch method | Alt+Shift, Ctrl+Shift or Win+Space |
| Interface language | Auto / Русский / English |
| Shift+CapsLock — normal Caps | allow the native caps toggle |
| Restart | restart the app (e.g. if Windows killed it) |
| Check for updates | compare with the latest GitHub release and install it |
| About | version, GitHub link, license |
| Exit | quit |

Settings are stored in `HKCU\Software\caps-switch` (`Enabled`, `ShiftCaps`, `Mode`, `Language`).

## Why not…?

| Option | Why not |
|---|---|
| `Win+Space` remap (PowerToys) | Windows draws the language overlay — a visible delay on every switch |
| PowerToys remap to `Alt+Shift` | Keyboard Manager does not accept modifier-only shortcuts as a target |
| Grave accent (`` ` ``) as the hotkey | CapsLock starts inserting `` ` `` into text fields |
| `caps-min` helpers | tiny unsigned binary with a keyboard hook, no source, known to drop the hook |

## Documentation

- [Architecture](docs/architecture.md) — how the hook, tray icon and settings work
- [Troubleshooting](docs/troubleshooting.md) — common problems and fixes

## Build

Requires a [Rust toolchain](https://rustup.rs/).

```powershell
# MSVC toolchain (Visual Studio Build Tools with C++ workload)
cargo build --release

# or GNU toolchain (MinGW-w64, e.g. WinLibs)
cargo +stable-x86_64-pc-windows-gnu build --release
```

Installer (optional, needs [Inno Setup 6](https://jrsoftware.org/isinfo.php)):

```powershell
cargo build --release
ISCC.exe /DMyAppVersion=0.3.2 installer\caps-switch.iss
```

Result: `dist\caps-switch-setup-0.3.2.exe`.

## Limitations

- A keyboard hook does not see keystrokes while an **elevated** window is focused, unless
  caps-switch runs elevated (`--install --elevated`).
- Turning off the Shift+CapsLock option dedicates CapsLock to layout switching — caps can no
  longer be toggled from the keyboard.
- The executable is **not code-signed**: antivirus heuristics and SmartScreen may warn.

## License

MIT — see [LICENSE](LICENSE). The bundled installer is built with Inno Setup (BSD-like license).
