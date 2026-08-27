import Foundation

func run(_ handlerArguments: [String], _ eventArguments: [String]) throws {
    let path = handlerArguments[0]
    let handle = FileHandle(forWritingAtPath: path) ?? {
        FileManager.default.createFile(atPath: path, contents: nil)
        return FileHandle(forWritingAtPath: path)!
    }()
    handle.seekToEndOfFile()
    handle.write("tick\n".data(using: .utf8)!)
    handle.closeFile()
}
