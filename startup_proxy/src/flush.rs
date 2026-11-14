//! Incoming queue flush after a load: a queued message is never appended to a chunked message still
//! arriving from the same connection.
//!
//! The message factory (`1407fba10`) keeps partly received chunked messages in a hash table keyed only
//! by connection ID, and appends any buffer from a connection with an open entry. The flush at the end
//! of a load (`14083f150`) hands each queued, complete message back to the factory with its connection
//! ID: when the host's snapshot was still arriving, the queued host messages were lost as snapshot
//! chunks, the snapshot came out corrupt and its last chunks were read as empty `NetGameConfig`.
//!
//! The flush's call to the factory is redirected. When the connection has an open entry, the factory
//! is called with a connection ID that has none (its "new message" path), then the message's
//! connection ID is set back. The table is only read. Receipt and the flush both run in the session
//! update, so no chunk can arrive during the call. Queued messages always have a chunk count of 0.
//!
//! Addresses of both exes of 2026-11-05, checked byte by byte at startup. `actif=off` installs
//! nothing; a pure bug fix, without its own setting.

use crate::{log, patch, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Base of the exe's Ghidra addresses.
const BASE: u64 = 0x1_4000_0000;

/// Chunk table entry: connection ID, chunks still expected, next entry of the bucket.
const ENTRY_KEY: usize = 0;
const ENTRY_LEFT: usize = 0x20;
const ENTRY_NEXT: usize = 0x28;
/// Created message: connection ID (written by the factory after reading), message type.
const MSG_CONN: usize = 0x10;
const MSG_TYPE: usize = 0x18;
/// A connection ID with no entry: the factory then builds a new message.
const NO_CONNECTION: u32 = 0x7fff_fff1;
/// Longest bucket chain walked (the table holds one entry per connection with an open message).
const MAX_CHAIN: usize = 256;

const X: Option<u8> = None;
const fn b(x: u8) -> Option<u8> {
    Some(x)
}

/// The flush, from `call+-0x20`: `lea rcx,[rsp+68h] ; mov r8d,[rdi+8] ; sub r8d,[rdi] ; mov rdx,[rdi] ;
/// call stream ; mov r8d,[rdi+24h] ; lea rcx,[rsp+68h] ; mov edx,[rdi+20h] ; call factory ;
/// mov rdx,[rbx+0e8h] ; mov rsi,rax ; mov rcx,[rbx+0e0h]`. `X`: rel32.
const FLUSH_CALL: [Option<u8>; 0x36] = [
    b(0x48), b(0x8d), b(0x4c), b(0x24), b(0x68), b(0x44), b(0x8b), b(0x47), b(0x08), b(0x44), b(0x2b), b(0x07), b(0x48), b(0x8b),
    b(0x17), b(0xe8), X, X, X, X, b(0x44), b(0x8b), b(0x47), b(0x24), b(0x48), b(0x8d), b(0x4c), b(0x24), b(0x68), b(0x8b),
    b(0x57), b(0x20), b(0xe8), X, X, X, X, b(0x48), b(0x8b), b(0x93), b(0xe8), b(0x00), b(0x00), b(0x00), b(0x48), b(0x8b),
    b(0xf0), b(0x48), b(0x8b), b(0x8b), b(0xe0), b(0x00), b(0x00), b(0x00),
];
/// Offset of the factory call in `FLUSH_CALL`.
const CALL_AT: usize = 0x20;

/// Factory head up to the bucket walk: prologue, `mov r8d,[count]`, `div`, `mov rdx,[buckets]`,
/// `cmp r13d,[rax]` (key at +0), `mov rax,[rax+28h]` (next at +0x28).
const FACTORY_HEAD: [Option<u8>; 0x72] = [
    b(0x44), b(0x89), b(0x44), b(0x24), b(0x18), b(0x89), b(0x54), b(0x24), b(0x10), b(0x55), b(0x53), b(0x56), b(0x57), b(0x41),
    b(0x54), b(0x41), b(0x55), b(0x41), b(0x56), b(0x41), b(0x57), b(0x48), b(0x8d), b(0xac), b(0x24), b(0xf8), b(0xfd), b(0xff),
    b(0xff), b(0x48), b(0x81), b(0xec), b(0x08), b(0x03), b(0x00), b(0x00), b(0x48), b(0x8b), b(0xd9), b(0x45), b(0x8b), b(0xf8),
    b(0x48), b(0x8d), b(0x8d), b(0xd0), b(0x00), b(0x00), b(0x00), b(0x44), b(0x8b), b(0xea), b(0xe8), X, X, X, X, b(0x44),
    b(0x8b), b(0x05), X, X, X, X, b(0x33), b(0xd2), b(0x41), b(0x8b), b(0xc5), b(0x41), b(0xf7), b(0xf0), b(0x8b), b(0xca),
    b(0x48), b(0x8b), b(0x15), X, X, X, X, b(0x48), b(0x8b), b(0x04), b(0xca), b(0x4c), b(0x8d), b(0x0c), b(0xca), b(0x48),
    b(0x85), b(0xc0), b(0x74), b(0x14), b(0x66), b(0x90), b(0x44), b(0x3b), b(0x28), b(0x0f), b(0x84), b(0xa7), b(0x02), b(0x00),
    b(0x00), b(0x48), b(0x8b), b(0x40), b(0x28), b(0x48), b(0x85), b(0xc0), b(0x75), b(0xee),
];
/// `mov r8d,[rip+count]` and `mov rdx,[rip+buckets]` in `FACTORY_HEAD`: displacement offsets.
const HEAD_COUNT_DISP: usize = 0x3c;
const HEAD_BUCKETS_DISP: usize = 0x4d;
/// Factory + 0x319, appending a chunk: `mov edi,[r14+20h]` (chunks left at +0x20).
const FACTORY_FOUND_AT: u64 = 0x319;
const FACTORY_FOUND: [u8; 15] = [0x41, 0x8b, 0x7e, 0x20, 0x48, 0x8b, 0xcb, 0x48, 0x8b, 0x03, 0xff, 0xcf, 0xff, 0x50, 0x10];
/// Factory + 0x46d, new message: `call [r8+18h]` (read) then `mov [r15+10h],r13d` (connection ID)
/// and `mov [r15+14h],eax` (bytes read).
const FACTORY_CREATE_AT: u64 = 0x46d;
const FACTORY_CREATE: [u8; 32] = [
    0x4c, 0x8b, 0x00, 0x48, 0x8b, 0xd3, 0x48, 0x8b, 0xc8, 0x41, 0xff, 0x50, 0x18, 0x45, 0x89, 0x6f, 0x10, 0x48, 0x8b, 0xcb, 0x48,
    0x8b, 0x13, 0xff, 0x52, 0x18, 0x2b, 0xc7, 0x41, 0x89, 0x47, 0x14,
];

struct Exe {
    name: &'static str,
    /// The flush's `call factory`.
    call: u64,
    factory: u64,
    /// Chunk table globals: bucket array pointer, bucket count.
    buckets: u64,
    count: u64,
}

const EXES: [Exe; 2] = [
    Exe { name: "DX12", call: 0x1_4083_f3f9, factory: 0x1_407f_ba10, buckets: 0x1_4129_4338, count: 0x1_4129_4340 },
    Exe { name: "DX11", call: 0x1_4083_d819, factory: 0x1_407f_9c30, buckets: 0x1_4128_2338, count: 0x1_4128_2340 },
];

type Factory = unsafe extern "C-unwind" fn(*mut c_void, u32, i32) -> *mut u8;

static MODULE: AtomicUsize = AtomicUsize::new(0);
static TABLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static FACTORY: AtomicUsize = AtomicUsize::new(0);

unsafe fn read<T: Copy>(address: usize) -> T {
    std::ptr::read_unaligned(address as *const T)
}

unsafe fn image(address: u64) -> usize {
    MODULE.load(Ordering::Relaxed) + (address - BASE) as usize
}

/// The entry of `conn` in a chunk table (`buckets`, `count`), if any.
unsafe fn find(buckets: usize, count: u32, conn: u32) -> Option<usize> {
    if buckets == 0 || count == 0 {
        return None;
    }
    let mut e: usize = read(buckets + 8 * (conn % count) as usize);
    for _ in 0..MAX_CHAIN {
        if e == 0 {
            return None;
        }
        if read::<u32>(e + ENTRY_KEY) == conn {
            return Some(e);
        }
        e = read(e + ENTRY_NEXT);
    }
    None
}

unsafe fn open_entry(t: &Exe, conn: u32) -> Option<usize> {
    find(read(image(t.buckets)), read(image(t.count)), conn)
}

/// Replaces the flush's call to the factory (same arguments: stream, connection ID, player ID).
unsafe extern "C-unwind" fn flush_factory(stream: *mut c_void, conn: u32, player: i32) -> *mut u8 {
    let factory: Factory = std::mem::transmute(FACTORY.load(Ordering::Relaxed));
    let Some(t) = EXES.get(TABLE.load(Ordering::Relaxed)) else { return factory(stream, conn, player) };
    let Some(entry) = open_entry(t, conn) else { return factory(stream, conn, player) };
    if open_entry(t, NO_CONNECTION).is_some() {
        return factory(stream, conn, player);
    }
    let left: u32 = read(entry + ENTRY_LEFT);
    let msg = factory(stream, NO_CONNECTION, player);
    if msg.is_null() {
        log(&format!("queue flush: queued message from connection {conn} unreadable ({left} chunks still expected)"));
        return msg;
    }
    std::ptr::write_unaligned(msg.add(MSG_CONN) as *mut u32, conn);
    let kind: u32 = read(msg as usize + MSG_TYPE);
    log(&format!("queue flush: message {kind:#x} from connection {conn} kept out of a chunked message ({left} chunks still expected)"));
    msg
}

/// Are this exe's bytes where the table says? `read(address, n)` returns n bytes.
fn fingerprint<'a>(t: &Exe, read: impl Fn(u64, usize) -> &'a [u8]) -> Result<(), String> {
    let same = |want: &[Option<u8>], have: &[u8]| have.len() == want.len() && want.iter().zip(have).all(|(w, h)| w.map_or(true, |w| w == *h));
    let rip = |b: &[u8], at: usize, next: u64| next.wrapping_add_signed(i64::from(i32::from_le_bytes(b[at..at + 4].try_into().unwrap())));
    let c = read(t.call - CALL_AT as u64, FLUSH_CALL.len());
    if !same(&FLUSH_CALL, c) || rip(c, CALL_AT + 1, t.call + 5) != t.factory {
        return Err(format!("queue flush: no call to the factory at {:#x}", t.call));
    }
    let h = read(t.factory, FACTORY_HEAD.len());
    if !same(&FACTORY_HEAD, h) {
        return Err(format!("factory: unexpected bytes at {:#x}", t.factory));
    }
    if rip(h, HEAD_COUNT_DISP, t.factory + HEAD_COUNT_DISP as u64 + 4) != t.count
        || rip(h, HEAD_BUCKETS_DISP, t.factory + HEAD_BUCKETS_DISP as u64 + 4) != t.buckets
    {
        return Err("factory: does not read the expected chunk table".into());
    }
    if read(t.factory + FACTORY_FOUND_AT, FACTORY_FOUND.len()) != FACTORY_FOUND
        || read(t.factory + FACTORY_CREATE_AT, FACTORY_CREATE.len()) != FACTORY_CREATE
    {
        return Err(format!("factory: unexpected chunk or message layout at {:#x}", t.factory));
    }
    Ok(())
}

