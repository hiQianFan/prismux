import SwiftUI

// MARK: - Account row

struct AccountTargetRow: View {
    let account: TargetAccount
    let active: Bool
    let switching: Bool
    let deleting: Bool
    let resetting: Bool
    let refreshing: Bool
    let confirmingDelete: Bool
    let confirmingReset: Bool
    let primary: QuotaWindow?
    let secondary: QuotaWindow?
    let accent: Color
    let switchAction: () -> Void
    let requestResetConfirmation: () -> Void
    let cancelResetConfirmation: () -> Void
    let resetAction: () -> Void
    let refreshAction: () -> Void
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deleteAction: () -> Void
    @AppStorage("dev.prismux.menubar.hidePersonalIdentifiers") private var hidePersonalIdentifiers = false

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            TargetIdentity(
                title: titleText,
                subtitle: metaText,
                active: active,
                accent: accent,
                subtitleAccessory: {
                    if isCodex {
                        ResetCreditBadge(
                            count: resetCreditCount,
                            expiryTimes: resetCreditExpiryTimes,
                            enabled: resetEnabled,
                            accent: accent
                        )
                    }
                }
            ) {
                TargetActionCluster(
                    active: active,
                    canActivate: account.actions?.canActivate ?? !active,
                    canRemove: account.actions?.canRemove ?? true,
                    switching: switching,
                    deleting: deleting,
                    resetting: resetting,
                    refreshing: refreshing,
                    confirmingDelete: confirmingDelete,
                    confirmingReset: confirmingReset,
                    disabledReason: account.actions?.disabledReason,
                    showRefreshAction: true,
                    showResetAction: isCodex,
                    resetCreditCount: resetCreditCount,
                    resetEnabled: resetEnabled,
                    resetDisabledReason: resetDisabledReason,
                    useLabel: account.actions?.primaryLabel ?? "Use this account",
                    accent: accent,
                    switchAction: switchAction,
                    refreshAction: refreshAction,
                    requestResetConfirmation: requestResetConfirmation,
                    cancelResetConfirmation: cancelResetConfirmation,
                    resetPopover: {
                        ResetConfirmPopover(
                            title: "Reset eligible usage limits?",
                            message: "Consumes 1 reset credit for \(deleteLabel).",
                            resetAction: resetAction,
                            cancelAction: cancelResetConfirmation
                        )
                    },
                    requestDeleteConfirmation: requestDeleteConfirmation,
                    cancelDeleteConfirmation: cancelDeleteConfirmation,
                    deletePopover: {
                        DeleteConfirmPopover(
                            title: "Delete \(deleteLabel)?",
                            message: "Removes the Prismux target and its managed snapshot.",
                            deleteAction: deleteAction,
                            cancelAction: cancelDeleteConfirmation
                        )
                    }
                )
            }

            if let diagnostic = account.diagnostic {
                Label("\(diagnostic.code): \(diagnostic.message)", systemImage: "exclamationmark.triangle.fill")
                    .font(.caption2)
                    .foregroundStyle(.orange)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }

            if account.quota != nil {
                VStack(alignment: .leading, spacing: 4) {
                    QuotaLine(window: primary, fallbackLabel: "5h")
                    QuotaLine(window: secondary, fallbackLabel: "7d")
                }
            }
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 4)
        .modifier(TargetRowFeedback(active: active, accent: accent))
        .accessibilityElement(children: .combine)
    }

    /// plan · refreshed — folded into the identity subtitle to avoid a separate
    /// misaligned meta line.
    private var metaText: String {
        let plan = account.plan ?? account.status
        guard let refreshed = account.quota?.refreshedAtUnix else { return plan }
        return "\(plan) · \(shortDateTimeLabel(refreshed))"
    }

    private var titleText: String {
        if hidePersonalIdentifiers {
            return "#\(account.displayNumber) Account"
        }
        return "#\(account.displayNumber) \(account.shortLabel)"
    }

    private var deleteLabel: String {
        hidePersonalIdentifiers ? "account #\(account.displayNumber)" : account.shortLabel
    }

    /// Reset credits are a Codex-only concept. Show the affordance on every
    /// Codex account (grey "0" when none), and never on other providers.
    private var isCodex: Bool { account.provider == "codex" }

    private var resetCreditCount: UInt32 {
        account.quota?.resetCredits?.availableCount ?? 0
    }

    private var resetCreditExpiryTimes: [Int64] {
        (account.quota?.resetCredits?.credits ?? [])
            .compactMap(\.expiresAtUnix)
            .sorted()
    }

    private var hasActiveLimit: Bool {
        account.status == "limited"
            || account.status == "exhausted"
            || account.quota?.windows.contains(where: { $0.exhausted == true }) == true
    }

    private var resetEnabled: Bool {
        resetCreditCount > 0 && hasActiveLimit
    }

    private var resetDisabledReason: String {
        resetCreditCount == 0 ? "No reset credits available" : "No active limit to reset"
    }
}

