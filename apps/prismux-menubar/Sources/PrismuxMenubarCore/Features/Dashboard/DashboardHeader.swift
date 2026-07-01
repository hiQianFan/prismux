import SwiftUI

struct DashboardHeader: View {
    let title: String
    let subtitle: String

    var body: some View {
        SectionHeader(title: title, subtitle: subtitle)
    }
}
