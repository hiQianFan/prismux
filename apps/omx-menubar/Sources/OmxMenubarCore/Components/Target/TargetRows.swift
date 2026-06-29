import SwiftUI

// MARK: - Shared layout

/// Shared column widths so every quota line (5h / 7d) aligns vertically across
/// all account cards. This is the fix for the "七扭八歪" misalignment: one set
/// of constants instead of ad-hoc per-row widths.
private enum TargetLayout {
    static let quotaLabelWidth: CGFloat = 30
}

// MARK: - Account row

struct AccountTargetRow: View {
    let account: MenubarAccount
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
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deleteAction: () -> Void
    @AppStorage("dev.openmux.menubar.hidePersonalIdentifiers") private var hidePersonalIdentifiers = false

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            TargetIdentity(
                title: titleText,
                subtitle: metaText,
                active: active,
                accent: accent,
                subtitleAccessory: {
                    if isCodex {
                        ResetCreditBadge(count: resetCreditCount, enabled: resetEnabled, accent: accent)
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
                    showResetAction: isCodex,
                    resetCreditCount: resetCreditCount,
                    resetEnabled: resetEnabled,
                    resetDisabledReason: resetDisabledReason,
                    useLabel: account.actions?.primaryLabel ?? "Use this account",
                    accent: accent,
                    switchAction: switchAction,
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
                            message: "Removes the OpenMux target and its managed snapshot.",
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
    let profile: MenubarProfile
    let active: Bool
    let switching: Bool
    let deleting: Bool
    let refreshing: Bool
    let confirmingDelete: Bool
    let switchAction: () -> Void
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deleteAction: () -> Void
    @AppStorage("dev.openmux.menubar.hidePersonalIdentifiers") private var hidePersonalIdentifiers = false

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
                    showResetAction: false,
                    resetCreditCount: 0,
                    resetEnabled: false,
                    resetDisabledReason: "Reset credits are only available for Codex accounts",
                    useLabel: profile.actions?.primaryLabel ?? "Use this profile",
                    accent: .accentColor,
                    switchAction: switchAction,
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
                            message: "Removes the OpenMux profile target and its managed secret if owned by OpenMux.",
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

/// Reset-credit count, display-only. Codex grants "rate limit reset credits";
/// this just surfaces how many the account holds. The actual reset action lives
/// in the overflow (···) menu — the badge is a status indicator, not a button.
/// Only rendered for Codex accounts (grey "0" when none); a count on every
/// non-Codex provider would be noise. The hover text explains what it is so the
/// Reset-credit count, display-only. Codex grants "rate limit reset credits";
/// this surfaces how many the account holds. The actual reset action lives in
/// the overflow (···) menu — the badge is a status indicator, not a button.
/// Only rendered for Codex accounts (grey "0" when none); a count on every
/// non-Codex provider would be noise. The "resets" word is self-explanatory, so
/// no hover is needed (and AppKit tooltips are unreliable inside a transient
/// NSPopover anyway).
private struct ResetCreditBadge: View {
    let count: UInt32
    let enabled: Bool
    let accent: Color

    var body: some View {
        Text("\(count) \(count == 1 ? "reset" : "resets")")
            .font(.caption2.monospacedDigit().weight(.bold))
            .foregroundStyle(enabled ? accent : Color.secondary)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background((enabled ? accent : Color.secondary).opacity(enabled ? 0.14 : 0.08), in: Capsule())
            .fixedSize(horizontal: true, vertical: false)
            .accessibilityLabel("\(count) Codex reset credit\(count == 1 ? "" : "s")")
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
    let showResetAction: Bool
    let resetCreditCount: UInt32
    let resetEnabled: Bool
    let resetDisabledReason: String
    let useLabel: String
    let accent: Color
    let switchAction: () -> Void
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
            if showResetAction {
                Button {
                    requestResetConfirmation()
                } label: {
                    Label("Reset usage limit", systemImage: "arrow.counterclockwise")
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
            .disabled(!canRemove || deleting || refreshing || resetting)
        } label: {
            Image(systemName: (deleting || resetting) ? "hourglass" : "ellipsis")
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

// MARK: - Quota line

/// One quota window, stacked: a text row (label · percent … full reset time)
/// over a thin full-width progress bar. The bar color encodes *health* of the
/// remaining quota (healthy → low → critical) rather than which window it is —
/// the window is already named by the "5h"/"7d" label, so color is free to warn.
/// Static tick marks at the warn/critical thresholds give the user a fixed
/// reference for "how close am I to trouble".
private struct QuotaLine: View {
    let window: QuotaWindow?
    let fallbackLabel: String

    /// Below these *remaining* fractions the quota is getting tight. Apple's
    /// battery/disk convention: comfortable → yellow → red.
    private static let warnThreshold = 0.50
    private static let criticalThreshold = 0.20

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 6) {
                Text(label)
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .frame(width: TargetLayout.quotaLabelWidth, alignment: .leading)

                Text(percentText)
                    .font(.caption2.monospacedDigit().weight(.semibold))
                    .foregroundStyle(barColor)

                Spacer(minLength: 8)

                if let reset = window?.resetAtUnix {
                    Text(fullDateTimeLabel(reset))
                        .font(.caption2.monospacedDigit())
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    Capsule().fill(Color.primary.opacity(0.12))
                    Capsule()
                        .fill(barColor)
                        .frame(width: max(3, proxy.size.width * fraction))

                    // Threshold ticks: neutral reference lines, not alarms.
                    tick(at: Self.warnThreshold, width: proxy.size.width)
                    tick(at: Self.criticalThreshold, width: proxy.size.width)
                }
            }
            .frame(height: 5)
        }
        .help(helpText)
        .accessibilityLabel(helpText)
    }

    private func tick(at value: Double, width: CGFloat) -> some View {
        Rectangle()
            .fill(Color.primary.opacity(0.3))
            .frame(width: 1, height: 5)
            .offset(x: width * value)
    }

    private var barColor: Color {
        guard window?.remainingPercentX100 != nil else { return .secondary }
        if fraction <= Self.criticalThreshold { return .red }
        if fraction <= Self.warnThreshold { return .yellow }
        return .green
    }

    private var label: String {
        let raw = window?.label ?? fallbackLabel
        return raw.lowercased().contains("week") ? "7d" : raw
    }

    private var fraction: Double {
        guard let remaining = window?.remainingPercentX100 else { return 0 }
        return max(0, min(1, Double(remaining) / 10_000.0))
    }

    private var percentText: String {
        guard let remaining = window?.remainingPercentX100 else { return "--" }
        return "\(Int(remaining) / 100)%"
    }

    private var helpText: String {
        let reset = window?.resetAtUnix.map { "resets \(fullDateTimeLabel($0))" } ?? "no reset data"
        return "\(label) \(percentText), \(reset)"
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

/// Full year-month-day reset label for the quota lines and tooltips.
private func fullDateTimeLabel(_ timestamp: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(timestamp))
    let formatter = DateFormatter()
    formatter.dateFormat = "yyyy-MM-dd HH:mm"
    return formatter.string(from: date)
}
