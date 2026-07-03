import AppKit
import SwiftUI

extension MenubarSettingsTab {
    // Sidebar icon-square tint, in the spirit of System Settings' color-coded
    // rows. Kept in the view layer so the store stays free of SwiftUI.
    var tint: Color {
        switch self {
        case .general: .gray
        case .providers: .blue
        case .about: .indigo
        }
    }
}

struct MenubarSettingsView: View {
    @ObservedObject var store: MenubarSettingsStore

    var body: some View {
        // System Settings idiom: a sidebar source list on the left, grouped
        // content on the right. All standard controls, so the whole thing
        // adopts the system look automatically (Liquid Glass on macOS 26+).
        NavigationSplitView {
            List(selection: sidebarSelection) {
                ForEach(MenubarSettingsTab.allCases) { tab in
                    Label {
                        Text(tab.title)
                    } icon: {
                        SidebarIcon(systemImage: tab.icon, tint: tab.tint)
                    }
                    .tag(tab)
                }
            }
            .listStyle(.sidebar)
            .navigationSplitViewColumnWidth(min: 176, ideal: 194, max: 220)
        } detail: {
            detail
                .navigationTitle(store.selectedTab.title)
                .overlay(alignment: .center) {
                    if store.loading, store.settings == nil {
                        ProgressView("Loading settings")
                            .controlSize(.small)
                            .padding(16)
                            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
                            .allowsHitTesting(false)
                    }
                }
                .overlay(alignment: .bottomTrailing) {
                    if store.saving {
                        ProgressView()
                            .controlSize(.small)
                            .padding(12)
                            .allowsHitTesting(false)
                    }
                }
        }
        .navigationSplitViewStyle(.balanced)
        .frame(minWidth: 680, idealWidth: 720, minHeight: 460, idealHeight: 500)
        .task { await store.load() }
    }

    private var sidebarSelection: Binding<MenubarSettingsTab?> {
        Binding(
            get: { store.selectedTab },
            set: { store.selectedTab = $0 ?? store.selectedTab }
        )
    }

    @ViewBuilder
    private var detail: some View {
        switch store.selectedTab {
        case .general: GeneralPane(store: store)
        case .providers: ProvidersPane(store: store)
        case .about: AboutPane(store: store)
        }
    }
}

// A System Settings-style sidebar glyph: a white SF Symbol on a small tinted
// rounded square.
private struct SidebarIcon: View {
    let systemImage: String
    let tint: Color

    var body: some View {
        Image(systemName: systemImage)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(.white)
            .frame(width: 20, height: 20)
            .background(tint.gradient, in: RoundedRectangle(cornerRadius: 5))
    }
}

// MARK: - General

private struct GeneralPane: View {
    @ObservedObject var store: MenubarSettingsStore

    var body: some View {
        Form {
            if let errorMessage = store.errorMessage {
                Section {
                    SettingsBanner(text: errorMessage, systemImage: "exclamationmark.triangle.fill", color: .orange)
                }
            }

            Section("Startup") {
                Toggle("Open Prismux at login", isOn: Binding(
                    get: { store.launchAtLogin },
                    set: { store.setLaunchAtLogin($0) }
                ))
            }

            if let settings = store.settings {
                Section {
                    Toggle("Hide personal account identifiers", isOn: privacyBinding(settings))
                } header: {
                    Text("Privacy")
                } footer: {
                    Text("Account and profile rows use coarse labels such as “#1 Account” instead of emails or profile names.")
                }

                Section("Network") {
                    ProxyRow(store: store, settings: settings)
                }
            }

            CommandLineSection(store: store)

            Section("Storage") {
                if let about = store.about {
                    PathRow(title: "State root", path: about.stateRoot) { store.reveal(about.stateRoot) }
                    PathRow(title: "Settings", path: about.settingsPath) { store.reveal(about.settingsPath) }
                }
            }

            if let status = store.supportStatus {
                Section { Text(status).font(.caption).foregroundStyle(.secondary) }
            }
        }
        .formStyle(.grouped)
    }

    private func privacyBinding(_ settings: SettingsView) -> Binding<Bool> {
        Binding(
            get: { store.settings?.privacy.hidePersonalIdentifiers ?? settings.privacy.hidePersonalIdentifiers },
            set: { store.updatePrivacy($0) }
        )
    }

}

// Command-line tool group, extracted so the General form stays readable.
// Explanatory copy lives in the section footer, matching System Settings.
private struct CommandLineSection: View {
    @ObservedObject var store: MenubarSettingsStore

