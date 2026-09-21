# الخصوصية

آخر تحديث: 2026-09-21

بدّل أداة محلية بالكامل. هذه الصفحة تشرح ما يقرأه، وما يحتفظ به، وما يرسله.

- **القراءة**: فقط حين تضغط الاختصار — التحديد إن وُجد، وإلا النص قبل المؤشر بقدر ما يلزم لإيجاد الكلمة السابقة — أو حين تطلب التراجع، فيُقرأ موضع النص المحوَّل ليُعاد. لا يُقرأ شيء في غير ذلك.
- **التخزين**: لا نص على القرص ولا في أي سجل. آخر تحويل يبقى في الذاكرة 30 ثانية فقط ليتيح التراجع، ثم يُمحى. ملف الإعدادات في `~/Library/Application Support/com.isltanx.baddel/` يحمل التفضيلات وحدها (الاختصارات، الاستثناءات، وما شابه)، لا أي نص كتبته.
- **الحافظة**: تعود إلى ما كانت عليه بعد كل تحويل. محتواها أثناء التحويل مؤقت، ويحمل وسمَي [nspasteboard.org](http://nspasteboard.org): «Transient» و«Concealed»، فتتجاهله مديرات الحافظة التي تحترمهما.
- **الشبكة**: لا اتصال إلا لفحص التحديث — طلب واحد يوميًا لملف `latest.json` من صفحة إصدارات هذا المستودع على GitHub، يمكن إيقافه من **الإعدادات ← عام**. الطلب لا يحمل شيئًا عنك ولا عن نصّك، ويصل GitHub منه ما يصله من أي تنزيل عادي، مثل عنوان IP. روابط بدّل على GitHub في نافذة «حول» تفتح في متصفحك.
- **الصلاحية**: تسهيلات الاستخدام (Accessibility) هي الوحيدة المطلوبة — لقراءة التحديد أو الكلمة قبل المؤشر واستبدالها، وإرسال ضغطات النسخ واللصق حين لا يكشف التطبيق نصّه. لا يطلب بدّل مراقبة الإدخال (Input Monitoring)، ولا تسجيل الشاشة (Screen Recording)، ولا الوصول الكامل إلى القرص (Full Disk Access).
- **الحقول المحمية والاستثناءات**: حين يفعّل macOS الإدخال الآمن (Secure Input)، لا يقرأ بدّل شيئًا ولا يكتب شيئًا. و14 استثناءً افتراضيًا يتوقف فيها الاختصار: 7 طرفيات و7 من مديري كلمات المرور، تُدار من **الإعدادات ← الاستثناءات**.
- **لا تتبّع**: لا حسابات، ولا تحليلات، ولا تتبّع، ولا تقارير أعطال.
- **التواصل**: لأي سؤال أو مشكلة، افتح مسألة في [صفحة المسائل](https://github.com/iSltanX/Baddel/issues).

---

# Privacy

Last updated: 2026-09-21

Baddel is fully local. This page explains what it reads, what it keeps, and what it sends.

- **Reading**: only when you press the shortcut — the selection if there is one, otherwise the text before the cursor, as far as needed to find the previous word — or when you ask for undo, which reads back the converted text to restore it. Nothing is read at any other time.
- **Storage**: no text is saved to disk or to any log. The last conversion stays in memory for 30 seconds only, to allow undo, then it is dropped. The settings file at `~/Library/Application Support/com.isltanx.baddel/` holds preferences only (shortcuts, exceptions, and the like), never any text you typed.
- **Clipboard**: restored after every conversion. Its content during a conversion is temporary and carries the [nspasteboard.org](http://nspasteboard.org) markers "Transient" and "Concealed", so clipboard managers that respect them ignore it.
- **Network**: no network access except the update check — one request a day for `latest.json` from this repository's GitHub releases page, which can be turned off in **Settings → General**. The request carries nothing about you or your text; GitHub receives whatever any ordinary download request carries, such as an IP address. The GitHub links in the About window open in your browser.
- **Permission**: the only permission Baddel asks for is **Accessibility** — to read the selection or the word before the cursor and replace it, and to send copy/paste keystrokes when an app does not expose its text. Baddel does not request Input Monitoring, Screen Recording, or Full Disk Access.
- **Protected fields and exceptions**: when macOS turns on Secure Input, Baddel reads nothing and writes nothing. There are 14 default exceptions where the shortcut does nothing: 7 terminals and 7 password managers, managed from **Settings → Exceptions**.
- **No tracking**: no accounts, no analytics, no tracking, no crash reporting.
- **Contact**: for any question or issue, open one at the [issues page](https://github.com/iSltanX/Baddel/issues).
