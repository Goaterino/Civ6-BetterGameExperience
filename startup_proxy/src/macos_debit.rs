//! Morceaux réseau plus gros sous macOS, le pendant de la release de debit_steam.rs (voir son
//! en-tête ; la trace `trace_steam` et le réglage `morceaux_reseau` des builds de diagnostic ne sont
//! pas portés). Règles communes : debit_regle.rs.
//!
//! - **Au lancement**, sur toute machine : le tampon de réception Steam du jeu (0x800 octets) est
//!   remplacé par un tampon de 128 Ko proche de l'exe, et la taille passée à `ReadP2PPacket` aussi.
//!   Sans risque : il reçoit aussi bien les morceaux d'origine. Si ce n'est pas possible, la machine
//!   annonce `-t0` (`announced_suffix`) et n'est jamais comptée comme équipée.
//! - **En partie seulement** (`partie.lua`, native `Civ6Proxy_MorceauxReseau(ids)`) : morceaux de
//!   16 384 octets quand les correctifs POL sont actifs et que chaque autre joueur connecté annonce
//!   la même version (`decide`). Retour aux 1 166 octets du jeu : liste vide, correctifs POL coupés
//!   (`pol_off`), ou premier paquet Steam, envoyé ou reçu, d'un pair hors de la liste.
//! - Le contrôle des pairs passe par `SteamNetworking006`, que l'exe obtient par son import
//!   `SteamInternal_FindOrCreateUserInterface` (libsteam_api.dylib) : sa case d'import est
//!   remplacée (comme `_dlopen`, macos.rs), puis `SendP2PPacket` (+0x00) et `ReadP2PPacket` (+0x10)
//!   sont détournés dans la table virtuelle de l'interface, dans le client Steam.
//!
//! **Différence avec Windows : aucune écriture de code en partie.** Sous macOS, une page de code est
//! remplacée entière (patch_mac.rs) : on ne le fait qu'au lancement, avant que le jeu n'exécute ces
//! pages. Le réglage de session et la remise à 1 166 sautent donc dès le lancement vers `reglage` et
//! `remise`, qui écrivent la taille du jeu (calcul d'origine) tant que les gros morceaux ne sont pas
//! actifs, et 16 384 sinon. Passer d'une taille à l'autre n'écrit que la globale et un booléen.
//!
//! Relevé dans Civ6_Exe_Child (Civ6 Aspyr 1.4.6), adresses de l'exe (base `0x100000000`) ; chaque
//! site est vérifié par son empreinte (FNV-1a) avant d'écrire, sinon rien n'est posé :
//! - x86_64 : réglage de session `0x100cace00` (`lea eax,[rdi-48Eh]` … `mov [rip+g],eax ; ret`,
//!   valeurs 1..1165 gardées, au moins 16, sinon 1 166), remise `0x100cace30`
//!   (`mov dword [rip+g],48Eh ; ret`), globale `0x101cd5c50` (aussi lue par `0x100caca30` et les
//!   deux divisions du compte des morceaux, `0x100cac6e6` et `0x100cac732`). Réception dans
//!   `FSteamTransportThread::Update` : `lea r14,[rip+tampon]` en `0x100cd3ad6` (tampon
//!   `0x1029737a0`, sa seule référence), `mov edx,800h` en `0x100cd3aee`, `call [rax+10h]`
//!   (`ReadP2PPacket`) en `0x100cd3af6`.
//! - arm64 : réglage `0x100bfe800`, remise `0x100bfe828`, globale `0x101ba1748`. Réception :
//!   `adrp x20` / `add x20` en `0x100c218f0` (tampon `0x10f08aef4`), `mov w2,#0x800` en
//!   `0x100c21904`, `blr x8` (`[x8+0x10]`, `ReadP2PPacket`) en `0x100c2190c`.

use super::journal::log;
use super::macho;
use super::options::{argument_n, LuaCFn};
use super::patch;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[path = "debit_regle.rs"]
mod regle;
use regle::{decide, DEFAULT_CHUNK, GAME_CHUNK, RECV_BUFFER};

/// Base des adresses de l'exe.
const BASE_EXE: usize = 0x1_0000_0000;

struct Site {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
}

