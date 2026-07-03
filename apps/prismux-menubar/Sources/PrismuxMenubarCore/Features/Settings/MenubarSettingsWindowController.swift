import AppKit
import SwiftUI

@MainActor
final class MenubarSettingsWindowController {
    private let store = MenubarSettingsStore()
    private var window: NSWindow?

    func show(tab: MenubarSettingsTab = .general) {
        store.selectedTab = tab
        if let window {
            window.makeKeyAndOrderFront(nil)
            NSApplication.shared.activate()
            return
        }

        let controller = NSHostingController(rootView: MenubarSettingsView(store: store))
        let window = NSWindow(contentViewController: controller)
        window.title = "Prismux Settings"
        window.styleMask = [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView]
        window.setContentSize(NSSize(width: 720, height: 500))
        window.contentMinSize = NSSize(width: 680, height: 460)
        window.isReleasedWhenClosed = false
        window.center()
        self.window = window
        window.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate()
    }
}
