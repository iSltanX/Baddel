//! The conversion notice: a small floating panel near the bottom of the screen.
//!
//! Native on purpose. A webview window would keep a WebKit process alive while the
//! app idles, and showing one risks pulling focus away from the app the user is
//! typing in — this panel is non-activating and ignores the mouse entirely.
//!
//! It is drawn to the A2 design (`HUD` in 02 — Components of the approved Figma file):
//! a flat ink pill with a hairline and one soft shadow, the same in light and dark, a
//! line icon tinted by the state, and on a conversion the original word, an arrow, the
//! result and a chip naming the layout it switched to. It mirrors with the interface.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::data::{CFData, CFDataRef};
use objc2::rc::Retained;
use objc2::{AllocAnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSBezierPath, NSBitmapImageFileType, NSBitmapImageRep, NSColor,
    NSDeviceRGBColorSpace, NSFont, NSGraphicsContext, NSImage, NSImageView, NSPanel, NSScreen, NSShadow, NSTextField,
    NSView, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSData, NSDictionary, NSPoint, NSRect, NSSize, NSString};
use tauri::{AppHandle, Manager};

use crate::sys::on_main;

const ALMARAI_REGULAR: &[u8] = include_bytes!("../../src/assets/fonts/Almarai-Regular.ttf");
const ALMARAI_BOLD: &[u8] = include_bytes!("../../src/assets/fonts/Almarai-Bold.ttf");

// ── Geometry (`HUD` in 02 — Components) ──────────────────────────────────────
const HEIGHT: f64 = 36.0;
/// Between the pill's edge and a message.
const PAD: f64 = 14.0;
/// On a conversion: on the icon's side, and beside the language chip.
const PAD_ICON_SIDE: f64 = 12.0;
const PAD_CHIP_SIDE: f64 = 8.0;
const GAP: f64 = 8.0;
const ICON: f64 = 16.0;
const ARROW: f64 = 14.0;
const CHIP_HEIGHT: f64 = 20.0;
const CHIP_PAD: f64 = 8.0;
const CHIP_RADIUS: f64 = 6.0;
/// Effect/HUD: one soft drop, 6pt down with a 16pt blur. The panel is this much larger than
/// the pill on every side, so the shadow is drawn with it rather than left to the window.
const SHADOW_Y: f64 = 6.0;
const SHADOW_BLUR: f64 = 16.0;
const SHADOW_PAD: f64 = 32.0;
/// Distance from the top of the Dock to the bottom of the pill.
const ABOVE_DOCK: f64 = 80.0;
/// How far the pill travels upwards as it appears.
const RISE: f64 = 4.0;

// ── Type ─────────────────────────────────────────────────────────────────────
/// Body/Regular and Body/Strong for Arabic (Almarai); Latin/Body for Latin runs (SF Pro).
const TEXT_ARABIC: f64 = 14.0;
const TEXT_LATIN: f64 = 13.0;
/// The chip: Caption/Strong for «ع», Keycap/SM for «EN».
const CHIP_ARABIC: f64 = 11.0;
const CHIP_LATIN: f64 = 10.0;
/// `NSFontWeight` values (AppKit's own constants), for SF Pro Semibold and Medium.
const WEIGHT_SEMIBOLD: f64 = 0.3;
const WEIGHT_MEDIUM: f64 = 0.23;

// ── Colour (`Color` collection, `color/hud/*` — one value in both appearances) ──
const HUD_BG: [f64; 4] = [15.0 / 255.0, 42.0 / 255.0, 40.0 / 255.0, 1.0];
const HUD_TEXT: [f64; 4] = [230.0 / 255.0, 244.0 / 255.0, 241.0 / 255.0, 1.0];
const HUD_MUTED: [f64; 4] = [230.0 / 255.0, 244.0 / 255.0, 241.0 / 255.0, 0.62];
const HUD_CHIP: [f64; 4] = [230.0 / 255.0, 244.0 / 255.0, 241.0 / 255.0, 0.12];
const HUD_MINT: [f64; 4] = [43.0 / 255.0, 208.0 / 255.0, 189.0 / 255.0, 1.0];
const HUD_WARNING: [f64; 4] = [242.0 / 255.0, 181.0 / 255.0, 68.0 / 255.0, 1.0];
/// The hairline: a touch stronger in dark mode, where it separates the pill from dark windows.
const HUD_BORDER_LIGHT: [f64; 4] = [1.0, 1.0, 1.0, 0.08];
const HUD_BORDER_DARK: [f64; 4] = [1.0, 1.0, 1.0, 0.14];
const HUD_SHADOW: [f64; 4] = [0.0, 18.0 / 255.0, 15.0 / 255.0, 0.14];

