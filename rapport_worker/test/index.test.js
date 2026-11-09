// Tests hors Cloudflare du relais : node --test rapport_worker/test/
// R2, la limite par IP et Discord sont simulés.
import { test } from "node:test";
import assert from "node:assert/strict";
import { deflateRawSync } from "node:zlib";
import worker, { lireMeta, brut, MAX_OCTETS } from "../src/index.js";

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
		LIMITE: { limit: async () => ({ success: !options.limite }) },
		RAPPORTS: {
			head: async (k) => (objets.has(k) ? {} : null),
			put: async (k, v, o) => objets.set(k, { data: v, ...o }),
			get: async (k) => {
				const o = objets.get(k);
				return o ? { body: o.data, customMetadata: o.customMetadata } : null;
			},
		},
	};
}

function envoi(corps, headers = {}) {
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
	const r = await worker.fetch(envoi(zip(META), { "x-bge-version": "0.7.1" }), e);
	assert.equal(r.status, 200);
	const { id } = await r.json();
	assert.match(id, /^BGE-[0-9A-F]{6}$/);
	const objet = e.objets.get(`rapports/${id}.zip`);
	assert.ok(objet);
	assert.equal(objet.customMetadata.version, "0.7.1");
	assert.equal(e.discord.length, 1);
	const { url, corps } = e.discord[0];
	assert.equal(url, "https://discord.example/api/webhooks/1/x?wait=true");
	assert.ok(corps.thread_name.startsWith(`${id} · Jean @everyone · tour 87`));
	assert.deepEqual(corps.allowed_mentions, { parse: [] });
	assert.deepEqual(corps.applied_tags, ["42"]);
	assert.ok(corps.content.includes("Jean \\@everyone (France), tour 87"), corps.content);
	assert.ok(corps.content.includes("multijoueur, hôte, en pause, 4 humain(s)"), corps.content);
	assert.ok(corps.content.includes("2 fichiers dont 1 sauvegarde(s)"), corps.content);
	const lien = corps.content.match(/Télécharger : (\S+)/)[1];
	assert.ok(lien.startsWith(`https://bge-rapports.exemple.workers.dev/rapport/${id}?cle=`));
	const d = await worker.fetch(new Request(lien), e);
	assert.equal(d.status, 200);
	assert.equal(d.headers.get("content-disposition"), `attachment; filename="${id}.zip"`);
	assert.deepEqual(new Uint8Array(await d.arrayBuffer()), objet.data);
	// Sans la bonne clé : inconnu.
	assert.equal((await worker.fetch(new Request(lien.replace(/cle=./, "cle=0")), e)).status, 404);
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
