//! Morceaux réseau plus gros (piste 3 de `analyses/passage_de_tour_reseau/README.md`), et trace
//! des envois et réceptions Steam.
//!
//! **Release (depuis la 0.15.0).** Morceaux de 16 384 octets (`DEFAULT_CHUNK`), sans réglage
//! joueur : tampon de réception agrandi au lancement sur chaque machine, gros morceaux en partie
//! seulement, entre machines qui annoncent toutes la même version de BGE (voir plus bas). Une
//! machine dont le tampon n'a pas pu être agrandi annonce `<version>-t0` (`announced_suffix`) :
//! elle n'est jamais comptée comme équipée, et les correctifs POL se coupent aussi avec elle.
//!
//! **Builds de diagnostic (feature `diag`)** : deux réglages de l'onglet, section Diagnostic, lus au
//! lancement du jeu. `trace_steam` n'existe que là ; `morceaux_reseau` (16384 par défaut, `off`
//! pour la taille du jeu) permet d'essayer d'autres tailles.
//!
//! **`trace_steam=on` : trace des envois et réceptions Steam.** Aucune adresse de l'exe.
//! - L'exe obtient `SteamNetworking005` par son import `SteamInternal_FindOrCreateUserInterface`
//!   (steam_api64). Son entrée dans la table des imports est redirigée. Quand l'interface demandée
//!   est `SteamNetworking005`, `SendP2PPacket` (+0x00) et `ReadP2PPacket` (+0x10) sont détournés
//!   dans sa table virtuelle, dans le client Steam. `GetP2PSessionState` (+0x30) sert à lire la file
//!   d'envoi de Steam. Places vérifiées dans l'exe : `call [rax+8]` (`IsP2PPacketAvailable`) et
//!   `call [rax+10h]` (`ReadP2PPacket`) dans `FSteamTransportThread::Update`.
//! - Toutes les 500 ms, pour chaque pair actif : paquets et octets envoyés et reçus, envois refusés
//!   par Steam (le jeu ignore ce retour), plus grand paquet, paquet reçu trop grand pour le tampon
//!   du jeu, et l'état de la session Steam : octets et paquets en attente d'envoi, relais ou direct.
//! - Fichier : `%LOCALAPPDATA%\Civ6RolloverProxy\steam_debit-<pid>.log`, hors des rapports de bug.
//!   Un pair y est désigné par un numéro et l'identifiant de compte de son SteamID (32 bits bas).
//! - **Lecture.** Pendant un gros envoi (sauvegarde, contrôle), la file d'attente se vide à un
//!   rythme mesurable. Avec des morceaux plus gros (réglage suivant), un débit en octets inchangé
//!   veut dire que Steam limite en octets (fenêtre ÷ ping) : la piste ne rapporte rien. Un débit qui
//!   grandit avec la taille des paquets veut dire qu'il limite en messages.
//!
//! **`morceaux_reseau=<octets>` : morceaux plus gros.** Un receveur au tampon d'origine
//! (2 048 octets) plante sur un morceau plus gros : un joueur sans BGE qui rejoignait un hôte réglé
//! à 60 000 plantait dans le salon (2026-11-23). Seul l'envoyeur compte les morceaux
//! (`1407f7ae0`), le receveur n'a besoin que d'un tampon assez grand.
//! - **Au lancement** : seul le tampon de réception est agrandi (voir plus bas), sans risque : il
//!   reçoit aussi bien les morceaux d'origine. Les envois gardent 1 166 octets. Il l'est sur toute
//!   machine, même avec `morceaux_reseau=off` (diagnostic) : elle reçoit les gros morceaux des
//!   autres.
//! - **En partie seulement** (`partie.lua`, native `Civ6Proxy_MorceauxReseau(ids)`) : quand les
//!   correctifs POL sont actifs (verdict de l'hôte) et que chaque joueur connecté annonce la même
//!   version de BGE, l'interface donne leurs SteamID. Les morceaux passent alors à la valeur
//!   choisie. Les builds `diag` annoncent `<version>-diag` (`salon::version_annoncee`) : une machine
//!   d'une autre version, peut-être au tampon d'origine, n'est jamais comptée.
//! - **Retour aux 1 166 octets** : liste vide (salon, correctifs POL coupés, joueur non équipé), ou
//!   premier paquet Steam, envoyé ou reçu, d'un pair hors de la liste. Un nouveau venu se manifeste
//!   avant que l'hôte lui envoie quoi que ce soit : le retour précède toute réponse. Un pair hors
//!   liste vu dans les 30 dernières secondes empêche aussi le passage aux gros morceaux.
//! - La taille des morceaux est la globale `14128f328` (DX12 .58). Pour l'appliquer :
//!   - le réglage de session `1407f7ab0` (plafond 1 166) saute directement à son écriture finale,
//!     dont la valeur devient celle choisie ;
//!   - la remise à la valeur par défaut `1407f7780` écrit la valeur choisie ;
//!   - la globale elle-même est écrite.
//!   Le retour remet les octets d'origine et 1 166 dans la globale.
//! - Le tampon de réception Steam de 0x800 octets (`14238e040`, une seule référence : le `lea` de
//!   `14085dc4c` dans `FSteamTransportThread::Update`) est remplacé par un tampon de 128 Ko proche de
//!   l'exe, et la taille passée à `ReadP2PPacket` (`mov r8d,800h` en `14085dc67`) devient 128 Ko.
//! - **Adresses fixes, relevées sur 1.0.12.58 et 1.0.12.74** (table `EXES`, DX12 et DX11). Chaque site est vérifié octet par octet avant d'écrire : sur une
//!   autre version, rien n'est posé et le journal le dit. Pour une nouvelle version, le test
//!   `chunk_sites_by_pattern` cherche les mêmes motifs (`SETTER`, `RESET`, `RECV`) dans chaque exe
//!   présent et affiche les adresses à recopier dans `EXES`. En jeu, la trace (premier réglage)
//!   n'en dépend pas.

