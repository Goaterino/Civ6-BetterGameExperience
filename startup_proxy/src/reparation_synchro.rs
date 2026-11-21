//! Réparation convergente des AutoArchives désynchronisées (`reparation_synchro=fulldelta`, par
//! défaut), et moitié hôte du banc de synchro (`banc_synchro`, diagnostic). Jeux 1.0.12.72 et
//! 1.0.12.74, exes DX11 et DX12, sites trouvés par motifs.
//!
//! **Hôte seulement.** À la fin d'un tour, chaque client envoie `NetGameSyncCheck` (un hachage par
//! AutoArchive, et les indices des variables qu'il a modifiées). L'hôte compare (`1407f46c0` en
//! DX11) et, pour une archive présente des deux côtés avec un autre hachage, appelle le constructeur
//! de delta (`1407f6b00`), qui ne sérialise que les variables marquées modifiées chez l'hôte ou chez
//! le client. Une divergence dans une variable que personne n'a modifiée n'est pas dans ce delta :
//! le client note « No desynced AutoVariables found », n'applique rien, et l'archive est de nouveau
//! désynchronisée au contrôle suivant, jusqu'à ce que la divergence touche l'existence des objets et
//! force un rechargement complet.
//!
//! Le client resynchronise déjà sur place une archive existante quand l'hôte la lui envoie en
//! *données de création* (« full AutoArchive snapshot, already exists on client: Syncing... »,
//! `140dbd170` : chaque variable relue, aucun snapshot complet demandé). Ce module redirige l'unique
//! appel de l'hôte au constructeur de delta et, pour une archive dont le delta ne porterait aucune
//! variable, ou qui était déjà désynchronisée au contrôle précédent du même joueur, envoie plus
//! fort :
//!
//! - `fulldelta` (par défaut) : un delta qui liste toutes les variables de l'archive. Le chemin de
//!   delta d'origine du client applique celles qui diffèrent et lance leurs DeltaDesyncCallbacks : il
//!   répare une variable que personne n'a modifiée, que le delta d'origine n'envoie jamais, et pour
//!   le reste fait comme lui ;
//! - `create` : les données de création de l'archive, par le constructeur de création de l'hôte
//!   (`1407f69d0`, celui de « doesn't exist on client »). **Pas pour une vraie partie.** Le chemin
//!   sur place du client désinscrit l'archive, la relit et la réinscrit, sans DeltaDesyncCallbacks.
//!   Sur une petite carte avec des IA, il a changé des boucles d'IA sans conséquence en rechargements
//!   complets : 15 points de dégâts sur une unité d'IA, chez le client seulement, donnent 2 à 6
//!   rechargements au lieu de 0 (jeu d'origine et `fulldelta`) ; si le client recharge au quatrième
//!   tour, il y en a 5 au lieu de 2. Deux causes, côté client : quatre lectures de variables en
//!   service ne rendent pas ce que l'hôte a écrit (`relecture`, proxy GameCore), et relire une
//!   variable qui a déjà la valeur de l'hôte change ensuite les décisions de l'IA
//!   (`relecture_sur_place`). Avec les deux chez le client (`relecture_fidele=on`), plus aucun
//!   rechargement en trop : 0 dans le premier cas, 1 (celui qu'il a demandé) dans le second. Reste
//!   réservé aux builds de diagnostic : l'hôte ne sait pas si ses clients ont ce correctif. Gardé
//!   pour le banc du chemin sur place ;
//! - `ladder` : `fulldelta` d'abord, `create` si l'archive est encore désynchronisée ensuite (même
//!   mise en garde que `create`) ;
//! - `count` : ne change rien, note ce que le mode par défaut (`fulldelta`) aurait envoyé.
//!
//! `sync_notes` redirige les mêmes appels, celui du contrôle et celui du delta (il compte ce qui
//! différait, pour le rechargement de tous) : ce module est posé après lui et appelle son relais, les
//! deux voient donc chaque contrôle et chaque archive réparée par delta.
//!
//! Les clients exécutent le code d'origine : il suffit que l'hôte ait ce proxy pour que ses clients
//! en profitent. Après `reparation_synchro_essais` escalades de suite pour une même archive, l'hôte
//! revient au delta d'origine (noté une fois) : une archive qui ne peut pas converger ainsi, comme
//! une table de hachage EASTL hachée dans l'ordre de ses seaux (voir `unites_promises`), ne doit pas
//! coûter une archive complète à chaque tour.
//!
//! **Banc de synchro.** `banc_synchro=<sous-chaîne du type>` fait traiter par l'hôte les archives de
//! type correspondant comme désynchronisées (il met à zéro le hachage annoncé par le client, avant la
//! comparaison), `banc_synchro_max` au plus par contrôle. Par défaut, seulement pour les contrôles
//! `testSync`, envoyés par un client avec `Network.TriggerTestSync()` : le client exécute alors tout
//! son traitement à blanc (il note chaque chemin qu'il prendrait, n'applique rien, ne demande rien).
//! `banc_synchro_reel=on` l'étend aux contrôles de fin de tour, où le client applique vraiment ce
//! qu'il reçoit (sans effet sur une archive qui était synchronisée). Les séries des contrôles de test
//! et des vrais contrôles sont comptées à part.
//!
//! Pas de réglage joueur : la release fait toujours `fulldelta`. Les builds de diagnostic (feature
//! `diag`) lisent ces clés dans `rollover_proxy.cfg` (au démarrage, puis au plus toutes les 3 s) :
//! `reparation_synchro=fulldelta|create|ladder|count|off`, `reparation_synchro_essais=3`,
//! `reparation_synchro_max=32` (escalades par contrôle et par joueur), `banc_synchro=`,
//! `banc_synchro_reel=off`, `banc_synchro_max=4`. `actif=off` (l'interrupteur général de BGE) ne pose
//! rien.

