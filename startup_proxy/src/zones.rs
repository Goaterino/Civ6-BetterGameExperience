//! Zones du profileur intégré de l'exe (`profil_zones=on`, diagnostic, désactivé par défaut).
//!
//! L'exe chronomètre environ 1 170 portées nommées (`Civ6App_UpdateFrame`, `UIManager_Update`,
//! `LuaContext::…`, `GameCoreEvent::…`) avec un objet de portée :
//! - `140998450` (DX12 ; `14098a780` en DX11) l'ouvre : `[0]` compteur de la zone, `[1]` son nom,
//!   `[2]` seuil en ms (double), `[3]` QPC de départ ;
//! - `140998480` (DX12 ; `14098a7b0` en DX11) le ferme : elle n'écrit dans `Profile.csv` que les
//!   zones qui dépassent leur seuil, 500 fois au plus par zone.
//!
//! On détourne la fermeture : chaque zone, même courte, est notée (thread, nom, début, fin) dans un
//! tampon circulaire, puis vidée toutes les 100 ms par un thread d'écriture dans
//! `%LOCALAPPDATA%\Civ6RolloverProxy\zones\zones-<pid>-<horodatage>.bin`. Les instants sont des QPC,
//! l'horloge des traces WPR : `rollover/analyze.py` y cale les échantillons CPU.
//!
//! Format du fichier, petit-boutiste : `BGDZONE1`, fréquence QPC (u64), pid (u32), 0 (u32), puis des
//! enregistrements :
//! - `1` nom : identifiant (u32), longueur (u8), octets ;
//! - `2` zone : thread (u32), identifiant du nom (u32), début (u64), fin (u64) ;
//! - `3` perte : nombre de zones écrasées avant d'être écrites (u64).

use crate::{exe, log, patch, win};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// Prologue de la fermeture, identique en DX12 et en DX11 : `push rbp ; push r15 ;
/// lea rbp, [rsp-0x2d8] ; sub rsp, 0x3d8 ; movaps [rsp+0x3a0], xmm6`. Les 19 premiers octets, sans
/// adressage relatif, sont recopiés dans le trampoline.
const PROLOGUE: [u8; 24] = [
    0x40, 0x55, 0x41, 0x57, 0x48, 0x8d, 0xac, 0x24, 0x28, 0xfd, 0xff, 0xff, 0x48, 0x81, 0xec, 0xd8, 0x03, 0x00,
    0x00, 0x0f, 0x29, 0xb4, 0x24, 0xa0,
];
const COPY: usize = 19;

const NAME_MAX: usize = 47;
const RING: usize = 1 << 16;

struct Slot {
    /// Indice d'écriture + 1 une fois la zone complète, 0 avant.
    seq: AtomicU64,
    data: UnsafeCell<Zone>,
}

#[derive(Clone, Copy)]
struct Zone {
    tid: u32,
    len: u8,
    name: [u8; NAME_MAX],
    start: u64,
    end: u64,
}

struct Ring([Slot; RING]);
unsafe impl Sync for Ring {}

static SLOTS: Ring = Ring([const {
    Slot { seq: AtomicU64::new(0), data: UnsafeCell::new(Zone { tid: 0, len: 0, name: [0; NAME_MAX], start: 0, end: 0 }) }
}; RING]);
static HEAD: AtomicU64 = AtomicU64::new(0);
static TRAMP: AtomicUsize = AtomicUsize::new(0);
static WRITER: AtomicBool = AtomicBool::new(false);

/// Le réglage `profil_zones` de `rollover_proxy.cfg` (dernière occurrence ; `off` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("profil_zones=")).last().is_some_and(|v| v.trim() == "on")
}

/// Début de la fermeture de zone dans `.text`, si la signature y est une seule fois.
pub fn find_close(text: &[u8]) -> Result<usize, String> {
    let hits: Vec<usize> = text.windows(PROLOGUE.len()).enumerate().filter(|(_, w)| *w == PROLOGUE).map(|(i, _)| i).collect();
    match hits[..] {
        [i] => Ok(i),
        [] => Err("fermeture de zone introuvable".into()),
        _ => Err(format!("fermeture de zone ambiguë ({} occurrences)", hits.len())),
    }
}

unsafe fn record(scope: *const u64, end: u64) {
    let name = *scope.add(1) as *const u8;
    let mut z = Zone { tid: win::GetCurrentThreadId(), len: 0, name: [0; NAME_MAX], start: *scope.add(3), end };
    if !name.is_null() {
        // Octet par octet jusqu'au zéro : jamais au-delà de la fin de la chaîne.
        while (z.len as usize) < NAME_MAX {
            let c = *name.add(z.len as usize);
            if c == 0 {
                break;
            }
            z.name[z.len as usize] = c;
            z.len += 1;
        }
    }
    let i = HEAD.fetch_add(1, Ordering::Relaxed);
    let slot = &SLOTS.0[(i as usize) % RING];
    slot.seq.store(0, Ordering::Relaxed);
    std::ptr::write_volatile(slot.data.get(), z);
    slot.seq.store(i + 1, Ordering::Release);
}

unsafe extern "C" fn hook_close(scope: *const u64) {
    if !scope.is_null() {
        record(scope, win::qpc() as u64);
        if !WRITER.swap(true, Ordering::Relaxed) {
            std::thread::spawn(writer);
        }
    }
    let real: unsafe extern "C" fn(*const u64) = std::mem::transmute(TRAMP.load(Ordering::Relaxed));
    real(scope)
}

