// Draws the installer window's background: an arrow from where Chord sits to where the
// Applications folder sits, and what to do in words. Usage: <output.png> <scale> <font file>
import AppKit
import CoreText

let arguments = CommandLine.arguments
let (output, scale, fontFile) = (arguments[1], CGFloat(Double(arguments[2])!), arguments[3])
let size = CGSize(width: 660, height: 400)

guard
  let fontData = FileManager.default.contents(atPath: fontFile),
  let descriptors = CTFontManagerCreateFontDescriptorsFromData(fontData as CFData) as? [CTFontDescriptor],
  let fontDescriptor = descriptors.first
else { fatalError("cannot load \(fontFile)") }
let weight = kCTFontVariationAttribute as String
let variation = [2_003_265_652: 560] as CFDictionary  // 'wght'
let weighted = CTFontDescriptorCreateCopyWithAttributes(
  fontDescriptor, [weight: variation] as CFDictionary)
let font = CTFontCreateWithFontDescriptor(weighted, 18, nil) as NSFont

let image = NSBitmapImageRep(
  bitmapDataPlanes: nil,
  pixelsWide: Int(size.width * scale),
  pixelsHigh: Int(size.height * scale),
  bitsPerSample: 8,
  samplesPerPixel: 4,
  hasAlpha: true,
  isPlanar: false,
  colorSpaceName: .deviceRGB,
  bytesPerRow: 0,
  bitsPerPixel: 0
)!
image.size = size
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: image)

NSColor(calibratedWhite: 0.965, alpha: 1).setFill()
CGRect(origin: .zero, size: size).fill()

// y grows upward here; Finder's icon positions are measured from the top.
let iconRow = size.height - 170
NSColor(calibratedWhite: 0.55, alpha: 1).setStroke()
let shaft = NSBezierPath()
shaft.lineWidth = 5
shaft.lineCapStyle = .round
shaft.move(to: CGPoint(x: 258, y: iconRow))
shaft.line(to: CGPoint(x: 398, y: iconRow))
shaft.stroke()
let head = NSBezierPath()
head.lineWidth = 5
head.lineCapStyle = .round
head.lineJoinStyle = .round
head.move(to: CGPoint(x: 380, y: iconRow + 18))
head.line(to: CGPoint(x: 400, y: iconRow))
head.line(to: CGPoint(x: 380, y: iconRow - 18))
head.stroke()

let caption = NSAttributedString(
  string: "Drag Chord to Applications",
  attributes: [
    .font: font,
    .foregroundColor: NSColor(calibratedWhite: 0.2, alpha: 1),
  ]
)
let captionSize = caption.size()
caption.draw(at: CGPoint(x: (size.width - captionSize.width) / 2, y: 62))

NSGraphicsContext.restoreGraphicsState()
try image.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: output))
