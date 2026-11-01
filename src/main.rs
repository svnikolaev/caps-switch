//! caps-switch — CapsLock переключает раскладку выбранным способом.
//!
//! Возможности:
//!   • CapsLock — переключение языка (Alt+Shift / Ctrl+Shift / Win+Space — на выбор);
//!   • Shift+CapsLock — обычное поведение CapsLock (включает/выключает Caps);
//!   • иконка в трее: переключение вкл/выкл, способ, язык (RU/EN/авто), Shift+Caps,
//!     перезапуск, выход;
//!   • настройки в HKCU\Software\caps-switch (через Win32 Registry API);
//!   • CLI: --install [--elevated], --uninstall, --status, --version, --help.
//!
//! Сборка: cargo build --release

#![windows_subsystem = "windows"]

macro_rules! say {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stdout(), $($arg)*);
    }};
}

macro_rules! esay {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

mod i18n;
mod settings;

use std::mem::{size_of, zeroed};
use std::process::Command;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

use i18n::{msgs, system_lang, Lang};

use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject,
};
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyboardLayout, GetKeyboardLayoutList, SendInput, INPUT, INPUT_KEYBOARD,
    KEYBDINPUT, KEYEVENTF_KEYUP, VK_CAPITAL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_SHIFT,
    VK_SPACE,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    GetWindowThreadProcessId, KillTimer, LoadIconW, MessageBoxW, PostMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetTimer, SetWindowsHookExW,
    TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, HICON, IDI_APPLICATION, KBDLLHOOKSTRUCT,
    MB_ICONERROR, MB_OK, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WH_KEYBOARD_LL, WM_APP, WM_COMMAND, WM_DESTROY,
    WM_INPUTLANGCHANGEREQUEST, WM_KEYDOWN, WM_KEYUP, WM_NULL, WM_RBUTTONUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP, WM_TIMER, WNDCLASSW, WS_POPUP,
};

const LLKHF_INJECTED: u32 = 0x10;
const TRAY_ID: u32 = 1;
const WM_TRAY: u32 = WM_APP + 1;
const TIMER_TRAY: usize = 1;

const MENU_TOGGLE: usize = 1001;
const MENU_SHIFT_CAPS: usize = 1002;
const MENU_RESTART: usize = 1003;
const MENU_EXIT: usize = 1004;
const MENU_MODE_BASE: usize = 1100;
const MENU_LANG_BASE: usize = 1200;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "caps-switch";
const TASK_NAME: &str = "caps-switch";
const MUTEX_NAME: &str = "Local\\caps-switch-singleton";
const EXIT_EVENT_NAME: &str = "Local\\caps-switch-exit";

static ENABLED: AtomicBool = AtomicBool::new(true);
static SHIFT_CAPS: AtomicBool = AtomicBool::new(true);
static MODE: AtomicU8 = AtomicU8::new(0);
static LANG_SETTING: AtomicU8 = AtomicU8::new(0);
static CAPS_DOWN: AtomicBool = AtomicBool::new(false);
static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum SwitchMode {
    AltShift = 0,
    CtrlShift = 1,
    WinSpace = 2,
}

impl SwitchMode {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::CtrlShift,
            2 => Self::WinSpace,
            _ => Self::AltShift,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::AltShift => "Alt+Shift",
            Self::CtrlShift => "Ctrl+Shift",
            Self::WinSpace => "Win+Space",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::AltShift => "alt_shift",
            Self::CtrlShift => "ctrl_shift",
            Self::WinSpace => "win_space",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "ctrl_shift" => Self::CtrlShift,
            "win_space" => Self::WinSpace,
            _ => Self::AltShift,
        }
    }

    fn keys(self) -> (u16, u16) {
        match self {
            Self::AltShift => (VK_LMENU, VK_LSHIFT),
            Self::CtrlShift => (VK_LCONTROL, VK_LSHIFT),
            Self::WinSpace => (VK_LWIN, VK_SPACE),
        }
    }
}