// ── Icons (the `Icon/*` components, exported as template images) ────────────
const ICON_CHECK: &[u8] = include_bytes!("../icons/hud/check-circle.png");
const ICON_UNDO: &[u8] = include_bytes!("../icons/hud/undo.png");
const ICON_LOCK: &[u8] = include_bytes!("../icons/hud/lock.png");
const ICON_WARNING: &[u8] = include_bytes!("../icons/hud/warning.png");
const ICON_TEXT_CURSOR: &[u8] = include_bytes!("../icons/hud/text-cursor.png");
const ICON_KEYBOARD: &[u8] = include_bytes!("../icons/hud/keyboard.png");
const ICON_ACCESSIBILITY: &[u8] = include_bytes!("../icons/hud/accessibility.png");
const ICON_PAUSE: &[u8] = include_bytes!("../icons/hud/pause.png");
const ICON_NO_EYE: &[u8] = include_bytes!("../icons/hud/no-eye.png");
const ICON_CHECKMARK: &[u8] = include_bytes!("../icons/hud/check.png");
const ICON_ARROW_LEFT: &[u8] = include_bytes!("../icons/hud/arrow-left.png");
const ICON_ARROW_RIGHT: &[u8] = include_bytes!("../icons/hud/arrow-right.png");

// ── Motion ───────────────────────────────────────────────────────────────────
const FADE_IN: Duration = Duration::from_millis(120);
const HOLD: Duration = Duration::from_millis(900);
const FADE_OUT: Duration = Duration::from_millis(200);
const FADE_IN_STEPS: u32 = 6;
const FADE_OUT_STEPS: u32 = 8;
/// How long, and how often, the launch-time watch looks for a stray activation (3s in all).
const PRIME_CHECKS: u32 = 30;
const PRIME_INTERVAL: Duration = Duration::from_millis(100);

/// The `State` variants of `HUD` in 02 — Components, one per state of the shared state system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Success,
    Undone,
    Blocked,
    TooLong,
    NoText,
    Unchanged,
    Failed,
    NoLayouts,
    NoPermission,
    /// Undo asked for in another app than the one that converted.
    Elsewhere,
    Paused,
    Excluded,
    /// Undo: the converted text is not where it was left.
    NotHere,
    /// Undo: the write did not go through; it can be tried again.
    UndoFailed,
    /// "Copy Original Text" from the menu.
    Copied,
}

impl Kind {
    /// The line icon and its tint. State is never carried by colour alone — every
    /// variant has its own icon too.
    fn icon(self) -> (&'static [u8], [f64; 4]) {
        match self {
            Kind::Success => (ICON_CHECK, HUD_MINT),
            Kind::Undone => (ICON_UNDO, HUD_MINT),
            Kind::Copied => (ICON_CHECKMARK, HUD_MINT),
            Kind::Blocked => (ICON_LOCK, HUD_WARNING),
            Kind::TooLong | Kind::Failed | Kind::UndoFailed => (ICON_WARNING, HUD_WARNING),
            Kind::NoLayouts => (ICON_KEYBOARD, HUD_WARNING),
            Kind::NoPermission => (ICON_ACCESSIBILITY, HUD_WARNING),
            Kind::Elsewhere | Kind::NotHere => (ICON_UNDO, HUD_WARNING),
            Kind::NoText | Kind::Unchanged => (ICON_TEXT_CURSOR, HUD_MUTED),
            Kind::Paused => (ICON_PAUSE, HUD_MUTED),
            Kind::Excluded => (ICON_NO_EYE, HUD_MUTED),
        }
    }
}