use crate::{exe, log, win};
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::io::Write;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[path = "debit_regle.rs"]
mod regle;
pub use regle::{decide, DEFAULT_CHUNK};
use regle::{GAME_CHUNK, RECV_BUFFER, STRANGER_MEMORY};

// ---------------------------------------------------------------------------------------------
// Réglages

#[derive(Debug, PartialEq)]
pub struct Config {
    pub trace: bool,
    /// Taille des morceaux, ou `None` pour celle du jeu.
    pub chunk: Option<u32>,
}

/// Bornes de `morceaux_reseau` : au-dessus de la taille du jeu, et dans le tampon de réception.
pub const CHUNK_MIN: u32 = 1167;
pub const CHUNK_MAX: u32 = 0x1_0000;

pub fn parse(cfg: &str) -> Config {
    // Chaque clé est écrite en clair dans son appel à strip_prefix : le test
    // `cles_communes_avec_le_proxy` (options.rs) les y lit pour les comparer à celles de l'onglet.
    let trace = cfg.lines().filter_map(|l| l.trim().strip_prefix("trace_steam=")).last().map(str::trim) == Some("on");
    let chunk = match cfg.lines().filter_map(|l| l.trim().strip_prefix("morceaux_reseau=")).last() {
        None => Some(DEFAULT_CHUNK),
        Some(v) => v.trim().parse::<u32>().ok().filter(|n| (CHUNK_MIN..=CHUNK_MAX).contains(n)),
    };
    Config { trace, chunk }
}

// ---------------------------------------------------------------------------------------------
// Trace

/// Places dans la table virtuelle de `ISteamNetworking` (version 005).
const SLOT_SEND: usize = 0;
const SLOT_READ: usize = 2;
const SLOT_STATE: usize = 6;
const INTERFACE: &[u8] = b"SteamNetworking005";
const REPORT_MS: u128 = 500;
/// Un pair reste dans le bilan tant qu'il a eu de l'activité dans ces dernières millisecondes, pour
/// voir sa file d'envoi se vider.
const KEEP_MS: u128 = 5000;

type FindFn = unsafe extern "C" fn(i32, *const c_char) -> *mut c_void;
type SendFn = unsafe extern "system" fn(*mut c_void, u64, *const c_void, u32, i32, i32) -> u8;
type ReadFn = unsafe extern "system" fn(*mut c_void, *mut c_void, u32, *mut u32, *mut u64, i32) -> u8;
type StateFn = unsafe extern "system" fn(*mut c_void, u64, *mut SessionState) -> u8;

/// `P2PSessionState_t`.
#[repr(C)]
#[derive(Default)]
struct SessionState {
    active: u8,
    connecting: u8,
    error: u8,
    relay: u8,
    bytes_queued: i32,
    packets_queued: i32,
    remote_ip: u32,
    remote_port: u16,
}

