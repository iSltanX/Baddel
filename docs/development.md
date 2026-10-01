# دليل التطوير — بدّل

المرجع الدائم لمن يعمل على المصدر: البنية، والثوابت التي لا تُكسر، ونظام التصميم والهوية، والفحص والإصدار، وما بقي مفتوحًا. استخدام التطبيق في [README](../README.md)، والخصوصية في [PRIVACY.md](../PRIVACY.md)، وتاريخ الإصدارات في [CHANGELOG.md](../CHANGELOG.md).

**آخر إصدار:** 2.0.1 (2026-10-01). الوسوم المنشورة `v1.0.0` و`v2.0.0` و`v2.0.1`، والإصدارات على GitHub Releases، وصفحة التنزيل من `main` و`/docs` على https://isltanx.github.io/Baddel/.

## البنية

| الجزء | ما فيه |
|---|---|
| `crates/baddel-core/` | المنطق الصِرف: خرائط التخطيطات، والتحويل، وحسم «لا»، وحماية الروابط والرموز التقنية. بلا Tauri ولا macOS، ويُختبر بـ `cargo test -p baddel-core` |
| `src-tauri/src/` | التطبيق: مسار التحويل (`controller.rs`)، والقائمة (`tray.rs`، ونصوصها في `menu_text.rs`)، والإشعار (`hud.rs`)، والاختصارات، والإعدادات وترحيلها، والمحدِّث، والتشخيص (`diagnostics.rs`) والبلاغ (`report.rs`)، والنوافذ (`windows.rs`) |
| `src-tauri/src/sys/` | **كل** `unsafe` واستدعاءات النظام: AX وCGEvent وNSPasteboard وCarbon TIS والصور والساعة، خلف واجهات آمنة |
| `src/` | الواجهة (Svelte 5 + TypeScript): الإعدادات والترحيب ونافذة البلاغ. عرض فقط عبر `invoke` والأحداث؛ و`src/lib/mock.ts` يحاكي أوامر Rust في `npm run dev` |
| `server/report-worker/` | Cloudflare Worker عام لقناة البلاغات المشتركة (القسم «الإبلاغ») |
| `design/icon/` | أصول أيقونة D2 المُصدَّرة من Figma وسكربت بنائها |
| `docs/` | صفحة التنزيل (GitHub Pages) وأصولها، ولقطات README، وهذا الدليل، و[التوقيع](signing.md)، و[مصفوفة الاختبار](test-matrix.md) |
| `scripts/` | الفحص والبناء والتوقيع والإصدار واللقطات واختبار الجهاز |

**مسار التحويل** (`controller.rs`): الحراسة (صلاحية، إيقاف، استثناء، إدخال آمن، حقل كلمة مرور) ← التحديد ← المدّ بالضغطة الثانية ← آخر كلمة. القراءة والكتابة عبر AX حيث يكشف التطبيق نطاق النص؛ فإن تجاهل كتابة AX كُتب باللصق مع مراقبة التحديد حتى ينطوي ثم تُستعاد الحافظة؛ وإلا فمسار المفاتيح (⌘C ثم لصق). مسار كل تطبيق وأبطأ إجابة ⌘C فيه يُحفظان في الذاكرة طوال الجلسة. التراجع مربوط بالتطبيق والعنصر، ولا يُعرض نص تحويل أطول من 24 حرفًا في القائمة أو الإشعار.

## ثوابت لا تُكسر

