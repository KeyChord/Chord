import Foundation

struct EchoMismatch: Error, CustomStringConvertible {
    let description: String
}

func run(_ handlerArguments: [String], _ eventArguments: [String]) throws {
    let env = ProcessInfo.processInfo.environment
    var problems: [String] = []
    if handlerArguments != ["Safari", "1"] { problems.append("handler=\(handlerArguments)") }
    if eventArguments != ["by-letters", "x"] { problems.append("event=\(eventArguments)") }
    if env["CHORD_PACKAGE_NAME"] != "test-pkg" { problems.append("CHORD_PACKAGE_NAME=\(env["CHORD_PACKAGE_NAME"] ?? "nil")") }
    if env["CHORD_CHORDS_FILE_PATHSLUG"] != "chords/macos.toml" { problems.append("CHORD_CHORDS_FILE_PATHSLUG=\(env["CHORD_CHORDS_FILE_PATHSLUG"] ?? "nil")") }
    if env["CHORD_HANDLER_ID"] != "echo" { problems.append("CHORD_HANDLER_ID=\(env["CHORD_HANDLER_ID"] ?? "nil")") }
    if env["CHORD_FOCUSED_APP_ID"] != "com.apple.Safari" { problems.append("CHORD_FOCUSED_APP_ID=\(env["CHORD_FOCUSED_APP_ID"] ?? "nil")") }
    if env["CHORD_INVOCATION_ID"] == nil { problems.append("CHORD_INVOCATION_ID missing") }
    if !problems.isEmpty {
        throw EchoMismatch(description: problems.joined(separator: "; "))
    }
}
