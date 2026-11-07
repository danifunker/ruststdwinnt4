//! `getaddrinfo` and `freeaddrinfo` (ws2_32 of Windows XP) on NT 4.0's Winsock 2:
//! IPv4 only, from `inet_addr` (a dotted address) or `gethostbyname` (a name). std uses
//! them to resolve a host name; it sets the port itself, so a service name is not
//! looked up (a number is taken).

use core::ffi::c_void;
use core::ptr;

use crate::sys;

provide! {
    getaddrinfo = getaddrinfo, 16;
    freeaddrinfo = freeaddrinfo, 4;
}

#[repr(C)]
struct AddrInfo {
    flags: i32,
    family: i32,
    socktype: i32,
    protocol: i32,
    addrlen: usize,
    canonname: *mut u8,
    addr: *mut SockaddrIn,
    next: *mut AddrInfo,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SockaddrIn {
    family: u16,
    port: u16, // network order
    addr: u32, // network order
    zero: [u8; 8],
}

#[repr(C)]
struct HostEnt {
    name: *mut u8,
    aliases: *mut *mut u8,
    addrtype: i16,
    length: i16,
    addr_list: *mut *mut u8,
}

const AF_INET: i32 = 2;
const AF_UNSPEC: i32 = 0;
const INADDR_NONE: u32 = 0xFFFF_FFFF;
const WSAHOST_NOT_FOUND: i32 = 11001;
const WSAEAFNOSUPPORT: i32 = 10047;
const WSA_NOT_ENOUGH_MEMORY: i32 = 8;

#[link(name = "ws2_32")]
extern "system" {
    fn inet_addr(cp: *const u8) -> u32;
    fn gethostbyname(name: *const u8) -> *mut HostEnt;
    fn WSAGetLastError() -> i32;
    fn WSASetLastError(e: i32);
}

/// The port of a numeric service string, in network order (0 without one).
unsafe fn port(service: *const u8) -> u16 {
    if service.is_null() {
        return 0;
    }
    let mut p = 0u32;
    let mut i = 0;
    // SAFETY: the caller's NUL-terminated string.
    unsafe {
        while let c @ b'0'..=b'9' = *service.add(i) {
            p = p * 10 + (c - b'0') as u32;
            i += 1;
        }
    }
    (p as u16).to_be()
}

unsafe extern "system" fn getaddrinfo(
    node: *const u8,
    service: *const u8,
    hints: *const AddrInfo,
    result: *mut *mut AddrInfo,
) -> i32 {
    // SAFETY (whole body): the caller's strings and structures; entries made here.
    unsafe {
        *result = ptr::null_mut();
        let (socktype, protocol) = if hints.is_null() {
            (0, 0)
        } else {
            let h = &*hints;
            if h.family != AF_INET && h.family != AF_UNSPEC {
                WSASetLastError(WSAEAFNOSUPPORT);
                return WSAEAFNOSUPPORT;
            }
            (h.socktype, h.protocol)
        };
        let port = port(service);

        // The addresses, in network order.
        let mut addrs = [0u32; 16];
        let mut count = 0;
        if node.is_null() {
            addrs[0] = 0x0100_007Fu32; // 127.0.0.1, as stored
            count = 1;
        } else {
            let a = inet_addr(node);
            if a != INADDR_NONE {
                addrs[0] = a;
                count = 1;
            } else {
                let h = gethostbyname(node);
                if h.is_null() || (*h).addrtype as i32 != AF_INET || (*h).length != 4 {
                    let e = if h.is_null() { WSAGetLastError() } else { WSAHOST_NOT_FOUND };
                    let e = if e == 0 { WSAHOST_NOT_FOUND } else { e };
                    WSASetLastError(e);
                    return e;
                }
                let mut l = (*h).addr_list;
                while !(*l).is_null() && count < addrs.len() {
                    addrs[count] = ptr::read_unaligned(*l as *const u32);
                    count += 1;
                    l = l.add(1);
                }
            }
        }

        // Each entry is one block: the ADDRINFO, then its SOCKADDR_IN.
        let heap = sys::GetProcessHeap();
        let mut head: *mut AddrInfo = ptr::null_mut();
        for &a in addrs[..count].iter().rev() {
            let block = sys::HeapAlloc(heap, 0, core::mem::size_of::<AddrInfo>() + core::mem::size_of::<SockaddrIn>());
            if block.is_null() {
                freeaddrinfo(head);
                WSASetLastError(WSA_NOT_ENOUGH_MEMORY);
                return WSA_NOT_ENOUGH_MEMORY;
            }
            let info = block as *mut AddrInfo;
            let sa = (block as *mut u8).add(core::mem::size_of::<AddrInfo>()) as *mut SockaddrIn;
            sa.write(SockaddrIn { family: AF_INET as u16, port, addr: a, zero: [0; 8] });
            info.write(AddrInfo {
                flags: 0,
                family: AF_INET,
                socktype,
                protocol,
                addrlen: core::mem::size_of::<SockaddrIn>(),
                canonname: ptr::null_mut(),
                addr: sa,
                next: head,
            });
            head = info;
        }
        *result = head;
        0
    }
}

unsafe extern "system" fn freeaddrinfo(mut info: *mut AddrInfo) {
    // SAFETY: entries made by getaddrinfo, one heap block each.
    unsafe {
        let heap = sys::GetProcessHeap();
        while !info.is_null() {
            let next = (*info).next;
            sys::HeapFree(heap, 0, info as *mut c_void);
            info = next;
        }
    }
}
