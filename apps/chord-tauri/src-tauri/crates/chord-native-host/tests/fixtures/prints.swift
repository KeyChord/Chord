import Foundation

func run(_ handlerArguments: [String], _ eventArguments: [String]) throws {
    print("hello from swift stdout")
    FileHandle.standardError.write("hello from swift stderr\n".data(using: .utf8)!)
}
