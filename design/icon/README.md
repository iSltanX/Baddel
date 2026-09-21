# أيقونة بدّل

**المصدر الوحيد:** ملف Figma المعتمد، صفحة `03 · App Icon`
https://www.figma.com/design/le5J63MNuS9wV7kN8tYpsX/Badeel

كل ما في هذا المجلد **مُصدَّر** من هناك. لا يُعدَّل يدويًا ولا يُرسم من الكود: غيّر الـMaster في Figma ثم أعد التصدير.

| الملف | ما هو | من أين |
|---|---|---|
| `icon-macos-1024.png` | الأيقونة بشكل macOS (جسم 824 داخل 1024، مع الظل) — مدخل `tauri icon` | المكوّن `AppIcon / Master 1024` |
| `0-background.svg` · `1-keycap-back.svg` · `2-keycap-front.svg` | طبقات Icon Composer: 1024 كاملة بلا قناع ولا ظلال، والحروف outlines، مرقّمة من الخلف للأمام | الإطار `Export / Icon Composer layers` |
| `icon-flat.svg` | الطبقات الثلاث مركّبة | الإطار نفسه |
| `preview.png` | اختبار المقاسات 512…16، مع مقارنة النسخة الصغيرة | اللوحة `Icon / Sizes` |
| `concepts/` | مفاهيم سابقة **مرفوضة**، محفوظة للمرجع فقط | — |

## الهندسة (موثّقة بالقياسات في لوحة `Icon / Construction grid`)

- جسم 824 داخل 1024 حسب قالب macOS، `r = 185`. المفتاح 400 بـ`r = 88` — النسبة نفسها (≈22%) في الاثنين.
- الإزاحة بين المفتاحين 160 على قطر 45°، والعمق 24 (6%). كل الإحداثيات على شبكة 8px.
- تصحيح بصري: المجموعة مُزاحة (−8, −8) لأن المفتاح الأبيض الأمامي وظلّه يسحبان مركز الثقل إلى أسفل اليمين.
- الطبقات في هذا المجلد هي الـMaster نفسه مكبَّرًا بنسبة 1024/824 ليملأ اللوحة (Icon Composer يضيف القناع والعمق).
- نسخة المقاسات الصغيرة (≤32px): المكوّن `AppIcon / Small ≤32` — مفاتيح 448، بلا «A»، و«ع» أثقل.

## إعادة توليد أيقونات الحزمة

```bash
source scripts/env.sh
npx tauri icon design/icon/icon-macos-1024.png -o /tmp/baddel-icons
cp /tmp/baddel-icons/{32x32.png,128x128.png,128x128@2x.png,icon.icns,icon.ico,icon.png} src-tauri/icons/
```

أيقونات شريط القوائم (`src-tauri/icons/tray*.png`) مُصدَّرة من المكوّن `MenuBarGlyph` بمقياس 2× (32px) داخل لوحة 36px، أسود صِرف مع alpha (template image).
