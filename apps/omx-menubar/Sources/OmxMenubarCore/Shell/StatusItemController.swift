import AppKit
import Combine
import SwiftUI

@MainActor
public final class StatusItemController: NSObject, NSPopoverDelegate {
    private let statusItem: NSStatusItem
    private let popover: NSPopover
    private let store: AppStore
    private let settingsWindowController = MenubarSettingsWindowController()
    private var cancellables: Set<AnyCancellable> = []
    private var refreshTimer: Timer?
    private var globalMouseMonitor: Any?
    private var localMouseMonitor: Any?

    private let trayModeKey = "dev.openmux.menubar.trayDisplayMode"
    private let refreshCadenceKey = "dev.openmux.menubar.backgroundRefreshCadence"

    public init(store: AppStore) {
        self.store = store
        self.statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        self.popover = NSPopover()
        super.init()

        popover.behavior = .transient
        popover.delegate = self
        popover.contentSize = NSSize(width: 390, height: 560)
        popover.contentViewController = NSHostingController(
            rootView: DashboardScreen(store: store) { [weak self] tab in
                self?.settingsWindowController.show(tab: tab)
            }
        )

        statusItem.button?.image = NSImage(systemSymbolName: "arrow.triangle.2.circlepath", accessibilityDescription: "OpenMux")
        statusItem.button?.target = self
        statusItem.button?.action = #selector(togglePopover)
        statusItem.button?.toolTip = "OpenMux account switcher"
        updateTrayTitle()

        store.$state
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in self?.updateTrayTitle() }
            .store(in: &cancellables)
        NotificationCenter.default
            .publisher(for: UserDefaults.didChangeNotification)
            .sink { [weak self] _ in
                self?.updateTrayTitle()
                self?.scheduleBackgroundRefresh()
            }
            .store(in: &cancellables)
        scheduleBackgroundRefresh()
    }

    @objc private func togglePopover() {
        guard let button = statusItem.button else { return }
        if popover.isShown {
            closePopover()
        } else {
            NSApplication.shared.activate()
            popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
            installMouseMonitors()
            Task { await store.refresh(kind: "interactive") }
        }
    }

    public func popoverDidClose(_ notification: Notification) {
        removeMouseMonitors()
    }

    private func closePopover() {
        popover.performClose(nil)
        removeMouseMonitors()
    }

    private func installMouseMonitors() {
        removeMouseMonitors()
        let mask: NSEvent.EventTypeMask = [.leftMouseDown, .rightMouseDown]
        globalMouseMonitor = NSEvent.addGlobalMonitorForEvents(matching: mask) { [weak self] _ in
            Task { @MainActor in self?.closePopover() }
        }
        localMouseMonitor = NSEvent.addLocalMonitorForEvents(matching: mask) { [weak self] event in
            guard let self else { return event }
            if self.shouldKeepPopoverOpen(for: event) {
                return event
            }
            self.closePopover()
            return event
        }
    }

    private func removeMouseMonitors() {
        if let globalMouseMonitor {
            NSEvent.removeMonitor(globalMouseMonitor)
            self.globalMouseMonitor = nil
        }
        if let localMouseMonitor {
            NSEvent.removeMonitor(localMouseMonitor)
            self.localMouseMonitor = nil
        }
    }

    private func shouldKeepPopoverOpen(for event: NSEvent) -> Bool {
        if let buttonWindow = statusItem.button?.window, event.window === buttonWindow {
            return true
        }
        if let popoverWindow = popover.contentViewController?.view.window, event.window === popoverWindow {
            return true
        }
        return false
    }

    private func updateTrayTitle() {
        let iconOnly = UserDefaults.standard.string(forKey: trayModeKey) == "icon_only"
        statusItem.button?.title = iconOnly ? "" : store.trayTitle
    }

    private func scheduleBackgroundRefresh() {
        refreshTimer?.invalidate()
        let cadence = UserDefaults.standard.integer(forKey: refreshCadenceKey)
        let interval = TimeInterval(cadence > 0 ? cadence : 300)
        refreshTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            guard let self else { return }
            Task { await self.store.refresh(kind: "background") }
        }
        refreshTimer?.tolerance = interval * 0.1
    }

    deinit {
        refreshTimer?.invalidate()
    }
}