// MARK: - Profile row

struct ProfileTargetRow: View {
    let profile: TargetProfile
    let active: Bool
    let switching: Bool
    let deleting: Bool
    let refreshing: Bool
    let confirmingDelete: Bool
    let switchAction: () -> Void
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deleteAction: () -> Void
    @AppStorage("dev.prismux.menubar.hidePersonalIdentifiers") private var hidePersonalIdentifiers = false

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            TargetIdentity(
                title: titleText,
                subtitle: subtitleText,
                active: active,
                accent: .accentColor,
                subtitleAccessory: { EmptyView() }
            ) {
                TargetActionCluster(
                    active: active,
                    canActivate: profile.actions?.canActivate ?? !active,
                    canRemove: profile.actions?.canRemove ?? true,
                    switching: switching,
                    deleting: deleting,
                    resetting: false,
                    refreshing: refreshing,
                    confirmingDelete: confirmingDelete,
                    confirmingReset: false,
                    disabledReason: profile.actions?.disabledReason,
                    showRefreshAction: false,
                    showResetAction: false,
                    resetCreditCount: 0,
                    resetEnabled: false,
                    resetDisabledReason: "Reset credits are only available for Codex accounts",
                    useLabel: profile.actions?.primaryLabel ?? "Use this profile",
                    accent: .accentColor,
                    switchAction: switchAction,
                    refreshAction: {},
                    requestResetConfirmation: {},
                    cancelResetConfirmation: {},
                    resetPopover: {
                        EmptyView()
                    },
                    requestDeleteConfirmation: requestDeleteConfirmation,
                    cancelDeleteConfirmation: cancelDeleteConfirmation,
                    deletePopover: {
                        DeleteConfirmPopover(
                            title: "Delete \(deleteLabel)?",
                            message: "Removes the Prismux profile target and its managed secret if owned by Prismux.",
                            deleteAction: deleteAction,
                            cancelAction: cancelDeleteConfirmation
                        )
                    }
                )
            }

            if let diagnostic = profile.diagnostic {
                Label("\(diagnostic.code): \(diagnostic.message)", systemImage: "exclamationmark.triangle.fill")
                    .font(.caption2)
                    .foregroundStyle(.orange)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 4)
        .modifier(TargetRowFeedback(active: active, accent: .accentColor))
        .accessibilityElement(children: .combine)
    }

    private var titleText: String {
        if hidePersonalIdentifiers {
            return profile.displayNumber > 0 ? "#\(profile.displayNumber) Profile" : "Profile"
        }
        return profile.displayNumber > 0 ? "#\(profile.displayNumber) \(profile.displayLabel)" : profile.displayLabel
    }

    private var subtitleText: String {
        if hidePersonalIdentifiers {
            return profile.status
        }
        return profile.secondaryLabel
    }

    private var deleteLabel: String {
        if hidePersonalIdentifiers {
            return profile.displayNumber > 0 ? "profile #\(profile.displayNumber)" : "profile"
        }
        return profile.displayLabel
    }
}

// MARK: - Identity (title + subtitle + trailing actions)

/// One identity line: active dot, title/subtitle, and the trailing action
/// cluster. A single source for the row header so accounts and profiles align.
private struct TargetIdentity<Trailing: View, SubtitleAccessory: View>: View {
    let title: String
    let subtitle: String
    let active: Bool
    let accent: Color
    @ViewBuilder let subtitleAccessory: SubtitleAccessory
    @ViewBuilder let trailing: Trailing

