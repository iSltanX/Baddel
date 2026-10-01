//! The general pasteboard: full snapshot/restore around our temporary use of it.

use std::thread::sleep;
use std::time::{Duration, Instant};

use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};

/// Markers from nspasteboard.org: clipboard managers skip entries carrying them.
const TRANSIENT_TYPES: [&str; 2] = ["org.nspasteboard.TransientType", "org.nspasteboard.ConcealedType"];

/// Every item on the pasteboard with every representation it offers.
pub struct Snapshot(Vec<Vec<(String, Vec<u8>)>>);

pub fn change_count() -> isize {
    NSPasteboard::generalPasteboard().changeCount()
}

pub fn snapshot() -> Snapshot {
    autoreleasepool(|_| {
        let items = NSPasteboard::generalPasteboard().pasteboardItems();
        Snapshot(
            items
                .iter()
                .flat_map(|items| items.iter())
                .map(|item| {
                    item.types()
                        .iter()
                        .filter_map(|ty| Some((ty.to_string(), item.dataForType(&ty)?.to_vec())))
                        .collect()
                })
                .collect(),
        )
    })
}

/// Puts `snapshot` back, unless the pasteboard has moved on since `expected` — the change count
/// right after our own last use of it. Anything newer was copied by the user (or another app)
/// while the conversion ran, and the newest copy wins: writing the snapshot over it would wipe
/// it out for good. Returns whether the snapshot went back.
pub fn restore_if_unchanged(snapshot: &Snapshot, expected: isize) -> bool {
    if change_count() != expected {
        return false;
    }
    restore(snapshot);
    true
}

fn restore(snapshot: &Snapshot) {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        let items: Vec<_> = snapshot
            .0
            .iter()
            .map(|representations| {
                let item = NSPasteboardItem::new();
                for (ty, bytes) in representations {
                    item.setData_forType(&NSData::with_bytes(bytes), &NSString::from_str(ty));
                }
                ProtocolObject::<dyn NSPasteboardWriting>::from_retained(item)
            })
            .collect();
        if !items.is_empty() {
            pasteboard.writeObjects(&NSArray::from_retained_slice(&items));
        }
    })
}

pub fn read_string() -> Option<String> {
    autoreleasepool(|_| {
        // SAFETY: AppKit constant, valid for the life of the process.
        let ty = unsafe { NSPasteboardTypeString };
        NSPasteboard::generalPasteboard().stringForType(ty).map(|s| s.to_string())
    })
}

/// Puts `text` on the pasteboard as an ordinary copy, for when the user asked to copy it
/// (diagnostics, a report). Unlike [`write_transient`], clipboard managers may keep it.
pub fn write_text(text: &str) {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        let item = NSPasteboardItem::new();
        // SAFETY: AppKit constant, valid for the life of the process.
        item.setData_forType(&NSData::with_bytes(text.as_bytes()), unsafe { NSPasteboardTypeString });
        let item = ProtocolObject::<dyn NSPasteboardWriting>::from_retained(item);
        pasteboard.writeObjects(&NSArray::from_retained_slice(&[item]));
    })
}

/// Puts `text` on the pasteboard, flagged so clipboard managers do not record it. Returns the
/// change count it leaves, so the caller can tell later whether anyone wrote after it.
pub fn write_transient(text: &str) -> isize {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        let item = NSPasteboardItem::new();
        // SAFETY: as above.
        item.setData_forType(&NSData::with_bytes(text.as_bytes()), unsafe { NSPasteboardTypeString });
        for marker in TRANSIENT_TYPES {
            item.setData_forType(&NSData::with_bytes(&[]), &NSString::from_str(marker));
        }
        let item = ProtocolObject::<dyn NSPasteboardWriting>::from_retained(item);
        pasteboard.writeObjects(&NSArray::from_retained_slice(&[item]));
        pasteboard.changeCount()
    })
}

/// Waits for the pasteboard to change from `since`. `false` on timeout.
pub fn wait_for_change(since: isize, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while change_count() == since {
        if Instant::now() >= deadline {
            return false;
        }
        sleep(Duration::from_millis(5));
    }
    true
}