/// Called by DllMain: no exe code has run yet.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("queue flush: not installed (actif=off)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let at = |address: u64| module + (address - BASE) as usize;
    let mut reasons = Vec::new();
    let Some(i) = EXES.iter().position(|t| {
        fingerprint(t, |address, n| std::slice::from_raw_parts(at(address) as *const u8, n)).map_err(|e| reasons.push(format!("{}: {e}", t.name))).is_ok()
    }) else {
        log(&format!("queue flush: unknown exe ({}), fix not installed", reasons.join("; ")));
        return;
    };
    let t = &EXES[i];
    let call = at(t.call);
    let Some(stub) = patch::alloc_near(call) else {
        log("queue flush: no memory within 2 GB of the exe, fix not installed");
        return;
    };
    let Some(rel) = patch::rel32(call, stub) else {
        log("queue flush: relay out of range, fix not installed");
        return;
    };
    // Relay: jmp [rip+0], then the hook's address.
    std::ptr::copy_nonoverlapping([0xffu8, 0x25, 0, 0, 0, 0].as_ptr(), stub as *mut u8, 6);
    std::ptr::write_unaligned((stub + 6) as *mut u64, flush_factory as Factory as usize as u64);
    MODULE.store(module, Ordering::Relaxed);
    TABLE.store(i, Ordering::Relaxed);
    FACTORY.store(at(t.factory), Ordering::Relaxed);
    patch::write_rel32(call, rel);
    log(&format!("queue flush: queued messages kept out of chunked messages ({})", t.name));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bucket array of 3 with a two-entry chain in bucket 1 (keys 4 and 7) and the -1 end marker.
    #[test]
    fn finds_entries_by_connection() {
        let mut e7 = [0u8; 0x30];
        e7[..4].copy_from_slice(&7u32.to_le_bytes());
        let mut e4 = [0u8; 0x30];
        e4[..4].copy_from_slice(&4u32.to_le_bytes());
        e4[ENTRY_NEXT..ENTRY_NEXT + 8].copy_from_slice(&(e7.as_ptr() as u64).to_le_bytes());
        let buckets: [u64; 4] = [0, e4.as_ptr() as u64, 0, u64::MAX];
        unsafe {
            let p = buckets.as_ptr() as usize;
            assert_eq!(find(p, 3, 4), Some(e4.as_ptr() as usize));
            assert_eq!(find(p, 3, 7), Some(e7.as_ptr() as usize));
            assert_eq!(find(p, 3, 1), None);
            assert_eq!(find(p, 3, 0), None);
            assert_eq!(find(p, 0, 4), None);
            assert_eq!(find(0, 3, 4), None);
        }
    }

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    const FILES: [&str; 2] = ["CivilizationVI_DX12.exe", "CivilizationVI.exe"];

    /// File offset of a virtual address, from the PE section table.
    fn offset(file: &[u8], address: u64) -> Option<usize> {
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        let rva = (address - BASE) as usize;
        (0..rd16(nt + 6)).map(|i| sections + 40 * i).find_map(|s| {
            let (va, size, raw) = (rd32(s + 12), rd32(s + 16), rd32(s + 20));
            (rva >= va && rva < va + size).then(|| rva - va + raw)
        })
    }

    /// Each table matches its own installed exe, and only that one.
    #[test]
    fn bytes_in_installed_exes() {
        for (i, name) in FILES.iter().enumerate() {
            let Ok(file) = std::fs::read(format!(r"{EXE_DIR}\{name}")) else {
                eprintln!("{name} missing, test skipped");
                continue;
            };
            let read = |address: u64, n: usize| offset(&file, address).map_or(&[][..], |o| &file[o..][..n]);
            for (j, t) in EXES.iter().enumerate() {
                let r = fingerprint(t, read);
                if i == j {
                    assert_eq!(r, Ok(()), "table {} on {name}", t.name);
                } else {
                    assert!(r.is_err(), "the {} table must not match {name}", t.name);
                }
            }
        }
    }
}