    var body: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(active ? accent : Color.primary.opacity(0.18))
                .frame(width: 7, height: 7)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)
                    .truncationMode(.middle)
                HStack(spacing: 6) {
                    subtitleText
                    subtitleAccessory
                }
            }

            Spacer(minLength: 8)
            trailing
        }
    }

    private var subtitleText: some View {
        Text(subtitle)
            .font(.caption2)
            .foregroundStyle(.secondary)
            .lineLimit(1)
            .truncationMode(.tail)
            .layoutPriority(1)
    }
}

// MARK: - Actions (single Use button + overflow Delete)

/// Reset-credit count. The badge opens a small details popover because native
/// popovers behave better than hover overlays inside the menubar window.
private struct ResetCreditBadge: View {
    let count: UInt32
    let expiryTimes: [Int64]
    let enabled: Bool
    let accent: Color
    @State private var showingDetails = false

    var body: some View {
        Button {
            if count > 0 {
                showingDetails.toggle()
            }
        } label: {
            Text("\(count) \(count == 1 ? "reset" : "resets")")
                .font(.caption2.monospacedDigit().weight(.bold))
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background((enabled ? accent : Color.secondary).opacity(enabled ? 0.14 : 0.08), in: Capsule())
        }
        .buttonStyle(.plain)
        .foregroundStyle(enabled ? accent : Color.secondary)
        .fixedSize(horizontal: true, vertical: false)
        .disabled(count == 0)
        .accessibilityLabel("\(count) Codex reset credit\(count == 1 ? "" : "s")")
        .help(count == 0 ? "No reset credits" : "Show reset credit expiry")
        .popover(isPresented: $showingDetails, arrowEdge: .top) {
            ResetCreditDetailsPopover(count: count, expiryTimes: expiryTimes)
        }
    }
}

private struct ResetCreditDetailsPopover: View {
    let count: UInt32
    let expiryTimes: [Int64]

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            Text("\(count) \(count == 1 ? "reset" : "resets") available")
                .font(.caption.weight(.semibold))

            if expiryTimes.isEmpty {
                Text("Expiry unavailable")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Expire")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(.secondary)

                    ForEach(Array(expiryTimes.prefix(2).enumerated()), id: \.offset) { index, timestamp in
                        Text("\(index + 1). \(fullDateTimeLabel(timestamp))")
                            .font(.caption2.monospacedDigit())
                    }
                }
            }

            Text("Used automatically")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .padding(10)
        .frame(minWidth: 170, alignment: .leading)
    }
}

private struct TargetActionCluster<ResetPopover: View, DeletePopover: View>: View {
    let active: Bool
    let canActivate: Bool
    let canRemove: Bool
    let switching: Bool
    let deleting: Bool
    let resetting: Bool
    let refreshing: Bool
    let confirmingDelete: Bool
    let confirmingReset: Bool
    let disabledReason: String?
    let showRefreshAction: Bool
    let showResetAction: Bool
    let resetCreditCount: UInt32
    let resetEnabled: Bool
    let resetDisabledReason: String
    let useLabel: String
    let accent: Color
    let switchAction: () -> Void
    let refreshAction: () -> Void
    let requestResetConfirmation: () -> Void
    let cancelResetConfirmation: () -> Void
    let resetPopover: () -> ResetPopover
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deletePopover: () -> DeletePopover

    var body: some View {
        HStack(spacing: 6) {
            useButton
            overflowMenu
                .popover(isPresented: resetConfirmationBinding, arrowEdge: .trailing, content: resetPopover)
                .popover(isPresented: deleteConfirmationBinding, arrowEdge: .trailing, content: deletePopover)
        }
    }

