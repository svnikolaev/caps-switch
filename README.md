# caps-switch

**CapsLock switches the keyboard layout** on Windows — instantly, without the language overlay.

A tiny, source-available alternative to opaque `caps-min`-style helpers, written in Rust.

```
CapsLock        → switch input language (emulates Alt+Shift)
Shift+CapsLock  → normal CapsLock (toggles caps)
```

## Features

- **Switch method**: `Alt+Shift` (default), `Ctrl+Shift` or `Win+Space` — chosen in the tray menu.
  Emulating `Alt+Shift`/`Ctrl+Shift` avoids the `Win+Space` overlay animation, so switching is instant.
- **CapsLock still works**: hold `Shift` and press CapsLock to toggle caps as usual
  (can be disabled in the tray menu).
- **Interface language**: Russian and English, detected from the system locale,
  switchable in the tray menu (Auto / Русский / English).
- **Tray icon** with a menu: enable/disable switching, switch method, language,
  `Shift+CapsLock` mode, restart, exit.
- **Restart** item — brings the app back if Windows killed it.
- **Settings persist** in `HKCU\Software\caps-switch`.
- **CLI**: install autostart for the current user, optionally as a scheduled task
  with highest privileges (covers elevated windows), plus `--status` and `--uninstall`.
- **No runtime dependencies**: only system DLLs, no .NET, no AutoHotkey.
- **~350 KB**, single `.exe`.

## How it works

`caps-switch` installs a low-level keyboard hook (`WH_KEYBOARD_LL`):

1. On **CapsLock press** (without Shift) the event is swallowed — caps never toggles —
   and `Alt+Shift` is injected with `SendInput`, which Windows treats as the system
   layout-switch hotkey.
2. If injection is refused, it falls back to posting `WM_INPUTLANGCHANGEREQUEST` with the next
   layout to the foreground window. Note: for **elevated** windows both paths are blocked by
   UIPI — see Limitations.
3. On **Shift+CapsLock** the event is passed through untouched, so the normal caps
   toggle happens.
4. Injected events are recognised (`LLKHF_INJECTED`) and skipped, so there is no recursion.

## Requirements

- Windows 10 or Windows 11 (x64)
- Input language hotkey should be `Left Alt+Shift` (default) or `Ctrl+Shift`
  (Settings → Time & language → Typing → Advanced keyboard settings → Input language hot keys)

## Install

Download `caps-switch.exe` from the [latest release](../../releases/latest) and put it
somewhere permanent, e.g. `%LOCALAPPDATA%\Programs\caps-switch\`.

Then choose an autostart mode:

```powershell
# Normal privileges — works everywhere except elevated windows
caps-switch.exe --install

# Highest privileges — also works when an elevated window is focused.
# Run this from an administrator console (or accept the UAC prompt).
caps-switch.exe --install --elevated

caps-switch.exe --status      # what is installed
caps-switch.exe --uninstall   # remove both autostart entries
```

Without arguments the program starts the tray icon and the keyboard hook.

> `--install --elevated` creates a Task Scheduler job `caps-switch` with trigger
> "at logon" and *Run with highest privileges*. Windows does not show a UAC prompt
> for it at logon.

## Build

Requires a [Rust toolchain](https://rustup.rs/).

```powershell
# MSVC toolchain (Visual Studio Build Tools with the C++ workload)
cargo build --release

