import SwiftUI

struct DiagnosticView: View {
    let diagnostic: Diagnostic

    var body: some View {
        StatusBanner(props: StatusBannerProps(
            severity: .warning,
            title: diagnostic.code,
            message: message
        ))
    }

    private var message: String {
        if let recoveryAction = diagnostic.recoveryAction, !recoveryAction.isEmpty {
            return "\(diagnostic.message)\n\(recoveryAction)"
        }
        return diagnostic.message
    }
}
