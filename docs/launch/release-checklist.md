# قائمة إصدار v1.0.0

الخطوة 5 من المرحلة 6 (EXECUTION.md) بأوامرها، بالترتيب. **كل خطوة معلَّمة 🔓 تحتاج موافقة صريحة من سلطان في الجلسة نفسها**، لأنها علنية أو صعبة التراجع.

## قبل النشر

1. **الطابور فارغ.** «ابدأ المرحلة 7» حتى يفرغ طابور الاختبار في STATUS.md. الاستثناء الوحيد البند 14 (اختبار التحديث)، لأنه يحتاج `latest.json` منشورًا للعموم، فيُختبر في الخطوة 11.
2. **بناء الإصدار:**
   ```bash
   ./scripts/release.sh 1.0.0 "أول إصدار عام"
   ```
   يرفع النسخة في الملفات الثلاثة ويبني `release/1.0.0/`.
3. **اللقطات بالنسخة الجديدة** (لوحتا «عام» و«حول» تعرضان رقم الإصدار). تفتح نوافذ بدّل على الشاشة نحو دقيقتين، والشاشة غير مقفلة:
   ```bash
   ./scripts/dev-build.sh && ./scripts/capture.sh
   ```
   ثم راجع اللقطات: 42 لقطة، و`hud-*-en.png` بنص إنجليزي.
4. **تاريخ الإصدار** في CHANGELOG.md: `## v1.0.0 — لم يُنشر بعد` ← `## v1.0.0 — <تاريخ اليوم>`، واحذف تعليق HTML تحته.
5. **GIF العرض** إن سُجّل: `docs/assets/demo.gif` والتبديل في README (انظر `demo-gif.md`).
6. 🔓 **Commit:** `release: v1.0.0` (النسخة، واللقطات، وCHANGELOG). الدفع في الخطوة 9.
7. **فحص جاف:**
   ```bash
   ./scripts/publish.sh 1.0.0
   ```
   يتحقق من الملفات الأربعة، ومن أن `latest.json` يشير إلى `v1.0.0` ويحمل توقيع الأرشيف، ومن نظافة `main`، وملاحظات الإصدار من CHANGELOG.

## النشر

8. 🔓 **جعل المستودع عامًا:**
   ```bash
   gh repo edit iSltanX/Baddel --visibility public --accept-visibility-change-consequences
   gh repo edit iSltanX/Baddel --add-topic macos,menu-bar-app,arabic,keyboard-layout,tauri,rust,svelte --homepage https://github.com/iSltanX/Baddel/releases/latest
   ```
9. 🔓 **الوسم والإصدار:**
   ```bash
   ./scripts/publish.sh 1.0.0 --yes
   ```
   يدفع `main` والوسم `v1.0.0`، وينشئ GitHub Release بالملفات الأربعة (DMG، والأرشيف، وتوقيعه، و`latest.json`)، ثم يتحقق أن `latest.json` صار متاحًا.
10. **صورة المعاينة الاجتماعية** (يدويًا، فـGitHub لا يتيحها عبر API): **Settings ← General ← Social preview ← Edit ← Upload an image…** ← `docs/assets/social-preview.png` (1280×640).

## بعد النشر (معيار قبول المرحلة 6)

11. **اختبار التحديث** (البند 14، في المرحلة 7): ثبّت `release/0.9.1/Baddel_0.9.1_universal.dmg`، وامنح الصلاحية، ثم «تحقّق من التحديثات…» ← 1.0.0. يجب أن تبقى الصلاحية، وألا يظهر تحذير Gatekeeper.
12. **على جهاز آخر أو حساب نظيف:** تنزيل DMG من صفحة الإصدار ← تثبيت ← تحذير Gatekeeper وطريقة الفتح الموثَّقة ← الترحيب ← الصلاحية ← تحويل ناجح.
13. **README على GitHub:** كل الروابط تعمل (ومنها الشارات وروابط الأقسام)، والصور تظهر في الوضعين الفاتح والداكن.
14. **منشور الإطلاق** (`announcement.md`): ينشره سلطان بنفسه.
15. STATUS.md: المرحلة 6 ✅.