fn out_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(&std::env::var_os("LOCALAPPDATA")?).join("Civ6RolloverProxy").join("zones");
    std::fs::create_dir_all(&dir).ok()?;
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    Some(dir.join(format!("zones-{}-{secs}.bin", std::process::id())))
}

/// Vide le tampon dans le fichier, toutes les 100 ms, jusqu'à la fin du processus.
fn writer() {
    let Some(path) = out_path() else {
        log("zones : dossier de sortie introuvable, rien n'est écrit");
        return;
    };
    let Ok(file) = std::fs::File::create(&path) else {
        log(&format!("zones : création de {} impossible", path.display()));
        return;
    };
    log(&format!("zones : écriture dans {}", path.display()));
    let mut out = std::io::BufWriter::with_capacity(1 << 20, file);
    let mut freq = 0i64;
    unsafe { win::QueryPerformanceFrequency(&mut freq) };
    let _ = out.write_all(b"BGDZONE1");
    let _ = out.write_all(&(freq as u64).to_le_bytes());
    let _ = out.write_all(&std::process::id().to_le_bytes());
    let _ = out.write_all(&0u32.to_le_bytes());
    let mut names: HashMap<Vec<u8>, u32> = HashMap::new();
    let mut tail = 0u64;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let head = HEAD.load(Ordering::Acquire);
        let mut lost = 0u64;
        if head - tail > RING as u64 {
            lost += head - tail - RING as u64;
            tail = head - RING as u64;
        }
        while tail < head {
            let slot = &SLOTS.0[(tail as usize) % RING];
            let seq = slot.seq.load(Ordering::Acquire);
            // 0 ou l'indice du tour précédent : la zone est réservée mais pas encore écrite.
            if seq < tail + 1 && HEAD.load(Ordering::Relaxed) - tail < RING as u64 {
                break; // au prochain tour
            }
            let z = unsafe { std::ptr::read_volatile(slot.data.get()) };
            // Écrasée pendant la lecture, ou déjà par une zone plus récente : perdue.
            if seq != tail + 1 || slot.seq.load(Ordering::Acquire) != seq {
                lost += 1;
                tail += 1;
                continue;
            }
            let key = z.name[..z.len as usize].to_vec();
            let next = names.len() as u32;
            let id = *names.entry(key).or_insert_with(|| {
                let _ = out.write_all(&[1]);
                let _ = out.write_all(&next.to_le_bytes());
                let _ = out.write_all(&[z.len]);
                let _ = out.write_all(&z.name[..z.len as usize]);
                next
            });
            let _ = out.write_all(&[2]);
            let _ = out.write_all(&z.tid.to_le_bytes());
            let _ = out.write_all(&id.to_le_bytes());
            let _ = out.write_all(&z.start.to_le_bytes());
            let _ = out.write_all(&z.end.to_le_bytes());
            tail += 1;
        }
        if lost > 0 {
            let _ = out.write_all(&[3]);
            let _ = out.write_all(&lost.to_le_bytes());
        }
        let _ = out.flush();
    }
}

/// Pose le détour si `profil_zones=on`. Appelée par DllMain : aucun code de l'exe n'a encore tourné,
/// la fermeture de zone n'est donc exécutée par aucun thread pendant qu'on l'écrase.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if !enabled(&cfg) {
        return;
    }
    let exe = win::GetModuleHandleW(std::ptr::null()) as usize;
    let Some((text, size)) = exe::text_section(exe) else {
        log("zones : section .text de l'exe introuvable, pas de profil");
        return;
    };
    let at = match find_close(std::slice::from_raw_parts(text as *const u8, size)) {
        Ok(off) => text + off,
        Err(e) => {
            log(&format!("zones : {e}, pas de profil"));
            return;
        }
    };
    match patch::install_at("fermeture de zone", at, &PROLOGUE[..COPY], at + COPY, hook_close as *const () as usize) {
        Ok(t) => {
            TRAMP.store(t, Ordering::Relaxed);
            log(&format!("zones : détour posé en exe+{:#x}", at - exe));
        }
        Err(e) => log(&format!("zones : {e}, pas de profil")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";

    #[test]
    fn reglage() {
        assert!(!enabled(""));
        assert!(!enabled("profil_zones=off\r\n"));
        assert!(enabled("mode=coalesce\r\nprofil_zones=on\r\n"));
        assert!(!enabled("profil_zones=on\nprofil_zones=off\n"), "la dernière occurrence compte");
    }

    /// La signature est unique dans les deux exes du jeu, et la recopie du prologue s'arrête à une
    /// frontière d'instruction (`movaps` commence en 19).
    #[test]
    fn signature_dans_les_exes() {
        for name in ["CivilizationVI_DX12.exe", "CivilizationVI.exe"] {
            let Ok(file) = std::fs::read(format!(r"{EXE_DIR}\{name}")) else {
                eprintln!("absent, test sauté : {name}");
                continue;
            };
            assert!(find_close(&file).is_ok(), "{name}");
            assert_eq!(&PROLOGUE[COPY..COPY + 3], &[0x0f, 0x29, 0xb4]);
        }
    }
}