const X86_REGLAGE: Site = Site { nom: "réglage de session", adresse: 0x1_00ca_ce00, taille: 0x2a, empreinte: 0xb604_10f4_32e5_5573 };
const X86_REMISE: Site = Site { nom: "remise à 1 166", adresse: 0x1_00ca_ce30, taille: 0x10, empreinte: 0x33b5_cebe_89a8_2469 };
/// `lea r14,[rip+tampon]` … `mov edx,800h ; xor r9d,r9d ; call [rax+10h]`.
const X86_RECEPTION: Site = Site { nom: "réception Steam", adresse: 0x1_00cd_3ad6, taille: 0x23, empreinte: 0xf61b_3f97_0079_6ae3 };
const X86_GLOBALE: usize = 0x1_01cd_5c50;
const X86_TAMPON: usize = 0x1_0297_37a0;

const ARM_REGLAGE: Site = Site { nom: "réglage de session", adresse: 0x1_00bf_e800, taille: 0x28, empreinte: 0x339e_32da_db79_8b7a };
const ARM_REMISE: Site = Site { nom: "remise à 1 166", adresse: 0x1_00bf_e828, taille: 0x10, empreinte: 0x2ef4_b392_536a_b799 };
/// `ldr x0,[x19,#0xb8]` … `adrp x20 ; add x20` … `mov w2,#0x800 ; mov w5,#0 ; blr x8`.
const ARM_RECEPTION: Site = Site { nom: "réception Steam", adresse: 0x1_00c2_18e4, taille: 0x2c, empreinte: 0xeb8b_846f_d7cf_3aba };
const ARM_GLOBALE: usize = 0x1_01ba_1748;
const ARM_TAMPON: usize = 0x1_0f08_aef4;

const X86: bool = cfg!(target_arch = "x86_64");
const REGLAGE: Site = if X86 { X86_REGLAGE } else { ARM_REGLAGE };
const REMISE: Site = if X86 { X86_REMISE } else { ARM_REMISE };
const RECEPTION: Site = if X86 { X86_RECEPTION } else { ARM_RECEPTION };
const GLOBALE: usize = if X86 { X86_GLOBALE } else { ARM_GLOBALE };
/// Le tampon de 0x800 octets du jeu, remplacé.
const TAMPON: usize = if X86 { X86_TAMPON } else { ARM_TAMPON };

/// Portée demandée au tampon : sous les ±2 Go d'un `lea [rip+disp32]` et les ±4 Go d'un `adrp`.
const PORTEE: usize = 0x7000_0000;

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

// ---------------------------------------------------------------------------------------------
// Code écrit au lancement (fonctions pures, testées sur l'exe)

/// La taille que le réglage de session d'origine écrit pour `v`.
fn reglage_du_jeu(v: u32) -> u32 {
    if (1..GAME_CHUNK).contains(&v) {
        v.max(16)
    } else {
        GAME_CHUNK
    }
}

/// Octets à écrire du début du réglage à la fin de la remise : un saut vers `reglage` au début de
/// l'un, vers `remise` au début de l'autre, le reste d'origine.
fn accroches(origine: &[u8], ecart_remise: usize, reglage: usize, remise: usize) -> Option<Vec<u8>> {
    let (a, b) = (patch::jump(reglage), patch::jump(remise));
    if a.len() > ecart_remise || ecart_remise + b.len() > origine.len() {
        return None;
    }
    let mut v = origine.to_vec();
    v[..a.len()].copy_from_slice(&a);
    v[ecart_remise..ecart_remise + b.len()].copy_from_slice(&b);
    Some(v)
}

/// x86_64 : à écrire en `at` (le `lea`), les 0x1d premiers octets du site, tampon et taille changés.
fn reception_x86(origine: &[u8], at: usize, tampon: usize) -> Option<Vec<u8>> {
    let disp = i32::try_from(tampon as i64 - (at as i64 + 7)).ok()?;
    let mut v = origine.get(..0x1d)?.to_vec();
    v[3..7].copy_from_slice(&disp.to_le_bytes());
    v[0x19..0x1d].copy_from_slice(&RECV_BUFFER.to_le_bytes());
    Some(v)
}

/// `adrp xd, cible` en `pc`.
fn adrp(pc: usize, cible: usize, rd: u32) -> Option<u32> {
    let pages = ((cible & !0xfff) as i64 - (pc & !0xfff) as i64) >> 12;
    if !(-(1i64 << 20)..1i64 << 20).contains(&pages) {
        return None;
    }
    let imm = pages as u32 & 0x1f_ffff;
    Some(0x9000_0000 | ((imm & 3) << 29) | ((imm >> 2) << 5) | rd)
}