fn effective_lang() -> Lang {
    match LANG_SETTING.load(Ordering::Relaxed) {
        1 => Lang::Ru,
        2 => Lang::En,
        _ => system_lang(),
    }
}

// ---------------------------------------------------------------- переключение

fn is_key_held(vk: u16) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

/// Эмулирует выбранное сочетание.
/// Модификатор, который пользователь уже держит, не инжектится — иначе его «отпустит»
/// наш `KEYEVENTF_KEYUP`, и ввод останется без Shift/Ctrl/Alt.
unsafe fn send_switch_keys(mode: SwitchMode) -> bool {
    let (modifier, key) = mode.keys();
    let send_modifier = !is_key_held(modifier);

    let mut sequence: Vec<(u16, bool)> = Vec::with_capacity(4);
    if send_modifier {
        sequence.push((modifier, false));
    }
    sequence.push((key, false));
    sequence.push((key, true));
    if send_modifier {
        sequence.push((modifier, true));
    }

    let mut inputs: Vec<INPUT> = vec![zeroed(); sequence.len()];
    for (i, (vk, keyup)) in sequence.iter().enumerate() {
        inputs[i].r#type = INPUT_KEYBOARD;
        inputs[i].Anonymous.ki = KEYBDINPUT {
            wVk: *vk,
            wScan: 0,
            dwFlags: if *keyup { KEYEVENTF_KEYUP } else { 0 },
            time: 0,
            dwExtraInfo: 0,
        };
    }

    SendInput(
        inputs.len() as u32,
        inputs.as_mut_ptr(),
        size_of::<INPUT>() as i32,
    ) == inputs.len() as u32
}

/// Индекс следующей раскладки относительно текущей раскладки активного окна.
fn next_layout_index(current: isize, all: &[isize]) -> Option<usize> {
    if all.len() <= 1 {
        return None;
    }
    Some(
        all.iter()
            .position(|h| *h == current)
            .map(|i| (i + 1) % all.len())
            .unwrap_or(0),
    )
}

