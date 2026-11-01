# caps-switch

<img src="assets/preview-256.png" width="96" alt="caps-switch icon" align="right">

**CapsLock переключает раскладку** на Windows — мгновенно, без всплывающей плашки.

```
CapsLock        → смена языка (Alt+Shift / Ctrl+Shift / Win+Space)
Shift+CapsLock  → обычный CapsLock (включает/выключает Caps)
```

**Языки:** [English](README.md) · [Русский](README.ru.md)

## Возможности

- **Мгновенное переключение** — эмулируется системное сочетание, а не `Win+Space`, поэтому
  нет анимации плашки и задержки.
- **CapsLock не потерян** — `Shift+CapsLock` работает как обычный Caps (можно отключить).
- **Способ переключения** — `Alt+Shift` (по умолчанию), `Ctrl+Shift` или `Win+Space`,
  выбор в меню трея.
- **Язык интерфейса** — русский и английский, определяется автоматически, переключается в трее.
- **Иконка в трее** — вкл/выкл, способ, язык, режим Shift+Caps, перезапуск, выход.
- **Настройки сохраняются** в `HKCU\Software\caps-switch`.
- **Автозапуск** — обычный (реестр) или задачей Планировщика с наивысшими правами, чтобы
  работало и в админских окнах.
- **Без зависимостей** — ни .NET, ни AutoHotkey; один exe ~370 КБ.

## Установка

### Вариант 1 — однофайловый установщик (проще всего)

Скачай **`caps-switch-setup-<версия>.exe`** из [последнего релиза](../../releases/latest)
и запусти. Мастер явно спросит режим автозапуска:

- **Обычный** — старт при входе без повышения прав (`HKCU\...\Run`);
- **С повышенными правами** — задача Планировщика с наивысшими правами, работает и в
  окнах администратора (один запрос UAC при установке).

Удаление — **Параметры → Приложения → Установленные приложения → caps-switch**.

### Вариант 2 — одна строка PowerShell

```powershell
irm https://raw.githubusercontent.com/svnikolaev/caps-switch/master/install.ps1 | iex
```

С автозапуском от администратора (будет запрос UAC):

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/svnikolaev/caps-switch/master/install.ps1))) -Elevated
```

Удаление: `.\install.ps1 -Uninstall`

### Вариант 3 — вручную

1. Скачай `caps-switch.exe`.
2. Положи в постоянную папку, например `%LOCALAPPDATA%\Programs\caps-switch\`.
3. Выполни одно из:

```powershell
caps-switch.exe --install              # автозапуск при входе, обычные права
caps-switch.exe --install --elevated   # задача Планировщика, наивысшие права
caps-switch.exe --status               # что установлено
caps-switch.exe --uninstall            # убрать автозапуск
```

## Использование

Правый клик по иконке в трее:

| Пункт | Значение |
|---|---|
| Включить / Отключить переключение | временно вернуть штатный CapsLock |
| Способ переключения | Alt+Shift, Ctrl+Shift или Win+Space |
| Язык интерфейса | Авто / Русский / English |
| Shift+CapsLock — обычный Caps | разрешить штатное переключение Caps |
| Перезапустить | поднять процесс заново (если Windows его завершил) |
| Проверить обновления | сверить с последним релизом на GitHub и установить |
| О программе | версия, ссылка на GitHub, лицензия |
| Выход | закрыть приложение |

Настройки хранятся в `HKCU\Software\caps-switch` (`Enabled`, `ShiftCaps`, `Mode`, `Language`).

## Почему не…

| Вариант | Почему нет |
|---|---|
| Ремап `Win+Space` (PowerToys) | Windows рисует плашку выбора языка — заметная задержка |
| Ремап в PowerToys на `Alt+Shift` | Keyboard Manager не принимает сочетания только из модификаторов |
| Тильда (`` ` ``) как хоткей | CapsLock начнёт вставлять `` ` `` в текстовые поля |
| Утилиты типа `caps-min` | крошечный неподписанный бинарник с хуком, без исходников, «отваливается» |

## Документация

- [Архитектура](docs/architecture.ru.md) — как устроены хук, трей и настройки
- [Решение проблем](docs/troubleshooting.ru.md) — частые ситуации и что делать

## Сборка

Нужен [тулчейн Rust](https://rustup.rs/).

```powershell
# MSVC (Visual Studio Build Tools с C++)
cargo build --release

# или GNU (MinGW-w64, например WinLibs)
cargo +stable-x86_64-pc-windows-gnu build --release
```

Установщик (по желанию, нужен [Inno Setup 6](https://jrsoftware.org/isinfo.php)):

```powershell
cargo build --release
ISCC.exe /DMyAppVersion=0.3.2 installer\caps-switch.iss
```

Результат: `dist\caps-switch-setup-0.3.2.exe`.

## Ограничения

- Хук не видит ввод, когда активно **админское** окно, если caps-switch запущен без прав
  администратора (`--install --elevated` решает это).
- Если выключить «Shift+CapsLock — обычный Caps», CapsLock полностью отдаётся переключению
  языка, и включить Caps с клавиатуры будет нечем.
- exe **не подписан**: возможны предупреждения антивируса и SmartScreen.

## Лицензия

MIT — см. [LICENSE](LICENSE). Установщик собран Inno Setup (BSD-подобная лицензия).
