// Prints the text Vision finds in a PNG as JSON lines: {"text","x","y","width","height"} in
// image pixels with the origin at the top left. Compiled and run by `screen.ts` on the host.
import Foundation
import Vision

guard CommandLine.arguments.count == 2 else {
  FileHandle.standardError.write(Data("usage: ocr <image.png>\n".utf8))
  exit(2)
}
let url = URL(fileURLWithPath: CommandLine.arguments[1])
guard
  let source = CGImageSourceCreateWithURL(url as CFURL, nil),
  let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
else {
  FileHandle.standardError.write(Data("cannot read \(url.path)\n".utf8))
  exit(1)
}

let width = CGFloat(image.width)
let height = CGFloat(image.height)
let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.usesLanguageCorrection = false
try VNImageRequestHandler(cgImage: image).perform([request])

for observation in request.results ?? [] {
  guard let candidate = observation.topCandidates(1).first else { continue }
  // Vision's normalized boxes start at the bottom left.
  let box = observation.boundingBox
  let line: [String: Any] = [
    "text": candidate.string,
    "x": Int((box.minX * width).rounded()),
    "y": Int(((1 - box.maxY) * height).rounded()),
    "width": Int((box.width * width).rounded()),
    "height": Int((box.height * height).rounded()),
  ]
  let json = try JSONSerialization.data(withJSONObject: line, options: [.sortedKeys])
  FileHandle.standardOutput.write(json)
  FileHandle.standardOutput.write(Data("\n".utf8))
}
