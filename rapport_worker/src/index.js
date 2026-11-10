// Relais des rapports de bug de BetterGameExperience (voir rapport_worker/README.md).
//
// POST /rapport            : reçoit le zip du proxy (champ « archive », multipart), le garde dans
//                            R2, ouvre un fil dans le forum Discord des rapports et rend
//                            {"id": "BGE-XXXXXX"}.
// GET  /rapport/<id>?cle=… : télécharge le zip ; la clé n'est donnée que dans le fil Discord.
//
// Liaisons (wrangler.toml) : RAPPORTS (bucket R2), LIMITE (limite par IP, facultative).
// Secret : DISCORD_WEBHOOK (webhook du salon forum). Variables facultatives : DISCORD_TAG (ID d'un
// tag du forum, posé sur chaque fil), PLAFOND_OCTETS (taille maximale du bucket, 9,5 Go par défaut),
// QUOTA_RAPPORTS et QUOTA_OCTETS (rapports acceptés sur 24 heures glissantes).

export const MAX_OCTETS = 50 * 1024 * 1024;
// Clé que chaque rapport doit porter (en-tête X-BGE-Cle), gravée dans winmm.dll
// (startup_proxy/src/rapport.rs, CLE_CLIENT ; un test vérifie qu'elles concordent). Elle écarte les
// envois qui ne viennent pas de la DLL ; elle se lit dans la DLL, ce n'est pas un secret. La variable
// CLE_CLIENT la remplace, pour en changer sans republier le Worker.
export const CLE_CLIENT = "16ea4c738b288eb2852868d2670e169e";
// Plafond du bucket, sous les 10 Go gratuits de R2 (compté en Go décimaux, le plus prudent). Avant
// de ranger un rapport, les plus anciens sont supprimés tant que le total, nouveau compris, le
// dépasserait. PLAFOND_OCTETS (variable) le remplace.
export const PLAFOND_OCTETS = 9.5e9;
// Quota sur 24 heures glissantes, tous joueurs confondus : au-delà, les rapports sont refusés (503)
// au lieu de faire partir les anciens. La clé et la limite par IP n'arrêtent pas un envoi de déchets
// répété depuis plusieurs adresses ; le quota borne ce qu'il range dans R2 et poste dans Discord.
// Compté sur les rapports du bucket (date de dépôt de R2), sans autre stockage. QUOTA_RAPPORTS et
// QUOTA_OCTETS (variables) le remplacent.
export const QUOTA_RAPPORTS = 60;
export const QUOTA_OCTETS = 1e9;
// meta.json décompressé, au plus : quelques Ko d'ordinaire (description de 1 000 caractères au plus,
// liste des fichiers).
export const MAX_META = 1024 * 1024;
const ID = /^BGE-[0-9A-F]{6}$/;

function json(data, status = 200) {
	return new Response(JSON.stringify(data), { status, headers: { "content-type": "application/json; charset=utf-8" } });
}

function hex(octets) {
	return [...octets].map((b) => b.toString(16).padStart(2, "0")).join("").toUpperCase();
}

function aleatoire(n) {
	return hex(crypto.getRandomValues(new Uint8Array(n)));
}

// meta.json, la première entrée du zip (startup_proxy/src/rapport.rs) : en-tête local de 30 octets,
// nom, extra, puis les données, brutes (méthode 0) ou en deflate (méthode 8).
export async function lireMeta(zip) {
	try {
		const v = new DataView(zip.buffer, zip.byteOffset, zip.byteLength);
		if (zip.byteLength < 30 || v.getUint32(0, true) !== 0x04034b50) return null;
		const methode = v.getUint16(8, true);
		const taille = v.getUint32(18, true);
		const lnom = v.getUint16(26, true);
		const lextra = v.getUint16(28, true);
		const nom = new TextDecoder().decode(zip.subarray(30, 30 + lnom));
		if (nom !== "meta.json" || taille > MAX_META) return null;
		const debut = 30 + lnom + lextra;
		let data = zip.subarray(debut, debut + taille);
		if (methode === 8) {
			data = await decompresser(data, MAX_META);
			if (data == null) return null;
		} else if (methode !== 0) {
			return null;
		}
		const meta = JSON.parse(new TextDecoder().decode(data));
		return meta != null && typeof meta === "object" && !Array.isArray(meta) ? meta : null;
	} catch {
		return null;
	}
}

