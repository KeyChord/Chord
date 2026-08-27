// ChordEntry.swift — Chord native handler ABI wrapper, version 1.
//
// This file is generated into a package's build directory by Chord's native build tooling and
// compiled together with the package's own Swift sources. It bridges the user's
//
//     func run(_ handlerArguments: [String], _ eventArguments: [String]) throws
//
// to the C symbol `chord_native_run_v1` that `chord-native-host` resolves. Do not edit copies of
// this file inside packages; change the tooling instead.
//
// Return codes: 0 success · 1 `run` threw · 2 invalid argument vectors · 3 wrapper failure.

import Foundation

private enum ChordEntryError: Error, CustomStringConvertible {
    case negativeArgumentCount(Int32)
    case missingArgumentVector
    case missingArgument(Int)

    var description: String {
        switch self {
        case .negativeArgumentCount(let count):
            return "negative argument count \(count)"
        case .missingArgumentVector:
            return "argument vector is null but count is non-zero"
        case .missingArgument(let index):
            return "argument \(index) is null"
        }
    }
}

private func chordDecodeArguments(
    count: Int32,
    values: UnsafePointer<UnsafePointer<CChar>?>?
) throws -> [String] {
    guard count >= 0 else {
        throw ChordEntryError.negativeArgumentCount(count)
    }
    if count == 0 {
        return []
    }
    guard let values else {
        throw ChordEntryError.missingArgumentVector
    }

    var result: [String] = []
    result.reserveCapacity(Int(count))
    for index in 0..<Int(count) {
        guard let value = values[index] else {
            throw ChordEntryError.missingArgument(index)
        }
        result.append(String(cString: value))
    }
    return result
}

private func chordWriteError(
    _ message: String,
    into buffer: UnsafeMutablePointer<UInt8>?,
    capacity: Int
) {
    guard let buffer, capacity > 0 else {
        return
    }

    var bytes = Array(message.utf8)
    if bytes.count >= capacity {
        let marker = Array(" … [truncated]".utf8)
        let keep = max(0, capacity - 1 - marker.count)
        bytes = Array(bytes.prefix(keep)) + marker
        if bytes.count >= capacity {
            bytes = Array(bytes.prefix(capacity - 1))
        }
    }

    for (offset, byte) in bytes.enumerated() {
        buffer[offset] = byte
    }
    buffer[bytes.count] = 0
}

private func chordDescribe(_ error: Error) -> String {
    let description: String
    if let localized = error as? LocalizedError, let text = localized.errorDescription {
        description = text
    } else {
        description = String(describing: error)
    }
    return "\(type(of: error)): \(description)"
}

@_cdecl("chord_native_run_v1")
public func chordNativeRunV1(
    _ handlerArgumentCount: Int32,
    _ handlerArgumentValues: UnsafePointer<UnsafePointer<CChar>?>?,
    _ eventArgumentCount: Int32,
    _ eventArgumentValues: UnsafePointer<UnsafePointer<CChar>?>?,
    _ errorBuffer: UnsafeMutablePointer<UInt8>?,
    _ errorBufferCapacity: Int
) -> Int32 {
    autoreleasepool {
        let handlerArguments: [String]
        let eventArguments: [String]
        do {
            handlerArguments = try chordDecodeArguments(
                count: handlerArgumentCount,
                values: handlerArgumentValues
            )
            eventArguments = try chordDecodeArguments(
                count: eventArgumentCount,
                values: eventArgumentValues
            )
        } catch {
            chordWriteError(chordDescribe(error), into: errorBuffer, capacity: errorBufferCapacity)
            return 2
        }

        do {
            try run(handlerArguments, eventArguments)
            return 0
        } catch {
            chordWriteError(chordDescribe(error), into: errorBuffer, capacity: errorBufferCapacity)
            return 1
        }
    }
}