use crate::{exe, log, patch, win};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------
// Signatures. Les octets `??` sont des déplacements rel32 et des offsets de pile, libres d'une
// compilation à l'autre. Chaque motif finit par l'appel `E8 rel32` redirigé (contrôle, delta) ou
// lu (création).

/// Parcours de la file `1407f7db0` -> gestionnaire de l'hôte `1407f46c0(mgr, check) -> bool traité`.
const SIG_CHECK: &str = "48 8B D7 48 8B CE E8 ?? ?? ?? ?? 84 C0 0F 84 ?? ?? ?? ?? 48 8B 56 40";
const SIG_CHECK_CALL: usize = 6;
/// Gestionnaire de l'hôte, branche des hachages différents -> constructeur de delta
/// `1407f6b00(mgr, snapshot, stream, &clientVars, key, archive)`.
const SIG_DELTA: &str = "48 8B 43 08 4D 8D 4E 10 48 8B B5 ?? ?? ?? ?? 4C 8D 45 ?? 48 89 44 24 28 \
                         48 8D 95 ?? ?? ?? ?? 48 8B 03 48 8B CE 48 89 44 24 20 E8 ?? ?? ?? ??";
const SIG_DELTA_CALL: usize = 42;
/// Gestionnaire de l'hôte, branche « doesn't exist on client » -> constructeur de création
/// `1407f69d0(mgr, snapshot, stream, key, archive)`.
const SIG_CREATE: &str = "48 8B 47 08 4C 8D 45 ?? 4C 8B 0F 48 8D 95 ?? ?? ?? ?? 48 8B 8D ?? ?? ?? ?? \
                          48 89 44 24 20 E8 ?? ?? ?? ??";
const SIG_CREATE_CALL: usize = 30;

/// Constructeur de delta : `lea rbp,[rdx+38h]` (table du delta) ; `lea r14,[rdx+0D8h]` (piles
/// d'appels).
const DELTA_MARK: &[u8] = &[0x48, 0x8d, 0x6a, 0x38, 0x4c, 0x8d, 0xb2, 0xd8, 0x00, 0x00, 0x00];
/// Constructeur de création : `lea rsi,[rdx+88h]` (table de création) ; `lea r15,[rdx+100h]`
/// (priorités).
const CREATE_MARK: &[u8] = &[0x48, 0x8d, 0xb2, 0x88, 0x00, 0x00, 0x00, 0x4c, 0x8d, 0xba, 0x00, 0x01, 0x00, 0x00];
/// Gestionnaire de l'hôte, 0x400 premiers octets : les champs de `NetGameSyncCheck` qu'il lit.
const HANDLER_MARKS: [&[u8]; 4] = [
    &[0x0f, 0xb6, 0x57, 0x38, 0x84, 0xd2], // movzx edx,byte [rdi+38h] ; test dl,dl  (testSync)
    &[0x48, 0x8b, 0x4f, 0x48],             // mov rcx,[rdi+48h]  (seaux de la table des archives du client)
    &[0x8b, 0x47, 0x50],                   // mov eax,[rdi+50h]  (nombre de seaux)
    &[0x8b, 0x57, 0x24],                   // mov edx,[rdi+24h]  (joueur)
];
/// Sérialiseur de delta, appelé par le constructeur de delta : ensemble des variables modifiées de
/// l'archive en +0x38, variables en +0x08..+0x10.
const SERIALIZER_MARKS: [&[u8]; 2] = [&[0x49, 0x8d, 0x5e, 0x38], &[0x49, 0x8b, 0x6e, 0x10, 0x49, 0x2b, 0x6e, 0x08]];

// ---------------------------------------------------------------------------------------------
// Structures (les mêmes dans les deux exes ; vérifiées par les marques ci-dessus).

