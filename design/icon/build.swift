// Builds the bundle icons in src-tauri/icons from the D2 app icon masters in this folder.
//   swift design/icon/build.swift
//
// The masters are the squircle alone, exported from Figma with a transparent background:
//   app-icon-824.png       `App Icon / 1024 · Mint` at 824 px — every size from 64 up
//   app-icon-small-26.png  `App Icon / Small ≤32 · Mint` at 26 px — the 32 px icons
//   app-icon-small-14.png  the same component at 14 px — the 16 px icon
// Each is placed on the macOS icon grid (the body is 824/1024 of the canvas, centred) and
// written as the iconset, `icon.icns` (through iconutil), and the PNGs and ICO Tauri lists.
// Flat: no shadow is added — the A2 identity draws none.
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let here = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
let root = here.deletingLastPathComponent().deletingLastPathComponent()
let output = root.appendingPathComponent("src-tauri/icons")

func load(_ name: String) -> CGImage {
    let url = here.appendingPathComponent(name)
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
    else {
        FileHandle.standardError.write(Data("missing master: \(name)\n".utf8))
        exit(1)
    }
    return image
}

let master = load("app-icon-824.png")
let small26 = load("app-icon-small-26.png")
let small14 = load("app-icon-small-14.png")

/// The body of a `canvas`-pixel icon on the macOS grid, and the master that draws it.
/// The two small sizes use the small-size master, drawn at its own pixel size.
func body(for canvas: Int) -> (size: Int, image: CGImage) {
    switch canvas {
    case 16: return (14, small14)
    case 32: return (26, small26)
    default: return (Int((Double(canvas) * 824.0 / 1024.0).rounded()), master)
    }
}

func icon(_ canvas: Int) -> CGImage {
    let (size, image) = body(for: canvas)
    guard let context = CGContext(
        data: nil, width: canvas, height: canvas, bitsPerComponent: 8, bytesPerRow: 0,
        space: CGColorSpace(name: CGColorSpace.sRGB)!,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { exit(1) }
    context.interpolationQuality = .high
    let inset = CGFloat(canvas - size) / 2
    context.draw(image, in: CGRect(x: inset, y: inset, width: CGFloat(size), height: CGFloat(size)))
    return context.makeImage()!
}

func write(_ images: [CGImage], to url: URL, type: UTType) {
    guard let destination = CGImageDestinationCreateWithURL(url as CFURL, type.identifier as CFString, images.count, nil)
    else { exit(1) }
    for image in images { CGImageDestinationAddImage(destination, image, nil) }
    guard CGImageDestinationFinalize(destination) else { exit(1) }
    print("✓ \(url.path.replacingOccurrences(of: root.path + "/", with: ""))")
}

// The iconset: every point size at 1× and 2×.
let iconset = FileManager.default.temporaryDirectory.appendingPathComponent("baddel.iconset")
try? FileManager.default.removeItem(at: iconset)
try FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)
for points in [16, 32, 128, 256, 512] {
    write([icon(points)], to: iconset.appendingPathComponent("icon_\(points)x\(points).png"), type: .png)
    write([icon(points * 2)], to: iconset.appendingPathComponent("icon_\(points)x\(points)@2x.png"), type: .png)
}

let iconutil = Process()
iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
iconutil.arguments = ["-c", "icns", iconset.path, "-o", output.appendingPathComponent("icon.icns").path]
try iconutil.run()
iconutil.waitUntilExit()
guard iconutil.terminationStatus == 0 else { exit(1) }
print("✓ src-tauri/icons/icon.icns")

// The files tauri.conf.json lists, plus the 512 px PNG the bundler falls back on.
write([icon(32)], to: output.appendingPathComponent("32x32.png"), type: .png)
write([icon(128)], to: output.appendingPathComponent("128x128.png"), type: .png)
write([icon(256)], to: output.appendingPathComponent("128x128@2x.png"), type: .png)
write([icon(512)], to: output.appendingPathComponent("icon.png"), type: .png)
write([16, 32, 64, 128, 256].map(icon), to: output.appendingPathComponent("icon.ico"), type: .ico)
write([icon(1024)], to: here.appendingPathComponent("icon-macos-1024.png"), type: .png)
