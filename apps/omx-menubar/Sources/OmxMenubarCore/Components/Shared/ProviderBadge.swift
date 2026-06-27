import SwiftUI

struct ProviderBadge: View {
    let provider: String
    let color: Color

    var body: some View {
        Badge(text: provider.capitalized, color: color)
    }
}
