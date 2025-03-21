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
}

struct IPv6ConnectionBridge {
    let a: (CChar, CChar, CChar, CChar)
    let b: (CChar, CChar, CChar, CChar)
    let c: (CChar, CChar, CChar, CChar)
    let d: (CChar, CChar, CChar, CChar)
    
    let e: (CChar, CChar, CChar, CChar)
    let f: (CChar, CChar, CChar, CChar)
    let g: (CChar, CChar, CChar, CChar)
    let h: (CChar, CChar, CChar, CChar)
    
    static func unpack(_ data: [String]) -> IPv6ConnectionBridge? {
        guard data.count == 8 else { return nil }
        
        var result = [(CChar, CChar, CChar, CChar)](repeating: (CChar("0")!, CChar("0")!, CChar("0")!, CChar("0")!), count: 8)
        for (index, value) in data.enumerated() {
            let transformed = value.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
            
            let as_utf8 = Array(transformed.utf8);
            guard as_utf8.count == 4 else { return nil }
            
            result[index] = (CChar(as_utf8[0]), CChar(as_utf8[1]), CChar(as_utf8[2]), CChar(as_utf8[3]))
        }
        
        return self.init(
            a: result[0],
            b: result[1],
            c: result[2],
            d: result[3],
            
            e: result[4],
            f: result[5],
            g: result[6],
            h: result[7]
        )
    }
    
    func intoIPv6() -> IPv6Connection {
        IPv6Connection(
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d,
            
            e: self.e,
            f: self.f,
            g: self.g,
            h: self.h
        )
    }
}
