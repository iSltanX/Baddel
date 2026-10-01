# أيقونة بدّل — D2 «النقطة تقرّر»

**المصدر الوحيد:** ملف Figma المعتمد، صفحة `04 — Identity`
https://www.figma.com/design/le5J63MNuS9wV7kN8tYpsX/Badeel

كل ما في هذا المجلد وفي `src-tauri/icons/` **مُصدَّر** من هناك أو مبني من المُصدَّر، ولا يُرسم من الكود. ولتغيير الأيقونة: غيّر المكوّن في Figma، ثم أعد التصدير والبناء بالخطوات أدناه.

- مرجع الهوية: `D2 — A1 Final Identity System` (`96:1563`) و`A2 — Identity Handoff` (`96:1834`).
- رمز الشريط: `D2 — Final Menu Bar System` (`105:1701`)، وتصديره في `09 · Export Masters` (`107:1713`).
- الأيقونة السابقة (مفتاحان A/ع) وطبقاتها في `90 — Archive`، وفي تاريخ git قبل تنفيذ A2.

## الملفات

| الملف | ما هو | من أين |
|---|---|---|
| `app-icon-824.png` | المربع النعناعي وحده بلا هوامش، 824 بكسل: أصل كل المقاسات من 64 فما فوق | المكوّن `App Icon / 1024 · Mint` (`95:1482`) بعرض 824 |
| `app-icon-small-26.png` | نسخة الأحجام الصغيرة بعرض 26: أصل مقاس 32 | المكوّن `App Icon / Small ≤32 · Mint` (`125:5211`) |
| `app-icon-small-14.png` | النسخة نفسها بعرض 14: أصل مقاس 16 | المكوّن نفسه |
| `icon-macos-1024.png` | الأيقونة على شبكة macOS: الجسم 824 في وسط لوحة 1024 شفافة | يبنيه `build.swift` |
| `build.swift` | يبني أيقونات الحزمة من الأصول الثلاثة | — |

## الهندسة

- **مسطحة:** لا تدرّج ولا ظل في الرمز ولا في الحاوية ([docs/development.md](../../docs/development.md#نظام-التصميم-والهوية)). macOS يضيف ظل الـ Dock بنفسه.
- **الحاوية:** مربع بزوايا 22.37٪ ونعومة 60٪، بلون الهوية `brand/mint` ‎#14B8A6. الرمز أبيض ونقطته `brand/coral` ‎#FF6B4A.
- **الرمز:** 62٪ من الحاوية، ومركزه البصري منزاح يمينًا 1.2٪ (الـ Handoff).
- **شبكة macOS:** الجسم 824 من 1024 في كل المقاسات (هامش 100 من كل جهة بمقياس 1024)؛ فالمقاس 16 جسمه 14، و32 جسمه 26، و64 جسمه 52، وهكذا.
- **الأحجام الصغيرة (16 و32):** بالـ Small-size Master: جسم الباء خط، والنقطة صلبة، والشبح بشفافية 55٪. من 64 فما فوق بالأصل.

## إعادة التصدير والبناء

1. صدّر من Figma بخلفية شفافة (PNG): `App Icon / 1024 · Mint` بعرض 824، و`App Icon / Small ≤32 · Mint` بعرضي 26 و14، بأسماء الملفات في الجدول.
2. ابنِ أيقونات الحزمة:

```bash
swift design/icon/build.swift
```

يكتب `src-tauri/icons/icon.icns` (عبر `iconutil`)، و`32x32.png` و`128x128.png` و`128x128@2x.png` و`icon.png` و`icon.ico`، و`icon-macos-1024.png` هنا. الصورة داخل الواجهة (`src/assets/app-icon.png`) هي المكوّن نفسه بلا هوامش بعرض 512، وتُستعمل في «حول» بالوضعين: نسخة Mint في الداكن أيضًا، بلا إطار.

## شريط القوائم

`src-tauri/icons/tray.png` و`tray-paused.png` و`tray-needs-permission.png`: حالات المكوّن `Menu Bar / Status Glyph` (Idle وPaused وNeedsPermission)، مُصدَّرة من إطارات `tray` و`tray-paused` و`tray-needs-permission` في `09 · Export Masters` بمقياس @2x (36×36). أسود صِرف مع alpha، والتطبيق يعلنها Template فيلوّنها macOS في الفاتح والداكن وعند الضغط (`src-tauri/src/tray.rs`).

## أيقونات الإشعار

`src-tauri/icons/hud/*.png`: مكوّنات `Icon/*` من `02 — Components` (checkCircle وundo وlock وwarning وtextCursor وkeyboard وaccessibility وpause وnoEye وcheck بعرض 32، وarrowLeft وarrowRight بعرض 28)، Template بالأسود والشفافية يلوّنها `src-tauri/src/hud.rs` بألوان `color/hud/*`.

تُصدَّر من نسخة الأيقونة داخل متغيّر `HUD` الذي يستعملها (16pt) بمقياس 2: تُنسخ النسخة، وتُجعل حدودها سوداء، ثم `exportAsync` PNG، ثم تُحذف النسخة. أُضيفت keyboard وaccessibility وpause وnoEye وcheck في v1.1 بهذه الطريقة، واختبار `every_state_icon_is_a_black_template_at_2x` في `hud.rs` يتحقق من الأبعاد ومن أن كل بكسل أسود.