// Deflate brut décompressé morceau par morceau : null dès que la sortie passerait `max` octets (une
// bombe de décompression ne remplit pas la mémoire).
async function decompresser(data, max) {
	const lecteur = new Blob([data]).stream().pipeThrough(new DecompressionStream("deflate-raw")).getReader();
	const morceaux = [];
	let total = 0;
	for (;;) {
		const { done, value } = await lecteur.read();
		if (done) break;
		total += value.byteLength;
		if (total > max) {
			await lecteur.cancel().catch(() => {});
			return null;
		}
		morceaux.push(value);
	}
	const out = new Uint8Array(total);
	let i = 0;
	for (const m of morceaux) {
		out.set(m, i);
		i += m.byteLength;
	}
	return out;
}

// Texte d'un joueur dans un message Discord : sans mise en forme ni mention.
export function brut(s, max = 100) {
	return String(s ?? "?").replace(/[\r\n]+/g, " ").replace(/([\\`*_~|>#@<\[\]()-])/g, "\\$1").slice(0, max);
}

// Steam ID 64 bits du joueur : celui du jeu, sinon le compte Steam connecté sur la machine.
export function steamId(m) {
	return [m.steam_id, m.steam_id_windows].map((v) => String(v ?? "")).find((v) => /^7656\d{13}$/.test(v)) ?? null;
}

// Message du fil Discord, en anglais : titre, description du joueur, puis le résumé de la partie.
// Il tient sous les 2 000 caractères d'un message Discord (description coupée à 1 400).
export function resume(id, meta, octets, lien) {
	const m = meta ?? {};
	const steam = steamId(m);
	const fichiers = Array.isArray(m.fichiers) ? m.fichiers : [];
	const saves = fichiers.filter((f) => String(f.nom).startsWith("saves/")).length;
	const description = String(m.description ?? "").trim();
	const lignes = [
		`**Bug report ${id}**`,
		"",
		"**Description**",
		description ? `> ${brut(description, 1400)}` : "> *(no description)*",
		"",
	];
	if (meta == null) {
		lignes.push("meta.json unreadable");
	} else {
		const partie = m.multijoueur === "1"
			? `multiplayer${m.hote === "1" ? ", host" : ""}${m.en_pause === "1" ? ", paused" : ""}, ${brut(m.humains, 4)} human player(s)`
			: "single player";
		lignes.push(
			`**Player:** ${brut(m.joueur)} (${brut(m.civilisation)}), turn ${brut(m.tour, 10)}`,
			steam ? `**Steam:** ${steam} <https://steamcommunity.com/profiles/${steam}>` : "**Steam:** unknown",
			`**Game:** ${partie}`,
			`**BGE:** ${brut(m.bge_release, 20)} (${brut(m.bge_version, 20)}), ${brut(m.exe, 40)}, language ${brut(m.langue, 12)}`,
		);
	}
	lignes.push(
		`**Archive:** ${(octets / 1048576).toFixed(1)} MB, ${fichiers.length} file(s) including ${saves} save(s)`,
		`**Download:** ${lien}`,
	);
	return lignes.join("\n");
}

function nombre(v, defaut) {
	return Number(v) > 0 ? Number(v) : defaut;
}

// Les rapports du bucket (1 000 par page) : clé, taille, date de dépôt.
export async function listerRapports(env) {
	const objets = [];
	let cursor;
	do {
		const page = await env.RAPPORTS.list({ prefix: "rapports/", cursor, limit: 1000 });
		objets.push(...page.objects);
		cursor = page.truncated ? page.cursor : undefined;
	} while (cursor);
	return objets;
}

// Quota des 24 dernières heures : rend la raison du refus si le rapport de `entrant` octets le
// dépasserait, sinon null.
export function quotaDepasse(env, objets, entrant, maintenant = Date.now()) {
	const quotaRapports = nombre(env.QUOTA_RAPPORTS, QUOTA_RAPPORTS);
	const quotaOctets = nombre(env.QUOTA_OCTETS, QUOTA_OCTETS);
	const recents = objets.filter((o) => maintenant - new Date(o.uploaded).getTime() < 86400e3);
	const octets = recents.reduce((t, o) => t + o.size, 0);
	if (recents.length + 1 > quotaRapports) return `${recents.length} rapport(s) en 24 h, quota de ${quotaRapports}`;
	if (octets + entrant > quotaOctets) return `${octets} octets en 24 h, quota de ${quotaOctets}`;
	return null;
}

// Fait de la place pour `entrant` octets : supprime les plus anciens rapports tant que le total
// dépasserait le plafond. `objets` : la liste déjà lue, sinon le bucket est listé. Rend les numéros
// supprimés.
export async function faireDeLaPlace(env, entrant, objets) {
	const plafond = nombre(env.PLAFOND_OCTETS, PLAFOND_OCTETS);
	objets = [...(objets ?? (await listerRapports(env)))];
	let total = objets.reduce((t, o) => t + o.size, 0);
	if (total + entrant <= plafond) return [];
	objets.sort((a, b) => new Date(a.uploaded) - new Date(b.uploaded));
	const supprimes = [];
	for (const o of objets) {
		if (total + entrant <= plafond) break;
		supprimes.push(o.key);
		total -= o.size;
	}
	for (let i = 0; i < supprimes.length; i += 1000) await env.RAPPORTS.delete(supprimes.slice(i, i + 1000));
	const ids = supprimes.map((k) => k.replace(/^rapports\/|\.zip$/g, ""));
	console.log(`plafond de ${plafond} octets : ${ids.length} rapport(s) supprimé(s) (${ids.join(", ")}), reste ${total} octets`);
	return ids;
}

async function publierDiscord(env, id, meta, octets, lien) {
	const titre = `${id} · ${String(meta?.joueur ?? "?").slice(0, 40)} · turn ${String(meta?.tour ?? "?").slice(0, 6)}`;
	const corps = {
		thread_name: titre.replace(/[\r\n]+/g, " "),
		content: resume(id, meta, octets, lien),
		allowed_mentions: { parse: [] },
	};
	if (env.DISCORD_TAG) corps.applied_tags = [env.DISCORD_TAG];
	const r = await fetch(`${env.DISCORD_WEBHOOK}?wait=true`, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify(corps),
	});
	if (!r.ok) throw new Error(`Discord ${r.status} : ${(await r.text()).slice(0, 300)}`);
}

