//! The handful of strings that live outside the webview.
//!
//! The tray menu and the notice panel are native, so they cannot read
//! `src/lib/i18n/*.json`. This table mirrors the `menu.*`, `hud.*`, `updateDialog.*` keys and `report.title` there;
//! the two are kept in step by hand.

use crate::settings::Language;

pub struct Strings {
    pub name: &'static str,
    pub ready: &'static str,
    pub needs_permission: &'static str,
    pub paused: &'static str,
    /// `{time}`
    pub paused_until: &'static str,
    pub hint: &'static str,
    pub undo: &'static str,
    /// The undo item when the text is too long to show in the menu.
    pub undo_last: &'static str,
    /// Offered after an undo did not go through; the original follows it in the menu.
    pub copy_original: &'static str,
    pub grant_permission: &'static str,
    pub switch_layout: &'static str,
    pub convert_line: &'static str,
    pub to_arabic: &'static str,
    pub to_latin: &'static str,
    /// The pause submenu's title; its items follow.
    pub pause: &'static str,
    pub pause_for_15: &'static str,
    pub pause_for_hour: &'static str,
    pub pause_until_resume: &'static str,
    pub resume: &'static str,
    pub exclude_current: &'static str,
    pub settings: &'static str,
    pub report_problem: &'static str,
    /// The report window's title.
    pub report_title: &'static str,
    pub check_updates: &'static str,
    /// `{version}`
    pub install_update: &'static str,
    pub quit: &'static str,
    pub hud_undone: &'static str,
    pub hud_blocked: &'static str,
    pub hud_too_long: &'static str,
    pub hud_no_text: &'static str,
    pub hud_unchanged: &'static str,
    pub hud_failed: &'static str,
    pub hud_no_layouts: &'static str,
    pub hud_no_permission: &'static str,
    /// `{app}`
    pub hud_elsewhere: &'static str,
    pub hud_paused: &'static str,
    /// `{time}`
    pub hud_paused_until: &'static str,
    /// `{app}`
    pub hud_excluded: &'static str,
    /// `{app}`
    pub hud_excluded_now: &'static str,
    pub hud_not_here: &'static str,
    pub hud_undo_failed: &'static str,
    pub hud_copied: &'static str,
    /// A conversion too long to show word for word. `{count}`: see [`characters`].
    pub hud_converted_long: &'static str,
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
    needs_permission: "يحتاج صلاحية",
    paused: "متوقف مؤقتًا",
    paused_until: "متوقف حتى {time}",
    hint: "حوّل التحديد أو آخر كلمة",
    undo: "تراجع",
    undo_last: "تراجع عن آخر تحويل",
    copy_original: "انسخ النص الأصلي",
    grant_permission: "امنح الصلاحية…",
    switch_layout: "بدّل لغة لوحة المفاتيح بعد التحويل",
    convert_line: "حوّل السطر",
    to_arabic: "حوّل إلى العربية",
    to_latin: "حوّل إلى الإنجليزية",
    pause: "أوقف بدّل مؤقتًا",
    pause_for_15: "15 دقيقة",
    pause_for_hour: "ساعة",
    pause_until_resume: "حتى أستأنفه",
    resume: "استأنف بدّل",
    exclude_current: "استثنِ التطبيق الحالي",
    settings: "الإعدادات…",
    report_problem: "أبلغ عن مشكلة…",
    report_title: "أبلغ عن مشكلة",
    check_updates: "تحقّق من التحديثات…",
    install_update: "ثبّت التحديث {version}…",
    quit: "إنهاء بدّل",
    hud_undone: "تم التراجع",
    hud_blocked: "حقل محمي — لم يُحوَّل شيء",
    hud_too_long: "التحديد طويل جدًا",
    hud_no_text: "لا يوجد نص لتحويله",
    hud_unchanged: "لا شيء هنا يحتاج تحويلًا",
    hud_failed: "تعذّر التحويل هنا — حدّد النص ثم أعد المحاولة",
    hud_no_layouts: "تعذّرت قراءة التخطيطات — افتح الإعدادات ← التخطيطات",
    hud_no_permission: "بدّل يحتاج صلاحية — افتح قائمته",
    hud_elsewhere: "التراجع متاح في {app} فقط",
    hud_paused: "بدّل متوقف مؤقتًا",
    hud_paused_until: "أُوقف بدّل حتى {time}",
    hud_excluded: "بدّل لا يعمل في {app}",
    hud_excluded_now: "استُثني {app} — يمكنك إزالته من الإعدادات",
    hud_not_here: "النص المحوَّل ليس هنا — انسخ الأصل من القائمة",
    hud_undo_failed: "تعذّر التراجع هنا — أعد المحاولة",
    hud_copied: "نُسخ",
    hud_converted_long: "تم التحويل — {count}",
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
    paused_until: "Paused until {time}",
    hint: "Convert the selection or last word",
    undo: "Undo",
    undo_last: "Undo Last Conversion",
    copy_original: "Copy Original Text",
    grant_permission: "Grant Permission…",
    switch_layout: "Switch keyboard layout after converting",
    convert_line: "Convert Line",
    to_arabic: "Convert to Arabic",
    to_latin: "Convert to English",
    pause: "Pause Baddel",
    pause_for_15: "For 15 Minutes",
    pause_for_hour: "For 1 Hour",
    pause_until_resume: "Until I Resume",
    resume: "Resume Baddel",
    exclude_current: "Exclude Current App",
    settings: "Settings…",
    report_problem: "Report a Problem…",
    report_title: "Report a Problem",
    check_updates: "Check for Updates…",
    install_update: "Install Update {version}…",
    quit: "Quit Baddel",
    hud_undone: "Undone",
    hud_blocked: "Protected field — nothing converted",
    hud_too_long: "Selection is too long",
    hud_no_text: "No text to convert",
    hud_unchanged: "Nothing here needs converting",
    hud_failed: "Couldn’t convert here — select the text and try again",
    hud_no_layouts: "Couldn’t read your layouts — open Settings → Layouts",
    hud_no_permission: "Baddel needs permission — open its menu",
    hud_elsewhere: "Undo is only available in {app}",
    hud_paused: "Baddel is paused",
    hud_paused_until: "Baddel paused until {time}",
    hud_excluded: "Baddel stays out of {app}",
    hud_excluded_now: "{app} excluded — remove it in Settings",
    hud_not_here: "Converted text isn’t here — copy the original from the menu",
    hud_undo_failed: "Couldn’t undo here — try again",
    hud_copied: "Copied",
    hud_converted_long: "Converted — {count}",
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

/// "419 حرفًا" / "419 characters". Arabic counts its noun by the last two digits: 3–10 take the
/// plural (أحرف), 11–99 the accusative singular (حرفًا), and the rest the singular (حرف).
pub fn characters(language: Language, count: usize) -> String {
    match language {
        Language::En if count == 1 => "1 character".into(),
        Language::En => format!("{count} characters"),
        Language::Ar => {
            let noun = match count % 100 {
                3..=10 => "أحرف",
                11..=99 => "حرفًا",
                _ if count == 2 => "حرفان",
                _ => "حرف",
            };
            format!("{count} {noun}")
        }
    }
}

pub fn strings(language: Language) -> &'static Strings {
    match language {
        Language::Ar => &AR,
        Language::En => &EN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table is kept in step with `src/lib/i18n/*.json` by hand; this is what keeps it honest.
    #[test]
    fn every_string_matches_the_interface_files() {
        for (language, file) in [
            (Language::Ar, include_str!("../../src/lib/i18n/ar.json")),
            (Language::En, include_str!("../../src/lib/i18n/en.json")),
        ] {
            let json: serde_json::Value = serde_json::from_str(file).unwrap();
            let s = strings(language);
            let pairs = [
                ("menu.name", s.name),
                ("menu.status.ready", s.ready),
                ("menu.status.needsPermission", s.needs_permission),
                ("menu.status.paused", s.paused),
                ("menu.status.pausedUntil", s.paused_until),
                ("menu.hint", s.hint),
                ("menu.undo", s.undo),
                ("menu.undoLast", s.undo_last),
                ("menu.copyOriginal", s.copy_original),
                ("menu.grantPermission", s.grant_permission),
                ("menu.switchLayoutAfterConvert", s.switch_layout),
                ("menu.convertLine", s.convert_line),
                ("menu.toArabic", s.to_arabic),
                ("menu.toLatin", s.to_latin),
                ("menu.pause", s.pause),
                ("menu.pauseFor15", s.pause_for_15),
                ("menu.pauseForHour", s.pause_for_hour),
                ("menu.pauseUntilResume", s.pause_until_resume),
                ("menu.resume", s.resume),
                ("menu.excludeCurrent", s.exclude_current),
                ("menu.settings", s.settings),
                ("menu.reportProblem", s.report_problem),
                ("report.title", s.report_title),
                ("menu.checkForUpdates", s.check_updates),
                ("menu.installUpdate", s.install_update),
                ("menu.quit", s.quit),
                ("hud.undo", s.hud_undone),
                ("hud.blocked", s.hud_blocked),
                ("hud.tooLong", s.hud_too_long),
                ("hud.noSelection", s.hud_no_text),
                ("hud.unchanged", s.hud_unchanged),
                ("hud.failed", s.hud_failed),
                ("hud.noLayouts", s.hud_no_layouts),
                ("hud.noPermission", s.hud_no_permission),
                ("hud.elsewhere", s.hud_elsewhere),
                ("hud.paused", s.hud_paused),
                ("hud.pausedUntil", s.hud_paused_until),
                ("hud.excluded", s.hud_excluded),
                ("hud.excludedNow", s.hud_excluded_now),
                ("hud.notHere", s.hud_not_here),
                ("hud.undoFailed", s.hud_undo_failed),
                ("hud.copied", s.hud_copied),
                ("hud.convertedLong", s.hud_converted_long),
                ("updateDialog.availableTitle", s.update_available_title),
                ("updateDialog.availableBody", s.update_available_body),
                ("updateDialog.installAndRestart", s.install_and_restart),
                ("updateDialog.later", s.later),
                ("updateDialog.upToDateTitle", s.up_to_date_title),
                ("updateDialog.upToDateBody", s.up_to_date_body),
                ("updateDialog.checkFailedTitle", s.check_failed_title),
                ("updateDialog.installFailedTitle", s.install_failed_title),
                ("updateDialog.failedBody", s.update_failed_body),
                ("updateDialog.ok", s.ok),
            ];
            for (path, rust) in pairs {
                let value = path.split('.').fold(&json, |v, key| &v[key]);
                assert_eq!(value.as_str(), Some(rust), "{language:?} {path}");
            }
        }
    }

    #[test]
    fn arabic_counts_take_the_right_noun() {
        let ar = |n| characters(Language::Ar, n);
        assert_eq!(ar(5), "5 أحرف");
        assert_eq!(ar(25), "25 حرفًا");
        assert_eq!(ar(100), "100 حرف");
        assert_eq!(ar(103), "103 أحرف");
        assert_eq!(ar(419), "419 حرفًا");
        assert_eq!(ar(1000), "1000 حرف");
        assert_eq!(characters(Language::En, 419), "419 characters");
    }
}