- **الخصوصية:** لا يُسجَّل ولا يُحفظ أي نص محوَّل؛ آخر تحويل في الذاكرة 30 ثانية للتراجع. الحافظة تعود كما كانت إلا إن نُسخ شيء أحدث أثناء التحويل، وما يكتبه بدّل فيها موسوم مؤقتًا وسريًّا. لا Input Monitoring.
- **الشبكة:** للمحدِّث وحده، ولبلاغ يكتبه المستخدم ويرى حمولته كاملة ويؤكّدها. لا نص محوَّل ولا حافظة في أي حمولة (يثبته الاختبار `payload_carries_no_conversion_text`). الواجهة بلا شبكة: CSP مقيَّدة، والخطوط والأصول محلية.
- **العمليات:** القائمة والإشعار أصيلان (tray menu وNSPanel لا يأخذ التركيز). نوافذ الإعدادات والترحيب والبلاغ تُنشأ عند الطلب وتُهدم عند الإغلاق، فلا عملية webview في الخمول. كل الحالة والمنطق في Rust.
- **اللغة:** عربية RTL أولًا مع إنجليزية كاملة. النصوص في `src/lib/i18n/{ar,en}.json`، وجدول القائمة والإشعار في `menu_text.rs` مختبَر مقابلهما.
- **الحد الأدنى:** macOS 13. البناء: Xcode Command Line Tools وRust stable وNode 20+ مع npm.
- **الاعتماديات المسموحة:** إضافات Tauri `global-shortcut` و`autostart` و`updater` و`store` و`process` و`dialog`؛ وللنظام `core-foundation` و`core-graphics` و`objc2` و`objc2-foundation` و`objc2-app-kit` و`accessibility-sys`؛ وللبلاغ `reqwest` (ميزتا `json` و`rustls-no-provider`) و`rustls` (ميزة `ring` لتثبيت مزوّد التشفير)، وكلاهما في البناء أصلًا عبر المحدِّث فلم تدخل حزمة جديدة. أي اعتمادية أخرى قرار للمالك.

## نظام التصميم والهوية

