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
    init(_ id: UUID, name: String, ip: IPConnection) {
        self.id = id
        self.name = name
        self.ip = ip
    }
    
    var id: UUID;
    var name: String;
    var ip: IPConnection;
}
