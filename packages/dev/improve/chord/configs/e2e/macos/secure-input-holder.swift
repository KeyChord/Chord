// Holds Secure Input from the background and never lets go, like an app that forgot
// to turn it off after a password prompt. The e2e tests run it inside the VM
// (secure-input.e2e.ts); it is compiled on the host.
import AppKit
import Carbon

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
guard EnableSecureEventInput() == noErr else {
  FileHandle.standardError.write(Data("EnableSecureEventInput failed\n".utf8))
  exit(1)
}
app.run()