// Prévient l'équipe sur Discord que le quota refuse des rapports : une fois par 24 heures au plus,
// retenue par un objet du bucket hors de `rapports/` (ni compté ni supprimé par le plafond). Un
// envoi en boucle n'ouvre donc qu'un fil par jour.
const ALERTE_QUOTA = "etat/alerte_quota";
export async function alerterQuota(env, raison, maintenant = Date.now()) {
	try {
		const vu = await env.RAPPORTS.head(ALERTE_QUOTA);
		if (vu != null && maintenant - new Date(vu.uploaded).getTime() < 86400e3) return false;
		await env.RAPPORTS.put(ALERTE_QUOTA, raison);
		const r = await fetch(`${env.DISCORD_WEBHOOK}?wait=true`, {
			method: "POST",
			headers: { "content-type": "application/json" },
			body: JSON.stringify({
				thread_name: "Bug report quota reached",
				content: `**Bug report quota reached**\nNew reports are refused until older ones leave the 24-hour window: ${brut(raison, 200)}.\nPlayers keep their report on their Desktop. If this is not real traffic, look at the Worker logs.`,
				allowed_mentions: { parse: [] },
			}),
		});
		if (!r.ok) throw new Error(`Discord ${r.status} : ${(await r.text()).slice(0, 300)}`);
		return true;
	} catch (e) {
		console.error(`alerte de quota : ${e.message}`);
		return false;
	}
}