static ORIG_FIND: AtomicUsize = AtomicUsize::new(0);
static ORIG_SEND: AtomicUsize = AtomicUsize::new(0);
static ORIG_READ: AtomicUsize = AtomicUsize::new(0);
static STATE: AtomicUsize = AtomicUsize::new(0);

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub sent: u32,
    pub sent_bytes: u64,
    pub refused: u32,
    pub largest_sent: u32,
    pub received: u32,
    pub received_bytes: u64,
    pub largest_received: u32,
    /// Paquets annoncés plus grands que le tampon du jeu.
    pub too_big: u32,
}

impl Window {
    fn idle(&self) -> bool {
        self.sent == 0 && self.received == 0 && self.refused == 0 && self.too_big == 0
    }
}

struct Peer {
    number: usize,
    window: Window,
    last_active: u128,
}

struct Trace {
    start: Instant,
    last_report: u128,
    peers: HashMap<u64, Peer>,
    file: Option<std::fs::File>,
}

static TRACE: Mutex<Option<Trace>> = Mutex::new(None);

fn open_file() -> Option<std::fs::File> {
    let dir = std::path::Path::new(&std::env::var_os("LOCALAPPDATA")?).join("Civ6RolloverProxy");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("steam_debit-{}.log", std::process::id()));
    let f = std::fs::OpenOptions::new().create(true).append(true).open(&path).ok()?;
    log(&format!("debit steam : trace dans {}", path.display()));
    Some(f)
}

/// Une ligne du bilan d'un pair.
pub fn line(ms: u128, number: usize, w: &Window, queue: Option<(i32, i32, bool, bool)>) -> String {
    let queue = match queue {
        Some((bytes, packets, relay, connecting)) => {
            format!(" queued={bytes}B/{packets}p {}{}", if relay { "relay" } else { "direct" }, if connecting { " connecting" } else { "" })
        }
        None => " queued=?".to_string(),
    };
    format!(
        "{ms} peer{number} sent={}p/{}B max={} refused={} recv={}p/{}B max={} too_big={}{queue}",
        w.sent, w.sent_bytes, w.largest_sent, w.refused, w.received, w.received_bytes, w.largest_received, w.too_big
    )
}

/// Note un paquet ; toutes les 500 ms, écrit le bilan de chaque pair actif.
unsafe fn record(this: *mut c_void, peer: u64, f: impl FnOnce(&mut Window)) {
    let mut g = TRACE.lock().unwrap_or_else(|e| e.into_inner());
    let t = g.get_or_insert_with(|| Trace { start: Instant::now(), last_report: 0, peers: HashMap::new(), file: open_file() });
    let now = t.start.elapsed().as_millis();
    let n = t.peers.len();
    let p = t.peers.entry(peer).or_insert_with(|| Peer { number: n, window: Window::default(), last_active: now });
    if p.number == n {
        let first = format!("{now} peer{n} = account {}", peer & 0xffff_ffff);
        if let Some(file) = t.file.as_mut() {
            let _ = writeln!(file, "{first}");
        }
    }
    f(&mut p.window);
    p.last_active = now;
    if now - t.last_report < REPORT_MS {
        return;
    }
    t.last_report = now;
    let state = STATE.load(Ordering::Relaxed);
    let mut out = String::new();
    for (&id, p) in t.peers.iter_mut() {
        if p.window.idle() && now - p.last_active > KEEP_MS {
            continue;
        }
        let queue = (state != 0).then(|| {
            let get: StateFn = std::mem::transmute(state);
            let mut s = SessionState::default();
            (get(this, id, &mut s) != 0).then_some((s.bytes_queued, s.packets_queued, s.relay != 0, s.connecting != 0))
        });
        out += &line(now, p.number, &p.window, queue.flatten());
        out.push('\n');
        p.window = Window::default();
    }
    if let Some(file) = t.file.as_mut() {
        let _ = file.write_all(out.as_bytes());
    }
}

unsafe extern "system" fn send_hook(this: *mut c_void, peer: u64, data: *const c_void, len: u32, kind: i32, channel: i32) -> u8 {
    gate_peer(peer);
    let original: SendFn = std::mem::transmute(ORIG_SEND.load(Ordering::Relaxed));
    let ok = original(this, peer, data, len, kind, channel);
    if !TRACE_ON.load(Ordering::Relaxed) {
        return ok;
    }
    record(this, peer, |w| {
        if ok != 0 {
            w.sent += 1;
            w.sent_bytes += len as u64;
            w.largest_sent = w.largest_sent.max(len);
        } else {
            w.refused += 1;
        }
    });
    ok
}

