//! caps-switch — CapsLock переключает раскладку выбранным способом.
//!
//! Возможности:
//!   • CapsLock — переключение языка (Alt+Shift / Ctrl+Shift / Win+Space — на выбор);
//!   • Shift+CapsLock — обычное поведение CapsLock (включает/выключает Caps);
//!   • иконка в трее: включить/выключить, способ переключения, язык (RU/EN/авто),
//!     режим Shift+Caps, перезапуск, выход;
//!   • настройки сохраняются в HKCU\Software\caps-switch;
//!   • CLI: --install [--elevated], --uninstall, --status, --help.
//!
//! Сборка: cargo build --release

#![windows_subsystem = "windows"]

mod i18n;

use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use i18n::{msgs, system_lang, Lang};

use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
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
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, PostMessageW,
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetWindowsHookExW,
    TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, WNDCLASSW,
    WH_KEYBOARD_LL, WM_APP, WM_COMMAND, WM_DESTROY, WM_INPUTLANGCHANGEREQUEST, WM_KEYDOWN, WM_KEYUP,
    WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, HWND_MESSAGE, IDI_APPLICATION, MF_CHECKED, MF_POPUP,
    MF_SEPARATOR, MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
};

const LLKHF_INJECTED: u32 = 0x10;
const TRAY_ID: u32 = 1;
const WM_TRAY: u32 = WM_APP + 1;

const MENU_TOGGLE: usize = 1001;
const MENU_SHIFT_CAPS: usize = 1002;
const MENU_RESTART: usize = 1003;
const MENU_EXIT: usize = 1004;
const MENU_MODE_BASE: usize = 1100; // 1100 + mode
const MENU_LANG_BASE: usize = 1200; // 1200 + 0(auto)/1(ru)/2(en)

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "caps-switch";
const TASK_NAME: &str = "caps-switch";
const SETTINGS_KEY: &str = r"HKCU\Software\caps-switch";

static ENABLED: AtomicBool = AtomicBool::new(true);
static SHIFT_CAPS: AtomicBool = AtomicBool::new(true);
static MODE: AtomicU8 = AtomicU8::new(0);
static LANG_SETTING: AtomicU8 = AtomicU8::new(0); // 0 = авто, 1 = ru, 2 = en
static LAST_CAPS_DOWN: AtomicBool = AtomicBool::new(false);
static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, PartialEq, Eq)]
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

unsafe fn send_switch_keys(mode: SwitchMode) -> bool {
    let (modifier, key) = mode.keys();
    let sequence = [(modifier, false), (key, false), (key, true), (modifier, true)];

    let mut inputs: [INPUT; 4] = zeroed();
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
        sequence.len() as u32,
        inputs.as_mut_ptr(),
        size_of::<INPUT>() as i32,
    ) == sequence.len() as u32
}

