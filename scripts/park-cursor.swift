// Moves the pointer to the middle of the main display's right edge — no click — so a
// window opening for a screenshot does not come up with a row or button in its hover state.
import CoreGraphics

let display = CGDisplayBounds(CGMainDisplayID())
CGWarpMouseCursorPosition(CGPoint(x: display.maxX - 2, y: display.midY))
CGAssociateMouseAndMouseCursorPosition(1)