/// What the notice says.
pub enum Body {
    /// A sentence, already localized.
    Message(String),
    /// The word as it was typed, and as it is now.
    Conversion { from: String, to: String },
}

/// One notice to display.
pub struct Notice {
    pub kind: Kind,
    pub body: Body,
    /// The input source we switched to ("EN" / "ع"), shown as a small chip.
    pub badge: Option<String>,
    /// Right to left when the interface is Arabic: the whole pill mirrors.
    pub rtl: bool,
}

/// Bumped on every show; a fade still running for an older notice sees the change
/// and gives up instead of hiding the new one.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Puts the (fully transparent) panel on screen once at launch, so the notice never has
/// to be the app's first window.
///
/// macOS activates an app the first time one of its windows appears if an activation is
/// still pending from launch (see `sys::chrome::resign_activation`), and it does so about
/// a second *after* the window shows. Left to the first real notice, that lands in the
/// middle of the user's typing. Here it lands at launch instead, where a short, bounded
/// watch hands the activation straight back — unless a window of ours wants it.
pub fn prime(app: &AppHandle) {
    on_main(app, || {
        with_panel(|hud| {
            hud.panel.setAlphaValue(0.0);
            hud.panel.orderFrontRegardless();
        })
    });
    let app = app.clone();
    thread::Builder::new()
        .name("baddel-hud-prime".into())
        .spawn(move || {
            for _ in 0..PRIME_CHECKS {
                thread::sleep(PRIME_INTERVAL);
                let windows_open = !app.webview_windows().is_empty();
                on_main(&app, move || {
                    if !windows_open && crate::sys::chrome::is_active() {
                        crate::sys::chrome::resign_activation();
                    }
                });
            }
            // Nothing has been shown since: take the invisible panel back off screen.
            on_main(&app, || {
                if GENERATION.load(Ordering::SeqCst) == 0 {
                    with_panel(|hud| hud.panel.orderOut(None));
                }
            });
        })
        .ok();
}

/// Shows `notice` for about a second. Safe to call from any thread.
pub fn show(app: &AppHandle, notice: Notice) {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let handle = app.clone();
    on_main(app, move || present(notice));
    thread::Builder::new()
        .name("baddel-hud".into())
        .spawn(move || animate(&handle, generation))
        .ok();
}

/// Shows `notice` and leaves it there, for capturing the documentation screenshots.
/// The real notice fades, so a capture would otherwise catch it mid-fade.
#[cfg(debug_assertions)]
pub fn show_pinned(app: &AppHandle, notice: Notice) {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    on_main(app, move || {
        present(notice);
        with_panel(|hud| hud.panel.setAlphaValue(1.0));
    });
}

/// Writes what the panel is showing to a PNG.
///
/// macOS refuses to let `screencapture` photograph a floating, non-activating
/// panel, so the documentation screenshots of the notice are rendered by the app
/// itself, from the very views it puts on screen.
#[cfg(debug_assertions)]
pub fn capture(app: &AppHandle, path: String) -> bool {
    on_main(app, move || {
        with_panel(|hud| {
            let bounds = hud.content.bounds();
            let Some(rep) = hud.content.bitmapImageRepForCachingDisplayInRect(bounds) else {
                return false;
            };
            hud.content.cacheDisplayInRect_toBitmapImageRep(bounds, &rep);
            // SAFETY: the rep now holds drawn pixels, and PNG takes no required properties.
            let data = unsafe {
                rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
            };
            data.is_some_and(|data| std::fs::write(&path, data.to_vec()).is_ok())
        })
        .unwrap_or(false)
    })
    .unwrap_or(false)
}

