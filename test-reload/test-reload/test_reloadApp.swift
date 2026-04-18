//
//  test_reloadApp.swift
//  test-reload
//
//  Created by Jeffrey Asante on 18/04/2026.
//

import SwiftUI

@main
struct test_reloadApp: App {
    @StateObject private var vmBridge = SwiftVMBridge()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(vmBridge)
                .task {
                    await vmBridge.reload()
                }
        }
    }
}