unsafe extern "system" fn read_hook(this: *mut c_void, dest: *mut c_void, cap: u32, size: *mut u32, peer: *mut u64, channel: i32) -> u8 {
    let original: ReadFn = std::mem::transmute(ORIG_READ.load(Ordering::Relaxed));
    let ok = original(this, dest, cap, size, peer, channel);
    if ok != 0 && !size.is_null() && !peer.is_null() {
        let (n, id) = (*size, *peer);
        // Avant que le jeu ne lise le paquet : il ne répond à un nouveau venu qu'après.
        gate_peer(id);
        if !TRACE_ON.load(Ordering::Relaxed) {
            return ok;
        }
        record(this, id, |w| {
            w.received += 1;
            w.received_bytes += n as u64;
            w.largest_received = w.largest_received.max(n);
            if n > cap {
                w.too_big += 1;
            }
        });
    }
    ok
}

/// Redirige `SendP2PPacket` et `ReadP2PPacket` dans la table virtuelle de l'interface, une fois.
unsafe fn hook_interface(iface: *mut c_void) {
    let vtable = *(iface as *const usize);
    if vtable == 0 {
        return;
    }
    let send = vtable + 8 * SLOT_SEND;
    let read = vtable + 8 * SLOT_READ;
    if exe::rd::<usize>(send) == send_hook as SendFn as usize {
        return;
    }
    ORIG_SEND.store(exe::rd::<usize>(send), Ordering::Relaxed);
    ORIG_READ.store(exe::rd::<usize>(read), Ordering::Relaxed);
    STATE.store(exe::rd::<usize>(vtable + 8 * SLOT_STATE), Ordering::Relaxed);
    if exe::write_slot(send, send_hook as SendFn as usize) && exe::write_slot(read, read_hook as ReadFn as usize) {
        HOOKED.store(true, Ordering::Relaxed);
        log("debit steam : SendP2PPacket et ReadP2PPacket détournés");
    } else {
        log("debit steam : table de SteamNetworking005 non modifiable, ni trace ni gros morceaux");
    }
}

unsafe extern "C" fn find_hook(user: i32, version: *const c_char) -> *mut c_void {
    let original: FindFn = std::mem::transmute(ORIG_FIND.load(Ordering::Relaxed));
    let iface = original(user, version);
    if !iface.is_null() && !version.is_null() && CStr::from_ptr(version).to_bytes() == INTERFACE {
        hook_interface(iface);
    }
    iface
}

/// Détourne l'obtention de `SteamNetworking005` : trace, et contrôle des pairs des gros morceaux.
unsafe fn install_hooks(module: usize) {
    let Some(slot) = exe::iat_slot(module, "steam_api64.dll", "SteamInternal_FindOrCreateUserInterface") else {
        log("debit steam : import SteamInternal_FindOrCreateUserInterface introuvable, ni trace ni gros morceaux");
        return;
    };
    ORIG_FIND.store(exe::rd::<usize>(slot), Ordering::Relaxed);
    if exe::write_slot(slot, find_hook as FindFn as usize) {
        log("debit steam : en attente de SteamNetworking005");
    }
}

// ---------------------------------------------------------------------------------------------
// Essai de morceaux plus gros (adresses de 1.0.12.58 et 1.0.12.74)

const BASE: u64 = 0x1_4000_0000;

struct Exe {
    name: &'static str,
    /// Réglage de session : plafond à 1 166, puis écriture de la globale.
    setter: u64,
    /// Remise à 1 166 : `mov dword [rip+globale],48Eh ; ret`.
    reset: u64,
    global: u64,
    /// `lea rbx,[rip+tampon]` de `FSteamTransportThread::Update` ; `mov r8d,800h` 27 octets plus loin.
    recv_lea: u64,
}

const EXES: [Exe; 4] = [
    Exe { name: "DX12 1.0.12.74", setter: 0x1_407f_d2e0, reset: 0x1_407f_cfb0, global: 0x1_4129_4328, recv_lea: 0x1_4086_45ac },
    Exe { name: "DX11 1.0.12.74", setter: 0x1_407f_c1e0, reset: 0x1_407f_beb0, global: 0x1_4128_2328, recv_lea: 0x1_4086_1f9c },
    Exe { name: "DX12 1.0.12.58", setter: 0x1_407f_7ab0, reset: 0x1_407f_7780, global: 0x1_4128_f328, recv_lea: 0x1_4085_dc4c },
    Exe { name: "DX11 1.0.12.58", setter: 0x1_407f_7330, reset: 0x1_407f_7000, global: 0x1_4127_c328, recv_lea: 0x1_4085_d96c },
];