/// Steps the pill in, holds it, and steps it out. Each step checks that this is
/// still the notice on screen.
fn animate(app: &AppHandle, generation: u64) {
    let reduce_motion = on_main(app, reduce_motion).unwrap_or(false);
    let rise = if reduce_motion { 0.0 } else { RISE };

    for step in 1..=FADE_IN_STEPS {
        thread::sleep(FADE_IN / FADE_IN_STEPS);
        let progress = f64::from(step) / f64::from(FADE_IN_STEPS);
        if !frame(app, generation, progress, rise * (1.0 - progress)) {
            return;
        }
    }
    thread::sleep(HOLD);
    for step in 1..=FADE_OUT_STEPS {
        thread::sleep(FADE_OUT / FADE_OUT_STEPS);
        let progress = f64::from(step) / f64::from(FADE_OUT_STEPS);
        if !frame(app, generation, 1.0 - progress, 0.0) {
            return;
        }
    }
    on_main(app, move || {
        if GENERATION.load(Ordering::SeqCst) == generation {
            with_panel(|hud| hud.panel.orderOut(None));
        }
    });
}

/// Applies one animation step. Returns false once this notice has been superseded.
fn frame(app: &AppHandle, generation: u64, alpha: f64, offset: f64) -> bool {
    if GENERATION.load(Ordering::SeqCst) != generation {
        return false;
    }
    on_main(app, move || {
        if GENERATION.load(Ordering::SeqCst) != generation {
            return false;
        }
        with_panel(|hud| {
            hud.panel.setAlphaValue(alpha);
            let mut origin = hud.panel.frame().origin;
            origin.y = hud.base_y - offset;
            hud.panel.setFrameOrigin(origin);
        });
        true
    })
    .unwrap_or(false)
}

fn reduce_motion() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

// ── The panel ────────────────────────────────────────────────────────────────

struct Hud {
    panel: Retained<NSPanel>,
    content: Retained<NSView>,
    /// The shadow, the pill and the chip's background, drawn as one image.
    pill: Retained<NSImageView>,
    icon: Retained<NSImageView>,
    arrow: Retained<NSImageView>,
    from: Retained<NSTextField>,
    to: Retained<NSTextField>,
    message: Retained<NSTextField>,
    chip: Retained<NSTextField>,
    /// Where the panel sits once it has finished rising.
    base_y: f64,
}

thread_local! {
    /// Built once and reused: the panel is cheap to keep and costs nothing while hidden.
    static PANEL: std::cell::RefCell<Option<Hud>> = const { std::cell::RefCell::new(None) };
}

/// Runs `f` against the panel, building it on first use. Main thread only.
fn with_panel<T>(f: impl FnOnce(&mut Hud) -> T) -> Option<T> {
    let mtm = MainThreadMarker::new()?;
    PANEL.with(|cell| {
        let mut slot = cell.borrow_mut();
        let hud = slot.get_or_insert_with(|| build(mtm));
        Some(f(hud))
    })
}

