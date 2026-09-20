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

pub fn restore(snapshot: &Snapshot) {
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

/// Puts `text` on the pasteboard, flagged so clipboard managers do not record it.
pub fn write_transient(text: &str) {
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