**المرجع الوحيد للشكل:** ملف Figma «Baddel» (https://www.figma.com/design/le5J63MNuS9wV7kN8tYpsX/Badeel)، بصفحاته `00 — Cover` و`01 — Foundations` و`02 — Components` و`03 — Product UI` و`04 — Identity` و`05 — Reporting` و`06 — Onboarding` و`07 — Distribution` و`08 — Landing / Documentation Assets`، و`90 — Archive` لكل ما استُبدل (وهو ليس مرجعًا).

- **Figma أولًا:** أي عنصر أو شاشة أو أصل بصري جديد يُصمَّم في الملف أولًا، بمكوّنات `02 — Components` ورموز A2، بالوضعين واللغتين، ثم يُنفَّذ مطابقًا. الكود والوثائق تحدد الوظيفة والسلوك والقيود، وFigma يحدد الشكل والتباعد والألوان والحالات والنصوص الجديدة. إن خالف إطارٌ متطلبًا وظيفيًا أو أمنيًا فالمتطلب يُتبع ويُصحَّح الإطار.
- **الألوان (A2):** `src/lib/theme.css` يحمل مجموعتي `Color` (فاتح/داكن) و`Metrics` بأسماء الأدوار (`bg` و`text` و`border` و`action` و`accent` و`success|warning|danger` و`overlay` و`control` و`keycap` و`hud`)، ولا Hex في أي مكوّن. القيمة تتغيّر في Figma أولًا ويُعاد قياس تباينها في `A2 — Final Tokens` (AA: 4.5:1 للنص، 3:1 للكبير وغير النصي). `color/brand/*` لأصول الهوية وحدها، و`color/system/*` لا يدخل الكود.
  - النص بلون الهوية يُكتب بـ `text/brand` لا `action/primary`؛ الأخير للتعبئة والعلامات (تباينه نحو 3.2:1).
  - Mint ‎#14B8A6 وCoral ‎#FF6B4A للهوية؛ الواجهة تستعمل `action/primary` و`accent` المشتقين. نقطة Coral مرة واحدة في الشاشة على الأكثر، والإشعار وشريط القوائم أحاديا اللون.
- **المسافات والزوايا:** السلّم `0 2 4 8 12 16 20 24 32 48`، والزوايا `4 6 10 16` والدائرية (`A2 — Spacing & Radius`).
- **الحالات:** الحالة الواحدة لها نغمة وأيقونة ونص واحد أينما ظهرت، ومرجعها `A2 — States`.
- **الخطوط:** Cairo Bold (700) وحده للعلامة والعناوين، وAlmarai (400/700) لبقية النص، واللاتينية بخط النظام. مضمَّنة (OFL). Cairo SemiBold أُزيل ولا يعود. الشعار الكتابي لا يُكتب نصًّا حيًّا.
- **الهوية D2 «النقطة تقرّر»:** مسطحة بلا تدرّج ولا ظل، والفاتح والداكن نظام واحد. أيقونة التطبيق، ورموز الشريط الثلاثة `tray*.png` (Idle وPaused وNeeds Permission، Template أحادية)، وأيقونات الإشعار `src-tauri/icons/hud/`، كلها مُصدَّرة من Figma وتُبنى بخطوات [design/icon/README.md](../design/icon/README.md)، ولا تُرسم من الكود ولا تُعدَّل يدويًا.
- **أصول README والصفحة:** الرأس، والمعاينة الاجتماعية، ودليل «افتح على أي حال»، وخلفية DMG مُصدَّرة من `07 — Distribution` و`08 — Landing / Documentation Assets`. ولقطات `docs/screenshots/` يلتقطها `scripts/capture.sh` من بناء debug.

## الإبلاغ

- **القناة:** مستودع `iSltanX/app-reports` الخاص، وREADME فيه هو العقد (v1). بدّل يرسل `"product": "baddel"` إلى `https://app-reports.isultantf.workers.dev/v1/reports`.
- **الحمولة** تُبنى مرة في Rust عند المعاينة، و«أرسل» و«انسخ البلاغ» يستعملان القيمة نفسها، مع `Idempotency-Key` لكل حمولة. الوصف حتى 1,000 حرف، وصورة واحدة تُعاد ترميزها إلى ≤ 3MB (المدخل حتى 20MB)، والصورة لا تعبر إلى الواجهة. لا تُقرأ الحافظة إلا حين يلصق المستخدم صورة في نافذة البلاغ.
- **حد المعدل** في الـ Worker: 5 لكل IP في الساعة و100 في اليوم، دون إعفاء لبلاغات `test` (العلم يضعه العميل).
- **التشغيل:** `server/report-worker/`: `npm ci && npm test && ./scripts/deploy.sh` (يقرأ `.env` في الجذر، ولا يطبع قيمة). في debug: `BADDEL_REPORT_ENDPOINT=http://127.0.0.1:9/v1/reports` لمسار الفشل، واختبار القناة الحيّ `BADDEL_LIVE_REPORT=1 cargo test -p baddel live_report -- --ignored --nocapture`.

## الفحص والاختبار

```bash
./scripts/check.sh                  # اختبارات Rust، وclippy بلا تحذير، وsvelte-check، وبناء الواجهة
python3 scripts/check-docs.py       # روابط README وCHANGELOG وPRIVACY وصفحة التنزيل وقواعد RTL (--online للخارجية)
(cd server/report-worker && npm test)
./scripts/dev-build.sh              # حزمة debug موقَّعة في target/debug/bundle/macos/
```

- **متغيرات debug وحده:** `BADDEL_WINDOW` (`settings[:pane]` أو `onboarding[:step]` أو `report` أو `hud:<state>`) و`BADDEL_THEME` و`BADDEL_HUD_CAPTURE` للقطات، و`BADDEL_FORCE_KEYS` لفرض مسار المفاتيح، و`BADDEL_LOG` لسجل النتائج (بلا نص).
- **اللقطات:** `./scripts/dev-build.sh && ./scripts/capture.sh` يعيد لقطات README كلها، ويحفظ إعدادات المستخدم ويعيد تشغيل بدّل المثبَّت بعده.
- **اختبار الجهاز** (`scripts/device-test/`) يرسل ضغطات حقيقية إلى تطبيقات المستخدم، فلا يُشغَّل إلا بطلب صريح من المالك وهو بعيد عن الجهاز. طريقته وبيئته ونتائجه في [test-matrix.md](test-matrix.md).
- **ملاحظة صلاحية:** نسخة تُشغَّل من صدفة طرفية ترث صلاحية تطبيق الطرفية (TCC تحاسب العملية المسؤولة)؛ لاختبار الصلاحية فعلًا: `open -n --env BADDEL_WINDOW=… target/debug/bundle/macos/Baddel.app`.

## الإصدار

المفاتيح وسبب الشهادة الذاتية في [signing.md](signing.md). الخطوات العلنية (الدفع والوسم والنشر) تحتاج موافقة المالك الصريحة في الجلسة نفسها.

1. **بناء:** `./scripts/release.sh <version> "<title>"` يرفع النسخة في `package.json` وقفله و`Cargo.toml` (ومعه `Cargo.lock` بالبناء) و`tauri.conf.json` وصفحة التنزيل، ويبني Universal موقَّعًا، ويجمع في `release/<version>/` الملفات الستة: DMG، وأرشيف المحدِّث وتوقيعه، و`latest.json`، و`install.sh`، و`SHA256SUMS`.
2. **تحقق محلي:** `(cd release/<v> && shasum -a 256 -c SHA256SUMS)`، وافتح DMG للقراءة فقط وتحقق من النافذة والخلفية.
3. **CHANGELOG:** تاريخ الإصدار تحت عنوانه، ثم commit `release: v<version>`.
4. **فحص جاف:** `./scripts/publish.sh <version>` (الملفات، و`latest.json` وتوقيعه، ونظافة `main`، وملاحظات الإصدار).
5. **النشر:** `./scripts/publish.sh <version> --yes` يدفع `main` والوسم وينشئ GitHub Release بالملفات الستة ويتحقق من `latest.json`.
6. **بعد النشر:** `curl -fsSL https://github.com/iSltanX/Baddel/releases/latest/download/install.sh | bash -s -- --dry-run`، ثم التحديث من الإصدار السابق المثبَّت مع بقاء الصلاحية، ثم `python3 scripts/check-docs.py --online`. Pages تُبنى من `main`؛ وإن لم يطلقها الدفع: `gh api -X POST repos/iSltanX/Baddel/pages/builds`.

`release/` مخرج محلي مستثنى من git؛ النسخة المرجعية للملفات المنشورة هي GitHub Releases. صورة المعاينة الاجتماعية (`docs/assets/social-preview.png`) تُرفع يدويًا من إعدادات المستودع عند تغيّرها.

## ما بقي مفتوحًا

- **مصفوفة التوافق (مؤجَّلة):** TextEdit وSafari وChrome وBrave لكل الأوامر الجديدة، ثم VS Code وSlack وDiscord وPages وWord وSpotlight، وأجهزة macOS 13–15 وIntel. الحالات جاهزة في `scripts/device-test/v12.py` و`matrix.py`.
- **مرشّحات غير معتمدة** (تصير متطلبًا بقرار مكتوب من المالك، وشكلها من Figma): الواصلة والفاصلة العليا في حسم «لا» (`high-level` بتخطيط Arabic – PC)؛ تخطي الكلمات الإنجليزية الصحيحة بالقاموس خيارًا (يحتاج قياسًا)؛ قواعد لكل تطبيق؛ «لا تستخدم الحافظة أبدًا» تحت «متقدم»؛ سطر التخطيطات في القائمة؛ وصول البلاغ دون الصورة (تغيير في العقد)؛ `baddel-core` حزمة مستقلة؛ نطاق مخصص لنقطة البلاغ.
- **مستبعد:** النقر المزدوج على ⇧ (يحتاج Input Monitoring)، ولوحة معاينة قبل التحويل (تأخذ التركيز وتبطئ).
- **تصحيحات Figma منتظرة:** نصوص وعلامات مربوطة بـ `action/primary` تُنقل إلى `text/brand` (تسميات `Button` Ghost، و`Recorder` Recording، و`Key` Highlight، و`PaneTab` Selected، و`Badge` Brand، ونسخها في الشاشات)؛ «أبلغ عن مشكلة…» في إطارات `Menu bar · Menus` وفي `Menu / Paused until`؛ قيم خارج السلّم في مكوّنات الترحيب و`Callout` و`HUD` وبطاقة «جرّبها هنا»؛ وأبعاد الإعدادات و«حول» المنفَّذة (حشوة 16/20/20، وأيقونة «حول» 88، و«من الصانع نفسه»).
