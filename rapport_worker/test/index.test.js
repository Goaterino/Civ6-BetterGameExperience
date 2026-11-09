// Tests hors Cloudflare du relais : node --test rapport_worker/test/
// R2, la limite par IP et Discord sont simulés.
import { test } from "node:test";
import assert from "node:assert/strict";
import { deflateRawSync } from "node:zlib";
import worker, { lireMeta, brut, steamId, faireDeLaPlace, CLE_CLIENT, MAX_OCTETS, PLAFOND_OCTETS } from "../src/index.js";

// Un zip comme celui du proxy : meta.json en premier, en deflate.
function zip(meta) {
	const data = deflateRawSync(Buffer.from(JSON.stringify(meta)));
	const nom = Buffer.from("meta.json");
	const h = Buffer.alloc(30);
	h.writeUInt32LE(0x04034b50, 0);
	h.writeUInt16LE(20, 4);
	h.writeUInt16LE(8, 8);
	h.writeUInt32LE(data.length, 18);
	h.writeUInt16LE(nom.length, 26);
	const fin = Buffer.alloc(22);
	fin.writeUInt32LE(0x06054b50, 0);
	return new Uint8Array(Buffer.concat([h, nom, data, fin]));
}

const META = { joueur: "Jean @everyone", tour: "87", civilisation: "France", multijoueur: "1", hote: "1", en_pause: "1", humains: "4",
	description: "La partie se désynchronise *à chaque* tour après la guerre @here",
	steam_id: "76561198000000042", steam_id_windows: "76561198000000099",
	bge_release: "0.7.1", bge_version: "0.7.1", exe: "CivilizationVI_DX12.exe", langue: "fr_FR",
	fichiers: [{ nom: "logs/Lua.log", octets: 10 }, { nom: "saves/BGE_rapport.Civ6Save", octets: 5 }] };

function env(options = {}) {
	const objets = new Map();
	const discord = [];
	globalThis.fetch = async (url, init) => {
		discord.push({ url, corps: JSON.parse(init.body) });
		return new Response("{}", { status: options.discordEnPanne ? 500 : 200 });
	};
	return {
		objets, discord,
		DISCORD_WEBHOOK: "https://discord.example/api/webhooks/1/x",
		DISCORD_TAG: options.tag,
		PLAFOND_OCTETS: options.plafond,
		LIMITE: { limit: async () => ({ success: !options.limite }) },
		RAPPORTS: {
			head: async (k) => (objets.has(k) ? {} : null),
			put: async (k, v, o) => objets.set(k, { data: v, size: v.byteLength, uploaded: new Date(Date.now() + objets.size), ...o }),
			delete: async (ks) => { for (const k of [].concat(ks)) objets.delete(k); },
			// Pages de 2 objets, pour essayer la pagination.
			list: async ({ prefix, cursor }) => {
				const tout = [...objets].filter(([k]) => k.startsWith(prefix)).map(([key, o]) => ({ key, size: o.size, uploaded: o.uploaded }));
				const debut = Number(cursor ?? 0);
				const page = tout.slice(debut, debut + 2);
				return { objects: page, truncated: debut + 2 < tout.length, cursor: String(debut + 2) };
			},
			get: async (k) => {
				const o = objets.get(k);
				return o ? { body: o.data, customMetadata: o.customMetadata } : null;
			},
		},
	};
}

function envoi(corps, headers = { "x-bge-cle": CLE_CLIENT }) {
	const form = new FormData();
	if (corps) form.append("archive", new Blob([corps], { type: "application/zip" }), "BGE_rapport_1.zip");
	return new Request("https://bge-rapports.exemple.workers.dev/rapport", { method: "POST", body: form, headers });
}

test("meta.json lu dans le zip", async () => {
	assert.deepEqual(await lireMeta(zip(META)), META);
	assert.equal(await lireMeta(new Uint8Array([1, 2, 3])), null);
});