/// A view on the pill and the width it takes, in the order they read on screen.
enum Item<'a> {
    Image(&'a NSImageView, f64),
    Text(&'a NSTextField, f64),
    /// The chip: its label, and the width of its background.
    Chip(&'a NSTextField, f64),
}

impl Item<'_> {
    fn width(&self) -> f64 {
        match self {
            Item::Image(_, width) | Item::Text(_, width) | Item::Chip(_, width) => *width,
        }
    }
}

fn present(notice: Notice) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let dark = dark_appearance(mtm);
    with_panel(|hud| {
        let (icon, tint) = notice.kind.icon();
        set_icon(&hud.icon, icon, ICON, tint);

        let conversion = matches!(notice.body, Body::Conversion { .. });
        hud.from.setHidden(!conversion);
        hud.to.setHidden(!conversion);
        hud.arrow.setHidden(!conversion);
        hud.message.setHidden(conversion);

        let mut items: Vec<Item> = vec![Item::Image(&hud.icon, ICON)];
        match &notice.body {
            Body::Conversion { from, to } => {
                hud.from.setFont(Some(&text_font(from, false)));
                hud.to.setFont(Some(&text_font(to, true)));
                let from_width = measure(&hud.from, from);
                let to_width = measure(&hud.to, to);
                // The arrow points the way the interface reads.
                let arrow = if notice.rtl { ICON_ARROW_LEFT } else { ICON_ARROW_RIGHT };
                set_icon(&hud.arrow, arrow, ARROW, HUD_MUTED);
                items.push(Item::Text(&hud.from, from_width));
                items.push(Item::Image(&hud.arrow, ARROW));
                items.push(Item::Text(&hud.to, to_width));
            }
            Body::Message(text) => {
                hud.message.setFont(Some(&text_font(text, false)));
                let width = measure(&hud.message, text);
                items.push(Item::Text(&hud.message, width));
            }
        }

        let chip = conversion.then_some(notice.badge.as_deref()).flatten();
        hud.chip.setHidden(chip.is_none());
        if let Some(label) = chip {
            hud.chip.setFont(Some(&chip_font(label)));
            let width = measure(&hud.chip, label) + CHIP_PAD * 2.0;
            items.push(Item::Chip(&hud.chip, width));
        }

        // Read in the interface's direction: the icon leads, the chip trails.
        let (lead, trail) = if conversion {
            (PAD_ICON_SIDE, if chip.is_some() { PAD_CHIP_SIDE } else { PAD_ICON_SIDE })
        } else {
            (PAD, PAD)
        };
        let content: f64 = items.iter().map(Item::width).sum::<f64>() + GAP * (items.len() as f64 - 1.0);
        let width = (lead + content + trail).ceil();
        if notice.rtl {
            items.reverse();
        }
        let start = if notice.rtl { trail } else { lead };
        let chip_rect = place(&items, SHADOW_PAD + start);

        let border = if dark { HUD_BORDER_DARK } else { HUD_BORDER_LIGHT };
        position(hud, width, mtm);
        hud.pill.setImage(Some(&pill(width, border, chip_rect)));

        hud.panel.setAlphaValue(0.0);
        hud.panel.orderFrontRegardless();
    });
}

/// Lays the items out left to right from `x`, each centred on the pill's height, and returns
/// the chip's background rectangle (in panel coordinates) when there is one.
fn place(items: &[Item], mut x: f64) -> Option<NSRect> {
    let middle = SHADOW_PAD + HEIGHT / 2.0;
    let mut chip = None;
    for item in items {
        match item {
            Item::Image(view, size) => {
                view.setFrame(NSRect::new(NSPoint::new(x, middle - size / 2.0), NSSize::new(*size, *size)));
            }
            Item::Text(field, width) => {
                let height = field.frame().size.height;
                field.setFrame(NSRect::new(
                    NSPoint::new(x, (middle - height / 2.0).round()),
                    NSSize::new(*width, height),
                ));
            }
            Item::Chip(field, width) => {
                let rect =
                    NSRect::new(NSPoint::new(x, middle - CHIP_HEIGHT / 2.0), NSSize::new(*width, CHIP_HEIGHT));
                let height = field.frame().size.height;
                field.setFrame(NSRect::new(
                    NSPoint::new(x + CHIP_PAD, (middle - height / 2.0).round()),
                    NSSize::new(width - CHIP_PAD * 2.0, height),
                ));
                chip = Some(rect);
            }
        }
        x += item.width() + GAP;
    }
    chip
}

/// Places the panel: the pill centred above the Dock, with room around it for its shadow.
fn position(hud: &mut Hud, width: f64, mtm: MainThreadMarker) {
    let screen = visible_frame(mtm);
    let size = NSSize::new(width + SHADOW_PAD * 2.0, HEIGHT + SHADOW_PAD * 2.0);
    let x = (screen.origin.x + (screen.size.width - width) / 2.0 - SHADOW_PAD).round();
    hud.base_y = (screen.origin.y + ABOVE_DOCK - SHADOW_PAD).round();
    hud.panel.setFrame_display(NSRect::new(NSPoint::new(x, hud.base_y), size), true);
    let bounds = NSRect::new(NSPoint::new(0.0, 0.0), size);
    hud.content.setFrame(bounds);
    hud.pill.setFrame(bounds);
}

