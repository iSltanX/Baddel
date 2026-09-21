//! The handful of strings that live outside the webview.
//!
//! The tray menu and the notice panel are native, so they cannot read
//! `src/lib/i18n/*.json`. This table mirrors the `menu.*` and `hud.*` keys there;
//! the two are kept in step by hand.

use crate::settings::Language;

pub struct Strings {
    pub name: &'static str,
    pub ready: &'static str,
    pub needs_permission: &'static str,
    pub paused: &'static str,
    pub hint: &'static str,
    pub undo: &'static str,
    pub grant_permission: &'static str,
    pub switch_layout: &'static str,
    pub pause: &'static str,
    pub resume: &'static str,
    pub settings: &'static str,
    pub check_updates: &'static str,
    pub quit: &'static str,
    pub hud_undone: &'static str,
    pub hud_blocked: &'static str,
    pub hud_too_long: &'static str,
    pub hud_no_text: &'static str,
}

const AR: Strings = Strings {
    name: "بدّل",
    ready: "جاهز",
    needs_permission: "تحتاج صلاحية",
    paused: "متوقف مؤقتًا",
    hint: "حوّل التحديد أو آخر كلمة",
    undo: "تراجع",
    grant_permission: "امنح الصلاحية…",
    switch_layout: "بدّل لغة لوحة المفاتيح بعد التحويل",
    pause: "أوقف بدّل مؤقتًا",
    resume: "استأنف بدّل",
    settings: "الإعدادات…",
    check_updates: "تحقّق من التحديثات…",
    quit: "إنهاء بدّل",
    hud_undone: "تم التراجع",
    hud_blocked: "حقل محمي — لم يُحوَّل شيء",
    hud_too_long: "التحديد طويل جدًا",
    hud_no_text: "لا يوجد نص لتحويله",
};

const EN: Strings = Strings {
    name: "Baddel",
    ready: "Ready",
    needs_permission: "Needs Permission",
    paused: "Paused",
    hint: "Convert the selection or last word",
    undo: "Undo",
    grant_permission: "Grant Permission…",
    switch_layout: "Switch keyboard layout after converting",
    pause: "Pause Baddel",
    resume: "Resume Baddel",
    settings: "Settings…",
    check_updates: "Check for Updates…",
    quit: "Quit Baddel",
    hud_undone: "Undone",
    hud_blocked: "Protected field — nothing converted",
    hud_too_long: "Selection is too long",
    hud_no_text: "No text to convert",
};

pub fn strings(language: Language) -> &'static Strings {
    match language {
        Language::Ar => &AR,
        Language::En => &EN,
    }
}
