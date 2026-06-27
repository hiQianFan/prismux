import SwiftUI

struct TargetQuotaView: View {
    let props: QuotaMeterProps

    var body: some View {
        MeterRing(props: props)
    }
}
