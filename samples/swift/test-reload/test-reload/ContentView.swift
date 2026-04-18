//
//  ContentView.swift
//  test-reload
//
//  Created by Jeffrey Asante on 18/04/2026.
//

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var vm: SwiftVMBridge

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "paintpalette.fill")
                .font(.system(size: 60))
                .foregroundColor(mapColor(vm.string("titleColor")))

            Text(vm.string("titleText"))
                .font(.largeTitle)
                .fontWeight(.black)
                .foregroundColor(mapColor(vm.string("titleColor")))

            Text(vm.string("subtitleText"))
                .font(.headline)
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)

            Divider()

            HStack {
                Text("Counter Value:")
                Text("\(vm.int("count"))")
                    .bold()
                    .padding(8)
                    .background(Color.secondary.opacity(0.1))
                    .cornerRadius(8)
            }
        }
        .padding(CGFloat(vm.int("padding", default: 20)))
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(white: 0.98))
    }

    private func mapColor(_ name: String) -> Color {
        switch name.lowercased() {
        case "red": return .red
        case "blue": return .blue
        case "green": return .green
        case "orange": return .orange
        case "purple": return .purple
        default: return .primary
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(SwiftVMBridge())
}
