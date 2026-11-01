//! caps-switch — CapsLock переключает раскладку через Alt+Shift.
//!
//! Возможности:
//!   • CapsLock (без Shift) — переключение раскладки (эмуляция Alt+Shift);
//!   • Shift+CapsLock — обычное поведение CapsLock (включить/выключить Caps);
//!   • иконка в трее с меню: включить/выключить, режим Shift+Caps, выход;
//!   • CLI: --install [--elevated], --uninstall, --status, --help.
//!
//! Сборка: cargo build --release

#![windows_subsystem = "windows"]

use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyboardLayout, GetKeyboardLayoutList, SendInput, INPUT, INPUT_KEYBOARD,
    KEYBDINPUT, KEYEVENTF_KEYUP, VK_CAPITAL, VK_LMENU, VK_LSHIFT, VK_SHIFT,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CheckMenuItem, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    SetWindowsHookExW, TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG,
    WNDCLASSW, WH_KEYBOARD_LL, WM_APP, WM_COMMAND, WM_DESTROY, WM_INPUTLANGCHANGEREQUEST, WM_KEYDOWN,
    WM_KEYUP, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, HWND_MESSAGE, IDI_APPLICATION, MF_CHECKED,
    MF_SEPARATOR, MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
};

const LLKHF_INJECTED: u32 = 0x10;
const TRAY_ID: u32 = 1;
const WM_TRAY: u32 = WM_APP + 1;

const MENU_TOGGLE: usize = 1001;
const MENU_SHIFT_CAPS: usize = 1002;
const MENU_EXIT: usize = 1003;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "caps-switch";
const TASK_NAME: &str = "caps-switch";

static ENABLED: AtomicBool = AtomicBool::new(true);
static SHIFT_CAPS: AtomicBool = AtomicBool::new(true);

// ---------------------------------------------------------------- переключение

unsafe fn send_alt_shift() -> bool {
    let mut inputs: [INPUT; 4] = zeroed();
    let sequence = [
        (VK_LMENU, false),
        (VK_LSHIFT, false),
        (VK_LSHIFT, true),
        (VK_LMENU, true),
    ];

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

        if !injected && kb.vkCode == VK_CAPITAL as u32 && ENABLED.load(Ordering::Relaxed) {
            let shift_down = SHIFT_CAPS.load(Ordering::Relaxed)
                && (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;

            // Shift+CapsLock — родное поведение: просто пропускаем событие.
            if !shift_down {
                let is_down =
                    wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
                let is_up = wparam == WM_KEYUP as usize || wparam == WM_SYSKEYUP as usize;

                if is_down {
                    if !send_alt_shift() {
                        switch_layout_via_message();
                    }
                    return 1;
                }
                if is_up {
                    return 1;
                }
            }
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}

// ---------------------------------------------------------------- трей

unsafe fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn update_tray(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_TIP;
    let enabled = ENABLED.load(Ordering::Relaxed);
    let tip = if enabled {
        "caps-switch: CapsLock = смена языка"
    } else {
        "caps-switch: выключено"
    };
    for (dst, src) in nid.szTip.iter_mut().zip(wide(tip)) {
        *dst = src;
    }
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }

    let enabled = ENABLED.load(Ordering::Relaxed);
    let shift_caps = SHIFT_CAPS.load(Ordering::Relaxed);

    let item_enable = wide(if enabled {
        "Отключить переключение"
    } else {
        "Включить переключение"
    });
    let item_shift = wide("Shift+CapsLock — обычный Caps");

    AppendMenuW(menu, MF_STRING, MENU_TOGGLE, item_enable.as_ptr());
    AppendMenuW(menu, MF_STRING, MENU_SHIFT_CAPS, item_shift.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
    let item_exit = wide("Выход");
    AppendMenuW(menu, MF_STRING, MENU_EXIT, item_exit.as_ptr());

    if shift_caps {
        CheckMenuItem(menu, MENU_SHIFT_CAPS as u32, MF_CHECKED);
    }

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

    match cmd as usize {
        MENU_TOGGLE => {
            let now = !ENABLED.load(Ordering::Relaxed);
            ENABLED.store(now, Ordering::Relaxed);
            update_tray(hwnd);
        }
        MENU_SHIFT_CAPS => {
            let now = !SHIFT_CAPS.load(Ordering::Relaxed);
            SHIFT_CAPS.store(now, Ordering::Relaxed);
        }
        MENU_EXIT => {
            PostQuitMessage(0);
        }
        _ => {}
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            if lparam as u32 == WM_RBUTTONUP {
                show_menu(hwnd);
            }
            0
        }
        WM_COMMAND => {
            // На случай, если меню когда-нибудь будет вызываться без TPM_RETURNCMD.
            match wparam & 0xFFFF {
                MENU_TOGGLE => {
                    let now = !ENABLED.load(Ordering::Relaxed);
                    ENABLED.store(now, Ordering::Relaxed);
                    update_tray(hwnd);
                }
                MENU_SHIFT_CAPS => {
                    let now = !SHIFT_CAPS.load(Ordering::Relaxed);
                    SHIFT_CAPS.store(now, Ordering::Relaxed);
                }
                MENU_EXIT => PostQuitMessage(0),
                _ => {}
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

    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
    for (dst, src) in nid.szTip.iter_mut().zip(wide("caps-switch: CapsLock = смена языка")) {
        *dst = src;
    }
    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
        // Не смогли поставить иконку — работаем без неё, хук всё равно нужен.
    }

    let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), null_mut(), 0);
    if hook.is_null() {
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
                "/create",
                "/tn",
                TASK_NAME,
                "/tr",
                &exe,
                "/sc",
                "onlogon",
                "/rl",
                "highest",
                "/f",
            ],
        );
        if ok {
            println!("Установлено: задача Планировщика «{TASK_NAME}» (при входе, наивысшие права).");
            println!("exe: {exe}");
        } else {
            eprintln!(
                "Не удалось создать задачу. Запусти из консоли администратора:\n  {exe} --install --elevated"
            );
            std::process::exit(1);
        }
    } else {
        let quoted = format!("\"{exe}\"");
        let ok = run(
            "reg",
            &[
                "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &quoted, "/f",
            ],
        );
        if ok {
            println!("Установлено: автозапуск в HKCU\\...\\Run (обычные права).");
            println!("exe: {exe}");
            println!("Для работы в админских окнах: {exe} --install --elevated");
        } else {
            eprintln!("Не удалось записать автозапуск в реестр.");
            std::process::exit(1);
        }
    }
}

