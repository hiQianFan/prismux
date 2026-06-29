import AppKit
import SwiftUI

struct MenubarSettingsView: View {
    @ObservedObject var store: MenubarSettingsStore
    @AppStorage("dev.openmux.menubar.trayDisplayMode") private var trayDisplayMode = "text"

    var body: some View {
        NavigationSplitView {
            List(MenubarSettingsTab.allCases, selection: $store.selectedTab) { tab in
                Label(tab.title, systemImage: tab.icon)
                    .tag(tab)
            }
            .navigationSplitViewColumnWidth(150)
        } detail: {
            detail
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .frame(width: 680, height: 460)
        .task { await store.load() }
    }

    @ViewBuilder
    private var detail: some View {
        VStack(alignment: .leading, spacing: 0) {
            titleBar
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    if let errorMessage = store.errorMessage {
                        SettingsBanner(text: errorMessage, systemImage: "exclamationmark.triangle.fill", color: .orange)
                    }
                    if store.loading, store.settings == nil {
                        ProgressView("Loading settings")
                            .frame(maxWidth: .infinity, alignment: .center)
                            .padding(.top, 120)
                    } else {
                        switch store.selectedTab {
                        case .general:
                            generalPane
                        case .providers:
                            providersPane
                        case .about:
                            aboutPane
                        }
                    }
                }
                .padding(20)
            }
        }
    }

    private var titleBar: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(store.selectedTab.title)
                    .font(.title3.weight(.semibold))
                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if store.saving {
                ProgressView()
                    .controlSize(.small)
            }
            Button {
                Task { await store.load() }
            } label: {
                Image(systemName: "arrow.clockwise")
            }
            .buttonStyle(.borderless)
            .help("Refresh settings")
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 14)
    }

    private var subtitle: String {
        switch store.selectedTab {
        case .general:
            "Menubar behavior and privacy"
        case .providers:
            "Provider availability and data source policy"
        case .about:
            "Version, paths, and safe support information"
        }
    }

    private var generalPane: some View {
        VStack(alignment: .leading, spacing: 16) {
            SettingsSection(title: "Menubar") {
                Picker("Tray display", selection: $trayDisplayMode) {
                    Text("Icon and summary").tag("text")
                    Text("Icon only").tag("icon_only")
                }
                .pickerStyle(.segmented)

                if let settings = store.settings {
                    Picker(
                        "Background refresh",
                        selection: refreshCadenceBinding(settings)
                    ) {
                        Text("5 min").tag(UInt64(300))
                        Text("15 min").tag(UInt64(900))
                        Text("30 min").tag(UInt64(1800))
                    }
                    .pickerStyle(.segmented)
                    Text("Interactive refresh still happens when opening the popover or pressing refresh.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            SettingsSection(title: "Privacy") {
                if let settings = store.settings {
                    Toggle(
                        "Hide personal account identifiers in menubar rows",
                        isOn: privacyBinding(settings)
                    )
                    Text("When enabled, account and profile rows use coarse labels such as #1 Account instead of emails or profile names.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var providersPane: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let providers = store.settings?.providers {
                ForEach(providers) { provider in
                    ProviderSettingsCard(
                        provider: provider,
                        enabled: Binding(
                            get: { provider.enabled },
                            set: { store.updateProviderEnabled(provider, enabled: $0) }
                        ),
                        sourcePreference: Binding(
                            get: { provider.sourcePreference },
                            set: { store.updateProviderSource(provider, sourcePreference: $0) }
                        )
                    )
                }
            }
        }
    }

    private var aboutPane: some View {
        VStack(alignment: .leading, spacing: 16) {
            if let about = store.about {
                SettingsSection(title: "OpenMux") {
                    SettingsKeyValueRow(label: "Version", value: about.appVersion)
                    SettingsKeyValueRow(label: "Control plane schema", value: "\(about.controlPlaneSchemaVersion)")
                    SettingsKeyValueRow(label: "State schema", value: "\(about.stateSchemaVersion)")
                    SettingsKeyValueRow(label: "Settings schema", value: "\(about.settingsSchemaVersion)")
                    SettingsKeyValueRow(label: "Runtime", value: about.runtime.statusText)
                }

                SettingsSection(title: "Storage") {
                    PathRow(title: "State root", path: about.stateRoot) { store.reveal(about.stateRoot) }
                    PathRow(title: "Settings", path: about.settingsPath) { store.reveal(about.settingsPath) }
                }

                SettingsSection(title: "Support") {
                    HStack {
                        Button("Copy Version Info") { store.copyVersionInfo() }
                        Button("Copy Redacted Support Report") {
                            Task { await store.copySupportReport() }
                        }
                    }
                    .buttonStyle(.bordered)
                    if let supportStatus = store.supportStatus {
                        Text(supportStatus)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }

                SettingsSection(title: "Links") {
                    ForEach(about.links) { link in
                        Button(link.label) {
                            if let url = URL(string: link.url) {
                                NSWorkspace.shared.open(url)
                            }
                        }
                        .buttonStyle(.link)
                    }
                }
            } else {
                Text("About information is unavailable.")
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func refreshCadenceBinding(_ settings: SettingsView) -> Binding<UInt64> {
        Binding(
            get: { store.settings?.general.refreshCadenceSeconds ?? settings.general.refreshCadenceSeconds },
            set: { store.updateRefreshCadence($0) }
        )
    }

    private func privacyBinding(_ settings: SettingsView) -> Binding<Bool> {
        Binding(
            get: { store.settings?.privacy.hidePersonalIdentifiers ?? settings.privacy.hidePersonalIdentifiers },
            set: { store.updatePrivacy($0) }
        )
    }
}

private struct ProviderSettingsCard: View {
    let provider: ProviderSettings
    @Binding var enabled: Bool
    @Binding var sourcePreference: SourcePreference

    var body: some View {
        SettingsSection(title: provider.displayLabel) {
            HStack(alignment: .center, spacing: 12) {
                Circle()
                    .fill(toneColor(provider.status.statusTone))
                    .frame(width: 9, height: 9)
                VStack(alignment: .leading, spacing: 2) {
                    Text(provider.status.statusText)
                        .font(.subheadline.weight(.medium))
                    Text(provider.provider)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Toggle("Enabled", isOn: $enabled)
                    .toggleStyle(.switch)
            }

            Picker("Usage data source", selection: $sourcePreference) {
                ForEach(provider.sourceOptions) { option in
                    Text(option.label)
                        .tag(option.value)
                        .disabled(option.disabledReason != nil)
                }
            }
            .pickerStyle(.segmented)

            Text("Auto uses the healthiest supported local source first. Local only disables future remote usage collection for this provider.")
                .font(.caption)
                .foregroundStyle(.secondary)

            ForEach(Array(provider.diagnostics.enumerated()), id: \.offset) { _, diagnostic in
                SettingsBanner(text: diagnostic.message, systemImage: "exclamationmark.triangle.fill", color: .orange)
            }
        }
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

private struct SettingsSection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.headline)
            VStack(alignment: .leading, spacing: 10) {
                content
            }
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 8))
        }
    }
}

private struct SettingsKeyValueRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(label)
                .foregroundStyle(.secondary)
            Spacer()
            Text(value)
                .multilineTextAlignment(.trailing)
                .textSelection(.enabled)
        }
        .font(.subheadline)
    }
}

private struct PathRow: View {
    let title: String
    let path: AboutPath
    let reveal: () -> Void

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.subheadline.weight(.medium))
                Text(path.display)
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
            }
            Spacer()
            Button("Reveal") { reveal() }
                .disabled(path.revealPath == nil)
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