/// arm64 : à écrire en `at` (le `adrp x20`, site + 0xc), six mots : `adrp x20 ; add x20,x20,#lo`
/// vers le tampon, trois mots d'origine, `mov w2,#0x20000`.
fn reception_arm(origine: &[u8], at: usize, tampon: usize) -> Option<Vec<u8>> {
    let mot = |i: usize| origine.get(i..i + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()));
    // `movz w2, #imm16, lsl #16` : RECV_BUFFER est un multiple de 0x10000.
    if RECV_BUFFER & 0xffff != 0 || RECV_BUFFER >> 16 > 0xffff {
        return None;
    }
    let mots = [
        adrp(at, tampon, 20)?,
        0x9100_0294 | ((tampon as u32 & 0xfff) << 10),
        mot(0x14)?,
        mot(0x18)?,
        mot(0x1c)?,
        0x52a0_0002 | ((RECV_BUFFER >> 16) << 5),
    ];
    Some(mots.iter().flat_map(|m| m.to_le_bytes()).collect())
}

/// Décalage, depuis le début du site de réception, de ce qui y est écrit.
const RECEPTION_ECRIT: usize = if X86 { 0 } else { 0xc };

// ---------------------------------------------------------------------------------------------
// État

/// Adresse de la globale de taille des morceaux, 0 tant que les accroches ne sont pas posées.
static GLOBAL: AtomicUsize = AtomicUsize::new(0);
/// Gros morceaux actifs, lu par `reglage` et `remise`.
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// Écriture de la globale : par `apply`, ou par le jeu (`reglage`, `remise`).
static ECRITURE: Mutex<()> = Mutex::new(());
/// Tampon de réception agrandi.
static BUFFER_OK: AtomicBool = AtomicBool::new(false);
/// `SendP2PPacket` et `ReadP2PPacket` détournés.
static HOOKED: AtomicBool = AtomicBool::new(false);

struct Gate {
    active: bool,
    approved: BTreeSet<u64>,
    strangers: BTreeMap<u64, Instant>,
}

static GATE: Mutex<Gate> = Mutex::new(Gate { active: false, approved: BTreeSet::new(), strangers: BTreeMap::new() });

/// Suffixe de la version annoncée (debit_regle.rs) : `-t0` si le tampon de réception n'a pas pu
/// être agrandi. Pas de `morceaux_reseau` sous macOS.
pub(crate) fn announced_suffix() -> String {
    regle::suffix(BUFFER_OK.load(Ordering::Relaxed), 0, false)
}

/// Écrit la globale, sous `ECRITURE` : gros morceaux si actifs, sinon `jeu`.
fn ecrire_globale(jeu: u32) {
    let _e = ECRITURE.lock().unwrap_or_else(|e| e.into_inner());
    let g = GLOBAL.load(Ordering::Relaxed);
    if g != 0 {
        let v = if ACTIVE.load(Ordering::Relaxed) { DEFAULT_CHUNK } else { jeu };
        unsafe { std::ptr::write_volatile(g as *mut u32, v) };
    }
}

/// Le réglage de session du jeu désormais.
extern "C" fn reglage(v: u32) {
    ecrire_globale(reglage_du_jeu(v));
}

/// La remise à 1 166 du jeu désormais.
extern "C" fn remise() {
    ecrire_globale(GAME_CHUNK);
}

/// Gros morceaux (`on`) ou taille du jeu.
fn apply(g: &mut Gate, on: bool, why: &str) {
    if GLOBAL.load(Ordering::Relaxed) == 0 || g.active == on {
        return;
    }
    {
        let _e = ECRITURE.lock().unwrap_or_else(|e| e.into_inner());
        ACTIVE.store(on, Ordering::Relaxed);
    }
    ecrire_globale(GAME_CHUNK);
    g.active = on;
    log(&format!("debit steam : morceaux de {} octets ({why})", if on { DEFAULT_CHUNK } else { GAME_CHUNK }));
}

/// Chaque paquet Steam, envoyé ou reçu : un pair hors liste ramène aussitôt la taille du jeu.
fn gate_peer(peer: u64) {
    if GLOBAL.load(Ordering::Relaxed) == 0 {
        return;
    }
    let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
    if g.approved.contains(&peer) {
        return;
    }
    g.strangers.insert(peer, Instant::now());
    if g.active {
        apply(&mut g, false, &format!("pair {} hors de la liste des joueurs équipés", peer & 0xffff_ffff));
    }
}

