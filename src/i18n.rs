//! Локализация интерфейса: русский и английский.
//!
//! Язык определяется по системной локали (`GetUserDefaultUILanguage`), но его
//! можно переопределить в меню трея (Авто / Русский / English). Выбор хранится
//! в HKCU\Software\caps-switch.

use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

/// Язык интерфейса.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ru,
    En,
}

/// Набор строк для одного языка.
pub struct Msgs {
    pub toggle_on: &'static str,
    pub toggle_off: &'static str,
    pub mode: &'static str,
    pub language: &'static str,
    pub lang_auto: &'static str,
    pub lang_ru: &'static str,
    pub lang_en: &'static str,
    pub shift_caps: &'static str,
    pub restart: &'static str,
    pub exit: &'static str,
    pub tip_on: &'static str,
    pub tip_off: &'static str,
}

const RU: Msgs = Msgs {
    toggle_on: "Включить переключение",
    toggle_off: "Отключить переключение",
    mode: "Способ переключения",
    language: "Язык интерфейса",
    lang_auto: "Авто",
    lang_ru: "Русский",
    lang_en: "English",
    shift_caps: "Shift+CapsLock — обычный Caps",
    restart: "Перезапустить",
    exit: "Выход",
    tip_on: "caps-switch: CapsLock переключает язык",
    tip_off: "caps-switch: выключено",
};

const EN: Msgs = Msgs {
    toggle_on: "Enable switching",
    toggle_off: "Disable switching",
    mode: "Switch method",
    language: "Interface language",
    lang_auto: "Auto",
    lang_ru: "Русский",
    lang_en: "English",
    shift_caps: "Shift+CapsLock — normal Caps Lock",
    restart: "Restart",
    exit: "Exit",
    tip_on: "caps-switch: CapsLock switches layout",
    tip_off: "caps-switch: disabled",
};

/// Язык по умолчанию для текущей системы.
pub fn system_lang() -> Lang {
    let langid = unsafe { GetUserDefaultUILanguage() };
    // Младшие 10 бит — primary language id; 0x19 — русский.
    if langid & 0x03FF == 0x19 {
        Lang::Ru
    } else {
        Lang::En
    }
}

/// Строки для языка.
pub fn msgs(lang: Lang) -> &'static Msgs {
    match lang {
        Lang::Ru => &RU,
        Lang::En => &EN,
    }
}
