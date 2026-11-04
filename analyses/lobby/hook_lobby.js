// Hook Frida du salon multijoueur : création, config, infos joueurs et changements d'emplacement.
// Chargé par hook_lobby.py, sur n'importe quelle machine (hôte ou client), exe DX12 1.0.12.54.
// Adresses : projet Ghidra ~/Tools/ghidra-projects/civ6exe (base 0x140000000), voir README.md.
// Chaque site est vérifié octet par octet avant d'être accroché ; rien n'est modifié.
//
// Événements envoyés par send() : { t, tid, kind, ...données }. Le texte complet des messages
// NetGameConfig / NetPlayerInfo (que net_message_debug.log coupe à 32 Ko) part en pièce jointe
// (deuxième argument de send), le script Python l'écrit dans configs/.

const SLOT = { 0: 'OPEN', 1: 'COMPUTER', 2: 'CLOSED', 3: 'TAKEN', 4: 'OBSERVER', 5: 'STATUS_5' };
const slotName = (v) => (v in SLOT ? SLOT[v] : String(v));

function emit(kind, data, blob) {
    send({ t: Date.now(), tid: Process.getCurrentThreadId(), kind, ...data }, blob);
}

function checkBytes(base, rva, hex) {
    const got = base.add(rva).readByteArray(hex.length / 2);
    const s = Array.from(new Uint8Array(got), (b) => b.toString(16).padStart(2, '0')).join('');
    return s === hex;
}

const EXE = {
    // Liaisons Lua de la table Network (enregistrées par 14026d5a0)
    hostGameImpl: { rva: 0x27de80, bytes: '48895c241044884424185556' },                        // NetworkManager::HostGame(serverType, bool)
    broadcastGameConfig: { rva: 0x26acc0, bytes: '48895c2408574881ecf00400' }, // lBroadcastGameConfig
    broadcastPlayerInfo: { rva: 0x26ad60, bytes: '48895c2410564881ecc00000' }, // lBroadcastPlayerInfo
    requestIdChange: { rva: 0x28ee60, bytes: '48895c2408574883ec60488b' },                     // NetworkManager : envoie NetRequestChangePlayerID
    launchGame: { rva: 0x282e20, bytes: '40554156488dac24d8dbffff' },    // NetworkManager::LaunchGame
    isGameHost: { rva: 0x27eff0, bytes: '405553488dac2468e0ffffb8' },    // NetworkManager::IsGameHost
    // Traitement des messages reçus (méthode Process de chaque message)
    procGameConfig: { rva: 0x2662d0, bytes: '48895c241055565741564157' },
    procPlayerInfo: { rva: 0x266800, bytes: '405356b898200000e813cfc3' },
    procRequestChange: { rva: 0x810ae0, bytes: '4885d2745448895c24085748' },
    procChangeId: { rva: 0x80f6a0, bytes: '48895c2408574883ec208b59' },
    // ToString des messages : (this, tampon, taille)
    strGameConfig: { rva: 0x266170, bytes: '48895c240848897424105748' },
    strPlayerInfo: { rva: 0x266270, bytes: '48895c240848897424105748' },
    // FireWire : décision de l'hôte (par joueur, à chaque mise à jour) et application du changement
    decideChange: { rva: 0x814000, bytes: '40535641574881ec80000000' },
    applyChange: { rva: 0x812ca0, bytes: '895424105356574883ec508b' },
    // Rappels de session de l'application (NetSessionCallbacks.cpp)
    playersSwapped: { rva: 0x274660, bytes: '48894c240855535657415441' },
    sendJoinConfig: { rva: 0x2893f0, bytes: '48895c2408574881ec300500' },
    playerConnected: { rva: 0x272910, bytes: '48895c240855565741544155' },
    playerLeft: { rva: 0x273c40, bytes: '48895c241855565741564157' },
    // Config locale : tableau des joueurs (0x40 octets chacun) et accesseurs
    configApp: { rva: 0x100a60, bytes: '4883ec38488b0535e6200248' },
    getSlot: { rva: 0x115ed0, bytes: '4883ec28488b49188b158a91' },
    getNetName: { rva: 0x115bd0, bytes: '4883ec28488b49188b155a94' },
    getReady: { rva: 0x115d30, bytes: '4883ec28488b49188b153293' },
    netManager: 0x2325f48, // pointeur global vers NetworkManager ; +8 : FNetSession
};

let base = null;
let fn = {};

function netManager() {
    return base.add(EXE.netManager).readPointer();
}

function isHost() {
    try {
        const m = netManager();
        return m.isNull() ? null : (fn.isGameHost(m) & 0xff) !== 0;
    } catch (e) {
        return null;
    }
}

