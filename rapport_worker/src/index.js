// Relais des rapports de bug de BetterGameExperience (voir rapport_worker/README.md).
//
// POST /rapport            : reçoit le zip du proxy (champ « archive », multipart), le garde dans
//                            R2, ouvre un fil dans le forum Discord des rapports et rend
//                            {"id": "BGE-XXXXXX"}.
// GET  /rapport/<id>?cle=… : télécharge le zip ; la clé n'est donnée que dans le fil Discord.
//
// Liaisons (wrangler.toml) : RAPPORTS (bucket R2), LIMITE (limite par IP, facultative).
// Secret : DISCORD_WEBHOOK (webhook du salon forum). Variables facultatives : DISCORD_TAG (ID d'un
// tag du forum, posé sur chaque fil), PLAFOND_OCTETS (taille maximale du bucket, 9,5 Go par défaut).

export const MAX_OCTETS = 50 * 1024 * 1024;
// Plafond du bucket, sous les 10 Go gratuits de R2 (compté en Go décimaux, le plus prudent). Avant
// de ranger un rapport, les plus anciens sont supprimés tant que le total, nouveau compris, le
// dépasserait. PLAFOND_OCTETS (variable) le remplace.
export const PLAFOND_OCTETS = 9.5e9;
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
		if (nom !== "meta.json" || taille > 1024 * 1024) return null;
		const debut = 30 + lnom + lextra;
		let data = zip.subarray(debut, debut + taille);
		if (methode === 8) {
			const flux = new Blob([data]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
			data = new Uint8Array(await new Response(flux).arrayBuffer());
		} else if (methode !== 0) {
			return null;
		}
		return JSON.parse(new TextDecoder().decode(data));
	} catch {
		return null;
	}
}

// Texte d'un joueur dans un message Discord : sans mise en forme ni mention.
export function brut(s, max = 100) {
	return String(s ?? "?").replace(/[\r\n]+/g, " ").replace(/([\\`*_~|>#@<\[\]()-])/g, "\\$1").slice(0, max);
}

export function resume(id, meta, octets, lien) {
	const m = meta ?? {};
	const fichiers = Array.isArray(m.fichiers) ? m.fichiers : [];
	const saves = fichiers.filter((f) => String(f.nom).startsWith("saves/")).length;
	const lignes = [
		`**Rapport ${id}**`,
		`Joueur : ${brut(m.joueur)} (${brut(m.civilisation)}), tour ${brut(m.tour, 10)}`,
		`Partie : ${m.multijoueur === "1" ? "multijoueur" : "solo"}${m.hote === "1" ? ", hôte" : ""}${m.en_pause === "1" ? ", en pause" : ""}, ${brut(m.humains, 4)} humain(s)`,
		`BGE : ${brut(m.bge_release, 20)} (${brut(m.bge_version, 20)}), ${brut(m.exe, 40)}, langue ${brut(m.langue, 12)}`,
		`Archive : ${(octets / 1048576).toFixed(1)} Mo, ${fichiers.length} fichiers dont ${saves} sauvegarde(s)`,
		`Télécharger : ${lien}`,
	];
	if (meta == null) lignes.splice(1, 4, "meta.json illisible");
	return lignes.join("\n");
}

// Fait de la place pour `entrant` octets : liste les rapports (1 000 par page), puis supprime les
// plus anciens tant que le total dépasserait le plafond. Rend les numéros supprimés.
export async function faireDeLaPlace(env, entrant) {
	const plafond = Number(env.PLAFOND_OCTETS) > 0 ? Number(env.PLAFOND_OCTETS) : PLAFOND_OCTETS;
	const objets = [];
	let cursor;
	do {
		const page = await env.RAPPORTS.list({ prefix: "rapports/", cursor, limit: 1000 });
		objets.push(...page.objects);
		cursor = page.truncated ? page.cursor : undefined;
	} while (cursor);
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
	const titre = `${id} · ${String(meta?.joueur ?? "?").slice(0, 40)} · tour ${String(meta?.tour ?? "?").slice(0, 6)}`;
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

async function recevoir(request, env) {
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
	let id;
	for (let i = 0; i < 5 && id == null; i++) {
		const essai = `BGE-${aleatoire(3)}`;
		if ((await env.RAPPORTS.head(`rapports/${essai}.zip`)) == null) id = essai;
	}
	if (id == null) return json({ erreur: "numéro de rapport indisponible" }, 503);
	await faireDeLaPlace(env, zip.byteLength);
	const cle = aleatoire(16);
	const meta = await lireMeta(zip);
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