fn build(mtm: MainThreadMarker) -> Hud {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, HEIGHT));
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        mtm.alloc(),
        rect,
        // Borderless and non-activating: showing the notice must never take keyboard
        // focus away from the app the user is typing in.
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setOpaque(false);
    // The pill draws its own shadow (Effect/HUD); the window's would be a second one.
    panel.setHasShadow(false);
    panel.setIgnoresMouseEvents(true);
    // Explicit rather than left to NSPanel's defaults: the app is inactive whenever a
    // notice shows, so the panel must not hide with it, and it must never ask to be key.
    panel.setHidesOnDeactivate(false);
    panel.setFloatingPanel(true);
    panel.setBecomesKeyOnlyIfNeeded(true);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    // Above ordinary windows and full-screen apps, but below the menu bar.
    panel.setLevel(FLOATING_WINDOW_LEVEL);
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );

    let content = NSView::initWithFrame(mtm.alloc(), rect);
    let pill_view = NSImageView::initWithFrame(mtm.alloc(), rect);
    let icon = NSImageView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(ICON, ICON)));
    let arrow = NSImageView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(ARROW, ARROW)));
    let from = plain_label(HUD_MUTED, mtm);
    let to = plain_label(HUD_TEXT, mtm);
    let message = plain_label(HUD_TEXT, mtm);
    let chip = plain_label(HUD_TEXT, mtm);

    content.addSubview(&pill_view);
    for view in [&icon, &arrow] {
        content.addSubview(view);
    }
    for field in [&from, &to, &message, &chip] {
        content.addSubview(field);
    }
    panel.setContentView(Some(&content));

    Hud { panel, content, pill: pill_view, icon, arrow, from, to, message, chip, base_y: 0.0 }
}

/// Sets a label's text and returns the width it needs.
///
/// `sizeToFit` wraps to the field's current width, so a label left at the width of
/// the previous notice would clip this one. Widening it first lets the text decide.
fn measure(field: &NSTextField, text: &str) -> f64 {
    field.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(MEASURE_WIDTH, HEIGHT)));
    field.setStringValue(&NSString::from_str(text));
    field.sizeToFit();
    field.frame().size.width
}

/// Wider than any notice: the conversion line is capped well below this.
const MEASURE_WIDTH: f64 = 2000.0;

/// The pill is drawn at this multiple of its point size, for Retina displays.
const PILL_SCALE: f64 = 2.0;

/// A non-editable, non-selectable text field: a label, not a control.
fn plain_label(colour: [f64; 4], mtm: MainThreadMarker) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    field.setTextColor(Some(&srgb(colour)));
    field.setDrawsBackground(false);
    field.setBezeled(false);
    field.setEditable(false);
    field.setSelectable(false);
    field
}

/// Shows one of the template icons at `size` points, tinted.
fn set_icon(view: &NSImageView, png: &'static [u8], size: f64, tint: [f64; 4]) {
    let data = NSData::with_bytes(png);
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        image.setSize(NSSize::new(size, size));
        image.setTemplate(true);
        view.setImage(Some(&image));
    }
    view.setContentTintColor(Some(&srgb(tint)));
}

fn srgb([r, g, b, a]: [f64; 4]) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a)
}