/// `Civ6Proxy_MorceauxReseau(ids)` (partie.lua) : les SteamID des autres joueurs connectés, tous
/// de la même version de BGE, ou une chaîne vide. Ne rend rien.
pub(crate) unsafe extern "C-unwind" fn morceaux_reseau(l: *mut c_void) -> c_int {
    let ids = argument_n(l, 1);
    if GLOBAL.load(Ordering::Relaxed) == 0 {
        return 0;
    }
    let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
    let (mut approved, mut strangers) = (std::mem::take(&mut g.approved), std::mem::take(&mut g.strangers));
    let pol = super::pol::BgdPolActifs() != 0;
    let reseau = super::absent::BgdPartieReseau() != 0;
    let verdict = decide(&ids, pol, reseau, HOOKED.load(Ordering::Relaxed), &mut approved, &mut strangers, Instant::now());
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

/// Natives de `partie.lua`, à poser avec celles de macos_salon.rs (`NATIVES_PARTIE`).
pub(crate) const NATIVES_PARTIE: [(LuaCFn, &str); 1] = [(morceaux_reseau, "Civ6Proxy_MorceauxReseau\0")];

/// Correctifs POL coupés (salon ouvert, verdict négatif) : taille du jeu. Appelé par
/// `Civ6Proxy_FixerPol` (macos_pol.rs).
pub(crate) fn pol_off() {
    let mut g = GATE.lock().unwrap_or_else(|e| e.into_inner());
    g.approved.clear();
    apply(&mut g, false, "correctifs POL coupés");
}

// ---------------------------------------------------------------------------------------------
// Steam

const SLOT_SEND: usize = 0;
const SLOT_READ: usize = 2;
const INTERFACE: &[u8] = b"SteamNetworking006";

type FindFn = unsafe extern "C" fn(i32, *const c_char) -> *mut c_void;
type SendFn = unsafe extern "C" fn(*mut c_void, u64, *const c_void, u32, i32, i32) -> u8;
type ReadFn = unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut u32, *mut u64, i32) -> u8;

static ORIG_FIND: AtomicUsize = AtomicUsize::new(0);
static ORIG_SEND: AtomicUsize = AtomicUsize::new(0);
static ORIG_READ: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    static mach_task_self_: u32;
    fn getpagesize() -> c_int;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    fn mach_vm_protect(task: u32, addr: u64, size: u64, set_maximum: c_int, prot: c_int) -> c_int;
    fn mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, off: i64) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

/// Écrit une case de pointeur (table virtuelle du client Steam, en données).
unsafe fn write_slot(at: usize, value: usize) -> bool {
    const PROT_READ: c_int = 1;
    const PROT_WRITE: c_int = 2;
    const VM_PROT_COPY: c_int = 0x10;
    let ps = getpagesize() as usize;
    let page = at & !(ps - 1);
    let ok = mprotect(page as *mut c_void, ps, PROT_READ | PROT_WRITE) == 0
        || mach_vm_protect(mach_task_self_, page as u64, ps as u64, 0, PROT_READ | PROT_WRITE | VM_PROT_COPY) == 0;
    if ok {
        std::ptr::write_volatile(at as *mut usize, value);
    }
    ok
}

unsafe extern "C" fn send_hook(this: *mut c_void, peer: u64, data: *const c_void, len: u32, kind: i32, channel: i32) -> u8 {
    gate_peer(peer);
    let original: SendFn = std::mem::transmute(ORIG_SEND.load(Ordering::Relaxed));
    original(this, peer, data, len, kind, channel)
}

unsafe extern "C" fn read_hook(this: *mut c_void, dest: *mut c_void, cap: u32, size: *mut u32, peer: *mut u64, channel: i32) -> u8 {
    let original: ReadFn = std::mem::transmute(ORIG_READ.load(Ordering::Relaxed));
    let ok = original(this, dest, cap, size, peer, channel);
    if ok != 0 && !peer.is_null() {
        // Avant que le jeu ne lise le paquet : il ne répond à un nouveau venu qu'après.
        gate_peer(*peer);
    }
    ok
}

