//
//  MainView.swift
//  Regis
//
//  Created by Hollan Sellars on 3/19/25.
//

import SwiftUI;
import SwiftData;

struct MainView: View {
    var body: some View {
        NavigationSplitView {
            List {
                NavigationLink() {
                    Homepage()
                } label: {
                    Text("Home")
                }
                
                NavigationLink() {
                    VStack {
                        
                    }
                } label: {
                    Text("Current Metrics")
                }
                
                NavigationLink() {
                    VStack {
                        
                    }
                } label: {
                    Text("Historical Metrics")
                }
                
                NavigationLink() {
                    
                } label: {
                    Text("Server Settings")
                }
                
                NavigationLink() {
                    
                } label: {
                    Text("Settings")
                }
            }
        } detail: {
            Homepage()
        }
    }
}

#Preview {
    MainView()
}