/// `lea eax,[rcx-1] ; cmp eax,48Ch ; ja +11h ; mov eax,10h ; cmp ecx,eax ; cmovb ecx,eax ;
/// mov [rip+g],ecx ; ret ; mov dword [rip+g],48Eh ; ret`.
const SETTER: &str = "8D 41 FF 3D 8C 04 00 00 77 11 B8 10 00 00 00 3B C8 0F 42 C8 89 0D ?? ?? ?? ?? C3 C7 05 ?? ?? ?? ?? 8E 04 00 00 C3";
/// Écriture finale du réglage, depuis son début, et place de sa valeur.
const SETTER_STORE: u64 = 0x1b;
const SETTER_VALUE: u64 = 0x21;
/// `jmp short +19h` : du début du réglage à son écriture finale.
const SETTER_JUMP: [u8; 2] = [0xeb, 0x19];
const RESET: &str = "C7 05 ?? ?? ?? ?? 8E 04 00 00 C3";
const RESET_VALUE: u64 = 6;
/// `lea rbx,[rip+tampon] ; mov [rsp+?],r13b ; lea r9,[rbp+?] ; and dword [rsp+?],0FFF00000h ;
/// mov r8d,800h`.
const RECV: &str = "48 8D 1D ?? ?? ?? ?? 44 88 6C 24 ?? 4C 8D 8D ?? ?? ?? ?? 81 64 24 ?? 00 00 F0 FF 41 B8 00 08 00 00";
const RECV_SIZE: u64 = 29;

fn matches(pattern: &str, bytes: &[u8]) -> bool {
    let p = crate::pe::pattern(pattern);
    bytes.len() >= p.len() && p.iter().zip(bytes).all(|(p, b)| p.is_none_or(|p| p == *b))
}

/// Cible d'un adressage `[rip+disp32]` dont le déplacement est en `at + disp` et l'instruction
/// finit en `at + end`.
fn rip_target(read: &dyn Fn(u64, usize) -> Vec<u8>, at: u64, disp: u64, end: u64) -> Option<u64> {
    let b = read(at + disp, 4);
    Some((at + end).wrapping_add_signed(i32::from_le_bytes(b.try_into().ok()?) as i64))
}

/// Vérifie chaque site de l'entrée ; `read(adresse, n)` rend n octets.
fn fingerprint(exe: &Exe, read: &dyn Fn(u64, usize) -> Vec<u8>) -> Result<(), String> {
    if !matches(SETTER, &read(exe.setter, 38)) {
        return Err(format!("chunk size setter {:#x}: unexpected bytes", exe.setter));
    }
    for (what, at, disp, end) in [("setter", exe.setter, 22, 26), ("setter store", exe.setter + SETTER_STORE, 2, 10), ("reset", exe.reset, 2, 10)] {
        if rip_target(read, at, disp, end) != Some(exe.global) {
            return Err(format!("{what} {at:#x} does not write {:#x}", exe.global));
        }
    }
    if !matches(RESET, &read(exe.reset, 11)) {
        return Err(format!("chunk size reset {:#x}: unexpected bytes", exe.reset));
    }
    if !matches(RECV, &read(exe.recv_lea, 33)) {
        return Err(format!("receive buffer {:#x}: unexpected bytes", exe.recv_lea));
    }
    Ok(())
}

unsafe fn write(at: usize, bytes: &[u8]) -> bool {
    let mut old = 0u32;
    if win::VirtualProtect(at as *mut c_void, bytes.len(), win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        return false;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), at as *mut u8, bytes.len());
    win::VirtualProtect(at as *mut c_void, bytes.len(), old, &mut old);
    win::FlushInstructionCache(win::GetCurrentProcess(), at as *const c_void, bytes.len());
    true
}

/// Tampon de `size` octets à moins de 2 Go de `near`.
unsafe fn buffer_near(near: usize, size: usize) -> Option<usize> {
    const STEP: usize = 0x1_0000;
    for i in 1..(0x7000_0000 / STEP) {
        for at in [near.wrapping_sub(i * STEP), near.wrapping_add(i * STEP)] {
            let p = win::VirtualAlloc((at & !(STEP - 1)) as *mut c_void, size, win::MEM_COMMIT | win::MEM_RESERVE, 0x04);
            if !p.is_null() {
                return Some(p as usize);
            }
        }
    }
    None
}