/// Резервный путь: попросить активное окно переключиться на следующую раскладку.
unsafe fn switch_layout_via_message() {
    let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
    if hwnd.is_null() {
        return;
    }

    let count = GetKeyboardLayoutList(0, null_mut());
    if count <= 1 {
        return;
    }

    let mut layouts = vec![null_mut(); count as usize];
    let filled = GetKeyboardLayoutList(count, layouts.as_mut_ptr());
    if filled <= 1 {
        return;
    }

    let current = GetKeyboardLayout(0);
    let index = layouts
        .iter()
        .position(|h| *h == current)
        .map(|i| (i + 1) % layouts.len())
        .unwrap_or(0);

    PostMessageW(
        hwnd,
        WM_INPUTLANGCHANGEREQUEST,
        0,
        layouts[index] as isize as LPARAM,
    );
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        let injected = kb.flags & LLKHF_INJECTED != 0;

        if !injected && kb.vkCode == VK_CAPITAL as u32 {
            let is_down = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
            let is_up = wparam == WM_KEYUP as usize || wparam == WM_SYSKEYUP as usize;

            let shift_down = SHIFT_CAPS.load(Ordering::Relaxed)
                && (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;

            // Выключено или зажат Shift — обычное поведение CapsLock.
            if !ENABLED.load(Ordering::Relaxed) || shift_down {
                LAST_CAPS_DOWN.store(false, Ordering::Relaxed);
                return CallNextHookEx(null_mut(), code, wparam, lparam);
            }

            if is_up {
                LAST_CAPS_DOWN.store(false, Ordering::Relaxed);
                return 1;
            }

            if is_down {
                // Автоповтор зажатой клавиши не должен переключать язык много раз.
                if LAST_CAPS_DOWN.swap(true, Ordering::Relaxed) {
                    return 1;
                }
                let mode = SwitchMode::from_u8(MODE.load(Ordering::Relaxed));
                if !send_switch_keys(mode) {
                    switch_layout_via_message();
                }
                return 1;
            }
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}

// ---------------------------------------------------------------- настройки

fn reg_query(name: &str) -> Option<String> {
    let out = Command::new("reg")
        .args(["query", SETTINGS_KEY, "/v", name])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains(name))?;
    let value = line
        .split("REG_SZ")
        .nth(1)
        .or_else(|| line.split("REG_DWORD").nth(1))?;
    Some(value.trim().to_string())
}

fn reg_set(name: &str, value: &str, kind: &str) {
    let _ = Command::new("reg")
        .args([
            "add", SETTINGS_KEY, "/v", name, "/t", kind, "/d", value, "/f",
        ])
        .output();
}

fn load_settings() {
    if let Some(v) = reg_query("Enabled") {
        ENABLED.store(v == "1", Ordering::Relaxed);
    }
    if let Some(v) = reg_query("ShiftCaps") {
        SHIFT_CAPS.store(v == "1", Ordering::Relaxed);
    }
    if let Some(v) = reg_query("Mode") {
        MODE.store(SwitchMode::from_str(&v) as u8, Ordering::Relaxed);
    }
    if let Some(v) = reg_query("Language") {
        let lang = match v.as_str() {
            "ru" => 1,
            "en" => 2,
            _ => 0,
        };
        LANG_SETTING.store(lang, Ordering::Relaxed);
    }
}

/// Сохраняем настройки в отдельном потоке: меню трея не должно ждать reg.exe.
fn save_settings_async() {
    let enabled = ENABLED.load(Ordering::Relaxed);
    let shift_caps = SHIFT_CAPS.load(Ordering::Relaxed);
    let mode = SwitchMode::from_u8(MODE.load(Ordering::Relaxed));
    let lang = match LANG_SETTING.load(Ordering::Relaxed) {
        1 => "ru",
        2 => "en",
        _ => "auto",
    };

    std::thread::spawn(move || {
        reg_set("Enabled", if enabled { "1" } else { "0" }, "REG_DWORD");
        reg_set("ShiftCaps", if shift_caps { "1" } else { "0" }, "REG_DWORD");
        reg_set("Mode", mode.as_str(), "REG_SZ");
        reg_set("Language", lang, "REG_SZ");
    });
}

// ---------------------------------------------------------------- трей

unsafe fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn add_tray_icon(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);

    let m = msgs(effective_lang());
    let tip = if ENABLED.load(Ordering::Relaxed) {
        format!("{} ({})", m.tip_on, SwitchMode::from_u8(MODE.load(Ordering::Relaxed)).title())
    } else {
        m.tip_off.to_string()
    };
    for (dst, src) in nid.szTip.iter_mut().zip(wide(&tip)) {
        *dst = src;
    }

    Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn update_tray(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_TIP;

    let m = msgs(effective_lang());
    let tip = if ENABLED.load(Ordering::Relaxed) {
        format!("{} ({})", m.tip_on, SwitchMode::from_u8(MODE.load(Ordering::Relaxed)).title())
    } else {
        m.tip_off.to_string()
    };
    for (dst, src) in nid.szTip.iter_mut().zip(wide(&tip)) {
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

    // Способ переключения.
    let mode_menu = CreatePopupMenu();
    for mode in [SwitchMode::AltShift, SwitchMode::CtrlShift, SwitchMode::WinSpace] {
        let text = wide(mode.title());
        let flags = MF_STRING | if mode == current_mode { MF_CHECKED } else { 0 };
        AppendMenuW(mode_menu, flags, MENU_MODE_BASE + mode as usize, text.as_ptr());
    }
    let mode_title = wide(m.mode);
    AppendMenuW(menu, MF_POPUP, mode_menu as usize, mode_title.as_ptr());

    // Язык интерфейса.
    let lang_menu = CreatePopupMenu();
    let lang_items = [
        (0u8, m.lang_auto),
        (1, m.lang_ru),
        (2, m.lang_en),
    ];
    for (value, title) in lang_items {
        let text = wide(title);
        let flags = MF_STRING | if value == current_lang { MF_CHECKED } else { 0 };
        AppendMenuW(lang_menu, flags, MENU_LANG_BASE + value as usize, text.as_ptr());
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
    DestroyMenu(menu);

    handle_command(hwnd, cmd as usize);
}

fn restart_app() {
    if let Ok(exe) = std::env::current_exe() {
        // --restarted обходит проверку единственного экземпляра, пока старый процесс жив.
        let _ = Command::new(exe).arg("--restarted").spawn();
    }
    unsafe { PostQuitMessage(0) };
}

unsafe fn handle_command(hwnd: HWND, cmd: usize) {
    match cmd {
        MENU_TOGGLE => {
            let now = !ENABLED.load(Ordering::Relaxed);
            ENABLED.store(now, Ordering::Relaxed);
            LAST_CAPS_DOWN.store(false, Ordering::Relaxed);
            save_settings_async();
            update_tray(hwnd);
        }
        MENU_SHIFT_CAPS => {
            let now = !SHIFT_CAPS.load(Ordering::Relaxed);
            SHIFT_CAPS.store(now, Ordering::Relaxed);
            save_settings_async();
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

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Explorer перезапустился — возвращаем иконку.
    if msg != 0 && msg == TASKBAR_CREATED_MSG.load(Ordering::Relaxed) {
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
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn run_tray() -> ! {
    let hinstance: HINSTANCE = GetModuleHandleW(null_mut());
    let class_name = wide("caps_switch_tray_window");

    let mut wc: WNDCLASSW = zeroed();
    wc.lpfnWndProc = Some(wnd_proc);
    wc.hInstance = hinstance;
    wc.lpszClassName = class_name.as_ptr();
    RegisterClassW(&wc);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        class_name.as_ptr(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        null_mut(),
        hinstance,
        null_mut(),
    );

    if hwnd.is_null() {
        std::process::exit(1);
    }

    let taskbar_created = wide("TaskbarCreated");
    TASKBAR_CREATED_MSG.store(RegisterWindowMessageW(taskbar_created.as_ptr()), Ordering::Relaxed);

    add_tray_icon(hwnd);

    let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), null_mut(), 0);
    if hook.is_null() {
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TRAY_ID;
        Shell_NotifyIconW(NIM_DELETE, &nid);
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

fn install(elevated: bool) {
    let exe = exe_path();

    if elevated {
        let ok = run(
            "schtasks",
            &[
                "/create", "/tn", TASK_NAME, "/tr", &exe, "/sc", "onlogon", "/rl", "highest", "/f",
            ],
        );
        if ok {
            println!("Installed: scheduled task «{TASK_NAME}» (at logon, highest privileges).");
            println!("exe: {exe}");
        } else {
            eprintln!(
                "Failed to create the task. Run from an administrator console:\n  {exe} --install --elevated"
            );
            std::process::exit(1);
        }
    } else {
        let quoted = format!("\"{exe}\"");
        let ok = run(
            "reg",
            &["add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &quoted, "/f"],
        );
        if ok {
            println!("Installed: autostart in HKCU\\...\\Run (normal privileges).");
            println!("exe: {exe}");
            println!("For elevated windows as well: {exe} --install --elevated");
        } else {
            eprintln!("Failed to write the autostart registry value.");
            std::process::exit(1);
        }
    }
}

fn uninstall() {
    let run_removed = run("reg", &["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
    let task_removed = run("schtasks", &["/delete", "/tn", TASK_NAME, "/f"]);

    println!(
        "Autostart (registry): {}",
        if run_removed { "removed" } else { "not found" }
    );
    println!(
        "Scheduled task: {}",
        if task_removed { "removed" } else { "not found" }
    );
}

fn status() {
    let has_reg = Command::new("reg")
        .args(["query", RUN_KEY, "/v", RUN_VALUE])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let has_task = Command::new("schtasks")
        .args(["/query", "/tn", TASK_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    println!("registry (HKCU Run):  {}", if has_reg { "enabled" } else { "none" });
    println!("scheduled task:       {}", if has_task { "enabled" } else { "none" });
    println!("exe: {}", exe_path());
}

fn help() {
    println!("caps-switch — CapsLock switches the keyboard layout.\n");
    println!("Usage:");
    println!("  caps-switch                        run (tray icon + keyboard hook)");
    println!("  caps-switch --install              autostart at logon, normal privileges");
    println!("  caps-switch --install --elevated   autostart at logon, highest privileges");
    println!("  caps-switch --uninstall            remove autostart entries");
    println!("  caps-switch --status               show autostart state");
    println!("  caps-switch --help                 this help\n");
    println!("Switch method, interface language and Shift+CapsLock mode are in the tray menu.");
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

    load_settings();

    unsafe {
        let name = wide("Global\\caps-switch-singleton");
        let _ = CreateMutexW(null_mut(), 0, name.as_ptr());
        let restarted = args.iter().any(|a| a == "--restarted");
        if GetLastError() == ERROR_ALREADY_EXISTS && args.is_empty() && !restarted {
            std::process::exit(0);
        }

        if args.is_empty() || restarted {
            run_tray();
        }

        prepare_console();
    }

    match args[0].as_str() {
        "--install" => install(args.iter().any(|a| a == "--elevated")),
        "--uninstall" => uninstall(),
        "--status" => status(),
        "--help" | "-h" => help(),
        "--restarted" => {}
        other => {
            eprintln!("Unknown argument: {other}\n");
            help();
            std::process::exit(64);
        }
    }
}
