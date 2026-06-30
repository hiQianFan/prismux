import AppKit
import SwiftUI

struct MenubarSettingsView: View {
    @ObservedObject var store: MenubarSettingsStore
    @AppStorage("dev.openmux.menubar.trayDisplayMode") private var trayDisplayMode = "text"

    var body: some View {
        VStack(spacing: 0) {
            tabBar
            Divider()
            content
        }
        .frame(width: 540, height: 460)
        .task { await store.load() }
    }

    // Top toolbar tabs, in the macOS Preferences idiom — three first-class
    // categories, not a sidebar that dwarfs this little content.
    private var tabBar: some View {
        HStack(spacing: 6) {
            ForEach(MenubarSettingsTab.allCases) { tab in
                let selected = store.selectedTab == tab
                Button {
                    store.selectedTab = tab
                } label: {
                    VStack(spacing: 3) {
                        Image(systemName: tab.icon)
                            .font(.system(size: 16, weight: .regular))
                        Text(tab.title)
                            .font(.caption)
                    }
                    .frame(width: 70, height: 44)
                    .contentShape(Rectangle())
                    .foregroundStyle(selected ? Color.accentColor : Color.secondary)
                    .background(
                        RoundedRectangle(cornerRadius: 6)
                            .fill(selected ? Color.accentColor.opacity(0.12) : .clear)
                    )
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
            }
            Spacer()
            if store.saving { ProgressView().controlSize(.small) }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var content: some View {
        if store.loading, store.settings == nil {
            ProgressView("Loading settings")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            switch store.selectedTab {
            case .general: GeneralPane(store: store, trayDisplayMode: $trayDisplayMode)
            case .providers: ProvidersPane(store: store)
            case .about: AboutPane(store: store)
            }
        }
    }
}

// MARK: - General

private struct GeneralPane: View {
    @ObservedObject var store: MenubarSettingsStore
    @Binding var trayDisplayMode: String

    var body: some View {
        Form {
            if let errorMessage = store.errorMessage {
                Section { SettingsBanner(text: errorMessage, systemImage: "exclamationmark.triangle.fill", color: .orange) }
            }

            Section("Appearance") {
                Picker("Tray display", selection: $trayDisplayMode) {
                    Text("Icon and summary").tag("text")
                    Text("Icon only").tag("icon_only")
                }
                if let settings = store.settings {
                    Toggle("Hide personal account identifiers", isOn: privacyBinding(settings))
                    Text("Account and profile rows use coarse labels such as #1 Account instead of emails or profile names.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Section("Startup") {
                Toggle("Open OpenMux at login", isOn: Binding(
                    get: { store.launchAtLogin },
                    set: { store.setLaunchAtLogin($0) }
                ))
            }

            Section("Command-line tool") {
                LabeledContent("omx command") {
                    StatusChip(text: store.cliStatus.statusText, tone: store.cliStatus.statusTone)
                }
                if let bundled = store.cliStatus.bundledPath {
                    LabeledContent("Bundled helper") {
                        Text(bundled)
                            .font(.caption.monospaced())
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .textSelection(.enabled)
                    }
                }
                if let helperVersion = store.cliStatus.helperVersion {
                    LabeledContent("Helper version") {
                        Text(helperVersion)
                            .font(.caption.monospaced())
                            .foregroundStyle(.secondary)
                    }
                }
                if let found = store.cliStatus.foundPath {
                    LabeledContent("Terminal omx") {
                        Text(found)
                            .font(.caption.monospaced())
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .textSelection(.enabled)
                    }
                }
                if let foundVersion = store.cliStatus.foundVersion {
                    LabeledContent("Terminal version") {
                        Text(foundVersion)
                            .font(.caption.monospaced())
                            .foregroundStyle(.secondary)
                    }
                }
                HStack {
                    Button("Enable omx command") { store.enableCliCommand() }
                        .disabled(!store.cliStatus.helperAvailable || store.cliStatus.resolution == .ready)
                    Button("Copy command") { store.copyCliCommand() }
                    if store.cliStatus.pathCommand != nil {
                        Button("Copy PATH command") { store.copyPathCommand() }
                    }
                }
                if store.cliStatus.resolution == .differentFound, let found = store.cliStatus.foundPath {
                    Text("A different omx is already on your PATH at \(found). OpenMux won't overwrite it; adjust it manually if you want the bundled helper.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else {
                    Text("The omx CLI ships inside OpenMux. This only links it onto your PATH — it does not download a separate tool.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                LabeledContent("Network proxy") {
                    Text(store.cliStatus.proxySource)
                        .foregroundStyle(.secondary)
                }
                Text("Effective for OpenMux refresh requests. Set OMUX_HTTPS_PROXY / HTTPS_PROXY / ALL_PROXY to change it.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

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

// MARK: - Providers

private struct ProvidersPane: View {
    @ObservedObject var store: MenubarSettingsStore

    var body: some View {
        Form {
            if let providers = store.settings?.providers {
                ForEach(providers) { provider in
                    Section(provider.displayLabel) {
                        HStack(spacing: 10) {
                            Circle()
                                .fill(toneColor(provider.status.statusTone))
                                .frame(width: 9, height: 9)
                            Text(provider.status.statusText)
                                .font(.subheadline)
                            Spacer()
                            Toggle("Enabled", isOn: Binding(
                                get: { provider.enabled },
                                set: { store.updateProviderEnabled(provider, enabled: $0) }
                            ))
                            .labelsHidden()
                        }

                        if shouldShowSource(provider) {
                            HStack(spacing: 4) {
                                Picker("Usage data source", selection: Binding(
                                    get: { provider.sourcePreference },
                                    set: { store.updateProviderSource(provider, sourcePreference: $0) }
                                )) {
                                    ForEach(provider.sourceOptions) { option in
                                        Text(option.label)
                                            .tag(option.value)
                                            .disabled(option.disabledReason != nil)
                                    }
                                }
                                HelpButton(text: "Auto uses the healthiest supported local source first. Local only disables future remote usage collection for this provider.")
                            }
                        }

                        ForEach(Array(provider.diagnostics.enumerated()), id: \.offset) { _, diagnostic in
                            SettingsBanner(text: diagnostic.message, systemImage: "exclamationmark.triangle.fill", color: .orange)
                        }
                    }
                }
            }

            Section {
                Text("Sign in, import profiles, and switch the active account or gateway profile from the OpenMux popover.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }

    // Only surface the source toggle when the provider actually exposes a
    // meaningful choice — a single non-disabled option is no choice at all.
    private func shouldShowSource(_ provider: ProviderSettings) -> Bool {
        provider.sourceOptions.filter { $0.disabledReason == nil }.count > 1
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
                    VStack(spacing: 6) {
                        Image(nsImage: NSApp.applicationIconImage)
                            .resizable()
                            .frame(width: 64, height: 64)
                        Text("OpenMux")
                            .font(.title3.weight(.semibold))
                        Text("Version \(about.appVersion)")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                        Text("CLI helper \(store.cliStatus.helperVersion ?? "unavailable")")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Text(about.runtime.statusText)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                        Text("Local account switcher for AI coding tools")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
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

                Section("Support") {
                    Button("Copy Version Info") { store.copyVersionInfo() }
                    Button("Copy Redacted Support Report") {
                        Task { await store.copySupportReport() }
                    }
                    if let status = store.supportStatus {
                        Text(status).font(.caption).foregroundStyle(.secondary)
                    }
                }

                Section {
                    Text("MIT License")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity, alignment: .center)
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

private struct HelpButton: View {
    let text: String
    @State private var showing = false

    var body: some View {
        Button {
            showing.toggle()
        } label: {
            Image(systemName: "questionmark.circle")
                .foregroundStyle(.secondary)
        }
        .buttonStyle(.borderless)
        .popover(isPresented: $showing) {
            Text(text)
                .font(.caption)
                .padding(12)
                .frame(width: 240)
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
