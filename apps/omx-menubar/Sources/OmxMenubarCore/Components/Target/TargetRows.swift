import SwiftUI

// MARK: - Shared layout

/// Shared column widths so every quota line (5h / 7d) aligns vertically across
/// all account cards. This is the fix for the "七扭八歪" misalignment: one set
/// of constants instead of ad-hoc per-row widths.
private enum TargetLayout {
    static let quotaLabelWidth: CGFloat = 30
    static let quotaPercentWidth: CGFloat = 40
}

// MARK: - Account row

struct AccountTargetRow: View {
    let account: MenubarAccount
    let active: Bool
    let switching: Bool
    let deleting: Bool
    let refreshing: Bool
    let confirmingDelete: Bool
    let primary: QuotaWindow?
    let secondary: QuotaWindow?
    let accent: Color
    let switchAction: () -> Void
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deleteAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            TargetIdentity(
                title: "#\(account.displayNumber) \(account.shortLabel)",
                subtitle: metaText,
                active: active,
                accent: accent
            ) {
                TargetActionCluster(
                    active: active,
                    canActivate: account.actions?.canActivate ?? !active,
                    canRemove: account.actions?.canRemove ?? true,
                    switching: switching,
                    deleting: deleting,
                    refreshing: refreshing,
                    confirmingDelete: confirmingDelete,
                    disabledReason: account.actions?.disabledReason,
                    useLabel: account.actions?.primaryLabel ?? "Use this account",
                    accent: accent,
                    switchAction: switchAction,
                    requestDeleteConfirmation: requestDeleteConfirmation,
                    cancelDeleteConfirmation: cancelDeleteConfirmation,
                    deletePopover: {
                        DeleteConfirmPopover(
                            title: "Delete \(account.shortLabel)?",
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
                    QuotaLine(window: primary, fallbackLabel: "5h", color: accent)
                    QuotaLine(window: secondary, fallbackLabel: "7d", color: .blue)
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

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            TargetIdentity(
                title: profile.displayNumber > 0 ? "#\(profile.displayNumber) \(profile.displayLabel)" : profile.displayLabel,
                subtitle: profile.secondaryLabel,
                active: active,
                accent: .accentColor
            ) {
                TargetActionCluster(
                    active: active,
                    canActivate: profile.actions?.canActivate ?? !active,
                    canRemove: profile.actions?.canRemove ?? true,
                    switching: switching,
                    deleting: deleting,
                    refreshing: refreshing,
                    confirmingDelete: confirmingDelete,
                    disabledReason: profile.actions?.disabledReason,
                    useLabel: profile.actions?.primaryLabel ?? "Use this profile",
                    accent: .accentColor,
                    switchAction: switchAction,
                    requestDeleteConfirmation: requestDeleteConfirmation,
                    cancelDeleteConfirmation: cancelDeleteConfirmation,
                    deletePopover: {
                        DeleteConfirmPopover(
                            title: "Delete \(profile.displayLabel)?",
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
}

// MARK: - Identity (title + subtitle + trailing actions)

/// One identity line: active dot, title/subtitle, and the trailing action
/// cluster. A single source for the row header so accounts and profiles align.
private struct TargetIdentity<Trailing: View>: View {
    let title: String
    let subtitle: String
    let active: Bool
    let accent: Color
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
                Text(subtitle)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }

            Spacer(minLength: 8)
            trailing
        }
    }
}

// MARK: - Actions (single Use button + overflow Delete)

/// The single action cluster: one Use button (becomes an "Active" checkmark
/// when current) plus an overflow menu holding Delete. Replaces the old
/// duplicated Use button + ActionRail.
private struct TargetActionCluster<DeletePopover: View>: View {
    let active: Bool
    let canActivate: Bool
    let canRemove: Bool
    let switching: Bool
    let deleting: Bool
    let refreshing: Bool
    let confirmingDelete: Bool
    let disabledReason: String?
    let useLabel: String
    let accent: Color
    let switchAction: () -> Void
    let requestDeleteConfirmation: () -> Void
    let cancelDeleteConfirmation: () -> Void
    let deletePopover: () -> DeletePopover

    var body: some View {
        HStack(spacing: 6) {
            useButton
            overflowMenu
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
            .disabled(!canActivate || switching || refreshing || deleting)
            .help(disabledReason ?? useLabel)
            .accessibilityLabel(useLabel)
        }
    }

    private var overflowMenu: some View {
        Menu {
            Button(role: .destructive) {
                requestDeleteConfirmation()
            } label: {
                Label("Delete", systemImage: "trash")
            }
            .disabled(!canRemove || deleting || refreshing)
        } label: {
            Image(systemName: deleting ? "hourglass" : "ellipsis")
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
}

// MARK: - Quota line

/// One aligned quota meter line. Label and percent live in shared fixed-width
/// columns so the 5h and 7d rows line up; the reset time moved to the tooltip.
private struct QuotaLine: View {
    let window: QuotaWindow?
    let fallbackLabel: String
    let color: Color

    var body: some View {
        HStack(spacing: 8) {
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .frame(width: TargetLayout.quotaLabelWidth, alignment: .leading)

            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    Capsule().fill(Color.primary.opacity(0.12))
                    Capsule()
                        .fill(color)
                        .frame(width: max(3, proxy.size.width * fraction))
                }
            }
            .frame(height: 6)

            Text(percentText)
                .font(.caption2.monospacedDigit().weight(.semibold))
                .frame(width: TargetLayout.quotaPercentWidth, alignment: .trailing)
        }
        .help(helpText)
        .accessibilityLabel(helpText)
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
        let reset = window?.resetAtUnix.map { "resets \(shortDateTimeLabel($0))" } ?? "no reset data"
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

// MARK: - Helpers

private func shortDateTimeLabel(_ timestamp: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(timestamp))
    let formatter = DateFormatter()
    formatter.dateFormat = "MM-dd HH:mm"
    return formatter.string(from: date)
}
