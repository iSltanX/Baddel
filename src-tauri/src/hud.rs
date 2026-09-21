//! The conversion notice: a small floating panel near the bottom of the screen.
//!
//! Native on purpose. A webview window would keep a WebKit process alive while the
//! app idles, and showing one risks pulling focus away from the app the user is
//! typing in — this panel is non-activating and ignores the mouse entirely.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::data::{CFData, CFDataRef};
use objc2::rc::Retained;
use objc2::{AllocAnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameVibrantDark, NSBackingStoreType, NSBezierPath,
    NSBitmapImageFileType, NSBitmapImageRep, NSColor, NSDeviceRGBColorSpace, NSFont, NSGraphicsContext, NSImage, NSImageView, NSPanel,
    NSScreen, NSTextField, NSView, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};
use tauri::{AppHandle, Manager};

use crate::sys::on_main;

const ALMARAI_REGULAR: &[u8] = include_bytes!("../../src/assets/fonts/Almarai-Regular.ttf");
const ALMARAI_BOLD: &[u8] = include_bytes!("../../src/assets/fonts/Almarai-Bold.ttf");

// ── Geometry (docs/baddel-figma-prompt-pro.md · PROMPT 5) ────────────────────
const HEIGHT: f64 = 36.0;
const PAD_X: f64 = 14.0;
const GAP: f64 = 8.0;
const ICON: f64 = 15.0;
const TEXT_SIZE: f64 = 14.0;
const BADGE_SIZE: f64 = 11.0;
/// Distance from the top of the Dock to the bottom of the pill.
const ABOVE_DOCK: f64 = 80.0;
/// How far the pill travels upwards as it appears.
const RISE: f64 = 4.0;

// ── Motion ───────────────────────────────────────────────────────────────────
const FADE_IN: Duration = Duration::from_millis(120);
const HOLD: Duration = Duration::from_millis(900);
const FADE_OUT: Duration = Duration::from_millis(200);
const FADE_IN_STEPS: u32 = 6;
const FADE_OUT_STEPS: u32 = 8;
/// How long, and how often, the launch-time watch looks for a stray activation (3s in all).
const PRIME_CHECKS: u32 = 30;
const PRIME_INTERVAL: Duration = Duration::from_millis(100);

/// Isolates a bidirectional run so the two sides of a conversion keep their own
/// direction inside one line, whichever scripts they are.
const FSI: char = '\u{2068}';
const PDI: char = '\u{2069}';
/// Forces the line's own direction, so the arrow always points the way the UI reads.
const RLI: char = '\u{2067}';
const LRI: char = '\u{2066}';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Success,
    Undone,
    Blocked,
    TooLong,
    NoText,
}

impl Kind {
    /// SF Symbol and its tint. State is never carried by colour alone — every
    /// variant has its own symbol too.
    fn symbol(self) -> (&'static str, [f64; 3]) {
        const WHITE: [f64; 3] = [1.0, 1.0, 1.0];
        const SUCCESS: [f64; 3] = [0.576, 0.710, 0.522]; // #93B585
        const WARNING: [f64; 3] = [0.851, 0.663, 0.353]; // #D9A95A
        match self {
            Kind::Success => ("checkmark.circle.fill", SUCCESS),
            Kind::Undone => ("arrow.uturn.backward", WHITE),
            Kind::Blocked => ("lock.fill", WARNING),
            Kind::TooLong => ("exclamationmark.triangle.fill", WARNING),
            Kind::NoText => ("info.circle", WHITE),
        }
    }
}

/// One notice to display.
pub struct Notice {
    pub kind: Kind,
    /// The message, already localized.
    pub text: String,
    /// The input source we switched to ("EN" / "ع"), shown as a small badge.
    pub badge: Option<String>,
}

/// Bumped on every show; a fade still running for an older notice sees the change
/// and gives up instead of hiding the new one.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Builds the conversion line with the arrow pointing the way the UI reads.
pub fn conversion_line(from: &str, to: &str, rtl: bool) -> String {
    let (open, arrow) = if rtl { (RLI, '←') } else { (LRI, '→') };
    format!("{open}{FSI}{from}{PDI} {arrow} {FSI}{to}{PDI}{PDI}")
}

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
    pill: Retained<NSImageView>,
    icon: Retained<NSImageView>,
    label: Retained<NSTextField>,
    badge: Retained<NSTextField>,
    /// Where the pill sits once it has finished rising.
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

fn present(notice: Notice) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    with_panel(|hud| {
        let (symbol, tint) = notice.kind.symbol();
        set_symbol(&hud.icon, symbol, tint, mtm);
        let text_width = measure(&hud.label, &notice.text);

        let badge_width = match &notice.badge {
            Some(text) => {
                hud.badge.setHidden(false);
                measure(&hud.badge, text) + GAP
            }
            None => {
                hud.badge.setHidden(true);
                0.0
            }
        };

        let width = (PAD_X * 2.0 + ICON + GAP + text_width + badge_width).ceil();
        layout(hud, width, text_width, badge_width, mtm);

        hud.panel.setAlphaValue(0.0);
        hud.panel.orderFrontRegardless();
    });
}

