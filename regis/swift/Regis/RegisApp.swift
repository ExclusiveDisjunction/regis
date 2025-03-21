//
//  RegisApp.swift
//  Regis
//
//  Created by Hollan Sellars on 3/18/25.
//

import SwiftUI
import SwiftData

@main
struct RegisApp: App {
    var sharedModelContainer: ModelContainer = {
        let schema = Schema([
            KnownHost.self
        ])
        let modelConfiguration = ModelConfiguration(schema: schema, isStoredInMemoryOnly: false)

        do {
            return try ModelContainer(for: schema, configurations: [modelConfiguration])
        } catch {
            fatalError("Could not create ModelContainer: \(error)")
        }
    }()

    var body: some Scene {
        WindowGroup {
            ConnectView()
        }
        .modelContainer(sharedModelContainer)
    }
}