/// The shadow, the pill and (when there is one) the chip's background, at the width of the
/// current notice, in an image the size of the panel.
///
/// A drawn shape rather than a view with rounded corners: rounding a view directly would
/// mean pulling in Core Animation.
fn pill(width: f64, border: [f64; 4], chip: Option<NSRect>) -> Retained<NSImage> {
    let size = NSSize::new(width + SHADOW_PAD * 2.0, HEIGHT + SHADOW_PAD * 2.0);
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    // Drawn at twice the size and declared at the point size, so it stays crisp on
    // a Retina display.
    let pixels = |points: f64| (points * PILL_SCALE) as isize;
    // SAFETY: a null `planes` pointer asks AppKit to own the pixel buffer; the rest
    // describes a plain 8-bit RGBA bitmap of the given size.
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(), std::ptr::null_mut(), pixels(size.width), pixels(size.height), 8, 4, true, false,
            NSDeviceRGBColorSpace, 0, 0,
        )
    };
    if let Some(rep) = rep {
        rep.setSize(size);
        if let Some(context) = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep) {
            NSGraphicsContext::saveGraphicsState_class();
            NSGraphicsContext::setCurrentContext(Some(&context));
            let radius = (HEIGHT - 1.0) / 2.0;
            // Inset by half the border width so the hairline lands inside the pill.
            let shape = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                NSRect::new(NSPoint::new(SHADOW_PAD + 0.5, SHADOW_PAD + 0.5), NSSize::new(width - 1.0, HEIGHT - 1.0)),
                radius,
                radius,
            );

            // The shadow goes with the fill only. Quartz takes shadow offsets and blur in
            // device pixels, not in the points the rest of the drawing uses.
            NSGraphicsContext::saveGraphicsState_class();
            let shadow = NSShadow::new();
            shadow.setShadowOffset(NSSize::new(0.0, -SHADOW_Y * PILL_SCALE));
            shadow.setShadowBlurRadius(SHADOW_BLUR * PILL_SCALE);
            shadow.setShadowColor(Some(&srgb(HUD_SHADOW)));
            shadow.set();
            srgb(HUD_BG).set();
            shape.fill();
            NSGraphicsContext::restoreGraphicsState_class();

            srgb(border).set();
            shape.setLineWidth(1.0);
            shape.stroke();

            if let Some(rect) = chip {
                srgb(HUD_CHIP).set();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect, CHIP_RADIUS, CHIP_RADIUS).fill();
            }
            NSGraphicsContext::restoreGraphicsState_class();
        }
        image.addRepresentation(&rep);
    }
    image
}

/// Whether the system is in dark mode, for the one value the HUD varies by appearance.
fn dark_appearance(mtm: MainThreadMarker) -> bool {
    NSApplication::sharedApplication(mtm).effectiveAppearance().name().to_string().contains("Dark")
}

// ── Fonts ────────────────────────────────────────────────────────────────────

/// Whether a run is Arabic, which decides its face: Almarai for Arabic, SF Pro for Latin,
/// as the A2 type roles set them.
fn is_arabic(text: &str) -> bool {
    text.chars().any(|c| matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}'))
}

/// Body/Regular or Body/Strong in Arabic; Latin/Body or Latin/Body Strong otherwise.
fn text_font(text: &str, strong: bool) -> Retained<NSFont> {
    if is_arabic(text) {
        almarai(TEXT_ARABIC, strong)
    } else {
        NSFont::systemFontOfSize_weight(TEXT_LATIN, if strong { WEIGHT_SEMIBOLD } else { 0.0 })
    }
}

/// The chip names a layout: «ع» in Caption/Strong, «EN» in Keycap/SM.
fn chip_font(label: &str) -> Retained<NSFont> {
    if is_arabic(label) {
        almarai(CHIP_ARABIC, true)
    } else {
        NSFont::systemFontOfSize_weight(CHIP_LATIN, WEIGHT_MEDIUM)
    }
}

/// Almarai ships inside the app, so the notice reads the same on every Mac.
///
/// The font is built straight from the embedded file rather than registered with
/// the system: nothing is installed, and no other app sees it. `CTFont` and
/// `NSFont` are the same object, so the result needs no conversion.
fn almarai(size: f64, bold: bool) -> Retained<NSFont> {
    let bytes = if bold { ALMARAI_BOLD } else { ALMARAI_REGULAR };
    font_from(bytes, size).unwrap_or_else(|| NSFont::systemFontOfSize(size))
}

fn font_from(bytes: &'static [u8], size: f64) -> Option<Retained<NSFont>> {
    let data = CFData::from_buffer(bytes);
    // SAFETY: valid CFData holding a complete TrueType file; the descriptor comes
    // back under the create rule.
    let descriptor = unsafe { CTFontManagerCreateFontDescriptorFromData(data.as_concrete_TypeRef()) };
    if descriptor.is_null() {
        return None;
    }
    let descriptor = unsafe { CFType::wrap_under_create_rule(descriptor) };
    // SAFETY: valid descriptor; a null transform means "no transform". The font is
    // created, and `NSFont` is the toll-free bridged form of `CTFont`.
    let font = unsafe { CTFontCreateWithFontDescriptor(descriptor.as_CFTypeRef(), size, std::ptr::null()) };
    unsafe { Retained::from_raw(font.cast_mut().cast()) }
}

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    fn CTFontManagerCreateFontDescriptorFromData(data: CFDataRef) -> CFTypeRef;
    fn CTFontCreateWithFontDescriptor(descriptor: CFTypeRef, size: f64, matrix: *const c_void) -> CFTypeRef;
}

