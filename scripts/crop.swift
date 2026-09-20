// Crops a screen capture to a rectangle given in points, with a margin.
//   swift scripts/crop.swift <in.png> <out.png> <x> <y> <w> <h> [margin]
// Used for the notice panel: `screencapture -l` cannot photograph a floating
// non-activating panel, and the panel is translucent, so it is cut out of a full
// screen capture together with a little of what it is blurring.
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let arguments = CommandLine.arguments
guard arguments.count >= 7,
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: arguments[1]) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
else {
    FileHandle.standardError.write(Data("usage: crop.swift in out x y w h [margin]\n".utf8))
    exit(1)
}

let values = arguments[3...6].compactMap(Double.init)
guard values.count == 4 else { exit(1) }
let margin = arguments.count > 7 ? (Double(arguments[7]) ?? 40) : 40

// The capture is in pixels, the window bounds in points.
let scale = Double(image.width) / Double(CGDisplayPixelsWide(CGMainDisplayID()))
let rect = CGRect(
    x: (values[0] - margin) * scale,
    y: (values[1] - margin) * scale,
    width: (values[2] + margin * 2) * scale,
    height: (values[3] + margin * 2) * scale
).intersection(CGRect(x: 0, y: 0, width: Double(image.width), height: Double(image.height)))

guard let cropped = image.cropping(to: rect),
      let destination = CGImageDestinationCreateWithURL(
          URL(fileURLWithPath: arguments[2]) as CFURL, UTType.png.identifier as CFString, 1, nil)
else { exit(1) }

CGImageDestinationAddImage(destination, cropped, nil)
exit(CGImageDestinationFinalize(destination) ? 0 : 1)
