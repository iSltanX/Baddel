# بدّل — Baddel · برومبت Figma الاحترافي (v2)

نسخة مبنية على إرشادات Apple Human Interface Guidelines وتوصيات Figma الرسمية لكتابة البرومبتات.
الفرق عن [النسخة الأولى](baddel-figma-prompt.md): هذه **سلسلة برومبتات مرحلية** بدل برومبت واحد ضخم، وكل قرار تصميمي فيها مسنود بقاعدة من Apple.

---

## أولًا — ما الذي تغيّر بعد البحث؟

| # | ما وجدته | المصدر | أثره على التصميم |
|---|---|---|---|
| 1 | أيقونة شريط القوائم تفتح **قائمة (menu) لا popover** إلا إذا كانت الوظيفة أعقد من قائمة | HIG · The menu bar | استبدلت الـpopover بقائمة أصيلة في رأسها صف حالة مخصص. |
| 2 | نافذة الإعدادات في macOS: **شريط أدوات بأزرار للأقسام (panes)** غير قابل للتخصيص، وعنوان النافذة يتبع القسم الحالي، وزرّا التصغير والتكبير معطّلان، وآخر قسم يُستعاد عند الفتح | HIG · Settings | استبدلت الشريط الجانبي بشريط أدوات علوي. |
| 3 | الخط الافتراضي 13pt والأدنى 10pt. **تجنّب الأوزان الخفيفة** (Light/Thin). قلّل عدد الخطوط | HIG · Typography | حذفت Almarai 300. الخطان فقط: Cairo وAlmarai. |
| 4 | العربية تبدو أصغر من اللاتينية بالحجم نفسه، والتوصية **زيادة نحو 2pt** للنص العربي | HIG · Right to left | نص الواجهة العربي 14–15pt بدل 13. |
| 5 | الفقرات (3 أسطر فأكثر) تُحاذى حسب **لغة النص** لا اتجاه الواجهة. الأرقام لا يُعكس ترتيبها. أسهم التنقل تُعكس. الشعارات وعلامات الصح والساعات لا تُعكس | HIG · Right to left | صارت هذه قواعد صريحة في البرومبت. |
| 6 | الترحيب قصير واختياري. **علّم بالممارسة لا بالشرح**. اطلب الصلاحية أثناء الترحيب إذا كانت أساسية، مع توضيح الفائدة | HIG · Onboarding | ثلاث خطوات كحد أقصى، وفيها حقل تجربة حيّ وزر «تخطَّ». |
| 7 | الأيقونة: **طبقات** (خلفية + طبقة أمامية أو أكثر) على لوحة 1024×1024 **مربعة بلا قصّ**، والنظام يطبّق الزوايا | HIG · App icons | أعدت كتابة برومبت الأيقونة بالكامل. |
| 8 | **لا تضف** ظلالًا ولا لمعانًا ولا bevels ولا blur، فالنظام (Liquid Glass) يضيفها. الحواف **واضحة الحدود** لا ناعمة | HIG · App icons + Icon Composer | الأيقونة في Figma مسطّحة ونظيفة، والعمق يُضاف في Icon Composer. |
| 9 | ست حالات مظهر: Default وDark وClear (فاتح/داكن) وTinted (فاتح/داكن)، مع **ثبات العناصر والظل العام (silhouette)** بينها | HIG · App icons | أضفت لوحة معاينة للحالات الست. |
| 10 | التصدير لـIcon Composer: طبقات **SVG** بأسماء مرقّمة من الخلف للأمام، و**النصوص محوّلة إلى outlines**، وبحد أقصى **4 مجموعات**، بلا لون خلفية مدمج وبلا قناع | Xcode · Icon Composer | أضفت قسم تسليم خاصًا بالأيقونة. |
| 11 | النص في الأيقونة «فقط إن كان جوهريًا» لأنه لا يُترجم ويصغر | HIG · App icons | حرفا «ع» و«A» هنا هما الفكرة نفسها، فأُبقيا كأشكال outlines، مع استكشاف واحد بلا حروف. |
| 12 | Figma Make: كن محددًا ومباشرًا، **صف البنية لا الأسلوب المجرد**، ابدأ ببرومبت أول مقيّد ثم كرّر، وأرفق تصاميم وصورًا مرجعية (مع العلم أنه لا يلتقط الألوان بدقة من الصور) | Figma Docs + Help Center | قسّمت العمل إلى 6 برومبتات، والألوان مكتوبة قيمًا نصية. |

