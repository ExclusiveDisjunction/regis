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
class IPv4ViewModel: Identifiable {
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
struct IPv4Entry: View {
    @Bindable var data: IPv4ViewModel;
    
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


#Preview {
    var data = IPv4ViewModel();
    
    IPv4Entry(data: data)
}
