import Foundation
import NodeAPI

enum ProbeError: Error { case expectedFailure }

// Explicit registration also allows compiling this fixture against an existing
// NodeSwift build without rebuilding the macro compiler.
@_cdecl("node_swift_register")
public func register(env: OpaquePointer) -> OpaquePointer? {
    NodeModuleRegistrar(env).register {
        ["probe": try NodeFunction { (fail: Bool) async throws -> String in
            for _ in 0..<3 {
                try await Task.sleep(nanoseconds: 10_000_000)
                try await MainActor.run {
                    precondition(Thread.isMainThread)
                    if fail { throw ProbeError.expectedFailure }
                }
            }
            return "main-thread-ok"
        }]
    }
}
