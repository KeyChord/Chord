import AppKit

struct NoFrontmostApp: Error {}

func run(_ handlerArguments: [String], _ eventArguments: [String]) throws {
    // Touch AppKit from the host main thread without side effects.
    _ = NSWorkspace.shared.runningApplications.count
    _ = NSScreen.main
}