/// Резервный путь: попросить активное окно переключиться на следующую раскладку.
/// Работает только для окон того же или более низкого integrity level (UIPI).
unsafe fn switch_layout_via_message() {
    let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
    if hwnd.is_null() {
        return;
    }

    let count = GetKeyboardLayoutList(0, null_mut());
    if count <= 1 {
        return;
    }

    let mut layouts = vec![null_mut::<std::ffi::c_void>() as isize; count as usize];
    let filled = GetKeyboardLayoutList(count, layouts.as_mut_ptr() as *mut _);
    if filled <= 1 {
        return;
    }

    // Раскладка именно того потока, которому адресовано сообщение.
    let tid = GetWindowThreadProcessId(hwnd, null_mut());
    let current = GetKeyboardLayout(tid) as isize;

    if let Some(index) = next_layout_index(current, &layouts) {
        PostMessageW(hwnd, WM_INPUTLANGCHANGEREQUEST, 0, layouts[index] as LPARAM);
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Контракт LL-хука: nCode < 0 — вызывать следующий хук без обработки.
    if code < 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
    let injected = kb.flags & LLKHF_INJECTED != 0;
    let is_caps = kb.vkCode == VK_CAPITAL as u32 || kb.scanCode == 0x3A;

    if !injected && is_caps {
        let is_down = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
        let is_up = wparam == WM_KEYUP as usize || wparam == WM_SYSKEYUP as usize;

        // Выключено или зажат Shift — обычное поведение CapsLock.
        if !ENABLED.load(Ordering::Relaxed)
            || (SHIFT_CAPS.load(Ordering::Relaxed) && is_key_held(VK_SHIFT))
        {
            CAPS_DOWN.store(false, Ordering::SeqCst);
            return CallNextHookEx(null_mut(), code, wparam, lparam);
        }

        if is_up {
            CAPS_DOWN.store(false, Ordering::SeqCst);
            return 1;
        }

        if is_down {
            // Автоповтор и потерянный keyup не должны переключать язык повторно.
            if CAPS_DOWN.swap(true, Ordering::SeqCst) {
                return 1;
            }
            let mode = SwitchMode::from_u8(MODE.load(Ordering::Relaxed));
            if !send_switch_keys(mode) {
                switch_layout_via_message();
            }
            return 1;
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}

// ---------------------------------------------------------------- настройки

fn load_settings() {
    if let Some(v) = settings::get_dword("Enabled") {
        ENABLED.store(v != 0, Ordering::Relaxed);
    }
    if let Some(v) = settings::get_dword("ShiftCaps") {
        SHIFT_CAPS.store(v != 0, Ordering::Relaxed);
    }
    if let Some(v) = settings::get_string("Mode") {
        MODE.store(SwitchMode::from_str(&v) as u8, Ordering::Relaxed);
    }
    if let Some(v) = settings::get_string("Language") {
        LANG_SETTING.store(settings::lang_setting_to_u8(&v), Ordering::Relaxed);
    }
}

/// Сохраняем настройки в отдельном потоке, чтобы не блокировать UI и хук.
fn save_settings_async() {
    let enabled = ENABLED.load(Ordering::Relaxed);
    let shift_caps = SHIFT_CAPS.load(Ordering::Relaxed);
    let mode = SwitchMode::from_u8(MODE.load(Ordering::Relaxed));
    let lang = settings::lang_u8_to_setting(LANG_SETTING.load(Ordering::Relaxed));

    std::thread::spawn(move || {
        settings::set_dword("Enabled", if enabled { 1 } else { 0 });
        settings::set_dword("ShiftCaps", if shift_caps { 1 } else { 0 });
        settings::set_string("Mode", mode.as_str());
        settings::set_string("Language", lang);
    });
}

// ---------------------------------------------------------------- иконка

static ICON_BYTES: &[u8] = include_bytes!("../assets/icon.ico");

/// Иконка приложения из встроенного .ico: берём ближайший к 32x32 PNG-энтри.
/// При любой проблеме — стандартная системная иконка.
unsafe fn load_app_icon() -> HICON {
    let bytes = ICON_BYTES;
    if bytes.len() < 6 {
        return LoadIconW(null_mut(), IDI_APPLICATION);
    }

    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let mut best: Option<(i32, usize, usize)> = None;

    for i in 0..count {
        let base = 6 + i * 16;
        if base + 16 > bytes.len() {
            break;
        }
        let width = bytes[base] as i32;
        let width = if width == 0 { 256 } else { width };
        let size = u32::from_le_bytes([
            bytes[base + 8],
            bytes[base + 9],
            bytes[base + 10],
            bytes[base + 11],
        ]) as usize;
        let offset = u32::from_le_bytes([
            bytes[base + 12],
            bytes[base + 13],
            bytes[base + 14],
            bytes[base + 15],
        ]) as usize;

        if offset + size > bytes.len() {
            continue;
        }

        let score = (width - 32).abs();
        let better = match best {
            None => true,
            Some((best_score, _, _)) => score < best_score,
        };
        if better {
            best = Some((score, offset, size));
        }
    }

    if let Some((_, offset, size)) = best {
        let ptr = bytes[offset..].as_ptr();
        let icon = CreateIconFromResourceEx(ptr, size as u32, 1, 0x0003_0000, 0, 0, 0);
        if !icon.is_null() {
            return icon;
        }
    }

    LoadIconW(null_mut(), IDI_APPLICATION)
}

// ---------------------------------------------------------------- трей

unsafe fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn tray_tooltip() -> String {
    let m = msgs(effective_lang());
    if ENABLED.load(Ordering::Relaxed) {
        format!(
            "{} ({})",
            m.tip_on,
            SwitchMode::from_u8(MODE.load(Ordering::Relaxed)).title()
        )
    } else {
        m.tip_off.to_string()
    }
}

unsafe fn add_tray_icon(hwnd: HWND) -> bool {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = load_app_icon();
    for (dst, src) in nid.szTip.iter_mut().zip(wide(&tray_tooltip())) {
        *dst = src;
    }
    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}

unsafe fn update_tray(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_TIP;
    for (dst, src) in nid.szTip.iter_mut().zip(wide(&tray_tooltip())) {
        *dst = src;
    }
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }

    let m = msgs(effective_lang());
    let enabled = ENABLED.load(Ordering::Relaxed);
    let shift_caps = SHIFT_CAPS.load(Ordering::Relaxed);
    let current_mode = SwitchMode::from_u8(MODE.load(Ordering::Relaxed));
    let current_lang = LANG_SETTING.load(Ordering::Relaxed);

    let item_enable = wide(if enabled { m.toggle_off } else { m.toggle_on });
    AppendMenuW(menu, MF_STRING, MENU_TOGGLE, item_enable.as_ptr());

    let mode_menu = CreatePopupMenu();
    for mode in [
        SwitchMode::AltShift,
        SwitchMode::CtrlShift,
        SwitchMode::WinSpace,
    ] {
        let text = wide(mode.title());
        let flags = MF_STRING | if mode == current_mode { MF_CHECKED } else { 0 };
        AppendMenuW(
            mode_menu,
            flags,
            MENU_MODE_BASE + mode as usize,
            text.as_ptr(),
        );
    }
    let mode_title = wide(m.mode);
    AppendMenuW(menu, MF_POPUP, mode_menu as usize, mode_title.as_ptr());

    let lang_menu = CreatePopupMenu();
    for (value, title) in [(0u8, m.lang_auto), (1, m.lang_ru), (2, m.lang_en)] {
        let text = wide(title);
        let flags = MF_STRING | if value == current_lang { MF_CHECKED } else { 0 };
        AppendMenuW(
            lang_menu,
            flags,
            MENU_LANG_BASE + value as usize,
            text.as_ptr(),
        );
    }
    let lang_title = wide(m.language);
    AppendMenuW(menu, MF_POPUP, lang_menu as usize, lang_title.as_ptr());

    let item_shift = wide(m.shift_caps);
    AppendMenuW(
        menu,
        MF_STRING | if shift_caps { MF_CHECKED } else { 0 },
        MENU_SHIFT_CAPS,
        item_shift.as_ptr(),
    );

    AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());

    let item_restart = wide(m.restart);
    AppendMenuW(menu, MF_STRING, MENU_RESTART, item_restart.as_ptr());

    let item_exit = wide(m.exit);
    AppendMenuW(menu, MF_STRING, MENU_EXIT, item_exit.as_ptr());

    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);

    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
        pt.x,
        pt.y,
        0,
        hwnd,
        null_mut(),
    );
    // Закрывает меню при клике мимо него.
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);

    handle_command(hwnd, cmd as usize);
}

