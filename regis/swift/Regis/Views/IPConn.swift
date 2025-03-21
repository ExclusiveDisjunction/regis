//
//  IPConn.swift
//  Regis
//
//  Created by Hollan Sellars on 3/19/25.
//

import SwiftData;
import Foundation;
import SwiftUI;

@Observable
class IPv4Data: Identifiable {
    init(_ id: UUID, _ a: Int, _ b: Int, _ c: Int, _ d: Int) {
        self.id = id
        self.a = a
        self.b = b
        self.c = c
        self.d = d
    }
    convenience init() {
        self.init(UUID(), 0, 0, 0, 0)
    }

    var id: UUID;
    var a: Int;
    var b: Int;
    var c: Int;
    var d: Int;
    
    func parseIntoConnection() -> IPv4ConnectionBridge? {
        let raw = [self.a, self.b, self.c, self.d];
        
        return IPv4ConnectionBridge.unpack(raw)
    }
}
struct IP4Entry: View {
    @Bindable var data: IPv4Data;
    
    var body: some View {
        HStack {
            TextField("A", value: $data.a, format: .number).labelsHidden()
            Text(".")
            TextField("B", value: $data.b, format: .number).labelsHidden()
            Text(".")
            TextField("C", value: $data.c, format: .number).labelsHidden()
            Text(".")
            TextField("D", value: $data.d, format: .number).labelsHidden()
        }
    }
}

@Observable
class IPv6Data: Identifiable {
    init(_ id: UUID, _ a: String, _ b: String, _ c: String, _ d: String, _ e: String, _ f: String, _ g: String, _ h: String) {
        self.id = id
        self.a = a
        self.b = b
        self.c = c
        self.d = d
        self.e = e
        self.f = f
        self.g = g
        self.h = h
    }
    convenience init() {
        self.init(UUID(), "", "", "", "", "", "", "", "");
    }

    var id: UUID;
    var a: String;
    var b: String;
    var c: String;
    var d: String;
    var e: String;
    var f: String;
    var g: String;
    var h: String;
    
    func parseIntoConnection() -> IPv6ConnectionBridge? {
        let orig = [self.a, self.b, self.c, self.d, self.e, self.f, self.g, self.h]
        
        return IPv6ConnectionBridge.unpack(orig)
    }
}

struct IPv6Entry: View {
    @Bindable var data: IPv6Data;
    
    var body: some View {
        HStack {
            TextField("A", text: $data.a).labelsHidden().frame(minWidth: 25)
            Text(":")
            TextField("B", text: $data.a).labelsHidden().frame(minWidth: 25)
            Text(":")
            TextField("C", text: $data.a).labelsHidden().frame(minWidth: 25)
            Text(":")
            TextField("D", text: $data.a).labelsHidden().frame(minWidth: 25)
            Text(":")
            TextField("E", text: $data.a).labelsHidden().frame(minWidth: 25)
            Text(":")
            TextField("F", text: $data.a).labelsHidden().frame(minWidth: 25)
            Text(":")
            TextField("G", text: $data.a).labelsHidden().frame(minWidth: 25)
            Text(":")
            TextField("H", text: $data.a).labelsHidden().frame(minWidth: 25)
        }
    }
}


enum IPData {
    case v6(IPv6Data)
    case v4(IPv4Data)
}

struct IPEntry: View {
    @Binding var data: IPData;
    
    private var numberFormatter: NumberFormatter {
            let formatter = NumberFormatter()
            formatter.numberStyle = .none
            return formatter
    }
    
    var body: some View {
        HStack {
            switch self.data {
            case .v4(let $target): IP4Entry(data: $target)
            case .v6(let $target): IPv6Entry(data: $target)
            }
            
            Button(action : {
                let new_data: IPData;
                switch self.data {
                case .v4(_): new_data = .v6(.init())
                case .v6(_): new_data = .v4(.init())
                }
                
                self.data = new_data
            }) {
                switch self.data {
                    case .v4(_): Text("v6")
                    case .v6(_): Text("v4")
                }
            }
        }
    }
}


#Preview {
    var data: IPData = .v4(.init());
    let binding = Binding(get: { data }, set: { data = $0 } );
    
    IPEntry(data: binding)
}