/// Copie de `NetGameSyncCheck` mise en file (0x68 octets, `FGameSyncManager + 0x40`).
const MSG_PLAYER: usize = 0x24;
const MSG_TEST: usize = 0x38;
const MSG_BUCKETS: usize = 0x48;
const MSG_NBUCKETS: usize = 0x50;
/// Les nœuds de sa table : clé, hachage annoncé (u32), indices des variables modifiées (`VarList`),
/// suivant.
const CNODE_HASH: usize = 0x08;
const CNODE_NEXT: usize = 0x28;
/// `FGameSyncManager` : archives inscrites, nœuds {clé, archive, suivant}.
const MGR_BUCKETS: usize = 0x18;
const MGR_NBUCKETS: usize = 0x20;
const HNODE_ARCHIVE: usize = 0x08;
const HNODE_NEXT: usize = 0x10;
/// AutoArchive : pointeurs des variables [+0x08, +0x10), ensemble des modifiées (std::set, taille
/// en +0x40), nom du type [+0x60, +0x68).
const AR_VARS: usize = 0x08;
const AR_VARS_END: usize = 0x10;
const AR_DIRTY_LEN: usize = 0x40;
const AR_TYPE: usize = 0x60;
const AR_TYPE_END: usize = 0x68;

/// Liste d'indices de variables telle que le jeu la passe : données, capacité (u32), taille (u32 en
/// +0x10).
#[repr(C)]
struct VarList {
    data: *const u32,
    cap: u32,
    _pad0: u32,
    len: u32,
    _pad1: u32,
}

type CheckFn = unsafe extern "system-unwind" fn(usize, usize) -> u8;
type DeltaFn = unsafe extern "system-unwind" fn(usize, usize, usize, *const VarList, u64, usize);
type CreateFn = unsafe extern "system-unwind" fn(usize, usize, usize, u64, usize);

static ORIG_CHECK: AtomicUsize = AtomicUsize::new(0);
static ORIG_DELTA: AtomicUsize = AtomicUsize::new(0);
static CREATE: AtomicUsize = AtomicUsize::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 400;

fn log_limited(msg: &str) {
    let n = LOGS.fetch_add(1, Ordering::Relaxed);
    if n < MAX_LOGS {
        log(msg);
    } else if n == MAX_LOGS {
        log("sync repair: log limit reached, further escalations are not logged");
    }
}

// ---------------------------------------------------------------------------------------------
// Réglages

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    Count,
    Create,
    FullDelta,
    Ladder,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub mode: Mode,
    pub attempts: u32,
    pub max_per_check: u32,
    pub bench: Option<String>,
    pub bench_real: bool,
    pub bench_max: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config { mode: Mode::FullDelta, attempts: 3, max_per_check: 32, bench: None, bench_real: false, bench_max: 4 }
    }
}

/// Lignes `clé=valeur`, la dernière l'emporte (comme `read_config` ailleurs dans BGE). Une valeur
/// inconnue laisse la valeur par défaut.
pub fn parse_config(text: &str) -> Config {
    let mut c = Config::default();
    for line in text.lines() {
        let Some((k, v)) = line.trim().split_once('=') else { continue };
        let v = v.trim();
        let num = |d: u32| v.parse::<u32>().unwrap_or(d);
        match k.trim() {
            "reparation_synchro" => {
                c.mode = match v {
                    "off" => Mode::Off,
                    "count" => Mode::Count,
                    "create" => Mode::Create,
                    "fulldelta" => Mode::FullDelta,
                    "ladder" => Mode::Ladder,
                    _ => c.mode,
                }
            }
            "reparation_synchro_essais" => c.attempts = num(c.attempts),
            "reparation_synchro_max" => c.max_per_check = num(c.max_per_check),
            "banc_synchro" => c.bench = (!v.is_empty()).then(|| v.to_ascii_lowercase()),
            "banc_synchro_reel" => c.bench_real = v == "on",
            "banc_synchro_max" => c.bench_max = num(c.bench_max),
            _ => {}
        }
    }
    c
}

static CONFIG: Mutex<Option<(Config, Instant)>> = Mutex::new(None);