/// Redirige `SendP2PPacket` et `ReadP2PPacket` dans la table virtuelle de l'interface, une fois.
unsafe fn hook_interface(iface: *mut c_void) {
    let vtable = *(iface as *const usize);
    if vtable == 0 || HOOKED.load(Ordering::Relaxed) {
        return;
    }
    let (send, read) = (vtable + 8 * SLOT_SEND, vtable + 8 * SLOT_READ);
    if *(send as *const usize) == send_hook as SendFn as usize {
        return;
    }
    ORIG_SEND.store(*(send as *const usize), Ordering::Relaxed);
    ORIG_READ.store(*(read as *const usize), Ordering::Relaxed);
    if write_slot(send, send_hook as SendFn as usize) && write_slot(read, read_hook as ReadFn as usize) {
        HOOKED.store(true, Ordering::Relaxed);
        log("debit steam : SendP2PPacket et ReadP2PPacket détournés");
    } else {
        // La case d'envoi a pu être écrite : elle relaie vers l'original, sans effet.
        log("debit steam : table de SteamNetworking006 non modifiable, pas de gros morceaux");
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

/// Remplace l'import `SteamInternal_FindOrCreateUserInterface` de l'exe.
unsafe fn install_hooks(mh: *const macho::MachHeader64, slide: isize) {
    const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
    let vrai = dlsym(RTLD_DEFAULT, c"SteamInternal_FindOrCreateUserInterface".as_ptr());
    if vrai.is_null() {
        log("debit steam : SteamInternal_FindOrCreateUserInterface introuvable, pas de gros morceaux");
        return;
    }
    ORIG_FIND.store(vrai as usize, Ordering::Relaxed);
    match macho::hook_import(mh, slide, "_SteamInternal_FindOrCreateUserInterface", find_hook as FindFn as usize) {
        Ok(0) => log("debit steam : import SteamInternal_FindOrCreateUserInterface introuvable, pas de gros morceaux"),
        Ok(_) => log("debit steam : en attente de SteamNetworking006"),
        Err(e) => log(&format!("debit steam : import SteamInternal_FindOrCreateUserInterface : {e}, pas de gros morceaux")),
    }
}

// ---------------------------------------------------------------------------------------------
// Lancement

/// Tampon de `taille` octets (zéros, lecture et écriture) à moins de `PORTEE` de `pres`.
unsafe fn tampon_pres(pres: usize, taille: usize) -> Option<usize> {
    const PROT_RW: c_int = 3;
    const MAP_PRIVATE: c_int = 0x2;
    const MAP_ANON: c_int = 0x1000;
    const PAS: usize = 0x100_0000;
    for k in 1..PORTEE / PAS {
        for hint in [pres.checked_add(k * PAS), pres.checked_sub(k * PAS)].into_iter().flatten() {
            let p = mmap((hint & !(PAS - 1)) as *mut c_void, taille, PROT_RW, MAP_PRIVATE | MAP_ANON, -1, 0);
            if p as isize == -1 {
                continue;
            }
            if (p as usize).abs_diff(pres) + taille < PORTEE {
                return Some(p as usize);
            }
            munmap(p, taille);
        }
    }
    None
}

/// Appelé par le constructeur (macos.rs), avant que le jeu n'exécute son code réseau.
pub unsafe fn installer(mh: *const macho::MachHeader64, slide: isize) -> Result<(), String> {
    // Toujours : le contrôle des pairs (retour aux morceaux du jeu) en dépend.
    install_hooks(mh, slide);
    let image = |a: usize| (a as isize + slide) as usize;
    for s in [&REGLAGE, &REMISE, &RECEPTION] {
        if fnv(std::slice::from_raw_parts(image(s.adresse) as *const u8, s.taille)) != s.empreinte {
            return Err(format!("exe inattendu ({} en {:#x}), tampon de réception d'origine, pas de gros morceaux", s.nom, s.adresse));
        }
    }
    let site = image(RECEPTION.adresse);
    let tampon = tampon_pres(site, RECV_BUFFER as usize).ok_or("pas de tampon à portée de l'exe, tampon de réception d'origine, pas de gros morceaux")?;
    let origine = std::slice::from_raw_parts(site as *const u8, RECEPTION.taille);
    let at = site + RECEPTION_ECRIT;
    let octets = if X86 { reception_x86(origine, at, tampon) } else { reception_arm(origine, at, tampon) };
    let octets = octets.ok_or("tampon hors de portée, tampon de réception d'origine, pas de gros morceaux")?;
    let methode = patch::write_code(at, &octets).map_err(|e| format!("écriture refusée ({e}), tampon de réception d'origine, pas de gros morceaux"))?;
    BUFFER_OK.store(true, Ordering::Relaxed);
    log(&format!("debit steam : tampon de réception de {RECV_BUFFER} octets à la place de celui du jeu (exe {:#x}, lu en {:#x} ; {methode})", TAMPON - BASE_EXE, RECEPTION.adresse - BASE_EXE));

    // Le réglage de session et la remise : taille du jeu tant que les gros morceaux sont inactifs.
    let debut = image(REGLAGE.adresse);
    let ecart = REMISE.adresse - REGLAGE.adresse;
    let origine = std::slice::from_raw_parts(debut as *const u8, ecart + REMISE.taille);
    let octets = accroches(origine, ecart, reglage as extern "C" fn(u32) as usize, remise as extern "C" fn() as usize).ok_or("accroches trop longues")?;
    GLOBAL.store(image(GLOBALE), Ordering::Relaxed);
    if let Err(e) = patch::write_code(debut, &octets) {
        GLOBAL.store(0, Ordering::Relaxed);
        return Err(format!("réglage de session non accroché ({e}), pas de gros morceaux"));
    }
    log(&format!("debit steam : morceaux de {DEFAULT_CHUNK} octets en partie seulement, quand chaque joueur annonce la même version de BGE"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn exe() -> Option<Vec<u8>> {
        let home = std::env::var_os("HOME")?;
        let p = std::path::Path::new(&home).join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/Civ6_Exe_Child");
        std::fs::read(p).ok()
    }

    /// Tranche `cpu` d'un binaire universel.
    fn tranche(b: &[u8], cpu: u32) -> Option<&[u8]> {
        let be = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
        if be(0)? != 0xcafe_babe {
            return None;
        }
        (0..be(4)? as usize).find_map(|i| {
            let e = 8 + 20 * i;
            (be(e)? == cpu).then(|| b.get(be(e + 8)? as usize..(be(e + 8)? + be(e + 12)?) as usize)).flatten()
        })
    }

    /// `__TEXT` et `__DATA` (données initialisées) suivent le fichier : décalage = adresse - base.
    fn octets(t: &[u8], a: usize, n: usize) -> &[u8] {
        &t[a - BASE_EXE..a - BASE_EXE + n]
    }

    fn mot(t: &[u8], a: usize) -> u32 {
        u32::from_le_bytes(octets(t, a, 4).try_into().unwrap())
    }

    fn rel32(t: &[u8], a: usize, fin: usize) -> usize {
        (fin as i64 + i32::from_le_bytes(octets(t, a, 4).try_into().unwrap()) as i64) as usize
    }

    /// Cible de `adrp` en `pc` suivi de `add`/`ldr`/`str` 32 bits sur le même registre.
    fn adrp_cible(pc: usize, adrp: u32, suite: u32) -> usize {
        let imm = (((adrp >> 29) & 3) | (((adrp >> 5) & 0x7ffff) << 2)) as i64;
        let imm = if imm & (1 << 20) != 0 { imm - (1 << 21) } else { imm };
        let page = ((pc as i64) & !0xfff) + (imm << 12);
        let lo = if suite & 0xff00_0000 == 0x9100_0000 { (suite >> 10) & 0xfff } else { ((suite >> 10) & 0xfff) << 2 };
        page as usize + lo as usize
    }

    #[test]
    fn exe_x86_64() {
        let Some(b) = exe() else { return };
        let t = tranche(&b, 0x0100_0007).expect("tranche x86_64");
        for s in [&X86_REGLAGE, &X86_REMISE, &X86_RECEPTION] {
            assert_eq!(fnv(octets(t, s.adresse, s.taille)), s.empreinte, "{}", s.nom);
        }
        // Réglage : mov [rip+g],eax en +0x22 ; remise : mov dword [rip+g],48Eh en +4.
        assert_eq!(octets(t, X86_REGLAGE.adresse + 0x22, 2), [0x89, 0x05]);
        assert_eq!(rel32(t, X86_REGLAGE.adresse + 0x24, X86_REGLAGE.adresse + 0x28), X86_GLOBALE);
        assert_eq!(octets(t, X86_REMISE.adresse + 4, 2), [0xc7, 0x05]);
        assert_eq!(rel32(t, X86_REMISE.adresse + 6, X86_REMISE.adresse + 14), X86_GLOBALE);
        assert_eq!(mot(t, X86_REMISE.adresse + 10), GAME_CHUNK);
        assert_eq!(mot(t, X86_GLOBALE), GAME_CHUNK, "valeur initiale de la globale");
        // Réception : lea r14,[rip+tampon] ; mov edx,800h ; call [rax+10h].
        let r = X86_RECEPTION.adresse;
        assert_eq!(octets(t, r, 3), [0x4c, 0x8d, 0x35]);
        assert_eq!(rel32(t, r + 3, r + 7), X86_TAMPON);
        assert_eq!(octets(t, r + 0x18, 5), [0xba, 0x00, 0x08, 0x00, 0x00]);
        assert_eq!(octets(t, r + 0x20, 3), [0xff, 0x50, 0x10]);
        // Le tampon d'origine n'a qu'une référence : celle-ci.
        let text = octets(t, BASE_EXE, 0x1900_000);
        let refs = (0..text.len() - 4).filter(|&i| (BASE_EXE + i + 4).wrapping_add_signed(i32::from_le_bytes(text[i..i + 4].try_into().unwrap()) as isize) == X86_TAMPON).count();
        assert_eq!(refs, 1);
        // Ce qui sera écrit.
        let at = r;
        let tampon = r + 0x1000_0000;
        let v = reception_x86(octets(t, r, X86_RECEPTION.taille), at, tampon).unwrap();
        assert_eq!(v.len(), 0x1d);
        assert_eq!((at as i64 + 7 + i32::from_le_bytes(v[3..7].try_into().unwrap()) as i64) as usize, tampon);
        assert_eq!(u32::from_le_bytes(v[0x19..0x1d].try_into().unwrap()), RECV_BUFFER);
        assert_eq!(v[..3], octets(t, r, 3)[..]);
        assert_eq!(v[7..0x19], octets(t, r + 7, 0x12)[..]);
        assert!(reception_x86(octets(t, r, X86_RECEPTION.taille), at, at + 0x9000_0000).is_none(), "hors de portée");
    }

    #[test]
    fn exe_arm64() {
        let Some(b) = exe() else { return };
        let t = tranche(&b, 0x0100_000c).expect("tranche arm64");
        for s in [&ARM_REGLAGE, &ARM_REMISE, &ARM_RECEPTION] {
            assert_eq!(fnv(octets(t, s.adresse, s.taille)), s.empreinte, "{}", s.nom);
        }
        // adrp x9 ; str w8,[x9,#off] ; ret, dans les deux fonctions.
        for (a, quoi) in [(ARM_REGLAGE.adresse + 0x1c, "réglage"), (ARM_REMISE.adresse + 4, "remise")] {
            assert_eq!(mot(t, a) & 0x9f00_001f, 0x9000_0009, "{quoi} : adrp x9");
            assert_eq!(mot(t, a + 4) & 0xffc0_03ff, 0xb900_0128, "{quoi} : str w8,[x9,#…]");
            assert_eq!(adrp_cible(a, mot(t, a), mot(t, a + 4)), ARM_GLOBALE, "{quoi}");
        }
        assert_eq!(mot(t, ARM_REMISE.adresse), 0x5280_91c8, "mov w8,#0x48e");
        assert_eq!(mot(t, ARM_GLOBALE), GAME_CHUNK, "valeur initiale de la globale");
        // Réception : ldr x8,[x8,#0x10] ; adrp x20 ; add x20 ; mov x1,x20 ; mov w2,#0x800 ; blr x8.
        let r = ARM_RECEPTION.adresse;
        assert_eq!(mot(t, r + 8), 0xf940_0908, "ldr x8,[x8,#0x10]");
        assert_eq!(adrp_cible(r + 0xc, mot(t, r + 0xc), mot(t, r + 0x10)), ARM_TAMPON);
        assert_eq!(mot(t, r + 0x1c), 0xaa14_03e1, "mov x1,x20");
        assert_eq!(mot(t, r + 0x20), 0x5281_0002, "mov w2,#0x800");
        assert_eq!(mot(t, r + 0x28), 0xd63f_0100, "blr x8");
        // Ce qui sera écrit.
        let at = r + 0xc;
        let tampon = 0x1_1234_5000usize + 0x7f0;
        let v = reception_arm(octets(t, r, ARM_RECEPTION.taille), at, tampon).unwrap();
        let w = |i: usize| u32::from_le_bytes(v[4 * i..4 * i + 4].try_into().unwrap());
        assert_eq!(v.len(), 24);
        assert_eq!(adrp_cible(at, w(0), w(1)), tampon);
        assert_eq!(w(0) & 0x9f00_001f, 0x9000_0014, "adrp x20");
        assert_eq!(w(1) & 0xffc0_03ff, 0x9100_0294, "add x20,x20,#lo");
        assert_eq!([w(2), w(3), w(4)], [mot(t, r + 0x14), mot(t, r + 0x18), mot(t, r + 0x1c)]);
        assert_eq!(w(5), 0x52a0_0042, "mov w2,#0x20000");
        assert!(reception_arm(octets(t, r, ARM_RECEPTION.taille), at, at + (5 << 30)).is_none(), "hors de portée");
    }

    #[test]
    fn interface_de_l_exe() {
        let Some(b) = exe() else { return };
        for cpu in [0x0100_0007, 0x0100_000c] {
            let t = tranche(&b, cpu).unwrap();
            let n = t.windows(INTERFACE.len() + 1).filter(|w| w[..INTERFACE.len()] == *INTERFACE && w[INTERFACE.len()] == 0).count();
            assert_eq!(n, 1, "{cpu:#x} : chaîne SteamNetworking006");
            let nom = b"_SteamInternal_FindOrCreateUserInterface\0";
            assert!(t.windows(nom.len()).any(|w| w == nom), "{cpu:#x} : import");
        }
    }

    #[test]
    fn reglage_comme_le_jeu() {
        // lea eax,[rdi-48Eh] ; cmp eax,-48Dh ; cmovae (max(v,16)) ; sinon 48Eh.
        let jeu = |v: u32| {
            let ecx = if v >= 0x11 { v } else { 0x10 };
            if v.wrapping_sub(0x48e) >= 0xffff_fb73 { ecx } else { 0x48e }
        };
        for v in [0, 1, 15, 16, 17, 1000, 1165, 1166, 1167, 1200, 16384, u32::MAX] {
            assert_eq!(reglage_du_jeu(v), jeu(v), "{v}");
        }
    }

    #[test]
    fn accroches_aux_bons_endroits() {
        let origine: Vec<u8> = (0..(REMISE.adresse - REGLAGE.adresse + REMISE.taille) as u8).collect();
        let ecart = REMISE.adresse - REGLAGE.adresse;
        let v = accroches(&origine, ecart, 0x1111, 0x2222).unwrap();
        let (a, b) = (patch::jump(0x1111), patch::jump(0x2222));
        assert_eq!(v.len(), origine.len());
        assert_eq!(&v[..a.len()], a.as_slice());
        assert_eq!(&v[ecart..ecart + b.len()], b.as_slice());
        assert_eq!(&v[a.len()..ecart], &origine[a.len()..ecart]);
        assert!(a.len() <= REGLAGE.taille && b.len() <= REMISE.taille, "chaque saut tient dans sa fonction");
    }

    /// Le verdict, comme sous Windows (debit_steam.rs, `chunk_gate`).
    #[test]
    fn verdict() {
        let now = Instant::now();
        let (mut a, mut s) = (BTreeSet::new(), BTreeMap::new());
        assert!(decide("76561198049438351", true, true, true, &mut a, &mut s, now).is_ok());
        assert!(decide("", true, true, true, &mut a, &mut s, now).is_err(), "liste vide");
        assert!(decide("1", false, true, true, &mut a, &mut s, now).is_err(), "POL coupé");
        assert!(decide("1", true, false, true, &mut a, &mut s, now).is_err(), "partie locale");
        assert!(decide("1", true, true, false, &mut a, &mut s, now).is_err(), "Steam non détourné");
        s.insert(2, now);
        assert!(decide("1", true, true, true, &mut a, &mut s, now).is_err(), "inconnu vu à l'instant");
        assert!(decide("1,2", true, true, true, &mut a, &mut s, now).is_ok(), "devenu équipé");
        s.insert(3, now);
        assert!(decide("1", true, true, true, &mut a, &mut s, now + Duration::from_secs(30)).is_ok(), "parti depuis 30 s");
    }

    #[test]
    fn rien_sans_accroches() {
        assert_eq!(announced_suffix(), "-t0", "tampon non agrandi : jamais compté comme équipé");
        pol_off();
        gate_peer(5);
        let mut g = GATE.lock().unwrap();
        apply(&mut g, true, "essai");
        assert!(!g.active && !ACTIVE.load(Ordering::Relaxed), "sans globale, rien n'est actif");
    }

    #[test]
    fn native_attendue_par_partie_lua() {
        use super::super::salon_texte::PARTIE_BLOCK;
        let nom = NATIVES_PARTIE[0].1.trim_end_matches('\0');
        assert!(PARTIE_BLOCK.contains(&format!("{nom}(ids)")), "partie.lua appelle {nom}");
        assert!(PARTIE_BLOCK.contains("Civ6Proxy_PolActifs() ~= 1"), "partie.lua lit les correctifs POL (cale de macos_salon.rs)");
    }
}
