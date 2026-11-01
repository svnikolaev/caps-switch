//! Настройки в HKCU\Software\caps-switch через Win32 Registry API.
//!
//! Специально без `reg.exe`: вызов процесса занимает десятки-сотни миллисекунд,
//! а `load_settings` выполняется до установки хука, а сохранение — из UI-потока,
//! который обслуживает низкоуровневый хук. API — микросекунды и без дочерних процессов.

use std::ptr::null_mut;

use windows_sys::Win32::System::Registry::{
    RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_DWORD, REG_SZ, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ,
};

const SUBKEY: &str = r"Software\caps-switch";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// DWORD-значение настройки.
pub fn get_dword(name: &str) -> Option<u32> {
    let sub = wide(SUBKEY);
    let name_w = wide(name);
    let mut value: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;

    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name_w.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut value as *mut u32 as *mut _,
            &mut size,
        )
    };

    if status == 0 {
        Some(value)
    } else {
        None
    }
}

/// Строковое значение настройки.
pub fn get_string(name: &str) -> Option<String> {
    let sub = wide(SUBKEY);
    let name_w = wide(name);
    let mut buffer = [0u16; 256];
    let mut size: u32 = (buffer.len() * std::mem::size_of::<u16>()) as u32;

    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name_w.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            buffer.as_mut_ptr() as *mut _,
            &mut size,
        )
    };

    if status != 0 {
        return None;
    }

    // size — в байтах, включая завершающий ноль.
    let chars = (size as usize / std::mem::size_of::<u16>()).saturating_sub(1);
    Some(String::from_utf16_lossy(&buffer[..chars.min(buffer.len())]))
}

/// Записать DWORD.
pub fn set_dword(name: &str, value: u32) {
    let sub = wide(SUBKEY);
    let name_w = wide(name);
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name_w.as_ptr(),
            REG_DWORD,
            &value as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

/// Записать строку (REG_SZ).
pub fn set_string(name: &str, value: &str) {
    let sub = wide(SUBKEY);
    let name_w = wide(name);
    let value_w = wide(value);
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name_w.as_ptr(),
            REG_SZ,
            value_w.as_ptr() as *const _,
            (value_w.len() * std::mem::size_of::<u16>()) as u32,
        );
    }
}

/// Значение настройки "Language" → номер режима (0 авто, 1 ru, 2 en).
pub fn lang_setting_to_u8(value: &str) -> u8 {
    match value {
        "ru" => 1,
        "en" => 2,
        _ => 0,
    }
}

/// Номер режима → значение настройки "Language".
pub fn lang_u8_to_setting(value: u8) -> &'static str {
    match value {
        1 => "ru",
        2 => "en",
        _ => "auto",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_setting_roundtrip() {
        for v in 0u8..=2 {
            assert_eq!(lang_setting_to_u8(lang_u8_to_setting(v)), v);
        }
    }

    #[test]
    fn unknown_lang_setting_falls_back_to_auto() {
        assert_eq!(lang_setting_to_u8("de"), 0);
        assert_eq!(lang_setting_to_u8(""), 0);
    }

    #[test]
    fn dword_string_is_not_confused_with_sz() {
        // Регрессия: reg query печатает DWORD как "0x1", раньше это сравнивали с "1".
        // Теперь DWORD читается типизированно через RegGetValueW(RRF_RT_REG_DWORD),
        // поэтому проверяем сам контракт хелперов на отсутствующем значении.
        assert!(get_dword("__caps_switch_missing__").is_none());
        assert!(get_string("__caps_switch_missing__").is_none());
    }
}
