//! Preparing the image a user attaches to a problem report.
//!
//! The picture is decoded and drawn again into a fresh bitmap, so nothing of the original file
//! survives but its pixels: no EXIF, no GPS, no colour-profile comments, no file name. It is
//! scaled down if it is larger than a report needs, and encoded as PNG — or as JPEG when a PNG
//! would not fit under [`MAX_BYTES`]. Runs off the main thread: drawing into an offscreen bitmap
//! context is safe there, and a large photo takes longer than the main thread should be held.

use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::AnyObject;
use objc2::AllocAnyThread;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSDeviceRGBColorSpace, NSGraphicsContext, NSImageCompressionFactor,
    NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeTIFF,
};
use objc2_foundation::{NSData, NSDictionary, NSNumber, NSPoint, NSRect, NSSize};

/// The most a report attachment may weigh once prepared (the shared contract's ceiling).
pub const MAX_BYTES: usize = 3 * 1024 * 1024;
/// The largest file the picker accepts before preparing it.
pub const MAX_INPUT_BYTES: usize = 20 * 1024 * 1024;
/// Longest side of the prepared image, in pixels. A screenshot of a 5K display is halved;
/// anything a report needs to show stays legible.
const MAX_SIDE: f64 = 2560.0;
/// Longest side of the preview thumbnail.
const THUMB_SIDE: f64 = 160.0;
const JPEG_QUALITY: f64 = 0.85;

#[derive(Debug, PartialEq, Eq)]
pub enum ImageError {
    /// Not PNG or JPEG (from a file), or not an image at all.
    Unsupported,
    TooLarge,
    /// Decoded, but could not be drawn or encoded.
    Failed,
}

pub struct Prepared {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub width: u32,
    pub height: u32,
    /// A small PNG for the preview.
    pub thumbnail: Vec<u8>,
}

/// PNG or JPEG, by signature. The picker's extension filter is a convenience, not a check.
pub fn is_png_or_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// Prepares a file the user picked. Only PNG and JPEG are accepted.
pub fn prepare_file(bytes: &[u8]) -> Result<Prepared, ImageError> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(ImageError::TooLarge);
    }
    if !is_png_or_jpeg(bytes) {
        return Err(ImageError::Unsupported);
    }
    prepare(bytes)
}

/// The image on the general pasteboard, if there is one, prepared. macOS screenshots copied
/// with ⌃⇧⌘4 arrive as PNG or TIFF.
pub fn prepare_pasteboard() -> Option<Result<Prepared, ImageError>> {
    let data = autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        // SAFETY: AppKit constants, valid for the life of the process.
        let types = unsafe { [NSPasteboardTypePNG, NSPasteboardTypeTIFF] };
        types.iter().find_map(|ty| pasteboard.dataForType(ty)).map(|data| data.to_vec())
    })?;
    if data.len() > MAX_INPUT_BYTES {
        return Some(Err(ImageError::TooLarge));
    }
    Some(prepare(&data))
}

fn prepare(bytes: &[u8]) -> Result<Prepared, ImageError> {
    autoreleasepool(|_| prepare_in_pool(bytes))
}

fn prepare_in_pool(bytes: &[u8]) -> Result<Prepared, ImageError> {
    {
        let source = NSBitmapImageRep::imageRepWithData(&NSData::with_bytes(bytes)).ok_or(ImageError::Unsupported)?;
        let (w, h) = (source.pixelsWide() as f64, source.pixelsHigh() as f64);
        if w < 1.0 || h < 1.0 {
            return Err(ImageError::Unsupported);
        }
        let mut scale = (MAX_SIDE / w.max(h)).min(1.0);
        let thumb_scale = (THUMB_SIDE / w.max(h)).min(1.0);
        let thumb = redraw(&source, w * thumb_scale, h * thumb_scale)?;
        let thumbnail = encode(&thumb, false)?;
        loop {
            let (width, height) = ((w * scale).round().max(1.0), (h * scale).round().max(1.0));
            let bitmap = redraw(&source, width, height)?;
            let png = encode(&bitmap, false)?;
            let (bytes, mime) = if png.len() <= MAX_BYTES {
                (png, "image/png")
            } else {
                (encode(&bitmap, true)?, "image/jpeg")
            };
            if bytes.len() <= MAX_BYTES {
                return Ok(Prepared { bytes, mime, width: width as u32, height: height as u32, thumbnail });
            }
            if width <= 320.0 {
                return Err(ImageError::TooLarge);
            }
            scale *= 0.8;
        }
    }
}