/// Tampon de réception agrandi ; `chunk` : taille envoyée en partie, 0 pour celle du jeu.
unsafe fn install_chunks(module: usize, chunk: u32) {
    let image = |a: u64| module + (a - BASE) as usize;
    let read = |a: u64, n: usize| std::slice::from_raw_parts(image(a) as *const u8, n).to_vec();
    let mut reasons = Vec::new();
    let Some(exe) = EXES.iter().find(|e| {
        crate::exe::for_this_game(e.name) && fingerprint(e, &read).map_err(|r| reasons.push(format!("{}: {r}", e.name))).is_ok()
    }) else {
        log(&format!(
            "debit steam : essai de morceaux non posé ({}) ; adresses relevées sur 1.0.12.58 et 1.0.12.74",
            crate::exe::no_table(&reasons)
        ));
        return;
    };
    let lea = image(exe.recv_lea);
    let Some(buffer) = buffer_near(lea, RECV_BUFFER as usize) else {
        log("debit steam : pas de tampon à moins de 2 Go de l'exe, essai de morceaux non posé");
        return;
    };
    let Ok(disp) = i32::try_from(buffer as i64 - (lea as i64 + 7)) else {
        log("debit steam : tampon hors de portée, essai de morceaux non posé");
        return;
    };
    // Le tampon seul au lancement : les envois gardent la taille du jeu jusqu'à `set_peers`.
    if !(write(lea + 3, &disp.to_le_bytes()) && write(lea + RECV_SIZE as usize, &RECV_BUFFER.to_le_bytes())) {
        log("debit steam : écriture refusée, tampon de réception d'origine, pas de gros morceaux");
        return;
    }
    BUFFER_OK.store(true, Ordering::Relaxed);
    let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
    g.sites = Some(Sites {
        setter: image(exe.setter),
        setter_value: image(exe.setter + SETTER_VALUE),
        reset_value: image(exe.reset + RESET_VALUE),
        global: image(exe.global),
        setter_head: [*(image(exe.setter) as *const u8), *((image(exe.setter) + 1) as *const u8)],
    });
    g.chunk = chunk;
    log(&format!(
        "debit steam : tampon de réception de {RECV_BUFFER} octets ({}) ; {}",
        exe.name,
        if chunk == 0 {
            "morceaux du jeu (morceaux_reseau=off)".to_string()
        } else {
            format!("morceaux de {chunk} octets en partie seulement, quand chaque joueur annonce la même version de BGE")
        }
    ));
}

// ---------------------------------------------------------------------------------------------
// Gros morceaux en partie, entre machines équipées


/// Adresses absolues des écritures.
struct Sites {
    setter: usize,
    setter_value: usize,
    reset_value: usize,
    global: usize,
    /// Deux premiers octets d'origine du réglage de session (remplacés par le saut).
    setter_head: [u8; 2],
}

struct Gate {
    sites: Option<Sites>,
    chunk: u32,
    active: bool,
    /// SteamID des pairs annoncés par l'interface (`set_peers`).
    approved: BTreeSet<u64>,
    /// Pairs hors liste, et quand ils ont été vus pour la dernière fois.
    strangers: BTreeMap<u64, Instant>,
}

static GATE: Mutex<Gate> = Mutex::new(Gate {
    sites: None,
    chunk: 0,
    active: false,
    approved: BTreeSet::new(),
    strangers: BTreeMap::new(),
});
static TRACE_ON: AtomicBool = AtomicBool::new(false);
/// `morceaux_reseau` lu au lancement (0 : désactivé), pour la version annoncée.
static CONFIGURED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Tampon de réception agrandi.
static BUFFER_OK: AtomicBool = AtomicBool::new(false);

/// Suffixe de la version annoncée : `-t0` si le tampon de réception n'a pas pu être agrandi (cette
/// machine planterait sur les gros morceaux des autres) ; sinon rien en release, et `-m<taille>`
/// avec `morceaux_reseau` dans un build de diagnostic.
pub fn announced_suffix() -> String {
    regle::suffix(BUFFER_OK.load(Ordering::Relaxed), CONFIGURED.load(Ordering::Relaxed), cfg!(feature = "diag"))
}
static HOOKED: AtomicBool = AtomicBool::new(false);

