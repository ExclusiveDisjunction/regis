//
//  ContentView.swift
//  Regis
//
//  Created by Hollan Sellars on 3/18/25.
//

import SwiftUI
import SwiftData

struct PrevHostsPicker: View {
    @Binding var selectedHost: UUID?;
    @Query var hosts: [KnownHost];
    
    var body: some View {
        Table(hosts, selection: $selectedHost) {
            TableColumn("Name") { value in
                
            }
            TableColumn("IP") { value in
                
            }
        }
    }
}

enum ConnectViewResult {
    case usePrev
    case useNew
}

struct ConnectView: View {
    @State var selectedHost: UUID?;
    @State var ip: IPData = .v4(.init())
    @State var saveNew: Bool = true;
    @State var name: String = "";
    @State var showSheet: Bool = false;
    @State var sheetComplete: ConnectViewResult?;
    
    @Query var oldHosts: [KnownHost];
    
    private func connect() {
        
    }
    
    var body: some View {
        VStack {
            Text("Welcome to Regis!").font(.title)
            Text("Please select a host to connect to").font(.subheadline)
            
            Grid {
                GridRow {
                    Text("Previous Hosts").font(.headline)
                    Text("New Host").font(.headline)
                }
                GridRow {
                    PrevHostsPicker(selectedHost: $selectedHost)
                    
                    VStack {
                        Toggle("Save to hosts?", isOn: $saveNew)
                        
                        Grid {
                            GridRow {
                                Text("IP:")
                                IPEntry(data: $ip)
                            }
                            
                            if saveNew {
                                GridRow {
                                    Text("Name: ")
                                    TextField("Name", text: $name)
                                }
                            }
                        }
                        Spacer()
                    }
                }
            }.padding(5)
            
            Button(action: {
                showSheet = true
            }) {
                Text("Connect").font(.title2)
            }.buttonStyle(.borderedProminent)
        }.padding(10)
    }
}

#Preview {
    ConnectView()
}