unsafe fn handle_command(hwnd: HWND, cmd: usize) {
    match cmd {
        MENU_TOGGLE => {
            ENABLED.store(!ENABLED.load(Ordering::Relaxed), Ordering::Relaxed);
            CAPS_DOWN.store(false, Ordering::SeqCst);
            save_settings_async();
            update_tray(hwnd);
        }
        MENU_SHIFT_CAPS => {
            SHIFT_CAPS.store(!SHIFT_CAPS.load(Ordering::Relaxed), Ordering::Relaxed);
            save_settings_async();
            update_tray(hwnd);
        }
        MENU_RESTART => restart_app(),
        MENU_EXIT => PostQuitMessage(0),
        n if (MENU_MODE_BASE..MENU_MODE_BASE + 3).contains(&n) => {
            MODE.store((n - MENU_MODE_BASE) as u8, Ordering::Relaxed);
            save_settings_async();
            update_tray(hwnd);
        }
        n if (MENU_LANG_BASE..MENU_LANG_BASE + 3).contains(&n) => {
            LANG_SETTING.store((n - MENU_LANG_BASE) as u8, Ordering::Relaxed);
            save_settings_async();
            update_tray(hwnd);
        }
        _ => {}
    }
}

unsafe fn restart_app() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe).arg("--restarted").spawn();
    }
    PostQuitMessage(0);
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_msg = TASKBAR_CREATED_MSG.load(Ordering::Relaxed);
    if taskbar_msg != 0 && msg == taskbar_msg {
        add_tray_icon(hwnd);
        return 0;
    }

    match msg {
        WM_TRAY => {
            if lparam as u32 == WM_RBUTTONUP {
                show_menu(hwnd);
            }
            0
        }
        WM_COMMAND => {
            handle_command(hwnd, wparam & 0xFFFF);
            0
        }
        WM_TIMER => {
            // Explorer ещё не готов — пробуем вернуть иконку ещё раз.
            if wparam == TIMER_TRAY && add_tray_icon(hwnd) {
                KillTimer(hwnd, TIMER_TRAY);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn fatal(message: &str) {
    unsafe {
        let text = wide(message);
        let title = wide("caps-switch");
        MessageBoxW(
            null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

unsafe fn run_tray(exit_event: HANDLE) -> ! {
    let hinstance: HINSTANCE = GetModuleHandleW(null_mut());
    let class_name = wide("caps_switch_tray_window");

    let mut wc: WNDCLASSW = zeroed();
    wc.lpfnWndProc = Some(wnd_proc);
    wc.hInstance = hinstance;
    wc.lpszClassName = class_name.as_ptr();
    RegisterClassW(&wc);

    // Скрытое top-level окно (не message-only): только такие получают broadcast
    // TaskbarCreated, по которому Windows сообщает о перезапуске Explorer.
    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        class_name.as_ptr(),
        WS_POPUP,
        0,
        0,
        0,
        0,
        null_mut(),
        null_mut(),
        hinstance,
        null_mut(),
    );

    if hwnd.is_null() {
        fatal("Не удалось создать окно для иконки в трее.");
        std::process::exit(1);
    }

    let taskbar_created = wide("TaskbarCreated");
    TASKBAR_CREATED_MSG.store(
        RegisterWindowMessageW(taskbar_created.as_ptr()),
        Ordering::Relaxed,
    );

    if !add_tray_icon(hwnd) {
        // Explorer мог ещё не подняться при автозапуске — пробуем по таймеру.
        SetTimer(hwnd, TIMER_TRAY, 1000, None);
    }

    let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), null_mut(), 0);
    if hook.is_null() {
        fatal("Не удалось установить хук клавиатуры.");
        std::process::exit(1);
    }

    let mut msg: MSG = zeroed();
    loop {
        let ret = GetMessageW(&mut msg, null_mut(), 0, 0);
        if ret <= 0 {
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    UnhookWindowsHookEx(hook);
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    Shell_NotifyIconW(NIM_DELETE, &nid);
    DestroyWindow(hwnd);

    // Сообщаем новому экземпляру, что можно стартовать.
    if !exit_event.is_null() {
        SetEvent(exit_event);
    }

    std::process::exit(0);
}

// ---------------------------------------------------------------- установка

fn exe_path() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

fn run(cmd: &str, args: &[&str]) -> bool {
    match Command::new(cmd).args(args).output() {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

fn has_task() -> bool {
    run("schtasks", &["/query", "/tn", TASK_NAME])
}

fn has_run_value() -> bool {
    run("reg", &["query", RUN_KEY, "/v", RUN_VALUE])
}

fn install(elevated: bool) {
    let exe = exe_path();

    if elevated {
        // Путь с пробелами должен быть в кавычках, иначе schtasks обрежет аргумент.
        let tr = format!("\"{exe}\"");
        let ok = run(
            "schtasks",
            &[
                "/create", "/tn", TASK_NAME, "/tr", &tr, "/sc", "onlogon", "/rl", "highest", "/f",
            ],
        );
        if !ok {
            esay!(
                "Не удалось создать задачу. Запусти из консоли администратора:\n  {exe} --install --elevated"
            );
            std::process::exit(1);
        }
        // Задача и запись в Run одновременно не нужны.
        if has_run_value() && run("reg", &["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]) {
            say!("Удалён дублирующий автозапуск из HKCU\\...\\Run.");
        }
        say!("Установлено: задача Планировщика «{TASK_NAME}» (при входе, наивысшие права).");
        say!("exe: {exe}");
    } else {
        if has_task() {
            say!("Внимание: уже есть задача Планировщика «{TASK_NAME}». Она главнее записи в реестре.");
        }
        let quoted = format!("\"{exe}\"");
        let ok = run(
            "reg",
            &[
                "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &quoted, "/f",
            ],
        );
        if !ok {
            esay!("Не удалось записать автозапуск в реестр.");
            std::process::exit(1);
        }
        say!("Установлено: автозапуск в HKCU\\...\\Run (обычные права).");
        say!("exe: {exe}");
        say!("Для работы в админских окнах: {exe} --install --elevated");
    }
}

fn uninstall() {
    let run_removed = run("reg", &["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
    let task_removed = run("schtasks", &["/delete", "/tn", TASK_NAME, "/f"]);

    say!(
        "Автозапуск в реестре: {}",
        if run_removed {
            "удалён"
        } else {
            "не найден"
        }
    );
    say!(
        "Задача Планировщика: {}",
        if task_removed {
            "удалена"
        } else {
            "не найдена"
        }
    );
    say!("Настройки в HKCU\\Software\\caps-switch сохранены (удаляются вручную).");
}

fn status() {
    say!("version:              {}", env!("CARGO_PKG_VERSION"));
    say!(
        "registry (HKCU Run):  {}",
        if has_run_value() { "enabled" } else { "none" }
    );
    say!(
        "scheduled task:       {}",
        if has_task() { "enabled" } else { "none" }
    );
    say!("exe: {}", exe_path());
}

fn help() {
    say!(
        "caps-switch {} — CapsLock switches the keyboard layout.\n",
        env!("CARGO_PKG_VERSION")
    );
    say!("Usage:");
    say!("  caps-switch                        run (tray icon + keyboard hook)");
    say!("  caps-switch --install              autostart at logon, normal privileges");
    say!("  caps-switch --install --elevated   autostart at logon, highest privileges");
    say!("  caps-switch --uninstall            remove autostart entries");
    say!("  caps-switch --status               show autostart state");
    say!("  caps-switch --version              print version");
    say!("  caps-switch --help                 this help\n");
    say!("Switch method, interface language and Shift+CapsLock mode are in the tray menu.");
}

// ---------------------------------------------------------------- консоль

unsafe fn prepare_console() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        AllocConsole, AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS,
        STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };

    let current = GetStdHandle(STD_OUTPUT_HANDLE);
    if !current.is_null() && current != INVALID_HANDLE_VALUE {
        return;
    }

    if AttachConsole(ATTACH_PARENT_PROCESS) == 0 && AllocConsole() == 0 {
        return;
    }

    let out = GetStdHandle(STD_OUTPUT_HANDLE);
    let err = GetStdHandle(STD_ERROR_HANDLE);
    if !out.is_null() && out != INVALID_HANDLE_VALUE {
        SetStdHandle(STD_OUTPUT_HANDLE, out);
    }
    if !err.is_null() && err != INVALID_HANDLE_VALUE {
        SetStdHandle(STD_ERROR_HANDLE, err);
    }
}

// ---------------------------------------------------------------- main

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    unsafe {
        // Меню трея позиционируется по физическим координатам.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    load_settings();

    let restarted = args.iter().any(|a| a == "--restarted");

    unsafe {
        // Новый экземпляр после «Перезапустить» ждёт, пока старый освободит мьютекс.
        if restarted {
            let ev_name = wide(EXIT_EVENT_NAME);
            let ev = CreateEventW(null_mut(), 1, 0, ev_name.as_ptr());
            if !ev.is_null() {
                WaitForSingleObject(ev, 5000u32);
            }
        }

        let name = wide(MUTEX_NAME);
        let _mutex = CreateMutexW(null_mut(), 0, name.as_ptr());
        let last_error = GetLastError();
        if last_error == ERROR_ALREADY_EXISTS && args.is_empty() && !restarted {
            std::process::exit(0);
        }
    }

    if args.is_empty() || restarted {
        unsafe {
            let ev_name = wide(EXIT_EVENT_NAME);
            let ev = CreateEventW(null_mut(), 1, 0, ev_name.as_ptr());
            run_tray(ev);
        }
    }

    unsafe { prepare_console() };

    match args[0].as_str() {
        "--install" => install(args.iter().any(|a| a == "--elevated")),
        "--uninstall" => uninstall(),
        "--status" => status(),
        "--version" | "-V" => say!("{}", env!("CARGO_PKG_VERSION")),
        "--help" | "-h" => help(),
        "--restarted" => {}
        other => {
            esay!("Unknown argument: {other}\n");
            help();
            std::process::exit(64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_mode_roundtrip() {
        for mode in [
            SwitchMode::AltShift,
            SwitchMode::CtrlShift,
            SwitchMode::WinSpace,
        ] {
            assert_eq!(SwitchMode::from_str(mode.as_str()) as u8, mode as u8);
            assert_eq!(SwitchMode::from_u8(mode as u8) as u8, mode as u8);
        }
    }

    #[test]
    fn unknown_mode_falls_back() {
        assert_eq!(
            SwitchMode::from_str("nope") as u8,
            SwitchMode::AltShift as u8
        );
        assert_eq!(SwitchMode::from_u8(42) as u8, SwitchMode::AltShift as u8);
    }

    #[test]
    fn next_layout_wraps_and_handles_unknown_current() {
        let all = [10isize, 20, 30];
        assert_eq!(next_layout_index(10, &all), Some(1));
        assert_eq!(next_layout_index(30, &all), Some(0));
        assert_eq!(next_layout_index(99, &all), Some(0));
        assert_eq!(next_layout_index(10, &[10]), None);
    }

    #[test]
    fn tooltip_strings_fit_128_wchars() {
        for lang in [Lang::Ru, Lang::En] {
            let m = msgs(lang);
            for s in [
                m.tip_on,
                m.tip_off,
                m.mode,
                m.language,
                m.shift_caps,
                m.restart,
                m.exit,
            ] {
                assert!(!s.is_empty());
                assert!(s.encode_utf16().count() < 120, "too long: {s}");
            }
        }
    }

    #[test]
    fn embedded_icon_has_valid_entries() {
        let bytes = ICON_BYTES;
        assert!(bytes.len() > 6, "icon is empty");
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 1, "not an ICO");

        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        assert!(count >= 5, "expected several sizes, got {count}");

        for i in 0..count {
            let base = 6 + i * 16;
            assert!(base + 16 <= bytes.len());
            let size = u32::from_le_bytes([
                bytes[base + 8],
                bytes[base + 9],
                bytes[base + 10],
                bytes[base + 11],
            ]) as usize;
            let offset = u32::from_le_bytes([
                bytes[base + 12],
                bytes[base + 13],
                bytes[base + 14],
                bytes[base + 15],
            ]) as usize;
            assert!(size > 0, "entry {i} is empty");
            assert!(offset + size <= bytes.len(), "entry {i} out of bounds");
        }
    }
}
