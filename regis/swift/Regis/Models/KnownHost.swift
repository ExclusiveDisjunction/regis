//
//  KnownHost.swift
//  Regis
//
//  Created by Hollan Sellars on 3/18/25.
//

import SwiftData;
import Foundation;

@Model
class KnownHost: Identifiable{
    init(_ id: UUID, name: String, ip: IPv4ConnectionBridge) {
        self.id = id
        self.name = name
        self.rawIP = ip.toString()
    }
    
    var id: UUID;
    var name: String;
    var rawIP: String;
    
    var ip: IPv4ConnectionBridge? {
        get {
            return IPv4ConnectionBridge.fromString(raw: self.rawIP)
        }
        set(v) {
            self.rawIP = v?.toString() ?? "";
        }
    }
}
