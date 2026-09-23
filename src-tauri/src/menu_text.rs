//! The handful of strings that live outside the webview.
//!
//! The tray menu and the notice panel are native, so they cannot read
//! `src/lib/i18n/*.json`. This table mirrors the `menu.*`, `hud.*` and `updateDialog.*` keys there;
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
    /// `{version}`
    pub install_update: &'static str,
    pub quit: &'static str,
    pub hud_undone: &'static str,
    pub hud_blocked: &'static str,
    pub hud_too_long: &'static str,
    pub hud_no_text: &'static str,
    pub update_available_title: &'static str,
    /// `{version}`, `{current}`
    pub update_available_body: &'static str,
    pub install_and_restart: &'static str,
    pub later: &'static str,
    pub up_to_date_title: &'static str,
    /// `{current}`
    pub up_to_date_body: &'static str,
    pub check_failed_title: &'static str,
    pub install_failed_title: &'static str,
    pub update_failed_body: &'static str,
    pub ok: &'static str,
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
    install_update: "ثبّت التحديث {version}…",
    quit: "إنهاء بدّل",
    hud_undone: "تم التراجع",
    hud_blocked: "حقل محمي — لم يُحوَّل شيء",
    hud_too_long: "التحديد طويل جدًا",
    hud_no_text: "لا يوجد نص لتحويله",
    update_available_title: "يتوفر إصدار جديد من بدّل",
    update_available_body: "الإصدار {version} متاح، ولديك {current}. يُعاد تشغيل بدّل بعد التثبيت.",
    install_and_restart: "ثبّت وأعد التشغيل",
    later: "لاحقًا",
    up_to_date_title: "بدّل محدَّث",
    up_to_date_body: "لديك أحدث إصدار ({current}).",
    check_failed_title: "تعذّر التحقق من التحديثات",
    install_failed_title: "تعذّر تثبيت التحديث",
    update_failed_body: "تحقّق من اتصالك بالإنترنت، ثم حاول مرة أخرى.",
    ok: "حسنًا",
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
    install_update: "Install Update {version}…",
    quit: "Quit Baddel",
    hud_undone: "Undone",
    hud_blocked: "Protected field — nothing converted",
    hud_too_long: "Selection is too long",
    hud_no_text: "No text to convert",
    update_available_title: "A New Version of Baddel Is Available",
    update_available_body: "Version {version} is available — you have {current}. Baddel restarts after installing.",
    install_and_restart: "Install and Restart",
    later: "Later",
    up_to_date_title: "Baddel Is Up to Date",
    up_to_date_body: "You have the latest version ({current}).",
    check_failed_title: "Couldn’t Check for Updates",
    install_failed_title: "Couldn’t Install the Update",
    update_failed_body: "Check your internet connection, then try again.",
    ok: "OK",
};

pub fn strings(language: Language) -> &'static Strings {
    match language {
        Language::Ar => &AR,
        Language::En => &EN,
    }
}
