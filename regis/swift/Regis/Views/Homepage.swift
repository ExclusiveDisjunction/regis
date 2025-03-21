//
//  Homepage.swift
//  Regis
//
//  Created by Hollan Sellars on 3/19/25.
//

import SwiftUI;

struct Homepage: View {
    
    var body: some View {
        VStack {
            Text("Welcome to Regis").font(.title)
            Grid {
                GridRow {
                    Text("Connected to: ").font(.footnote).italic()
                    Text("(Unknown)").font(.footnote).italic()
                }
                
                GridRow {
                    Text("UI Version: ").font(.footnote).italic()
                    Text("0.1.0").font(.footnote).italic()
                }
                
                GridRow {
                    Text("Framework Version: ").font(.footnote).italic()
                    Text("0.1.0").font(.footnote).italic()
                }
            }
            
            Button(action: {
                
            }) {
                Label("Disconnect", systemImage: "icloud.slash.fill").foregroundStyle(.red)
            }
        }.padding(10)
    }
}

#Preview {
    Homepage()
}