test("rapport reçu, gardé, publié, téléchargeable", async () => {
	const e = env({ tag: "42" });
	const r = await worker.fetch(envoi(zip(META), { "x-bge-version": "0.7.1", "x-bge-cle": CLE_CLIENT }), e);
	assert.equal(r.status, 200);
	const { id } = await r.json();
	assert.match(id, /^BGE-[0-9A-F]{6}$/);
	const objet = e.objets.get(`rapports/${id}.zip`);
	assert.ok(objet);
	assert.equal(objet.customMetadata.version, "0.7.1");
	assert.equal(e.discord.length, 1);
	const { url, corps } = e.discord[0];
	assert.equal(url, "https://discord.example/api/webhooks/1/x?wait=true");
	assert.ok(corps.thread_name.startsWith(`${id} · Jean @everyone · turn 87`));
	assert.deepEqual(corps.allowed_mentions, { parse: [] });
	assert.deepEqual(corps.applied_tags, ["42"]);
	assert.ok(corps.content.startsWith(`**Bug report ${id}**\n\n**Description**\n> La partie se désynchronise \\*à chaque\\* tour après la guerre \\@here\n`), corps.content);
	assert.ok(corps.content.includes("**Player:** Jean \\@everyone (France), turn 87"), corps.content);
	assert.ok(corps.content.includes("**Game:** multiplayer, host, paused, 4 human player(s)"), corps.content);
	assert.ok(corps.content.includes("**Steam:** 76561198000000042 <https://steamcommunity.com/profiles/76561198000000042>"), corps.content);
	assert.ok(corps.content.includes("2 file(s) including 1 save(s)"), corps.content);
	assert.ok(corps.content.length < 2000);
	const lien = corps.content.match(/\*\*Download:\*\* (\S+)/)[1];
	assert.ok(lien.startsWith(`https://bge-rapports.exemple.workers.dev/rapport/${id}?cle=`));
	const d = await worker.fetch(new Request(lien), e);
	assert.equal(d.status, 200);
	assert.equal(d.headers.get("content-disposition"), `attachment; filename="${id}.zip"`);
	assert.deepEqual(new Uint8Array(await d.arrayBuffer()), objet.data);
	// Sans la bonne clé : inconnu.
	assert.equal((await worker.fetch(new Request(lien.replace(/cle=\w+/, "cle=" + "0".repeat(32))), e)).status, 404);
	assert.equal((await worker.fetch(new Request(`https://x/rapport/${id}`), e)).status, 404);
});

test("Discord en panne : le joueur a quand même son numéro", async () => {
	const e = env({ discordEnPanne: true });
	const erreurs = console.error;
	console.error = () => {};
	const r = await worker.fetch(envoi(zip(META)), e);
	console.error = erreurs;
	assert.equal(r.status, 200);
	assert.equal(e.objets.size, 1);
});

test("refus : clé absente ou fausse, avant tout le reste", async () => {
	const e = env();
	assert.equal((await worker.fetch(envoi(zip(META), {}), e)).status, 403);
	assert.equal((await worker.fetch(envoi(zip(META), { "x-bge-cle": "0".repeat(32) }), e)).status, 403);
	assert.equal(e.objets.size, 0);
	assert.equal(e.discord.length, 0);
	// La variable CLE_CLIENT remplace la clé du code.
	const e2 = { ...env(), CLE_CLIENT: "autre" };
	assert.equal((await worker.fetch(envoi(zip(META)), e2)).status, 403);
	assert.equal((await worker.fetch(envoi(zip(META), { "x-bge-cle": "autre" }), e2)).status, 200);
});

test("description longue coupée, absente signalée", async () => {
	const e = env();
	await worker.fetch(envoi(zip({ ...META, description: "x".repeat(5000) })), e);
	await worker.fetch(envoi(zip({ ...META, description: "" })), e);
	assert.ok(e.discord[0].corps.content.length < 2000, String(e.discord[0].corps.content.length));
	assert.ok(e.discord[1].corps.content.includes("**Description**\n> *(no description)*"));
});

