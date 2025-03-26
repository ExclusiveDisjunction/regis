//
//  ConnectionBridge.swift
//  Regis
//
//  Created by Hollan Sellars on 3/19/25.
//

struct IPv4ConnectionBridge {
    let a: UInt8
    let b: UInt8
    let c: UInt8
    let d: UInt8
    
    static func unpack(_ data: [Int]) -> IPv4ConnectionBridge? {
        guard data.count == 4 else { return nil }
        
        var result = [UInt8](repeating: 0, count: 4);
        for (i, val) in data.enumerated() {
            guard val >= 0 && val <= 255 else { return nil }
            
            result[i] = UInt8(val)
        }
        
        return self.init(
            a: result[0],
            b: result[1],
            c: result[2],
            d: result[3]
        )
    }
    
    func intoIPv4() -> IPv4Connection {
        return IPv4Connection(
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d
        )
    }
    
    func toString() -> String {
        return "\(self.a).\(self.b).\(self.c).\(self.d)"
    }
    
    static func fromString(raw: String) -> IPv4ConnectionBridge? {
        let parts = raw.split(separator: ".").compactMap { UInt8($0) };
        guard parts.count == 4 else { return nil }
        
        return IPv4ConnectionBridge(a: parts[0], b: parts[1], c: parts[2], d: parts[3])
    }
}