    var body: some View {
        Section {
            LabeledContent("prismux command") {
                StatusChip(text: store.cliStatus.statusText, tone: store.cliStatus.statusTone)
            }
            if let bundled = store.cliStatus.bundledPath {
                MonoRow(title: "Bundled helper", value: bundled, truncate: true)
            }
            if let helperVersion = store.cliStatus.helperVersion {
                MonoRow(title: "Helper version", value: helperVersion)
            }
            if let found = store.cliStatus.foundPath {
                MonoRow(title: "Terminal prismux", value: found, truncate: true)
            }
            if let foundVersion = store.cliStatus.foundVersion {
                MonoRow(title: "Terminal version", value: foundVersion)
            }
            HStack {
                Button("Enable prismux command") { store.enableCliCommand() }
                    .disabled(!store.cliStatus.helperAvailable || store.cliStatus.resolution == .ready)
                Button("Copy command") { store.copyCliCommand() }
                if store.cliStatus.pathCommand != nil {
                    Button("Copy PATH command") { store.copyPathCommand() }
                }
            }
        } header: {
            Text("Command-Line Tool")
        } footer: {
            if store.cliStatus.resolution == .differentFound, let found = store.cliStatus.foundPath {
                Text("A different prismux is already on your PATH at \(found). Prismux won’t overwrite it; adjust it manually if you want the bundled helper.")
            } else {
                Text("The prismux CLI ships inside Prismux. This only links it onto your PATH — it does not download a separate tool.")
            }
        }
    }
}

// One row for the whole proxy setting: "Proxy ⓘ" on the left, then a single
// address input (host and port together, e.g. "http://127.0.0.1:7890") sitting
// directly left of the on/off toggle. The input is a plain placeholder field —
// no separate label — and greys out, non-editable, while the toggle is off.
// Local editing state keeps keystrokes from round-tripping to the backend one
// character at a time; commits on submit or when focus leaves.
private struct ProxyRow: View {
    @ObservedObject var store: MenubarSettingsStore
    let settings: SettingsView

    @State private var draft: String = ""
    @FocusState private var focused: Bool

    private var enabled: Bool {
        store.settings?.network.proxyEnabled ?? settings.network.proxyEnabled
    }

    private var storedURL: String {
        store.settings?.network.proxyURL ?? settings.network.proxyURL ?? ""
    }

    var body: some View {
        HStack(spacing: 8) {
            Text("Proxy")
            InfoPopoverButton(text: "When on, Prismux routes account usage refresh requests through this server. Enter the full address including port, e.g. http://host:port. Supports http, https, and socks5. Press Return or click away to save.")

            Spacer(minLength: 12)

            TextField(text: $draft, prompt: Text("http://host:port")) { EmptyView() }
                .labelsHidden()
                .textFieldStyle(.roundedBorder)
                .autocorrectionDisabled()
                .multilineTextAlignment(.leading)
                .frame(width: 210)
                .disabled(!enabled)
                .focused($focused)
                .onSubmit { commit() }
                .onChange(of: focused) { _, isFocused in
                    // Losing focus (Tab, or clicking another control) saves.
                    if !isFocused { commit() }
                }

            Toggle("Proxy", isOn: Binding(
                get: { enabled },
                set: { store.setProxyEnabled($0) }
            ))
            .labelsHidden()
        }
        .onAppear { draft = storedURL }
        .onChange(of: storedURL) { _, newValue in
            // Adopt server-side changes only while the user isn't editing.
            if !focused { draft = newValue }
        }
        // Safety net: don't lose an in-progress edit when the pane goes away
        // (tab switch or window close) before focus resigns.
        .onDisappear { commit() }
    }

    private func commit() {
        let trimmed = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed != storedURL else { return }
        store.updateProxyURL(trimmed)
    }
}

// A small ⓘ affordance that reveals explanatory copy in a popover on tap, and
// as a hover tooltip. Replaces inline footer text for optional detail.
private struct InfoPopoverButton: View {
    let text: String
    @State private var showing = false

    var body: some View {
        Button {
            showing.toggle()
        } label: {
            Image(systemName: "info.circle")
                .foregroundStyle(.secondary)
                .imageScale(.small)
        }
        .buttonStyle(.plain)
        .contentShape(Circle())
        .help(text)
        .popover(isPresented: $showing, arrowEdge: .bottom) {
            Text(text)
                .font(.callout)
                .fixedSize(horizontal: false, vertical: true)
                .padding(14)
                .frame(width: 260)
        }
        .accessibilityLabel("More information")
    }
}

// MARK: - Providers

private struct ProvidersPane: View {
    @ObservedObject var store: MenubarSettingsStore