function localId() {
    try {
        const sess = netManager().add(8).readPointer();
        if (sess.isNull()) return -1;
        const getLocal = new NativeFunction(sess.readPointer().add(0x270).readPointer(), 'pointer', ['pointer']);
        return getLocal(sess).add(8).readS32();
    } catch (e) {
        return -1;
    }
}

// État d'un emplacement dans la config locale : statut, nom réseau, prêt.
function slot(id) {
    try {
        const players = fn.configApp().add(0x5c * 8).readPointer();
        const p = players.add(id * 0x40);
        return {
            id,
            statut: slotName(fn.getSlot(p)),
            nom: fn.getNetName(p).readUtf8String(),
            pret: (fn.getReady(p) & 0xff) !== 0,
        };
    } catch (e) {
        return { id, erreur: String(e) };
    }
}

const BUF_SIZE = 4 * 1024 * 1024;
let buf = null;
function dump(toStr, msg) {
    try {
        if (buf === null) buf = Memory.alloc(BUF_SIZE);
        buf.writeU8(0);
        toStr(msg, buf, BUF_SIZE);
        // Copie brute jusqu'au zéro final : le texte n'est pas toujours de l'UTF-8 valide
        // (noms de joueurs, JSON de localisation), et le décoder ferait échouer la copie.
        const CHUNK = 65536;
        let len = 0;
        for (let off = 0; off < BUF_SIZE; off += CHUNK) {
            const bytes = new Uint8Array(buf.add(off).readByteArray(Math.min(CHUNK, BUF_SIZE - off)));
            const z = bytes.indexOf(0);
            if (z >= 0) { len = off + z; break; }
            len = off + bytes.length;
        }
        return { len, data: buf.readByteArray(len) };
    } catch (e) {
        return { len: -1, data: null, err: String(e) };
    }
}

// Évite de noter la décision de l'hôte à chaque image : seulement quand la demande change.
const lastDesired = new Map();