test("refus : limite, pas de zip, trop gros, mauvaise route", async () => {
	assert.equal((await worker.fetch(envoi(zip(META)), env({ limite: true }))).status, 429);
	assert.equal((await worker.fetch(envoi(new Uint8Array(100)), env())).status, 400);
	assert.equal((await worker.fetch(envoi(null), env())).status, 400);
	const gros = new Uint8Array(MAX_OCTETS + 1);
	gros.set([0x50, 0x4b, 3, 4]);
	assert.equal((await worker.fetch(envoi(gros), env())).status, 413);
	assert.equal((await worker.fetch(new Request("https://x/"), env())).status, 404);
	assert.equal((await worker.fetch(new Request("https://x/rapport/../x"), env())).status, 404);
});

test("texte des joueurs sans mise en forme", () => {
	assert.equal(brut("**a**\n<@1>"), "\\*\\*a\\*\\* \\<\\@1\\>");
	assert.equal(brut(undefined), "?");
});

test("plafond du bucket : les plus anciens partent d'abord", async () => {
	assert.equal(PLAFOND_OCTETS, 9.5e9);
	const e = env({ plafond: 1000 });
	const ajouter = (id, size, jour) => e.objets.set(`rapports/${id}.zip`, { data: new Uint8Array(0), size, uploaded: new Date(`2026-11-0${jour}T00:00:00Z`) });
	ajouter("BGE-000003", 300, 3);
	ajouter("BGE-000001", 300, 1);
	ajouter("BGE-000004", 300, 4);
	ajouter("BGE-000002", 300, 2);
	e.objets.set("autre/x", { data: new Uint8Array(0), size: 5000, uploaded: new Date(0) });
	const logs = console.log;
	console.log = () => {};
	// 1 200 + 50 > 1 000 : le plus ancien part, et 900 + 50 tient.
	assert.deepEqual(await faireDeLaPlace(e, 50), ["BGE-000001"]);
	assert.deepEqual([...e.objets.keys()].sort(), ["autre/x", "rapports/BGE-000002.zip", "rapports/BGE-000003.zip", "rapports/BGE-000004.zip"]);
	// Pile au plafond : rien ne part. Les objets hors de rapports/ ne comptent pas.
	assert.deepEqual(await faireDeLaPlace(e, 100), []);
	// 900 + 450 : deux rapports partent.
	assert.deepEqual(await faireDeLaPlace(e, 450), ["BGE-000002", "BGE-000003"]);
	// Par l'envoi : 300 + un zip de plus de 700 octets fait partir le dernier ancien.
	const gros = zip({ ...META, bourrage: Array.from({ length: 400 }, (_, i) => (i * 7919) % 1000).join(",") });
	assert.ok(gros.byteLength > 700, `zip de ${gros.byteLength} octets`);
	const r = await worker.fetch(envoi(gros), e);
	console.log = logs;
	assert.equal(r.status, 200);
	const { id } = await r.json();
	assert.ok(!e.objets.has("rapports/BGE-000004.zip"), "le plus ancien est parti");
	assert.ok(e.objets.has(`rapports/${id}.zip`) && e.objets.has("autre/x"));
});

test("message en anglais, sans meta.json lisible", async () => {
	const e = env();
	await worker.fetch(envoi(zip({ ...META, multijoueur: "0" })), e);
	assert.ok(e.discord[0].corps.content.includes("**Game:** single player"), e.discord[0].corps.content);
	const brutZip = new Uint8Array(30); brutZip.set([0x50, 0x4b, 3, 4]);
	await worker.fetch(envoi(brutZip), e);
	const c = e.discord[1].corps.content;
	assert.ok(c.includes("meta.json unreadable") && c.includes("> *(no description)*") && c.includes("**Download:**"), c);
	assert.ok(!/[àéèêç]|joueur|tour |Télécharger|Partie/.test(c), "pas de français : " + c);
});

test("Steam ID : celui du jeu, sinon celui de Windows, sinon inconnu", () => {
	assert.equal(steamId({ steam_id: "76561198000000042", steam_id_windows: "76561198000000099" }), "76561198000000042");
	assert.equal(steamId({ steam_id: "", steam_id_windows: "76561198000000099" }), "76561198000000099");
	assert.equal(steamId({ steam_id: "1234", steam_id_windows: "<@1>" }), null);
	assert.equal(steamId({}), null);
});