/// Écrit la taille choisie (`on`) ou celle du jeu, et l'état d'origine du réglage de session.
unsafe fn apply(g: &mut Gate, on: bool, why: &str) {
    let Some(s) = g.sites.as_ref() else { return };
    let on = on && g.chunk != 0;
    if g.active == on {
        return;
    }
    let size = if on { g.chunk } else { GAME_CHUNK };
    let head = if on { SETTER_JUMP } else { s.setter_head };
    let ok = write(s.reset_value, &size.to_le_bytes())
        && write(s.setter_value, &size.to_le_bytes())
        && write(s.setter, &head)
        && write(s.global, &size.to_le_bytes());
    g.active = on && ok;
    log(&format!("debit steam : morceaux de {size} octets ({why}){}", if ok { "" } else { ", écriture refusée" }));
}

/// Chaque paquet Steam, envoyé ou reçu : un pair hors liste ramène aussitôt la taille du jeu.
unsafe fn gate_peer(peer: u64) {
    let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
    if g.sites.is_none() || g.approved.contains(&peer) {
        return;
    }
    g.strangers.insert(peer, Instant::now());
    if g.active {
        apply(&mut g, false, &format!("pair {} hors de la liste des joueurs équipés", peer & 0xffff_ffff));
    }
}

/// `Civ6Proxy_MorceauxReseau(ids)` (partie.lua) : les SteamID des autres joueurs connectés, tous
/// de la même version de BGE, ou une chaîne vide.
pub unsafe extern "C-unwind" fn lua_set_peers(l: *mut c_void) -> i32 {
    let ids = crate::salon::arg_str(l, 1).unwrap_or_default();
    let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
    if g.sites.is_none() {
        return 0;
    }
    let (mut approved, mut strangers) = (std::mem::take(&mut g.approved), std::mem::take(&mut g.strangers));
    let verdict = decide(&ids, crate::salon::pol_actifs(), crate::salon::partie_reseau(), HOOKED.load(Ordering::Relaxed), &mut approved, &mut strangers, Instant::now());
    g.approved = approved;
    g.strangers = strangers;
    match verdict {
        Ok(()) => {
            let why = format!("{} joueur(s) équipé(s)", g.approved.len());
            apply(&mut g, true, &why)
        }
        Err(why) => apply(&mut g, false, &why),
    }
    0
}

/// Correctifs POL coupés (salon ouvert, verdict négatif) : taille du jeu.
pub fn pol_off() {
    let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
    g.approved.clear();
    unsafe { apply(&mut g, false, "correctifs POL coupés") };
}