function hookExe(mod) {
    base = mod.base;
    for (const k of Object.keys(EXE)) {
        if (typeof EXE[k] !== 'object' || EXE[k].bytes === null) continue;
        if (!checkBytes(base, EXE[k].rva, EXE[k].bytes)) {
            emit('erreur', { msg: `${mod.name} : octets inattendus en ${k}, rien d'accroché (lancer la version DX12 1.0.12.54)` });
            return;
        }
    }
    const at = (k) => base.add(EXE[k].rva);
    fn = {
        isGameHost: new NativeFunction(at('isGameHost'), 'uint8', ['pointer']),
        configApp: new NativeFunction(at('configApp'), 'pointer', []),
        getSlot: new NativeFunction(at('getSlot'), 'int', ['pointer']),
        getNetName: new NativeFunction(at('getNetName'), 'pointer', ['pointer']),
        getReady: new NativeFunction(at('getReady'), 'uint8', ['pointer']),
        strGameConfig: new NativeFunction(at('strGameConfig'), 'void', ['pointer', 'pointer', 'int']),
        strPlayerInfo: new NativeFunction(at('strPlayerInfo'), 'void', ['pointer', 'pointer', 'int']),
    };

    Interceptor.attach(at('hostGameImpl'), {
        onEnter(args) { emit('lua_host_game', { server_type: args[1].toInt32(), arg2: args[2].toInt32() & 0xff }); },
        onLeave(ret) { emit('lua_host_game_fin', { resultat: ret.toInt32() }); },
    });
    Interceptor.attach(at('broadcastGameConfig'), {
        // Ne fait rien si l'on n'est pas l'hôte : c'est ce qu'on veut voir.
        onEnter() { emit('lua_broadcast_game_config', { hote: isHost(), local: localId() }); },
    });
    Interceptor.attach(at('broadcastPlayerInfo'), {
        onEnter() { emit('lua_broadcast_player_info', { hote: isHost(), local: localId() }); },
    });
    Interceptor.attach(at('requestIdChange'), {
        onEnter(args) { emit('lua_request_player_id_change', { local: localId(), desire: args[1].toInt32() }); },
    });
    Interceptor.attach(at('launchGame'), {
        onEnter() { emit('lua_launch_game', { hote: isHost(), local: localId() }); },
        onLeave(ret) { emit('lua_launch_game_fin', { resultat: ret.toInt32() }); },
    });

    Interceptor.attach(at('procGameConfig'), {
        onEnter(args) {
            const msg = args[0];
            this.msg = msg;
            const d = dump(fn.strGameConfig, msg);
            // Liste des sections (racines GAME_ROOT, MAP_ROOT, PLAYER_n) : msg+0x448 (entrées de
            // 8 octets, hachage de la clé en tête), nombre en msg+0x458. Liste vide : tout s'applique.
            const n = msg.add(0x458).readU32();
            const tab = msg.add(0x448).readPointer();
            const sections = [];
            for (let i = 0; i < n && i < 80; i++) sections.push(tab.add(8 * i).readU32() >>> 0);
            emit('recu_game_config', {
                nb_sections: n,
                sections,
                depuis_cloud: msg.add(0x4c0).readU8(),
                taille_texte: d.len,
                ...(d.err ? { erreur_texte: d.err } : {}),
                hote: isHost(),
            }, d.data);
        },
        onLeave() { emit('recu_game_config_fin', {}); },
    });
    Interceptor.attach(at('procPlayerInfo'), {
        onEnter(args) {
            const msg = args[0];
            const id = msg.add(0x38).readS32();
            let emetteur = -1;
            try {
                const s = args[1];
                emetteur = new NativeFunction(s.readPointer().add(0x148).readPointer(), 'int', ['pointer'])(s);
            } catch (e) { /* session absente */ }
            this.id = id;
            const d = dump(fn.strPlayerInfo, msg);
            emit('recu_player_info', { joueur: id, emetteur, avant: id >= 0 && id < 64 ? slot(id) : null, taille_texte: d.len, ...(d.err ? { erreur_texte: d.err } : {}) }, d.data);
        },
        onLeave() { if (this.id >= 0 && this.id < 64) emit('recu_player_info_fin', { apres: slot(this.id) }); },
    });
    Interceptor.attach(at('procRequestChange'), {
        onEnter(args) {
            emit('recu_request_change', { actuel: args[0].add(0x3c).readS32(), desire: args[0].add(0x38).readS32(), local: localId(), hote: isHost() });
        },
    });
    Interceptor.attach(at('procChangeId'), {
        // Le journal du jeu inverse les libellés : « newID » y est l'ancien identifiant (+0x3c).
        onEnter(args) {
            const de = args[0].add(0x3c).readS32();
            const vers = args[0].add(0x38).readS32();
            this.ids = [de, vers];
            emit('recu_change_id', { de, vers, local: localId(), avant: [slot(de), slot(vers)] });
        },
        onLeave() { emit('recu_change_id_fin', { local: localId(), apres: this.ids.map(slot) }); },
    });

    Interceptor.attach(at('decideChange'), {
        onEnter(args) {
            const p = args[1];
            this.p = p;
            try {
                this.id = p.add(0x14).readS32(); // FNetPlayer::GetID (140f78900)
                this.desire = p.add(0x20).readS32();
            } catch (e) {
                this.desire = -1;
            }
        },
        onLeave() {
            if (this.desire === undefined) return;
            const apres = this.p.add(0x20).readS32();
            const cle = `${this.id}`;
            const etat = `${this.desire}>${apres}`;
            if (this.desire !== -1 && lastDesired.get(cle) !== etat) {
                lastDesired.set(cle, etat);
                emit('hote_decision', { joueur: this.id, desire: this.desire, accepte: apres === -1 && this.desire !== -1 });
            } else if (this.desire === -1) {
                lastDesired.delete(cle);
            }
        },
    });
    Interceptor.attach(at('applyChange'), {
        onEnter(args) { emit('firewire_change_id', { de: args[1].toInt32(), vers: args[2].toInt32() }); },
    });
    Interceptor.attach(at('playersSwapped'), {
        onEnter(args) {
            this.a = args[1].toInt32();
            this.b = args[2].toInt32();
            emit('app_players_swapped', { de: this.a, vers: this.b, avant: [slot(this.a), slot(this.b)] });
        },
        onLeave() { emit('app_players_swapped_fin', { apres: [slot(this.a), slot(this.b)] }); },
    });
    Interceptor.attach(at('sendJoinConfig'), {
        onEnter(args) { emit('hote_config_au_nouveau', { joueur: args[1].readS32(), local: localId() }); },
    });
    Interceptor.attach(at('playerConnected'), {
        onEnter(args) { emit('app_player_connected', { joueur: args[1].toInt32() }); },
    });
    Interceptor.attach(at('playerLeft'), {
        onEnter(args) { emit('app_player_left', { joueur: args[1].toInt32(), net_player_removed: args[2].toInt32() & 0xff }); },
    });
    emit('info', { msg: `${mod.name} accroché en ${base}`, hote: isHost(), local: localId() });
}

const seen = new Set();
function onModule(mod) {
    const name = mod.name.toLowerCase();
    if ((name === 'civilizationvi_dx12.exe' || name === 'civilizationvi.exe') && !seen.has(name)) {
        seen.add(name);
        hookExe(mod);
    }
}
for (const mod of Process.enumerateModules()) onModule(mod);

// Photo des emplacements à la demande (hook_lobby.py : touche Entrée).
rpc.exports = {
    emplacements(n) {
        const out = [];
        for (let i = 0; i < n; i++) out.push(slot(i));
        emit('photo_emplacements', { hote: isHost(), local: localId(), emplacements: out });
    },
};
emit('info', { msg: 'hook prêt' });
