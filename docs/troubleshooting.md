# Troubleshooting

## CapsLock does nothing

1. Check the tray icon: right-click → the first item should say "Disable switching"
   (meaning it is enabled).
2. Check the Windows input hotkey: Settings → Time & language → Typing → Advanced
   keyboard settings → Input language hot keys. The switch method chosen in caps-switch
   must match a hotkey that Windows actually uses (`Left Alt+Shift` by default).
3. Another keyboard hook may be swallowing CapsLock (PowerToys Keyboard Manager,
   AutoHotkey scripts, vendor utilities). Close them and retry. Note that
   `opencode`'s context is unrelated — this is a global OS hook.
4. Language layout list: with a single input language installed the switch has nothing
   to switch to. Add a second language.

## It works in normal windows but not in elevated ones

A low-level hook from a non-elevated process does not receive input while an elevated
window has focus (UIPI). Install the autostart as a scheduled task with highest
privileges:

```powershell
caps-switch.exe --install --elevated     # from an administrator console
```

or re-run the installer and tick "Work in elevated windows too", or reinstall choosing
"all users" mode.

## The tray icon disappeared

- The icon is restored automatically after Explorer restarts (`TaskbarCreated`).
- If it is missing right after logon, the app retries every second until Explorer is ready.
- Check hidden tray icons (the "^" chevron) — Windows hides new icons by default.
- Verify the process: `Get-Process caps-switch`.

## Switching shows a big overlay / feels slow

You selected the `Win+Space` method. Windows draws the language flyout for it. Choose
`Alt+Shift` or `Ctrl+Shift` in the tray menu — they switch without an overlay.

## Shift+CapsLock does not toggle caps

Enable "Shift+CapsLock — normal Caps" in the tray menu. With the option disabled
CapsLock is fully dedicated to layout switching.

## Two autostart entries / two icons

`--install` (Run) and `--install --elevated` (scheduled task) are mutually exclusive.
Clean up and install one of them:

```powershell
caps-switch.exe --uninstall
caps-switch.exe --install          # or --install --elevated
```

## Antivirus or SmartScreen warning

The executable is not code-signed and installs a global keyboard hook — heuristics may
flag it. Compare the SHA-256 of the downloaded file with the one published in the release,
or build it yourself from source (`cargo build --release`).

## Reset all settings

```powershell
Remove-Item -Recurse "HKCU:\Software\caps-switch"
```

## Full removal

```powershell
# installer-based
#   Settings → Apps → Installed apps → caps-switch → Uninstall
# script-based
.\install.ps1 -Uninstall
# manual
caps-switch.exe --uninstall
Remove-Item -Recurse "$env:LOCALAPPDATA\Programs\caps-switch"
```

`--uninstall` removes the autostart entries; the settings key is removed by the
installer's uninstaller (or manually, see above).
