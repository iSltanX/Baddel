# بدّل — Baddel · برومبت Figma

> **طريقة الاستخدام:** انسخ ما بين الخطّين أدناه إلى Figma Make (أو First Draft)،
> أو أعطه لـClaude مع موصل Figma ليبني الملف مباشرة.
> البرومبت مكتوب بالإنجليزية لأن أدوات Figma تستجيب لها أدق، ونصوص الواجهة داخله بالعربية جاهزة كما هي.
> الألوان والخطوط اقتراح أولي، عدّلها قبل الإرسال إن أردت هوية مختلفة.

---

## PROMPT

Design the complete UI for **Baddel (بدّل)**, a tiny native **macOS menu bar utility**.

### What the app does
The user typed text with the wrong keyboard layout (e.g. typed `اثممخ` while meaning `hello`, or `sghl` while meaning `سلام`). They press one global shortcut and Baddel converts the selected text — or the last word — in place, in any app, then switches the keyboard language. Fully local, no account, no network. It only needs the macOS Accessibility permission.

### Product personality
Calm, precise, invisible until needed. It belongs to a family of refined **Arabic‑first** indie Mac tools (a clipboard manager, a quiet writing editor, a command utility). Think: Things 3, Raycast settings, native macOS Sonoma/Sequoia panels. **Not** playful, no gradients, no glassmorphism excess, no illustrations of people, no emoji.

### Language & direction — critical
- **Primary UI is Arabic, RTL.** Every screen is designed RTL first: sidebar on the right, toggles on the left of their row, chevrons point left, text right‑aligned.
- Provide an **English LTR mirror** of the Settings › General screen only, to prove the layout mirrors cleanly.
- Keyboard shortcuts, keycaps and Latin sample text stay LTR inside RTL rows.
- Use real Arabic copy provided below — never lorem ipsum, never machine‑looking Arabic.

### Design tokens (create as Figma variables, with Light and Dark modes)

| Token | Light | Dark |
|---|---|---|
| `bg/window` | `#F6F4EF` | `#1B1C1F` |
| `bg/surface` | `#FFFFFF` | `#25272B` |
| `bg/sunken` | `#EDEAE3` | `#16171A` |
| `border/subtle` | `#E2DED5` | `#34373C` |
| `text/primary` | `#1C1917` | `#F2F0EB` |
| `text/secondary` | `#6B665E` | `#A19D95` |
| `accent/primary` | `#3F5673` (slate blue) | `#8FA9C9` |
| `accent/soft` | `#E4EAF1` | `#2A3644` |
| `state/success` | `#5F7F52` | `#93B585` |
| `state/warning` | `#B67F2E` | `#D9A95A` |
| `state/danger` | `#A8493B` | `#D98476` |

- **Type (Google Fonts, use exactly these two families — no other UI font):**
  - **`Cairo`** — display & headings: onboarding titles, section titles, window titles, app name, cover. Weights 600 (SemiBold) and 700 (Bold).
  - **`Almarai`** — everything else: body, row titles, descriptions, buttons, menu items, captions, HUD Arabic text. Weights 400 (Regular) and 700 (Bold); 300 (Light) for large secondary text only. Almarai has no 500/600 — never fake a medium weight.
  - Latin UI text uses the Latin glyphs of the same family as its context (Cairo in headings, Almarai in body) so mixed lines stay visually consistent.
  - Exception: keycaps, shortcuts and conversion samples (`اثممخ → hello`) use a monospace — `SF Mono` (fallback `IBM Plex Mono`) for Latin, with `Almarai` for the Arabic glyphs inside them.
  - Create text styles named `Heading/…` (Cairo) and `Body/…`, `Caption/…` (Almarai).