/// Draws `source` into a new 8-bit RGBA bitmap of the given size. The new bitmap carries
/// pixels only: none of the source's metadata comes along.
fn redraw(source: &NSBitmapImageRep, width: f64, height: f64) -> Result<Retained<NSBitmapImageRep>, ImageError> {
    let (width, height) = (width.round().max(1.0), height.round().max(1.0));
    // SAFETY: a null `planes` pointer asks AppKit to own the pixel buffer; the geometry describes
    // a plain 8-bit RGBA bitmap, and a row/pixel stride of 0 lets AppKit choose it.
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            width as isize,
            height as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )
    }
    .ok_or(ImageError::Failed)?;
    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap).ok_or(ImageError::Failed)?;
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));
    let drawn = source.drawInRect(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height)));
    NSGraphicsContext::restoreGraphicsState_class();
    drawn.then_some(bitmap).ok_or(ImageError::Failed)
}

fn encode(bitmap: &NSBitmapImageRep, jpeg: bool) -> Result<Vec<u8>, ImageError> {
    let data = if jpeg {
        let quality = NSNumber::new_f64(JPEG_QUALITY);
        let value: &AnyObject = &quality;
        // SAFETY: AppKit constant key; NSImageCompressionFactor takes an NSNumber between 0 and 1.
        let properties = unsafe { NSDictionary::from_slices(&[NSImageCompressionFactor], &[value]) };
        // SAFETY: the bitmap holds drawn pixels and the properties are of the documented types.
        unsafe { bitmap.representationUsingType_properties(NSBitmapImageFileType::JPEG, &properties) }
    } else {
        // SAFETY: as above; PNG takes no required properties.
        unsafe { bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }
    };
    data.map(|d| d.to_vec()).ok_or(ImageError::Failed)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn signatures() {
        assert!(is_png_or_jpeg(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]));
        assert!(is_png_or_jpeg(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(!is_png_or_jpeg(b"GIF89a"));
        assert!(!is_png_or_jpeg(b"II*\0")); // TIFF: only accepted from the pasteboard
        assert!(!is_png_or_jpeg(&[]));
    }

    /// A blank bitmap of the given size, encoded as PNG, as a picked file would arrive.
    pub(crate) fn png(width: f64, height: f64) -> Vec<u8> {
        autoreleasepool(|_| {
        // SAFETY: as in `redraw`.
        let bitmap = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(), std::ptr::null_mut(), width as isize, height as isize, 8, 4, true, false,
                NSDeviceRGBColorSpace, 0, 0,
            )
        }
        .unwrap();
        encode(&bitmap, false).unwrap()
        })
    }

    /// Inserts a `tEXt` chunk after IHDR, the way a camera or editor leaves notes in a file.
    fn with_text_chunk(png: &[u8], text: &[u8]) -> Vec<u8> {
        fn crc32(bytes: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFFu32;
            for &b in bytes {
                crc ^= u32::from(b);
                for _ in 0..8 {
                    crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
                }
            }
            !crc
        }
        let ihdr_end = 8 + 4 + 4 + 13 + 4;
        let mut body = b"tEXt".to_vec();
        body.extend_from_slice(text);
        let mut chunk = (text.len() as u32).to_be_bytes().to_vec();
        chunk.extend_from_slice(&body);
        chunk.extend_from_slice(&crc32(&body).to_be_bytes());
        [&png[..ihdr_end], &chunk[..], &png[ihdr_end..]].concat()
    }

    #[test]
    fn metadata_does_not_survive() {
        let marked = with_text_chunk(&png(64.0, 48.0), b"Comment\0secret-note");
        assert!(marked.windows(11).any(|w| w == b"secret-note"));
        let prepared = prepare_file(&marked).unwrap();
        assert!(!prepared.bytes.windows(11).any(|w| w == b"secret-note"));
        assert_eq!((prepared.width, prepared.height, prepared.mime), (64, 48, "image/png"));
        assert!(is_png_or_jpeg(&prepared.thumbnail));
    }

    #[test]
    fn large_images_are_scaled_down() {
        let prepared = prepare_file(&png(4000.0, 1000.0)).unwrap();
        assert_eq!((prepared.width, prepared.height), (2560, 640));
        assert!(prepared.bytes.len() <= MAX_BYTES);
    }

    #[test]
    fn rejects_non_images_and_oversized_files() {
        assert_eq!(prepare_file(b"not an image").err(), Some(ImageError::Unsupported));
        let huge = vec![0u8; MAX_INPUT_BYTES + 1];
        assert_eq!(prepare_file(&huge).err(), Some(ImageError::TooLarge));
    }
}
