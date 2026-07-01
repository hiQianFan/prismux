import SwiftUI

struct OverviewPage<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        content
    }
}