- **Scale:** 11 caption · 13 body (macOS default) · 15 row title · 20 section title · 28 onboarding title. Line‑height: Cairo headings 1.4 (it has tall ascenders — don't go tighter), Almarai body 1.6, Latin mono 1.4.
- **Radius:** 6 controls · 10 cards · 14 windows/popover · 999 HUD pill.
- **Spacing:** 4‑pt grid; rows 44 high; window padding 20.
- **Elevation:** one soft shadow for floating panels only (`0 8 30 rgba(0,0,0,.18)`), hairline borders elsewhere.

### Components to build (as a component set with variants)
1. **Keycap** — sizes S/M; variants: single glyph (`⌥`, `⇧`, `Space`, `A`, `ع`), pressed state. Subtle bottom border to feel physical.
2. **Shortcut recorder field** — states: empty («سجّل اختصارًا»), recording (accent ring, «اضغط المفاتيح…»), filled (keycaps + clear ×), conflict (warning text under it).
3. **Toggle row** — title, optional description, macOS switch; states: on/off/disabled.
4. **Select row** — title + popup button (used for choosing Arabic / Latin layout).
5. **Permission status card** — states: granted (success dot, «الصلاحية ممنوحة»), missing (warning, primary button «افتح إعدادات النظام»), checking (spinner).
6. **App list row** — app icon 24, app name, remove button; plus an «أضف تطبيقًا» add row.
7. **Conversion sample** — mono text: `اثممخ` → `hello` with a swap arrow; used in onboarding, HUD and About.
8. **HUD pill** — see screen 4.
9. **Menu bar icon** — template (monochrome) glyph, 18×18 @1x: two small keycaps, one with `ع` one with `A`, linked by a swap arrow. Variants: idle, active flash (filled), disabled/permission‑missing (with small warning dot).
10. **Buttons** — primary, secondary, plain/link; sizes regular/large; states default/hover/pressed/disabled.

### Screens

**1. Menu bar popover (RTL, ~280 wide)** — opened from the menu bar icon.
- Header row: app name «بدّل» + small status: «جاهز» with success dot (variant: «تحتاج صلاحية» with warning dot).
- Hero hint card: keycaps `⌥ ⇧ Space` and the line «حدّد نصًا، أو اضغط مباشرة بعد الكلمة الخاطئة».
- Last conversion (secondary, mono): `اثممخ ← hello` with a small «تراجع» link.
- Toggle: «بدّل لغة لوحة المفاتيح بعد التحويل».
- Toggle: «أوقف بدّل مؤقتًا».
- Divider, then menu items: «الإعدادات…  ⌘,» · «تحقّق من التحديثات» · «إنهاء بدّل  ⌘Q».
- Deliver: default, permission‑missing, and paused variants.

**2. Onboarding window (560×620, 3 steps, progress dots)**
- **Step 1 — الفكرة:** title «كتبتها باللغة الخطأ؟ بدّلها.» Body: «اختصار واحد يصحّح ما كتبته بتخطيط لوحة المفاتيح الخطأ — في أي تطبيق.» Center: large animated‑looking Conversion sample (`اثممخ` → `hello`, and below it `sghl` → `سلام`). Button «التالي».
- **Step 2 — الصلاحية:** title «صلاحية واحدة فقط». Body: «يحتاج بدّل صلاحية "تسهيلات الاستخدام" ليقرأ النص المحدَّد ويستبدله. لا يراقب ما تكتبه، ولا يتصل بالإنترنت.» Permission status card (show both missing and granted variants). Three small reassurance rows with check icons: «لا حساب» · «لا شبكة» · «لا سجل لما تكتب». Button «افتح إعدادات النظام», secondary «لاحقًا».
- **Step 3 — الاختصار:** title «اختر اختصارك». Shortcut recorder prefilled with `⌥ ⇧ Space`. A "try it" text field: placeholder «جرّب هنا: اكتب hello ولوحة المفاتيح عربية ثم اضغط الاختصار». Button «ابدأ».

**3. Settings window (680×480, macOS‑style, sidebar on the RIGHT in RTL)** — sidebar items with SF Symbols‑style icons: «عام» · «الاختصارات» · «التخطيطات» · «الاستثناءات» · «حول».
- **عام:** toggles — «تشغيل عند تسجيل الدخول» · «بدّل لغة لوحة المفاتيح بعد التحويل» · «أظهر إشعار التحويل» · «صوت خافت عند التحويل» · «تحقّق من التحديثات تلقائيًا». Select row: «لغة الواجهة» (العربية / English). Permission status card at the bottom.
- **الاختصارات:** recorder rows — «حوّل التحديد أو آخر كلمة» (`⌥ ⇧ Space`) · «تراجع عن آخر تحويل» (empty) · «أوقف بدّل مؤقتًا» (empty). Show one row in the **conflict** state. Footnote: «الضغط مرة ثانية خلال ثانيتين يمدّ التحويل كلمةً أخرى للخلف.»
- **التخطيطات:** two select rows — «التخطيط العربي» (Arabic – PC) · «التخطيط اللاتيني» (ABC). Below: a compact **keyboard map preview** — a 3‑row mini keyboard where each key shows the Latin glyph top‑left and the Arabic glyph bottom‑right, with the `B / لا` key highlighted in `accent/soft`. Also an **empty/warning state**: «لم نجد تخطيطًا عربيًا مفعّلًا» with button «افتح إعدادات لوحة المفاتيح».
- **الاستثناءات:** description «لن يعمل بدّل داخل هذه التطبيقات.» App list with 3 rows (Terminal, 1Password, iTerm) + add row. Also the **empty state**: «لا استثناءات بعد».
- **حول:** app icon, «بدّل · Baddel», version `1.0.0`, tagline «اكتب، ونحن نبدّل.», links «المصدر على GitHub» · «أبلغ عن مشكلة» · «الرخصة MIT», and a small row of sibling apps: رفّ · Luma · نفّذ.

**4. HUD pill (floating, bottom‑center of screen, shown for ~1s)** — dark translucent pill in both themes, height 36, mono text. Variants:
- success: `اثممخ  ←  hello` with a tiny keyboard‑language badge `EN` (or `ع`) showing the layout switched to.
- undo: «تم التراجع».
- blocked: «حقل محمي — لم يُحوَّل شيء» (warning tint).
- too long: «التحديد طويل جدًا».
Show one HUD composited over a blurred mock of a text editor for context.

**5. App icon (1024×1024, macOS rounded‑square grid)** — two overlapping keycaps, front one `ع`, back one `A`, with a swap arrow between them; slate‑blue (`#3F5673`) on warm off‑white, subtle depth, no gloss. Provide 3 explorations plus the monochrome menu bar glyph derived from the chosen one.

### Deliverables & file structure
Pages: `00 Cover` · `01 Tokens` · `02 Components` · `03 Menu Bar` · `04 Onboarding` · `05 Settings` · `06 HUD` · `07 App Icon`.
- Every screen in **Light and Dark**, built with auto layout and the variables above — no hard‑coded colors.
- All components as variants with clear property names (`state`, `size`, `direction`).
- Frames named `Screen / Variant / Theme` (e.g. `Settings / General / Dark`).
- Cover frame: app icon, «بدّل — Baddel», tagline «اكتب، ونحن نبدّل.», and the Conversion sample.

### Quality bar
- Arabic text must never be clipped, letter‑spaced, or set in any font other than Cairo (headings) and Almarai (body). No justified Arabic.
- Numerals and shortcuts stay Latin/LTR inside RTL layouts.
- Contrast ≥ 4.5:1 for body text in both themes.
- Hit targets ≥ 28pt; focus ring visible on every interactive component (accent, 2px, offset 2).
- It should feel like it ships with macOS — restraint over decoration.

---

## ملاحظات لك (خارج البرومبت)

- **اللون:** اخترت أزرق أردوازي `#3F5673` ليتميّز عن أخضر رفّ ونحاسي Luma وبرتقالي نفّذ وأخضر جُسور الداكن، مع بقاء العائلة كلها في الدرجات الهادئة المطفأة نفسها. بدّله بحرية.
- **الأيقونة:** لو كانت أيقونات تطبيقاتك تتبع شبكة أو أسلوبًا موحّدًا، أضف سطرًا في قسم App icon يصفه، أو أرفق صورة أيقونة رفّ كمرجع.
- **خريطة المفاتيح** في شاشة «التخطيطات» أجمل عنصر بصري في التطبيق، وتصلح لقطةً رئيسية في الـREADME.
- النصوص العربية هنا هي نفسها نصوص الواجهة في [خطة التنفيذ](baddel-implementation-plan.md). أي تعديل عليها عدّله في المكانين.
