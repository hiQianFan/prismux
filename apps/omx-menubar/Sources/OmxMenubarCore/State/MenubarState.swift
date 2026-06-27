import Foundation

enum MenubarState {
    case loading
    case ready(DashboardReport, stale: Bool)
    case failed(lastGood: DashboardReport?, message: String)
    case backendUnavailable(lastGood: DashboardReport?, message: String)
    case upgradeRequired(message: String)
}

struct TargetOperationState: Equatable {
    let targetId: String
    let kind: Kind

    enum Kind: Equatable {
        case refreshing
        case switching
        case deleting
    }
}