async function recevoir(request, env) {
	if ((request.headers.get("x-bge-cle") ?? "") !== (env.CLE_CLIENT || CLE_CLIENT)) return json({ erreur: "clé absente ou invalide" }, 403);
	const ip = request.headers.get("cf-connecting-ip") ?? "?";
	if (env.LIMITE) {
		const { success } = await env.LIMITE.limit({ key: ip });
		if (!success) return json({ erreur: "trop de rapports, réessayez dans une minute" }, 429);
	}
	const annonce = Number(request.headers.get("content-length") ?? 0);
	if (annonce > MAX_OCTETS + 65536) return json({ erreur: "archive trop grosse" }, 413);
	let archive;
	try {
		archive = (await request.formData()).get("archive");
	} catch {
		return json({ erreur: "formulaire illisible" }, 400);
	}
	if (archive == null || typeof archive === "string") return json({ erreur: "champ archive manquant" }, 400);
	if (archive.size > MAX_OCTETS) return json({ erreur: "archive trop grosse" }, 413);
	const zip = new Uint8Array(await archive.arrayBuffer());
	if (zip.byteLength < 22 || zip[0] !== 0x50 || zip[1] !== 0x4b || zip[2] !== 3 || zip[3] !== 4) {
		return json({ erreur: "ce n'est pas un zip" }, 400);
	}
	// Sans meta.json lisible en première entrée, ce n'est pas un rapport du jeu : refusé avant tout
	// rangement, toute suppression et tout message.
	const meta = await lireMeta(zip);
	if (meta == null) return json({ erreur: "meta.json absent ou illisible" }, 400);
	const objets = await listerRapports(env);
	const quota = quotaDepasse(env, objets, zip.byteLength);
	if (quota != null) {
		console.log(`rapport refusé : ${quota}`);
		await alerterQuota(env, quota);
		return json({ erreur: "trop de rapports aujourd'hui, réessayez demain" }, 503);
	}
	let id;
	for (let i = 0; i < 5 && id == null; i++) {
		const essai = `BGE-${aleatoire(3)}`;
		if ((await env.RAPPORTS.head(`rapports/${essai}.zip`)) == null) id = essai;
	}
	if (id == null) return json({ erreur: "numéro de rapport indisponible" }, 503);
	await faireDeLaPlace(env, zip.byteLength, objets);
	const cle = aleatoire(16);
	await env.RAPPORTS.put(`rapports/${id}.zip`, zip, {
		httpMetadata: { contentType: "application/zip" },
		customMetadata: { cle, version: request.headers.get("x-bge-version") ?? "", recu: new Date().toISOString() },
	});
	const lien = `${new URL(request.url).origin}/rapport/${id}?cle=${cle}`;
	try {
		await publierDiscord(env, id, meta, zip.byteLength, lien);
	} catch (e) {
		// Le zip est gardé : le joueur a son numéro, l'équipe le retrouve dans R2.
		console.error(`rapport ${id} : ${e.message}`);
	}
	return json({ id });
}

async function telecharger(url, env, id) {
	if (!ID.test(id)) return json({ erreur: "numéro inconnu" }, 404);
	const objet = await env.RAPPORTS.get(`rapports/${id}.zip`);
	const cle = url.searchParams.get("cle") ?? "";
	if (objet == null || cle.length !== 32 || objet.customMetadata?.cle !== cle) return json({ erreur: "numéro inconnu" }, 404);
	return new Response(objet.body, {
		headers: { "content-type": "application/zip", "content-disposition": `attachment; filename="${id}.zip"` },
	});
}

export default {
	async fetch(request, env) {
		const url = new URL(request.url);
		if (url.pathname === "/rapport" && request.method === "POST") return recevoir(request, env);
		const m = url.pathname.match(/^\/rapport\/([^/]+)$/);
		if (m && request.method === "GET") return telecharger(url, env, m[1]);
		return json({ erreur: "introuvable" }, 404);
	},
};