    var body: some View {
        Form {
            if let providers = store.settings?.providers {
                ForEach(providers) { provider in
                    Section(provider.displayLabel) {
                        Toggle(isOn: Binding(
                            get: { provider.enabled },
                            set: { store.updateProviderEnabled(provider, enabled: $0) }
                        )) {
                            HStack(spacing: 8) {
                                Circle()
                                    .fill(toneColor(provider.status.statusTone))
                                    .frame(width: 8, height: 8)
                                Text(provider.status.statusText)
                            }
                        }

                        ForEach(Array(provider.diagnostics.enumerated()), id: \.offset) { _, diagnostic in
                            SettingsBanner(text: diagnostic.message, systemImage: "exclamationmark.triangle.fill", color: .orange)
                        }
                    }
                }
            }

            Section {
                EmptyView()
            } footer: {
                Text("Sign in, import profiles, and switch the active account or gateway profile from the Prismux popover.")
            }
        }
        .formStyle(.grouped)
    }

    private func toneColor(_ tone: String) -> Color {
        switch tone {
        case "success": .green
        case "warning": .orange
        case "danger": .red
        default: .secondary
        }
    }
}

// MARK: - About

private struct AboutPane: View {
    @ObservedObject var store: MenubarSettingsStore

    var body: some View {
        Form {
            if let about = store.about {
                Section {
                    VStack(spacing: 8) {
                        Image(nsImage: NSApp.applicationIconImage)
                            .resizable()
                            .frame(width: 72, height: 72)
                        VStack(spacing: 2) {
                            Text("Prismux")
                                .font(.title2.weight(.semibold))
                            Text("Local account switcher for AI coding tools")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        VStack(spacing: 1) {
                            Text("Version \(about.appVersion)")
                            Text("CLI helper \(store.cliStatus.helperVersion ?? "unavailable")")
                            Text(about.runtime.statusText)
                        }
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 12)
                }

                Section("Project") {
                    ForEach(about.links) { link in
                        LinkRow(link: link) { store.openLink(link.url) }
                    }
                }

                if !about.authorLinks.isEmpty {
                    Section("Author") {
                        ForEach(about.authorLinks) { link in
                            LinkRow(link: link) { store.openLink(link.url) }
                        }
                    }
                }

                Section("Storage") {
                    PathRow(title: "State root", path: about.stateRoot) { store.reveal(about.stateRoot) }
                    PathRow(title: "Settings", path: about.settingsPath) { store.reveal(about.settingsPath) }
                }

                Section {
                    Button("Copy Version Info") { store.copyVersionInfo() }
                    Button("Copy Redacted Support Report") {
                        Task { await store.copySupportReport() }
                    }
                } header: {
                    Text("Support")
                } footer: {
                    VStack(alignment: .leading, spacing: 4) {
                        if let status = store.supportStatus {
                            Text(status)
                        }
                        Text("Released under the MIT License.")
                    }
                }
            } else {
                Section { Text("About information is unavailable.").foregroundStyle(.secondary) }
            }
        }
        .formStyle(.grouped)
    }
}

// MARK: - Shared rows

private struct LinkRow: View {
    let link: AboutLink
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack {
                Text(link.label)
                Spacer()
                Image(systemName: "arrow.up.right")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

private struct StatusChip: View {
    let text: String
    let tone: String

    var body: some View {
        HStack(spacing: 5) {
            Circle().fill(color).frame(width: 7, height: 7)
            Text(text).font(.caption)
        }
    }

    private var color: Color {
        switch tone {
        case "success": .green
        case "warning": .orange
        case "danger": .red
        default: .secondary
        }
    }
}

// A labeled row whose trailing value is monospaced and selectable — used for
// versions and resolved paths. Long paths truncate in the middle.
private struct MonoRow: View {
    let title: String
    let value: String
    var truncate: Bool = false

    var body: some View {
        LabeledContent(title) {
            Text(value)
                .font(.caption.monospaced())
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(truncate ? .middle : .tail)
                .textSelection(.enabled)
        }
    }
}

private struct PathRow: View {
    let title: String
    let path: AboutPath
    let reveal: () -> Void

    var body: some View {
        LabeledContent(title) {
            HStack(spacing: 8) {
                Text(path.display)
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
                Button("Reveal") { reveal() }
                    .disabled(path.revealPath == nil)
            }
        }
    }
}

private struct SettingsBanner: View {
    let text: String
    let systemImage: String
    let color: Color

    var body: some View {
        Label(text, systemImage: systemImage)
            .font(.caption)
            .foregroundStyle(color)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(color.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))
    }
}
