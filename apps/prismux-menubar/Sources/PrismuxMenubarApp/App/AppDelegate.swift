import AppKit
import PrismuxMenubarCore

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var shell: StatusItemController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        installEditMenu()
        let store = AppStore(backend: RustBackendClient())
        shell = StatusItemController(store: store)
        Task { await store.load() }
    }

    func applicationWillTerminate(_ notification: Notification) {
        shell = nil
    }

    /// An accessory app has no main menu, so Cmd+C/V/X/A/Z have no key
    /// equivalent to route through — paste silently fails in our text fields.
    /// A standard Edit menu fixes it; the selectors dispatch down the responder
    /// chain to whatever text view is first responder.
    private func installEditMenu() {
        let mainMenu = NSMenu()

        let editItem = NSMenuItem()
        mainMenu.addItem(editItem)
        let editMenu = NSMenu(title: "Edit")
        editItem.submenu = editMenu

        editMenu.addItem(withTitle: "Undo", action: Selector(("undo:")), keyEquivalent: "z")
        let redo = editMenu.addItem(withTitle: "Redo", action: Selector(("redo:")), keyEquivalent: "z")
        redo.keyEquivalentModifierMask = [.command, .shift]
        editMenu.addItem(.separator())
        editMenu.addItem(withTitle: "Cut", action: #selector(NSText.cut(_:)), keyEquivalent: "x")
        editMenu.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        editMenu.addItem(withTitle: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        editMenu.addItem(withTitle: "Select All", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")

        NSApp.mainMenu = mainMenu
    }
}