    @ViewBuilder
    private var useButton: some View {
        if active {
            Label("Active", systemImage: "checkmark.circle.fill")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.green)
                .padding(.horizontal, 9)
                .padding(.vertical, 4)
                .background(Color.green.opacity(0.12), in: Capsule())
                .accessibilityLabel("Current target")
        } else {
            Button(action: switchAction) {
                Group {
                    if switching {
                        ProgressView().controlSize(.small)
                    } else {
                        Text("Use")
                    }
                }
                .font(.caption.weight(.semibold))
                .frame(minWidth: 30)
                .padding(.horizontal, 9)
                .padding(.vertical, 4)
            }
            .buttonStyle(.borderless)
            .background(accent.opacity(0.14), in: Capsule())
            .foregroundStyle(accent)
            .disabled(!canActivate || switching || refreshing || deleting || resetting)
            .help(disabledReason ?? useLabel)
            .accessibilityLabel(useLabel)
        }
    }

    private var overflowMenu: some View {
        Menu {
            if showRefreshAction {
                Button {
                    refreshAction()
                } label: {
                    AlignedMenuText("Refresh usage")
                }
                .disabled(refreshing || switching || deleting || resetting)

                Divider()
            }

            if showResetAction {
                Button {
                    requestResetConfirmation()
                } label: {
                    AlignedMenuText("Reset usage limit")
                }
                .disabled(!resetEnabled || resetting || deleting || refreshing)
                .help(resetEnabled ? "Consume 1 reset credit to reset eligible usage limits" : resetDisabledReason)

                Divider()
            }

            Button(role: .destructive) {
                requestDeleteConfirmation()
            } label: {
                Label("Delete", systemImage: "trash")
            }
            // Not gated on `refreshing`: a background usage poll (popover-open or
            // the 300s timer) sets refreshingProvider and would otherwise gray
            // out Delete for the whole provider. Removal is unrelated to usage
            // refresh and the backend serializes operations anyway.
            .disabled(!canRemove || deleting)
        } label: {
            Image(systemName: "ellipsis")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
                .frame(width: 24, height: 24)
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .help("More actions")
        .accessibilityLabel("More actions")
    }

    private var deleteConfirmationBinding: Binding<Bool> {
        Binding(
            get: { confirmingDelete },
            set: { isPresented in
                if isPresented {
                    requestDeleteConfirmation()
                } else {
                    cancelDeleteConfirmation()
                }
            }
        )
    }

    private var resetConfirmationBinding: Binding<Bool> {
        Binding(
            get: { confirmingReset },
            set: { isPresented in
                if isPresented {
                    requestResetConfirmation()
                } else {
                    cancelResetConfirmation()
                }
            }
        )
    }
}

private struct AlignedMenuText: View {
    let title: String

    init(_ title: String) {
        self.title = title
    }

    var body: some View {
        HStack {
            Image(systemName: "trash")
                .hidden()
            Text(title)
        }
    }
}

// MARK: - Delete confirmation

private struct DeleteConfirmPopover: View {
    let title: String
    let message: String
    let deleteAction: () -> Void
    let cancelAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.headline)
            Text(message)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            HStack {
                Spacer()
                Button("Cancel", action: cancelAction)
                    .keyboardShortcut(.cancelAction)
                Button("Delete", role: .destructive, action: deleteAction)
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(12)
        .frame(width: 260)
    }
}

private struct ResetConfirmPopover: View {
    let title: String
    let message: String
    let resetAction: () -> Void
    let cancelAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.headline)
            Text(message)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            HStack {
                Spacer()
                Button("Cancel", action: cancelAction)
                    .keyboardShortcut(.cancelAction)
                Button("Reset", role: .destructive, action: resetAction)
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(12)
        .frame(width: 280)
    }
}

// MARK: - Helpers

private func shortDateTimeLabel(_ timestamp: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(timestamp))
    let formatter = DateFormatter()
    formatter.dateFormat = "MM-dd HH:mm"
    return formatter.string(from: date)
}

/// Full year-month-day reset label for quota lines and reset-credit details.
private func fullDateTimeLabel(_ timestamp: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(timestamp))
    let formatter = DateFormatter()
    formatter.dateFormat = "yyyy-MM-dd HH:mm"
    return formatter.string(from: date)
}

public func resetCreditHoverText(count: UInt32, expiryTimes: [Int64]) -> String {
    guard count > 0 else { return "" }
    var lines = ["\(count) \(count == 1 ? "reset" : "resets") available"]
    let expiries = expiryTimes.sorted().prefix(2)
    if expiries.isEmpty {
        lines.append("Expiry unavailable")
    } else {
        lines.append("Expire")
        for (index, timestamp) in expiries.enumerated() {
            lines.append("\(index + 1). \(fullDateTimeLabel(timestamp))")
        }
    }
    lines.append("Used automatically")
    return lines.joined(separator: "\n")
}
