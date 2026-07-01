import SwiftUI

enum PrismuxTokens {
    enum Spacing {
        static let xs: CGFloat = 4
        static let sm: CGFloat = 8
        static let md: CGFloat = 12
        static let lg: CGFloat = 16
    }

    enum Radius {
        static let row: CGFloat = 8
        static let panel: CGFloat = 8
    }

    enum StatusColor {
        static let healthy = Color.green
        static let warning = Color.orange
        static let failed = Color.red
        static let muted = Color.secondary
    }
}