/// Places the pill on screen and its contents inside it.
fn layout(hud: &mut Hud, width: f64, text_width: f64, badge_width: f64, mtm: MainThreadMarker) {
    let screen = visible_frame(mtm);
    let x = (screen.origin.x + (screen.size.width - width) / 2.0).round();
    hud.base_y = (screen.origin.y + ABOVE_DOCK).round();
    hud.panel.setFrame_display(NSRect::new(NSPoint::new(x, hud.base_y), NSSize::new(width, HEIGHT)), true);

    let bounds = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, HEIGHT));
    hud.content.setFrame(bounds);
    hud.pill.setFrame(bounds);
    hud.pill.setImage(Some(&pill(width, HEIGHT)));

    // Laid out left to right in panel coordinates: the icon leads, the badge trails.
    // The text itself carries the bidi controls that order its two sides.
    let mut x = PAD_X;
    hud.icon.setFrame(NSRect::new(NSPoint::new(x, (HEIGHT - ICON) / 2.0), NSSize::new(ICON, ICON)));
    x += ICON + GAP;
    let text_height = hud.label.frame().size.height;
    hud.label.setFrame(NSRect::new(
        NSPoint::new(x, ((HEIGHT - text_height) / 2.0).round()),
        NSSize::new(text_width, text_height),
    ));
    if badge_width > 0.0 {
        x += text_width + GAP;
        let badge_height = hud.badge.frame().size.height;
        hud.badge.setFrame(NSRect::new(
            NSPoint::new(x, ((HEIGHT - badge_height) / 2.0).round()),
            NSSize::new(badge_width - GAP, badge_height),
        ));
    }
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
    panel.setHasShadow(true);
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
    // The notice is dark in both appearances (PROMPT 5), so it does not follow the theme.
    panel.setAppearance(NSAppearance::appearanceNamed(unsafe { NSAppearanceNameVibrantDark }).as_deref());

    let content = NSView::initWithFrame(mtm.alloc(), rect);
    let pill_view = NSImageView::initWithFrame(mtm.alloc(), rect);

    let icon = NSImageView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(ICON, ICON)));
    let label = plain_label(TEXT_SIZE, false, mtm);
    let badge = plain_label(BADGE_SIZE, true, mtm);
    badge.setTextColor(Some(&NSColor::colorWithWhite_alpha(1.0, 0.7)));

    content.addSubview(&pill_view);
    content.addSubview(&icon);
    content.addSubview(&label);
    content.addSubview(&badge);
    panel.setContentView(Some(&content));

    Hud { panel, content, pill: pill_view, icon, label, badge, base_y: 0.0 }
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
fn plain_label(size: f64, bold: bool, mtm: MainThreadMarker) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    field.setFont(Some(&almarai(size, bold)));
    field.setTextColor(Some(&NSColor::whiteColor()));
    field.setDrawsBackground(false);
    field.setBezeled(false);
    field.setEditable(false);
    field.setSelectable(false);
    field
}

fn set_symbol(view: &NSImageView, name: &str, tint: [f64; 3], _mtm: MainThreadMarker) {
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), None);
    if let Some(image) = image {
        image.setTemplate(true);
        view.setImage(Some(&image));
    }
    view.setContentTintColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(tint[0], tint[1], tint[2], 1.0)));
}

/// The pill itself, drawn at the width of the current message.
///
/// A drawn shape rather than a vibrancy view behind a mask: the mask stretches, and
/// rounding a view's corners directly would mean pulling in Core Animation. The
/// fill is nearly opaque, so the notice reads the same over any window.
fn pill(width: f64, height: f64) -> Retained<NSImage> {
    let size = NSSize::new(width, height);
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    // Drawn at twice the size and declared at the point size, so it stays crisp on
    // a Retina display.
    let pixels = |points: f64| (points * PILL_SCALE) as isize;
    // SAFETY: a null `planes` pointer asks AppKit to own the pixel buffer; the rest
    // describes a plain 8-bit RGBA bitmap of the given size.
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(), std::ptr::null_mut(), pixels(width), pixels(height), 8, 4, true, false,
            NSDeviceRGBColorSpace, 0, 0,
        )
    };
    if let Some(rep) = rep {
        rep.setSize(size);
        if let Some(context) = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep) {
            NSGraphicsContext::saveGraphicsState_class();
            NSGraphicsContext::setCurrentContext(Some(&context));
            let radius = (height - 1.0) / 2.0;
            // Inset by half the border width so the stroke lands inside the image.
            let shape = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                NSRect::new(NSPoint::new(0.5, 0.5), NSSize::new(width - 1.0, height - 1.0)),
                radius,
                radius,
            );
            NSColor::colorWithSRGBRed_green_blue_alpha(0.078, 0.082, 0.094, 0.94).set();
            shape.fill();
            NSColor::colorWithWhite_alpha(1.0, 0.08).set();
            shape.setLineWidth(1.0);
            shape.stroke();
            NSGraphicsContext::restoreGraphicsState_class();
        }
        image.addRepresentation(&rep);
    }
    image
}

// ── Fonts ────────────────────────────────────────────────────────────────────

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