fn config(refresh: bool) -> Config {
    let mut g = CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    let stale = g.as_ref().map_or(true, |(_, t)| refresh && t.elapsed() > Duration::from_secs(3));
    if stale {
        // Correctif de stabilité sans réglage joueur : la release garde la configuration par défaut
        // (fulldelta) ; les autres modes et le banc ne sont lus que par les builds de diagnostic.
        let text = if cfg!(feature = "diag") { crate::options::read_config().map(|(t, _)| t).unwrap_or_default() } else { String::new() };
        *g = Some((parse_config(&text), Instant::now()));
    }
    g.as_ref().map(|(c, _)| c.clone()).unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// Décision

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Stock,
    FullDelta,
    Create,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Why {
    /// Aucune variable modifiée, d'un côté ni de l'autre : le delta ne réparerait rien.
    Empty,
    /// Déjà désynchronisée au contrôle précédent du même joueur.
    Consecutive,
}

/// Ce qu'il faut envoyer pour une archive désynchronisée. `streak` : contrôles de suite où cette
/// archive était désynchronisée (celui-ci compris) ; `escalated` : escalades déjà envoyées dans
/// cette série.
pub fn decide(mode: Mode, streak: u32, escalated: u32, empty: bool, attempts: u32, budget_left: bool) -> (Action, Option<Why>) {
    let why = if empty {
        Some(Why::Empty)
    } else if streak >= 2 {
        Some(Why::Consecutive)
    } else {
        None
    };
    if why.is_none() || escalated >= attempts || !budget_left {
        return (Action::Stock, why);
    }
    let action = match mode {
        Mode::Off => Action::Stock,
        Mode::Create => Action::Create,
        Mode::FullDelta | Mode::Count => Action::FullDelta,
        Mode::Ladder if escalated == 0 => Action::FullDelta,
        Mode::Ladder => Action::Create,
    };
    (action, why)
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Track {
    pub streak: u32,
    pub escalated: u32,
    /// La ligne d'abandon a été notée pour cette série.
    pub quiet: bool,
}

/// Archives désynchronisées au dernier contrôle traité, par (joueur, testSync).
#[derive(Default)]
pub struct State {
    prev: HashMap<(i32, bool), HashMap<u64, Track>>,
    cur: Option<(i32, bool, HashMap<u64, Track>, u32, u32)>, // joueur, test, archives, créations, deltas complets
}

impl State {
    pub fn begin(&mut self, player: i32, test: bool) {
        self.cur = Some((player, test, HashMap::new(), 0, 0));
    }

    /// Note un écart au contrôle en cours et rend son suivi, avant la décision de ce contrôle.
    pub fn mismatch(&mut self, key: u64) -> Track {
        let Some((p, t, seen, _, _)) = self.cur.as_mut() else { return Track { streak: 1, ..Track::default() } };
        let before = self.prev.get(&(*p, *t)).and_then(|m| m.get(&key)).copied().unwrap_or_default();
        let now = Track { streak: before.streak + 1, ..before };
        seen.insert(key, now);
        now
    }

    /// Marque la ligne d'abandon comme notée ; vrai la première fois dans une série.
    pub fn quiet(&mut self, key: u64) -> bool {
        let Some((_, _, seen, _, _)) = self.cur.as_mut() else { return false };
        seen.get_mut(&key).is_some_and(|t| !std::mem::replace(&mut t.quiet, true))
    }

    pub fn escalations(&self) -> u32 {
        self.cur.as_ref().map_or(0, |c| c.3 + c.4)
    }

    pub fn escalate(&mut self, key: u64, action: Action) {
        if let Some((_, _, seen, created, full)) = self.cur.as_mut() {
            if let Some(t) = seen.get_mut(&key) {
                t.escalated += 1;
            }
            match action {
                Action::Create => *created += 1,
                Action::FullDelta => *full += 1,
                Action::Stock => {}
            }
        }
    }

    /// Fin du gestionnaire de l'hôte. Un contrôle non traité (hôte pas prêt) reste dans la file et
    /// revient : il ne compte pas. Rend (joueur, test, archives désynchronisées, créations, deltas
    /// complets).
    pub fn end(&mut self, processed: bool) -> Option<(i32, bool, usize, u32, u32)> {
        let (p, t, seen, created, full) = self.cur.take()?;
        if !processed {
            return None;
        }
        let n = seen.len();
        self.prev.insert((p, t), seen);
        Some((p, t, n, created, full))
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(State::default))
}

// ---------------------------------------------------------------------------------------------
// Mémoire du jeu

unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

unsafe fn var_count(archive: usize) -> usize {
    let (b, e) = (rd::<usize>(archive + AR_VARS), rd::<usize>(archive + AR_VARS_END));
    if b == 0 || e < b { 0 } else { (e - b) / 8 }
}

unsafe fn type_name(archive: usize) -> String {
    let (b, e) = (rd::<usize>(archive + AR_TYPE), rd::<usize>(archive + AR_TYPE_END));
    if b == 0 || e <= b || e - b > 128 {
        return String::from("?");
    }
    String::from_utf8_lossy(std::slice::from_raw_parts(b as *const u8, e - b)).trim_end_matches('\0').to_string()
}

/// Nœud annoncé par le client pour `key` dans le contrôle en file, ou 0.
unsafe fn client_node(msg: usize, key: u64) -> usize {
    let (buckets, n) = (rd::<usize>(msg + MSG_BUCKETS), rd::<u32>(msg + MSG_NBUCKETS) as u64);
    if buckets == 0 || n == 0 {
        return 0;
    }
    let mut node = rd::<usize>(buckets + 8 * ((key & 0xffff_ffff) % n) as usize);
    while node != 0 {
        if rd::<u64>(node) == key {
            return node;
        }
        node = rd::<usize>(node + CNODE_NEXT);
    }
    0
}

/// Banc de synchro : les archives dont le type contient `pattern` sont annoncées avec le hachage 0,
/// qui ne correspond jamais (l'hôte traite aussi son propre hachage 0 comme un écart).
unsafe fn bench_force(mgr: usize, msg: usize, pattern: &str, max: u32) -> Vec<String> {
    let mut forced = Vec::new();
    let (buckets, n) = (rd::<usize>(mgr + MGR_BUCKETS), rd::<u32>(mgr + MGR_NBUCKETS) as usize);
    if buckets == 0 || n == 0 || n > 1 << 24 {
        return forced;
    }
    for i in 0..n {
        let mut node = rd::<usize>(buckets + 8 * i);
        while node != 0 && (forced.len() as u32) < max {
            let archive = rd::<usize>(node + HNODE_ARCHIVE);
            if archive != 0 {
                let name = type_name(archive);
                if name.to_ascii_lowercase().contains(pattern) {
                    let c = client_node(msg, rd::<u64>(node));
                    if c != 0 {
                        std::ptr::write_unaligned((c + CNODE_HASH) as *mut u32, 0);
                        forced.push(format!("{name}#{:x}", rd::<u64>(node)));
                    }
                }
            }
            node = rd::<usize>(node + HNODE_NEXT);
        }
    }
    forced
}

// ---------------------------------------------------------------------------------------------
// Détours

/// Remplace l'appel du parcours de la file au gestionnaire de l'hôte : encadre un contrôle traité.
unsafe extern "system-unwind" fn check_hook(mgr: usize, msg: usize) -> u8 {
    let orig: CheckFn = std::mem::transmute(ORIG_CHECK.load(Ordering::Relaxed));
    if mgr == 0 || msg == 0 {
        return orig(mgr, msg);
    }
    let cfg = config(true);
    let (player, test) = (rd::<i32>(msg + MSG_PLAYER), rd::<u8>(msg + MSG_TEST) != 0);
    with_state(|s| s.begin(player, test));
    if let Some(pattern) = cfg.bench.as_deref() {
        if test || cfg.bench_real {
            let forced = bench_force(mgr, msg, pattern, cfg.bench_max);
            if !forced.is_empty() {
                log_limited(&format!(
                    "sync repair: bench, player {player}{}: forced out of sync: {}",
                    if test { " (testSync)" } else { "" },
                    forced.join(", ")
                ));
            }
        }
    }
    let processed = orig(mgr, msg);
    if let Some((p, t, n, created, full)) = with_state(|s| s.end(processed != 0)) {
        if n > 0 {
            log_limited(&format!(
                "sync repair: player {p}{}: {n} archive(s) out of sync, {created} sent as creation data, {full} as full delta{}",
                if t { " (testSync)" } else { "" },
                if cfg.mode == Mode::Count { " (count mode: would have been)" } else { "" }
            ));
        }
    }
    processed
}

/// Remplace l'appel du gestionnaire de l'hôte au constructeur de delta.
unsafe extern "system-unwind" fn delta_hook(mgr: usize, snap: usize, stream: usize, vars: *const VarList, key: u64, archive: usize) {
    let orig: DeltaFn = std::mem::transmute(ORIG_DELTA.load(Ordering::Relaxed));
    let n_vars = if archive == 0 { 0 } else { var_count(archive) };
    if vars.is_null() || n_vars == 0 || n_vars > 1 << 20 {
        return orig(mgr, snap, stream, vars, key, archive);
    }
    let cfg = config(false);
    let list = &*vars;
    let client = if list.data.is_null() || list.len > 1 << 20 {
        0
    } else {
        std::slice::from_raw_parts(list.data, list.len as usize).iter().filter(|&&i| (i as usize) < n_vars).count()
    };
    let host = rd::<u64>(archive + AR_DIRTY_LEN);
    let empty = host == 0 && client == 0;
    let gave_up = |t: &Track| t.escalated >= cfg.attempts;
    let (action, why, track, say) = with_state(|s| {
        let track = s.mismatch(key);
        let (action, why) = decide(cfg.mode, track.streak, track.escalated, empty, cfg.attempts, s.escalations() < cfg.max_per_check);
        if action != Action::Stock {
            s.escalate(key, action);
        }
        // La ligne d'abandon une fois par série, toute autre décision à chaque fois.
        let say = why.is_some() && (action != Action::Stock || !gave_up(&track) || s.quiet(key));
        (action, why, track, say)
    });
    if let (Some(why), true) = (why, say) {
        let what = match (action, cfg.mode) {
            (Action::Stock, _) if gave_up(&track) => format!("stock delta (gave up after {} escalations)", track.escalated),
            (Action::Stock, _) => "stock delta (escalation budget for this check used up)".to_string(),
            (a, Mode::Count) => format!("{a:?} (count mode, not sent)"),
            (a, _) => format!("{a:?}"),
        };
        {
            log_limited(&format!(
                "sync repair: {}#{key:x} out of sync {} check(s) in a row ({why:?}; {host} host + {client} client modified of {n_vars}) -> {what}",
                type_name(archive),
                track.streak
            ));
        }
    }
    match (action, cfg.mode) {
        (Action::Stock, _) | (_, Mode::Count) => orig(mgr, snap, stream, vars, key, archive),
        (Action::FullDelta, _) => {
            // Le constructeur copie la liste (`140037880`) avant de sérialiser : la nôtre reste à nous.
            let all: Vec<u32> = (0..n_vars as u32).collect();
            let full = VarList { data: all.as_ptr(), cap: n_vars as u32, _pad0: 0, len: n_vars as u32, _pad1: 0 };
            orig(mgr, snap, stream, &full, key, archive);
        }
        (Action::Create, _) => {
            let create: CreateFn = std::mem::transmute(CREATE.load(Ordering::Relaxed));
            create(mgr, snap, stream, key, archive);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Recherche et installation

fn parse_sig(s: &str) -> Vec<Option<u8>> {
    s.split_whitespace().map(|t| if t == "??" { None } else { u8::from_str_radix(t, 16).ok() }).collect()
}

fn find_all(hay: &[u8], pat: &[Option<u8>]) -> Vec<usize> {
    let Some(Some(first)) = pat.first().copied() else { return Vec::new() };
    let mut out = Vec::new();
    let mut i = 0;
    while i + pat.len() <= hay.len() {
        match hay[i..hay.len() - pat.len() + 1].iter().position(|&b| b == first) {
            None => break,
            Some(k) => i += k,
        }
        if pat.iter().zip(&hay[i..]).all(|(p, b)| p.map_or(true, |p| p == *b)) {
            out.push(i);
        }
        i += 1;
    }
    out
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[derive(Debug, PartialEq)]
pub struct Sites {
    /// Adresses à l'exécution (ou relatives au fichier) : `text_va` + offset. Les fonctions du jeu,
    /// même quand un autre module (`sync_notes`) a déjà redirigé l'appel.
    pub check_call: usize,
    pub handler: usize,
    pub delta_call: usize,
    pub delta_builder: usize,
    pub create_call: usize,
    pub create_builder: usize,
    /// Ce que les appels du contrôle et du delta atteignent maintenant : la fonction du jeu, ou le
    /// relais du module qui les a redirigés le premier. Ce module appelle ceux-ci : les deux modules
    /// voient chaque appel.
    pub check_target: usize,
    pub delta_target: usize,
}

/// Trouve les trois sites d'appel dans `.text` et les recoupe. `text_va` est l'adresse de `text[0]`.
pub fn locate(text: &[u8], text_va: usize) -> Result<Sites, String> {
    let at = |va: usize, n: usize| -> &[u8] {
        let o = va.wrapping_sub(text_va);
        if o < text.len() { &text[o..(o + n).min(text.len())] } else { &[] }
    };
    // (site d'appel, fonction du jeu, ce que l'appel atteint maintenant).
    let call = |name: &str, sig: &str, k: usize| -> Result<(usize, usize, usize), String> {
        let hits = find_all(text, &parse_sig(sig));
        let [h] = hits[..] else { return Err(format!("{name}: {} matches instead of 1", hits.len())) };
        let site = text_va + h + k;
        let rel = i32::from_le_bytes(text[h + k + 1..h + k + 5].try_into().unwrap());
        let now = (site as i64 + 5 + rel as i64) as usize;
        Ok((site, crate::sync_notes::original_target(site).unwrap_or(now), now))
    };
    let (check_call, handler, check_target) = call("check", SIG_CHECK, SIG_CHECK_CALL)?;
    let (delta_call, delta_builder, delta_target) = call("delta", SIG_DELTA, SIG_DELTA_CALL)?;
    let (create_call, create_builder, _) = call("create", SIG_CREATE, SIG_CREATE_CALL)?;
    for (name, site) in [("delta", delta_call), ("create", create_call)] {
        if !(handler..handler + 0x2000).contains(&site) {
            return Err(format!("{name} call site {site:#x} outside the host handler {handler:#x}"));
        }
    }
    if !contains(at(delta_builder, 0x40), DELTA_MARK) {
        return Err(format!("delta builder {delta_builder:#x}: unexpected bytes (already redirected?)"));
    }
    if !contains(at(create_builder, 0x40), CREATE_MARK) {
        return Err(format!("creation builder {create_builder:#x}: unexpected bytes"));
    }
    let head = at(handler, 0x400);
    if let Some(i) = HANDLER_MARKS.iter().position(|m| !contains(head, m)) {
        return Err(format!("host handler {handler:#x}: field mark {i} missing"));
    }
    let code = at(delta_builder, 0x80);
    let serializer = (0..code.len().saturating_sub(5)).filter(|&i| code[i] == 0xe8).any(|i| {
        let t = (delta_builder as i64 + i as i64 + 5 + i32::from_le_bytes(code[i + 1..i + 5].try_into().unwrap()) as i64) as usize;
        SERIALIZER_MARKS.iter().all(|m| contains(at(t, 0x120), m))
    });
    if !serializer {
        return Err("delta serializer: archive layout marks missing".to_string());
    }
    Ok(Sites { check_call, handler, delta_call, delta_builder, create_call, create_builder, check_target, delta_target })
}

/// Appelé par DllMain : aucun code de l'exe n'a encore tourné.
pub unsafe fn install() {
    let text_cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&text_cfg) {
        log("sync repair: not installed (actif=off)");
        return;
    }
    let cfg = config(false);
    if cfg.mode == Mode::Off {
        log("sync repair: not installed (reparation_synchro=off)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let Some((start, size)) = exe::text_section(module) else {
        log("sync repair: .text of the exe not found, not installed");
        return;
    };
    let text = std::slice::from_raw_parts(start as *const u8, size);
    let sites = match locate(text, start) {
        Ok(s) => s,
        Err(e) => {
            log(&format!("sync repair: unknown exe ({e}), not installed"));
            return;
        }
    };
    let Some(page) = patch::alloc_near(module) else {
        log("sync repair: no memory within 2 GB of the exe, not installed");
        return;
    };
    // Deux relais : jmp [rip+0] ; dq détour. Des sauts, pas des appels : la pile reste celle de
    // l'appel d'origine.
    for (k, hook) in [(0usize, check_hook as CheckFn as usize), (16, delta_hook as DeltaFn as usize)] {
        let mut code = vec![0xff, 0x25, 0, 0, 0, 0];
        code.extend_from_slice(&(hook as u64).to_le_bytes());
        std::ptr::copy_nonoverlapping(code.as_ptr(), (page + k) as *mut u8, code.len());
    }
    let (Some(r_check), Some(r_delta)) = (patch::rel32(sites.check_call, page), patch::rel32(sites.delta_call, page + 16)) else {
        log("sync repair: relay out of range, not installed");
        return;
    };
    // Enchaîné après `sync_notes` quand il est posé : il voit toujours chaque contrôle et chaque
    // archive réparée par delta. Les données de création vont directement au constructeur du jeu :
    // une archive que le client possède n'est pas un objet qui lui manque.
    ORIG_CHECK.store(sites.check_target, Ordering::Relaxed);
    ORIG_DELTA.store(sites.delta_target, Ordering::Relaxed);
    CREATE.store(sites.create_builder, Ordering::Relaxed);
    // Le delta d'abord : tant que l'appel du contrôle n'est pas redirigé, un appel au détour du delta
    // n'a pas de contrôle en cours et compte comme un premier écart, ce qui est de toute façon la
    // décision d'origine.
    patch::write_rel32(sites.delta_call, r_delta);
    patch::write_rel32(sites.check_call, r_check);
    let ghidra = |a: usize| a - module + 0x1_4000_0000;
    log(&format!(
        "sync repair: installed, mode {:?} (host handler {:#x}, delta {:#x} -> {:#x}, creation {:#x}){}",
        cfg.mode,
        ghidra(sites.handler),
        ghidra(sites.delta_call),
        ghidra(sites.delta_builder),
        ghidra(sites.create_builder),
        cfg.bench.as_deref().map(|b| format!(", bench on '{b}'")).unwrap_or_default()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decisions() {
        use Action::*;
        // Premier écart, avec des variables modifiées : le delta d'origine peut le réparer.
        assert_eq!(decide(Mode::Create, 1, 0, false, 3, true), (Stock, None));
        // Rien de modifié, d'un côté ni de l'autre : le delta d'origine ne porterait aucune variable.
        assert_eq!(decide(Mode::Create, 1, 0, true, 3, true), (Create, Some(Why::Empty)));
        // Deuxième contrôle de suite.
        assert_eq!(decide(Mode::Create, 2, 0, false, 3, true), (Create, Some(Why::Consecutive)));
        assert_eq!(decide(Mode::FullDelta, 2, 0, false, 3, true), (FullDelta, Some(Why::Consecutive)));
        // Ladder : delta complet d'abord, puis données de création.
        assert_eq!(decide(Mode::Ladder, 2, 0, false, 3, true).0, FullDelta);
        assert_eq!(decide(Mode::Ladder, 3, 1, false, 3, true).0, Create);
        // Le mode count annonce ce que ferait le mode par défaut, fulldelta (le détour ne l'envoie pas).
        assert_eq!(decide(Mode::Count, 2, 0, false, 3, true).0, FullDelta);
        // Abandon après `attempts` escalades, et respect du budget par contrôle.
        assert_eq!(decide(Mode::Create, 5, 3, false, 3, true), (Stock, Some(Why::Consecutive)));
        assert_eq!(decide(Mode::Create, 2, 0, false, 3, false), (Stock, Some(Why::Consecutive)));
    }

    #[test]
    fn streaks_per_player_and_kind() {
        let mut s = State::default();
        s.begin(1, false);
        assert_eq!(s.mismatch(0xaa).streak, 1);
        s.escalate(0xaa, Action::Create);
        assert_eq!(s.end(true), Some((1, false, 1, 1, 0)));
        // Un autre joueur, ou un contrôle de test, ne voit pas la vraie série du joueur 1.
        s.begin(2, false);
        assert_eq!(s.mismatch(0xaa).streak, 1);
        s.end(true);
        s.begin(1, true);
        assert_eq!(s.mismatch(0xaa).streak, 1);
        s.end(true);
        // De nouveau le joueur 1 : deuxième de suite, une escalade déjà envoyée.
        s.begin(1, false);
        assert_eq!(s.mismatch(0xaa), Track { streak: 2, escalated: 1, quiet: false });
        // Non traité (hôte pas prêt) : ne compte pas.
        assert_eq!(s.end(false), None);
        s.begin(1, false);
        assert_eq!(s.mismatch(0xaa).streak, 2);
        s.end(true);
        // Synchronisée à ce contrôle : la série s'arrête.
        s.begin(1, false);
        s.end(true);
        s.begin(1, false);
        assert_eq!(s.mismatch(0xaa), Track { streak: 1, escalated: 0, quiet: false });
    }

    #[test]
    fn settings() {
        assert_eq!(parse_config(""), Config::default());
        assert_eq!(Config::default().mode, Mode::FullDelta);
        let c = parse_config("actif=on\nreparation_synchro=ladder\nreparation_synchro_essais=5\nbanc_synchro=Player\nbanc_synchro_reel=on\nreparation_synchro=nonsense\n");
        assert_eq!(c.mode, Mode::Ladder);
        assert_eq!(c.attempts, 5);
        assert_eq!(c.bench.as_deref(), Some("player"));
        assert!(c.bench_real);
        assert_eq!(parse_config("reparation_synchro=off").mode, Mode::Off);
        assert_eq!(parse_config("banc_synchro=").bench, None);
    }

    #[test]
    fn wildcard_search() {
        let pat = parse_sig("48 ?? 03");
        assert_eq!(find_all(&[0x00, 0x48, 0x77, 0x03, 0x48, 0x01, 0x03, 0x48], &pat), vec![1, 4]);
        assert!(find_all(&[0x48, 0x03], &pat).is_empty());
    }

    /// `.text` d'un fichier PE : (octets, adresse Ghidra de son premier octet).
    fn text_of(file: &[u8]) -> (&[u8], usize) {
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        let s = (0..rd16(nt + 6)).map(|i| sections + 40 * i).find(|&s| &file[s..s + 5] == b".text").unwrap();
        let (va, raw, rawsize) = (rd32(s + 12), rd32(s + 20), rd32(s + 16));
        (&file[raw..raw + rawsize], 0x1_4000_0000 + va)
    }

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    /// Gestionnaire, appel du delta et création du constructeur attendus dans chaque exe, dans
    /// l'ordre de `crate::test_exe` (1.0.12.72, 1.0.12.74, 1.0.12.58) : ceux des tables de
    /// `sync_notes`, dont ce module reprend les appels.
    const EXPECTED: [(&str, usize, usize, usize); 6] = [
        ("CivilizationVI_DX12.exe", 0x1_407f_65b0, 0x1_407f_717d, 0x1_407f_88c0),
        ("CivilizationVI.exe", 0x1_407f_46c0, 0x1_407f_528d, 0x1_407f_69d0),
        ("CivilizationVI_DX12.exe", 0x1_407f_65e0, 0x1_407f_71ad, 0x1_407f_88f0),
        ("CivilizationVI.exe", 0x1_407f_53d0, 0x1_407f_5f9d, 0x1_407f_76e0),
        ("CivilizationVI_DX12.exe", 0x1_407f_0b70, 0x1_407f_173d, 0x1_407f_2e80),
        ("CivilizationVI.exe", 0x1_407f_0630, 0x1_407f_11fd, 0x1_407f_2940),
    ];

    /// Les signatures dans chaque exe connu qui est là : uniques, et menant aux adresses
    /// attendues.
    #[test]
    fn sites_in_installed_exes() {
        let mut seen = 0;
        for (i, &(name, handler, delta_call, create_builder)) in EXPECTED.iter().enumerate() {
            let path = crate::test_exe(EXE_DIR, i, name);
            let Ok(file) = std::fs::read(&path) else {
                eprintln!("{path} missing, skipped");
                continue;
            };
            seen += 1;
            let (text, va) = text_of(&file);
            let s = locate(text, va).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!((s.handler, s.delta_call, s.create_builder), (handler, delta_call, create_builder), "{path}");
        }
        if seen == 0 {
            eprintln!("no game exe found, test skipped");
        }
    }
}