---

## ثانيًا — قبل أن تبدأ

1. **أي أداة؟**
   - **Figma Make** يولّد نموذجًا تفاعليًا (كود)، ويناسب تجربة التدفق والحركة.
   - إن أردت **طبقات Figma حقيقية** (مكوّنات ومتغيرات وauto layout) فاستخدم Claude مع موصل Figma. البرومبتات نفسها تعمل، وأقدر أنفذها لك مباشرة.
2. **أرفق مراجع** مع البرومبت الأول (Make يقبل لصق الإطارات والصور):
   - عدة Apple الرسمية **macOS UI Kit** من [Apple Design Resources](https://developer.apple.com/design/resources/)، وفيها قالب الأيقونة بالشبكة الصحيحة.
   - لقطة من رفّ أو نفّذ كمرجع لروح العائلة.
3. **الترتيب:** نفّذ البرومبتات بالتسلسل، ولا تنتقل للتالي حتى ترضى عن الحالي. صحّح بالتأشير على العنصر (point‑and‑edit) بدل إعادة توليد الشاشة كاملة.

---

## ثالثًا — سلسلة البرومبتات

### PROMPT 0 — السياق والقواعد (يُلصق أولًا، ويبقى مرجعًا لكل ما بعده)

```text
You are designing "Baddel" (بدّل), a native macOS menu bar utility. Follow Apple's
Human Interface Guidelines for macOS strictly. Do not generate any screens yet —
first confirm you understand these rules, then wait for the next prompt.

PRODUCT
- The user typed with the wrong keyboard layout (typed "اثممخ" meaning "hello", or
  "sghl" meaning "سلام"). One global shortcut converts the selected text — or the
  last word — in place, in any app, then switches the input language.
- 100% local. No account, no network, no keystroke monitoring. Needs only the macOS
  Accessibility permission.
- Audience: Arabic-speaking Mac users who type in Arabic and English all day.
- Personality: calm, precise, invisible until needed. Should feel like it ships
  with macOS. Restraint over decoration.

PLATFORM RULES (Apple HIG — non-negotiable)
- Menu bar extra opens a MENU, not a popover.
- Settings is a window with a non-customizable TOOLBAR of pane buttons (not a
  sidebar). Window title = current pane name. Minimize and zoom buttons dimmed.
  Settings apply immediately — no Save/Apply buttons.
- Body text 13pt minimum-default on macOS; never below 10pt. No Light/Thin weights.
- Use standard macOS controls: switches, pop-up buttons, push buttons, checkboxes,
  grouped form rows with inset rounded backgrounds. Do not invent custom controls.
- Translucent / glass material ONLY on the navigation layer (menu, toolbar, HUD).
  Content areas are opaque. Never stack glass on glass.
- Every screen in Light and Dark appearance.

DIRECTION & LANGUAGE (Arabic-first, RTL)
- Primary UI is Arabic, right-to-left. Mirror the whole layout: toolbar items start
  from the right, labels on the right and controls on the left of each form row,
  window traffic lights stay top-LEFT (the system never mirrors them).
- Flip: back/next chevrons, disclosure arrows, progress direction.
- Never flip: logos, the app icon, checkmarks, keyboard key glyphs (⌘ ⌥ ⇧), Latin
  sample text, digit order inside numbers.
- Short text (1–2 lines) aligns to the UI direction. Paragraphs of 3+ lines align
  to the language of the text itself.
- Keyboard shortcuts and Latin samples stay LTR runs inside RTL rows.
- Use real Arabic copy exactly as provided. No lorem ipsum. No letter-spacing on
  Arabic. No justified Arabic. No faux bold/italic.

TYPOGRAPHY — exactly two Google Fonts families, nothing else for UI text
- Cairo — headings only. Weights 600, 700.
- Almarai — all other UI text. Weights 400, 700 only (Almarai has no 500/600 —
  never simulate one; never use 300).
- Arabic is optically smaller than Latin at equal size, so the scale is +1–2pt
  over macOS defaults:
    Heading/Large   Cairo 700    26 / 36
    Heading/Title   Cairo 700    20 / 30
    Heading/Section Cairo 600    15 / 24
    Body/Default    Almarai 400  14 / 22
    Body/Emphasis   Almarai 700  14 / 22
    Body/Small      Almarai 400  12 / 19
    Caption         Almarai 400  11 / 17   (minimum; never smaller)
- Monospace (SF Mono, fallback IBM Plex Mono) is allowed ONLY inside keycaps and
  conversion samples, for the Latin glyphs. Arabic glyphs there use Almarai.

COLOR TOKENS — create as variables with Light / Dark modes
  bg/window        #F6F4EF   #1B1C1F
  bg/surface       #FFFFFF   #25272B
  bg/sunken        #EDEAE3   #16171A
  border/subtle    #E2DED5   #34373C
  text/primary     #1C1917   #F2F0EB
  text/secondary   #6B665E   #A19D95
  accent/primary   #3F5673   #8FA9C9
  accent/soft      #E4EAF1   #2A3644
  state/success    #5F7F52   #93B585
  state/warning    #B67F2E   #D9A95A
  state/danger     #A8493B   #D98476
- No hard-coded colors anywhere. Body text contrast ≥ 4.5:1, large text ≥ 3:1.
- Never signal state with color alone — pair with an icon or label.

LAYOUT
- 4pt grid. Form rows 44pt high. Window padding 20. Group gap 16.
- Radii: controls 6, grouped cards 10, windows follow system, HUD fully rounded.
- Hit targets ≥ 28×28pt. Visible focus ring on every interactive element
  (accent/primary, 2pt, offset 2) — keyboard navigation is a first-class path.
- Icons: SF Symbols style, 1.5pt stroke, monochrome, rendered in text color.

FILE HYGIENE
- Auto layout everywhere. Components with variant properties named
  state / size / direction / theme. Frames named "Screen / Variant / Theme".
```

---

### PROMPT 1 — المكوّنات

```text
Using the rules above, build ONLY the component library on one page. No screens.

1. Keycap — sizes S (20pt) / M (26pt). Content: ⌘ ⌥ ⇧ ⌃, "Space", a Latin letter,
   an Arabic letter. States: default, pressed. 1pt border + slightly darker bottom
   edge; no drop shadow.
2. Shortcut recorder — states: empty ("سجّل اختصارًا"), recording (accent focus
   ring, "اضغط المفاتيح…"), filled (keycaps + clear ✕), conflict (state/warning
   icon + "هذا الاختصار مستخدم في تطبيق آخر"), disabled.
3. Form row — variants: switch / pop-up button / recorder / plain value. Title
   (Body/Default) + optional description (Body/Small, text/secondary). RTL: text
   right, control left. Include an LTR variant.
4. Grouped form card — bg/surface, radius 10, hairline dividers inset from the
   text side, holds 1–6 form rows.
5. Permission card — states: granted (success icon + "الصلاحية ممنوحة"),
   missing (warning icon + "بدّل يحتاج صلاحية تسهيلات الاستخدام" + primary button
   "افتح إعدادات النظام"), checking (spinner + "جارٍ التحقق…").
6. Conversion sample — "اثممخ" ← arrow → "hello". Sizes: inline / hero.
   The arrow points in reading direction (leftwards in RTL).
7. App list row — 24pt app icon, name, remove button (appears on hover + always
   reachable by keyboard). Plus an "أضف تطبيقًا…" row.
8. Buttons — primary / secondary / plain; regular / large; default / hover /
   pressed / focused / disabled.
9. Toolbar pane button — icon above label (macOS settings style); default /
   hover / selected.
10. HUD pill — see Prompt 5 for variants; build the base here.
11. Menu bar glyph — 16×16pt drawing area inside the 24pt menu bar, TEMPLATE
    image: pure black + transparency only, no gray, no color (the system tints
    it). Concept: two small keycaps with a swap arrow. Variants: idle, paused
    (slashed), needs-permission (small dot badge). Must stay legible at 16pt —
    max 2 shapes + 1 arrow, 1.5pt strokes, no letters inside at this size.

Show every component in Light and Dark, RTL first.
```

---

### PROMPT 2 — قائمة شريط القوائم

```text
Design the menu that opens from the menu bar glyph. It is a native macOS MENU
(NSMenu look: system menu material, 6pt item radius on hover, 22–24pt item
height, standard separators) — NOT a popover, no custom chrome. Width ≈ 260pt. RTL.

Structure, top to bottom:
1. Custom header item (non-interactive): "بدّل" (Heading/Section) + status on the
   opposite side: success dot + "جاهز".
2. Hint item (non-interactive, text/secondary): keycaps ⌥ ⇧ Space + "حوّل التحديد
   أو آخر كلمة".
3. Separator.
4. Last conversion item: "اثممخ ← hello", with trailing action text "تراجع".
   Hidden when there is no recent conversion.
5. Separator.
6. Checkable item ✓ "بدّل لغة لوحة المفاتيح بعد التحويل".
7. Item "أوقف بدّل مؤقتًا".
8. Separator.
9. "الإعدادات…" with shortcut ⌘, — shortcuts sit on the LEFT edge in RTL.
10. "تحقّق من التحديثات…"
11. Separator.
12. "إنهاء بدّل" with ⌘Q.

Deliver 3 variants × 2 themes: Ready · Needs permission (header shows warning
icon + "تحتاج صلاحية"; item 2 replaced by an actionable item "امنح الصلاحية…";
items 4–7 disabled) · Paused (header "متوقف مؤقتًا"; item 7 becomes "استأنف بدّل").
Show each composited under a mock menu bar, glyph highlighted.
```

---

### PROMPT 3 — الترحيب

```text
Design a 3-step onboarding window. Fixed 560×600, not resizable, title bar
without title, traffic lights top-left. RTL. Keep it brief, skippable, and teach
by doing — not by explaining.

Shared chrome: page dots centered at the bottom; primary button bottom-LEFT
(the trailing edge in RTL); plain "تخطَّ" button bottom-right; no back button on
step 1, a flipped chevron back button on steps 2–3.

Step 1 — الفكرة
  Title (Heading/Large): "كتبتها باللغة الخطأ؟ بدّلها."
  Body: "اختصار واحد يصحّح ما كتبته بتخطيط لوحة المفاتيح الخطأ — في أي تطبيق."
  Hero: two Conversion samples stacked — "اثممخ ← hello" and "sghl ← سلام".
  Primary: "التالي".

Step 2 — الصلاحية (explain the benefit BEFORE sending the user to System Settings)
  Title: "صلاحية واحدة فقط"
  Body: "يحتاج بدّل صلاحية «تسهيلات الاستخدام» ليقرأ النص المحدَّد ويستبدله."
  Three reassurance rows, each icon + text: "لا يراقب ما تكتبه" · "لا يتصل
  بالإنترنت" · "لا يحفظ أي نص".
  Permission card (deliver missing / checking / granted variants).
  Primary: "افتح إعدادات النظام" → becomes "التالي" once granted.

Step 3 — جرّب بنفسك
  Title: "جرّبها الآن"
  Shortcut recorder prefilled ⌥ ⇧ Space, caption "يمكنك تغييره متى شئت".
  Large practice text field prefilled with "اثممخ", helper text under it:
  "ضع المؤشر بعد الكلمة واضغط الاختصار".
  Deliver a success variant: field shows "hello", success icon + "أحسنت — هكذا
  يعمل بدّل".
  Primary: "ابدأ".

6 frames (3 steps + 3 state variants) × Light/Dark.
```

---

### PROMPT 4 — الإعدادات

```text
Design the Settings window following macOS conventions exactly: a TOOLBAR of
pane buttons (icon above label), not a sidebar. Toolbar is not customizable and
always shows the selected pane. Window title equals the pane name. Minimize and
zoom buttons dimmed. Window width fixed at 560pt; height fits each pane's
content. No Save/Apply/Cancel — changes apply immediately. RTL: pane buttons
ordered from the right; traffic lights stay top-left.

Panes (right to left): عام · الاختصارات · التخطيطات · الاستثناءات · حول
Content uses Grouped form cards on bg/window.

عام
  Card 1: switch "تشغيل عند تسجيل الدخول" · switch "أظهر أيقونة بدّل في شريط
    القوائم" (description: "يبقى الاختصار يعمل حتى لو أخفيت الأيقونة") · pop-up
    "لغة الواجهة" (العربية / English).
  Card 2: switch "بدّل لغة لوحة المفاتيح بعد التحويل" · switch "أظهر إشعار
    التحويل" · switch "صوت خافت عند التحويل".
  Card 3: switch "تحقّق من التحديثات تلقائيًا".
  Bottom: Permission card.

الاختصارات
  Card: recorder rows — "حوّل التحديد أو آخر كلمة" (⌥ ⇧ Space) · "تراجع عن آخر
  تحويل" (empty) · "أوقف بدّل مؤقتًا" (empty).
  Footnote (Body/Small): "الضغط مرة ثانية خلال ثانيتين يمدّ التحويل كلمةً أخرى
  للخلف."
  Also deliver one variant with the first row in the conflict state.

التخطيطات
  Card: pop-up "التخطيط العربي" (Arabic – PC) · pop-up "التخطيط اللاتيني" (ABC).
  Keyboard map preview: 3 letter rows of a Mac keyboard, each key showing the
  Latin glyph top-left and the Arabic glyph bottom-right; the B key (لا)
  highlighted with accent/soft and a caption: "مفتاح واحد يكتب حرفين — بدّل
  يتعامل معه تلقائيًا." The keyboard itself is NOT mirrored (real-world object).
  Empty state variant: warning icon + "لم نجد تخطيطًا عربيًا مفعّلًا" + button
  "افتح إعدادات لوحة المفاتيح".

الاستثناءات
  Description: "لن يعمل بدّل داخل هذه التطبيقات."
  App list: Terminal, iTerm, 1Password + add row; standard +/− bar under the list.
  Empty state variant: "لا استثناءات بعد".

حول
  App icon 96pt, "بدّل · Baddel" (Heading/Title), "الإصدار 1.0.0", tagline "اكتب،
  ونحن نبدّل.", plain buttons: "المصدر على GitHub" · "أبلغ عن مشكلة" · "الرخصة
  MIT". Footer row "من الصانع نفسه": رفّ · Luma · نفّذ (small icons + names).

Also deliver the عام pane once in English LTR to prove the layout mirrors cleanly.
All frames × Light/Dark.
```

---

### PROMPT 5 — إشعار HUD

```text
Design a transient HUD shown for ~1 second after a conversion, bottom-center of
the screen, 80pt above the Dock. It never takes focus and has no buttons.
Fully rounded pill, height 36pt, horizontal padding 14, system HUD material
(dark translucent in both themes), text in white at ≥ 4.5:1.

Variants:
  success      "اثممخ ← hello" + small badge showing the input source switched
               to ("EN" or "ع")
  undo         return-arrow icon + "تم التراجع"
  blocked      lock icon + "حقل محمي — لم يُحوَّل شيء"
  too-long     warning icon + "التحديد طويل جدًا"
  no-selection info icon + "لا يوجد نص لتحويله"

Motion spec (annotate it on the frame): fade + 4pt rise in 120ms ease-out, hold
900ms, fade out 200ms. With Reduce Motion: opacity only, no movement.
Show the success variant composited over a blurred text-editor mock, Light and
Dark wallpapers.
```

---

### PROMPT 6 — الأيقونة (نفّذه في ملف أو صفحة مستقلة)

```text
Design the macOS app icon for "Baddel", prepared for Apple's Icon Composer
(layered Liquid Glass icons). Follow these constraints exactly.

CANVAS
- 1024×1024 px, SQUARE, full-bleed. Do NOT draw the rounded-rectangle mask, do
  not round the canvas corners — the system masks it. Show the rounded shape
  only as a separate, non-exported guide overlay.
- Keep all primary content centered inside the inner ~80% safe area of Apple's
  icon grid so nothing is clipped by the mask.

CONCEPT — one idea, few shapes
- Two overlapping keycaps exchanging places: the front keycap carries the Arabic
  letter "ع", the back keycap carries Latin "A", linked by a single compact swap
  arrow. Letters are the concept itself, so they are allowed — but draw them as
  custom OUTLINED shapes (no live text): "ع" based on Cairo Bold, "A" based on
  Cairo Bold, optically matched in weight and height.
- Maximum 3 foreground elements. Must be recognizable at 16×16 px — test it.
- No words, no photos, no replicas of macOS UI, no Apple hardware, no keyboard
  photographs.

LAYERS (back to front; name them exactly like this)
  0-background    solid #3F5673, or a very subtle top-to-bottom gradient
                  #4A6485 → #35485F. Full-bleed, opaque.
  1-keycap-back   the "A" keycap body + its letter, off-white #F6F4EF body,
                  letter #35485F
  2-arrow         swap arrow, #F6F4EF at 85% opacity
  3-keycap-front  the "ع" keycap body + its letter
- Maximum 4 groups. Every foreground shape has crisp, clearly defined vector
  edges — no feathering.

DO NOT BAKE IN (the system adds these dynamically; static ones conflict)
- no drop shadows, no inner shadows, no bevels, no gloss or specular highlights,
  no glows, no blurs, no glass textures, no 3D renders.
- Depth comes only from overlap, scale, and opacity differences between layers.

APPEARANCES — one artboard each, same silhouette and same elements in all six
  Default · Dark (background → #1E2733, keycaps slightly dimmer) ·
  Clear Light · Clear Dark · Tinted Light · Tinted Dark
  (for Clear/Tinted, show the foreground as monochrome luminance shapes so the
  hierarchy still reads: front keycap brightest, arrow mid, back keycap dimmest).

EXPLORATIONS
- Deliver 3 directions: (A) the concept above; (B) a single keycap split
  diagonally, "ع" on one half and "A" on the other; (C) NO letters — two keycaps
  and a swap arrow only, for maximum small-size legibility.
- For each: 1024 master, plus previews at 512, 256, 128, 64, 32, 16 px on light
  and dark backgrounds, plus a mock Dock row between Finder-like neighbors.

HANDOFF
- Each layer exportable as an individual SVG, 1024×1024, transparent (except
  0-background), text converted to outlines, named with the numbered names
  above so Icon Composer sorts them back-to-front.
- Also derive the 16pt menu bar TEMPLATE glyph from the chosen direction: pure
  black + alpha, no letters, 1.5pt strokes.
```

---

### PROMPT 7 — المراجعة (بعد اكتمال كل شيء)

```text
Audit everything you produced against this checklist and list every violation
with the frame name, then fix them:
1. Any text not in Cairo or Almarai (mono inside keycaps/samples excepted)?
2. Any Almarai weight other than 400/700, or Cairo other than 600/700?
3. Any text under 11pt? Any Arabic body text under 14pt?
4. Any hard-coded color instead of a variable? Any contrast below 4.5:1?
5. Any RTL errors: unmirrored row, shortcut on the wrong edge, flipped
   checkmark/logo/key glyph, reversed digits, chevron pointing the wrong way?
6. Any custom control where a standard macOS control exists?
7. Any translucent material used in a content area, or glass on glass?
8. Any state communicated by color alone?
9. Any interactive element without hover, pressed, focused and disabled states?
10. App icon: any baked shadow/gloss/blur? Any live text? Rounded mask drawn
    into the artwork? More than 4 groups? Illegible at 16px?
11. Every screen present in both Light and Dark?
```

---

## رابعًا — بعد Figma: تسليم الأيقونة إلى Icon Composer

1. صدّر كل طبقة **SVG** منفصلة بأسمائها المرقّمة (`0-background` … `3-keycap-front`).
2. افتح **Icon Composer** (من Xcode › Open Developer Tool) واسحب الملفات إليه. نظّمها في مجموعات لا تتجاوز أربعًا.
3. اضبط الخلفية (لون أو تدرّج) من داخل Icon Composer، لا من SVG.
4. أضف من هناك Specular وShadow وTranslucency، وعاين حالات Default وDark وMono (Clear/Tinted).
5. احفظ الملف باسم `AppIcon.icon` وأضفه إلى مشروع Xcode. يولّد Xcode نسخ الأنظمة الأقدم تلقائيًا.

---

## المصادر

**Apple**
- [HIG · App icons](https://developer.apple.com/design/human-interface-guidelines/app-icons)
- [HIG · The menu bar](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar)
- [HIG · Settings](https://developer.apple.com/design/human-interface-guidelines/settings)
- [HIG · Right to left](https://developer.apple.com/design/human-interface-guidelines/right-to-left)
- [HIG · Typography](https://developer.apple.com/design/human-interface-guidelines/typography)
- [HIG · Onboarding](https://developer.apple.com/design/human-interface-guidelines/onboarding)
- [Xcode · Creating your app icon using Icon Composer](https://developer.apple.com/documentation/Xcode/creating-your-app-icon-using-icon-composer)
- [Apple Design Resources (عدة macOS وقالب الأيقونة)](https://developer.apple.com/design/resources/)

**Figma**
- [How to write great prompts — Figma Developer Docs](https://developers.figma.com/docs/code/how-to-write-great-prompts/)
- [Explore Figma Make — Help Center](https://help.figma.com/hc/en-us/articles/31304412302231-Explore-Figma-Make)
- [8 essential tips for using Figma Make — Figma Blog](https://www.figma.com/blog/8-ways-to-build-with-figma-make/)

> قاعدة «الزجاج لطبقة التنقل فقط» مأخوذة من جلسات WWDC25 عن Liquid Glass ولم أتحقق منها نصًا في هذا البحث. بقية القواعد مستخرجة مباشرة من صفحات HIG أعلاه.