/// Appelé par DllMain. La release ne lit aucun réglage : morceaux de `DEFAULT_CHUNK`, sans trace.
pub unsafe fn install() {
    let text = if cfg!(feature = "diag") { crate::options::read_config().map(|(t, _)| t).unwrap_or_default() } else { String::new() };
    let cfg = parse(&text);
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    TRACE_ON.store(cfg.trace, Ordering::Relaxed);
    CONFIGURED.store(cfg.chunk.unwrap_or(0), Ordering::Relaxed);
    // Toujours : le contrôle des pairs (retour aux morceaux du jeu) en dépend.
    install_hooks(module);
    install_chunks(module, cfg.chunk.unwrap_or(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings() {
        assert_eq!(parse(""), Config { trace: false, chunk: Some(DEFAULT_CHUNK) }, "la release passe un texte vide");
        assert_eq!(parse("trace_steam=on\nmorceaux_reseau=16384\n"), Config { trace: true, chunk: Some(16384) });
        assert_eq!(parse("morceaux_reseau=off").chunk, None);
        assert_eq!(parse("morceaux_reseau=1166").chunk, None, "pas en dessous de la taille du jeu");
        assert_eq!(parse("morceaux_reseau=70000").chunk, None, "pas au-delà du tampon");
        assert_eq!(parse("trace_steam=on\ntrace_steam=off").trace, false, "la dernière ligne l'emporte");
    }

    /// Gros morceaux seulement avec une liste, POL actif, en réseau, et sans pair hors liste récent.
    #[test]
    fn chunk_gate() {
        let now = Instant::now();
        let (mut a, mut s) = (BTreeSet::new(), BTreeMap::new());
        assert!(decide("76561198049438351", true, true, true, &mut a, &mut s, now).is_ok());
        assert!(a.contains(&76561198049438351));
        assert!(decide("", true, true, true, &mut a, &mut s, now).is_err(), "liste vide");
        assert!(a.is_empty());
        assert!(decide("1", false, true, true, &mut a, &mut s, now).is_err(), "POL coupé");
        assert!(decide("1", true, false, true, &mut a, &mut s, now).is_err(), "partie locale");
        assert!(decide("1", true, true, false, &mut a, &mut s, now).is_err(), "Steam non détourné");
        s.insert(2, now);
        assert!(decide("1", true, true, true, &mut a, &mut s, now).is_err(), "inconnu vu à l'instant");
        assert!(decide("1,2", true, true, true, &mut a, &mut s, now).is_ok(), "devenu équipé");
        assert!(s.is_empty());
        s.insert(3, now);
        assert!(decide("1", true, true, true, &mut a, &mut s, now + STRANGER_MEMORY).is_ok(), "parti depuis 30 s");
    }

    #[test]
    fn report_line() {
        let w = Window { sent: 3, sent_bytes: 3498, largest_sent: 1167, ..Window::default() };
        assert_eq!(line(1500, 2, &w, Some((52000, 45, true, false))), "1500 peer2 sent=3p/3498B max=1167 refused=0 recv=0p/0B max=0 too_big=0 queued=52000B/45p relay");
        assert!(line(0, 0, &Window::default(), None).ends_with("queued=?"));
    }

    #[test]
    fn setter_jump_lands_on_the_final_store() {
        assert_eq!(SETTER_JUMP[1] as u64 + 2, SETTER_STORE);
        assert_eq!(crate::pe::pattern(SETTER).len(), 38);
        assert_eq!(crate::pe::pattern(RECV).len(), 33);
    }

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    const FILES: [&str; 4] = ["CivilizationVI_DX12.exe", "CivilizationVI.exe", "CivilizationVI_DX12.exe", "CivilizationVI.exe"];

    /// Les entrées de `EXES` sur leurs exes (1.0.12.74 installé ou `CIV6_EXE_74_DIR`, 1.0.12.58
    /// par `CIV6_EXE_58_DIR`), et aucune autre entrée sur chacun.
    #[test]
    fn chunk_table_on_its_exes() {
        for (i, name) in FILES.iter().enumerate() {
            let Ok(file) = std::fs::read(crate::test_exe(EXE_DIR, i, name)) else { continue };
            let img = crate::pe::Image::file(&file).unwrap();
            let read = |a: u64, n: usize| img.at((a - BASE) as u32, n).map(<[u8]>::to_vec).unwrap_or_default();
            let own = i;
            assert_eq!(fingerprint(&EXES[own], &read), Ok(()), "{}", EXES[own].name);
            for (_, other) in EXES.iter().enumerate().filter(|&(k, _)| k != own) {
                assert!(fingerprint(other, &read).is_err(), "{} matches {}", other.name, EXES[own].name);
            }
        }
    }

    /// Pour re-identifier les sites sur une autre version : les motifs dans chaque exe présent
    /// (1.0.12.74, 1.0.12.58), adresses à recopier dans `EXES`.
    #[test]
    fn chunk_sites_by_pattern() {
        for (i, name) in FILES.iter().enumerate() {
            let path = crate::test_exe(EXE_DIR, i, name);
            let Ok(file) = std::fs::read(&path) else { continue };
            let img = crate::pe::Image::file(&file).unwrap();
            let text = img.section(".text").unwrap();
            let code = img.data(&text);
            let at = |k: usize| BASE + text.rva as u64 + k as u64;
            let read = |a: u64, n: usize| img.at((a - BASE) as u32, n).map(<[u8]>::to_vec).unwrap_or_default();
            let setters: Vec<u64> = crate::pe::find_pattern(code, &crate::pe::pattern(SETTER)).into_iter().map(at).collect();
            let [setter] = setters[..] else { panic!("{path}: {} chunk size setters", setters.len()) };
            let global = rip_target(&read, setter, 22, 26).unwrap();
            let resets: Vec<u64> = crate::pe::find_pattern(code, &crate::pe::pattern(RESET))
                .into_iter()
                .map(at)
                .filter(|&a| a != setter + SETTER_STORE && rip_target(&read, a, 2, 10) == Some(global))
                .collect();
            let recvs: Vec<u64> = crate::pe::find_pattern(code, &crate::pe::pattern(RECV)).into_iter().map(at).collect();
            eprintln!("{path}: setter {setter:#x}, reset {resets:x?}, global {global:#x}, recv_lea {recvs:x?}");
            assert_eq!((resets.len(), recvs.len()), (1, 1), "{path}");
            let found = Exe { name: "", setter, reset: resets[0], global, recv_lea: recvs[0] };
            assert_eq!(fingerprint(&found, &read), Ok(()), "{path}");
        }
    }
}