# GNU toolchain (MinGW-w64, e.g. WinLibs)
cargo +stable-x86_64-pc-windows-gnu build --release
```

Result: `target\release\caps-switch.exe`.

## Limitations

- Like any keyboard hook, it does not see keystrokes while an **elevated** window is
  focused, unless `caps-switch` itself runs elevated (`--install --elevated`). The
  `WM_INPUTLANGCHANGEREQUEST` fallback does not help there — UIPI blocks it too.
- Turning **off** `Shift+CapsLock — normal Caps Lock` in the tray menu means CapsLock is
  fully dedicated to layout switching; there is then no way to toggle caps with the
  keyboard (the hook swallows it).
- The tray icon uses the stock Windows application icon; a custom icon is on the roadmap.
- Only layout switching is affected; IME-specific hotkeys are untouched.
- The executable is **not code-signed**. It installs a global low-level keyboard hook,
  so antivirus heuristics and SmartScreen may warn. Compare the SHA-256 published in the
  release with the downloaded file if you want to be sure.

## Manual test checklist

1. CapsLock switches the layout instantly; caps state does not change.
2. Hold CapsLock for 3 seconds — the layout must switch **once**, not repeatedly.
3. `Shift+CapsLock` toggles caps (with the option enabled).
4. Switch the method in the tray menu (Alt+Shift / Ctrl+Shift / Win+Space) — each works.
5. Change the interface language in the tray menu — the menu and tooltip follow.
6. Restart the app from the tray menu — one icon remains, switching still works.
7. Restart Explorer (`taskkill /f /im explorer.exe` then start it) — the icon returns.
8. Settings survive a reboot (enabled state, method, language).
9. In an elevated app: without `--install --elevated` CapsLock does nothing (expected).

## Why not…?

| Option | Why not |
|---|---|
| `Win+Space` remap (PowerToys) | Windows draws the language overlay with an animation — a visible delay on every switch |
| PowerToys "remap a key" to `Alt+Shift` | Keyboard Manager does not accept modifier-only shortcuts as a target |
| Grave accent (`` ` ``) as the layout hotkey | CapsLock starts inserting `` ` `` into text fields |
| `caps-min` helpers | tiny unsigned binary with a keyboard hook, no source, no version info, silently drops the hook |

## License

MIT — see [LICENSE](LICENSE).

---

## Русский

**CapsLock переключает раскладку** на Windows — мгновенно, без всплывающей плашки.

```
CapsLock        → смена языка (эмуляция Alt+Shift)
Shift+CapsLock  → обычный CapsLock (включить/выключить Caps)
```

### Возможности

- **Способ переключения**: `Alt+Shift` (по умолчанию), `Ctrl+Shift` или `Win+Space` —
  выбор в меню трея. Эмуляция `Alt+Shift`/`Ctrl+Shift` не показывает плашку, поэтому
  переключение мгновенное.
- CapsLock не потерян: `Shift+CapsLock` работает как обычный Caps (можно отключить
  в меню трея).
- **Язык интерфейса**: русский и английский, определяется по системной локали и
  переключается в меню трея (Авто / Русский / English).
- Иконка в трее: включить/выключить переключение, способ переключения, язык,
  режим `Shift+CapsLock`, перезапуск, выход.
- **Перезапуск** — на случай, если Windows завершит процесс.
- Настройки сохраняются в `HKCU\Software\caps-switch`.
- CLI: автозапуск для текущего пользователя, в том числе задачей Планировщика
  с наивысшими правами (работает и в админских окнах), `--status`, `--uninstall`.
- Никаких зависимостей: только системные DLL — без .NET и AutoHotkey.

### Установка

```powershell
caps-switch.exe --install              # обычный автозапуск
caps-switch.exe --install --elevated   # задача с наивысшими правами (из консоли админа)
caps-switch.exe --status
caps-switch.exe --uninstall
```

Без аргументов программа просто запускается: иконка в трее + хук клавиатуры.

### Ограничения

- Как и любой хук, не видит ввод, когда активно **админское** окно, если сама утилита
  запущена без прав администратора (лечится `--install --elevated`). Резервный путь через
  `WM_INPUTLANGCHANGEREQUEST` там тоже не сработает — UIPI блокирует и его.
- Если выключить в трее пункт «Shift+CapsLock — обычный Caps», CapsLock полностью
  отдаётся переключению языка, и включить Caps с клавиатуры будет нечем.
- Иконка в трее пока стандартная системная.
- exe **не подписан**. Утилита ставит глобальный хук клавиатуры, поэтому антивирусные
  эвристики и SmartScreen могут предупреждать. Сверяйте SHA-256 из релиза с файлом.