fn uninstall() {
    let run_removed = run("reg", &["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
    let task_removed = run("schtasks", &["/delete", "/tn", TASK_NAME, "/f"]);

    println!(
        "Автозапуск в реестре: {}",
        if run_removed { "удалён" } else { "не найден" }
    );
    println!(
        "Задача Планировщика: {}",
        if task_removed { "удалена" } else { "не найдена" }
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

    println!("реестр (HKCU Run):        {}", if has_reg { "включён" } else { "нет" });
    println!("задача (наивысшие права): {}", if has_task { "включена" } else { "нет" });
    println!("exe: {}", exe_path());
}

fn help() {
    println!("caps-switch — CapsLock переключает раскладку (эмуляция Alt+Shift).\n");
    println!("Использование:");
    println!("  caps-switch                        запустить (иконка в трее + хук)");
    println!("  caps-switch --install              автозапуск при входе, обычные права");
    println!("  caps-switch --install --elevated   автозапуск при входе, наивысшие права");
    println!("  caps-switch --uninstall            убрать автозапуск");
    println!("  caps-switch --status               показать состояние автозапуска");
    println!("  caps-switch --help                 эта справка\n");
    println!("Shift+CapsLock — обычное поведение CapsLock (включает/выключает Caps).");
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
        // Один экземпляр: второй запуск сразу выходит.
        let name = wide("Global\\caps-switch-singleton");
        let _ = CreateMutexW(null_mut(), 0, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS && args.is_empty() {
            std::process::exit(0);
        }

        if args.is_empty() {
            run_tray();
        }

        prepare_console();
    }

    match args[0].as_str() {
        "--install" => install(args.iter().any(|a| a == "--elevated")),
        "--uninstall" => uninstall(),
        "--status" => status(),
        "--help" | "-h" => help(),
        other => {
            eprintln!("Неизвестный аргумент: {other}\n");
            help();
            std::process::exit(64);
        }
    }
}
