//
//  Connection.h
//  Regis
//
//  Created by Hollan Sellars on 3/19/25.
//

#ifndef Connection_h
#define Connection_h

struct IPv4Connection {
    unsigned char a;
    unsigned char b;
    unsigned char c;
    unsigned char d;
};

struct IPv6Connection {
    char a[4];
    char b[4];
    char c[4];
    char d[4];
    
    char e[4];
    char f[4];
    char g[4];
    char h[4];
};

#endif /* Connection_h */
