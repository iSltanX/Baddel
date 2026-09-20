//! The optional click after a conversion. **Main thread only** — call through
//! [`super::on_main`].

use objc2_app_kit::NSSound;
use objc2_foundation::NSString;

/// A short, quiet system sound. Nothing is bundled: this is the one macOS already
/// uses for small confirmations.
const NAME: &str = "Tink";

pub fn click() {
    if let Some(sound) = NSSound::soundNamed(&NSString::from_str(NAME)) {
        sound.play();
    }
}