/// `NSFloatingWindowLevel`: above ordinary windows, below the menu bar.
const FLOATING_WINDOW_LEVEL: isize = 3;

/// The part of the screen the Dock and menu bar leave free.
fn visible_frame(mtm: MainThreadMarker) -> NSRect {
    NSScreen::mainScreen(mtm)
        .map(|screen| screen.visibleFrame())
        .unwrap_or_else(|| NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1440.0, 900.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_runs_are_told_apart_from_latin_ones() {
        assert!(is_arabic("اثممخ"));
        assert!(is_arabic("ع"));
        assert!(!is_arabic("hello"));
        assert!(!is_arabic("EN"));
    }

    /// The pill draws its shadow inside the panel image: transparent at the far corners,
    /// opaque ink in the middle, and a soft shade just under the pill, never above it.
    #[test]
    fn the_pill_carries_one_soft_shadow_below_it() {
        let width = 120.0;
        let image = pill(width, HUD_BORDER_LIGHT, None);
        let reps = image.representations();
        let rep = reps.firstObject().expect("a bitmap");
        let rep: Retained<NSBitmapImageRep> = rep.downcast().expect("a bitmap rep");
        let alpha = |x: f64, y_from_top: f64| -> f64 {
            let colour = rep.colorAtX_y((x * PILL_SCALE) as isize, (y_from_top * PILL_SCALE) as isize);
            colour.map(|c| c.alphaComponent()).unwrap_or(0.0)
        };
        let middle_x = SHADOW_PAD + width / 2.0;
        // Bitmap rows run top to bottom: the pill spans SHADOW_PAD .. SHADOW_PAD + HEIGHT.
        assert!(alpha(1.0, 1.0) < 0.01, "the corner stays clear");
        assert!(alpha(middle_x, SHADOW_PAD + HEIGHT / 2.0) > 0.99, "the pill is opaque");
        let below = alpha(middle_x, SHADOW_PAD + HEIGHT + SHADOW_Y);
        let above = alpha(middle_x, SHADOW_PAD - SHADOW_Y);
        assert!(below > 0.02 && below < 0.2, "a soft shade under the pill: {below}");
        assert!(above < below, "the shadow falls downwards: above {above}, below {below}");
    }

    /// Every state's icon is a template exported from `Icon/*`: 16pt at @2x, black and alpha only,
    /// so the tint alone decides its colour.
    #[test]
    fn every_state_icon_is_a_black_template_at_2x() {
        let kinds = [
            Kind::Success,
            Kind::Undone,
            Kind::Blocked,
            Kind::TooLong,
            Kind::NoText,
            Kind::Unchanged,
            Kind::Failed,
            Kind::NoLayouts,
            Kind::NoPermission,
            Kind::Elsewhere,
            Kind::Paused,
            Kind::Excluded,
            Kind::NotHere,
            Kind::UndoFailed,
            Kind::Copied,
        ];
        for kind in kinds {
            let (png, _) = kind.icon();
            let rep = NSBitmapImageRep::imageRepWithData(&NSData::with_bytes(png)).expect("a PNG");
            assert_eq!((rep.pixelsWide(), rep.pixelsHigh()), (32, 32), "{kind:?}");
            let mut ink = 0;
            for y in 0..32 {
                for x in 0..32 {
                    let colour = rep.colorAtX_y(x, y).expect("a pixel");
                    if colour.alphaComponent() > 0.0 {
                        ink += 1;
                        let rgb = [colour.redComponent(), colour.greenComponent(), colour.blueComponent()];
                        assert!(rgb.iter().all(|&c| c < 0.01), "{kind:?}: a coloured pixel at {x},{y}");
                    }
                }
            }
            assert!(ink > 20, "{kind:?}: the glyph is there");
        }
    }
}
