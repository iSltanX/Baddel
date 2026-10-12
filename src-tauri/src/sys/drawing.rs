//! The AppKit and Core Text calls the conversion notice ([`crate::hud`]) draws with.

use std::ffi::c_void;

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::data::{CFData, CFDataRef};
use objc2::rc::Retained;
use objc2::AllocAnyThread;
use objc2_app_kit::{NSBitmapImageRep, NSDeviceRGBColorSpace, NSFont};

/// An empty 8-bit RGBA bitmap of `width` × `height` pixels, its buffer owned by AppKit.
pub fn rgba_bitmap(width: isize, height: isize) -> Option<Retained<NSBitmapImageRep>> {
    // SAFETY: a null `planes` pointer asks AppKit to own the pixel buffer; the rest
    // describes a plain 8-bit RGBA bitmap of the given size.
    unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(), std::ptr::null_mut(), width, height, 8, 4, true, false,
            NSDeviceRGBColorSpace, 0, 0,
        )
    }
}

/// A drawn bitmap as PNG bytes.
#[cfg(debug_assertions)]
pub fn png(rep: &NSBitmapImageRep) -> Option<Vec<u8>> {
    use objc2_app_kit::NSBitmapImageFileType;
    use objc2_foundation::NSDictionary;
    // SAFETY: the rep holds drawn pixels, and PNG takes no required properties.
    let data = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) };
    data.map(|data| data.to_vec())
}

/// A font built straight from a TrueType file's bytes, without registering it with the system:
/// nothing is installed, and no other app sees it. `CTFont` and `NSFont` are the same object, so
/// the result needs no conversion.
pub fn font_from(bytes: &'static [u8], size: f64) -> Option<Retained<NSFont>> {
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
