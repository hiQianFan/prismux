import AppKit
import OmxMenubarCore

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var shell: StatusItemController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        let store = AppStore(backend: RustBackendClient())
        shell = StatusItemController(store: store)
        Task { await store.load() }
    }

    func applicationWillTerminate(_ notification: Notification) {
        shell = nil
    }
}
