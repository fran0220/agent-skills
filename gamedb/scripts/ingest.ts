#!/usr/bin/env npx tsx
/**
 * GameDB Ingest Script
 *
 * Reads the extracted gamedb-v3-knowledge-base directory,
 * then POSTs all items to the Worker /ingest endpoint in batches.
 *
 * Usage:
 *   npx tsx scripts/ingest.ts --data ../cognee-admin/data/game/extracted --endpoint https://gamedb.<account>.workers.dev --key <GAMEDB_API_KEY>
 *
 * Or for local dev:
 *   npx tsx scripts/ingest.ts --data ./extracted --endpoint http://localhost:8787 --key dev-key
 */

import { readFileSync, readdirSync, existsSync } from "fs";
import { join, basename } from "path";

const args = process.argv.slice(2);
function getArg(name: string): string {
	const idx = args.indexOf(`--${name}`);
	if (idx === -1 || idx + 1 >= args.length) {
		console.error(`Missing required argument: --${name}`);
		process.exit(1);
	}
	return args[idx + 1];
}

const DATA_DIR = getArg("data");
const ENDPOINT = getArg("endpoint").replace(/\/$/, "");
const API_KEY = getArg("key");
const BATCH_SIZE = 5; // small batches — each item needs embedding via Workers AI

interface IngestItem {
	id: string;
	json: Record<string, unknown>;
	markdown?: string;
	code?: string;
}

function readIfExists(path: string): string | undefined {
	return existsSync(path) ? readFileSync(path, "utf-8") : undefined;
}

function loadItems(type: "atoms" | "recipes" | "systems"): IngestItem[] {
	const jsonDir = join(DATA_DIR, type, "json");
	const mdDir = join(DATA_DIR, type, "md");
	const rsDir = join(DATA_DIR, type, "rs");

	if (!existsSync(jsonDir)) {
		console.warn(`  ⚠ Directory not found: ${jsonDir}`);
		return [];
	}

	const files = readdirSync(jsonDir).filter((f) => f.endsWith(".json"));
	const items: IngestItem[] = [];

	for (const file of files) {
		const id = basename(file, ".json");
		const jsonContent = JSON.parse(readFileSync(join(jsonDir, file), "utf-8"));
		const markdown = readIfExists(join(mdDir, `${id}.md`));
		const code = readIfExists(join(rsDir, `${id}.rs`));

		items.push({ id, json: jsonContent, markdown, code });
	}

	return items;
}

async function ingestBatch(items: IngestItem[]): Promise<{ uploaded: number }> {
	const res = await fetch(`${ENDPOINT}/ingest`, {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
			Authorization: `Bearer ${API_KEY}`,
		},
		body: JSON.stringify({ items }),
	});

	if (!res.ok) {
		const text = await res.text();
		throw new Error(`Ingest failed (${res.status}): ${text}`);
	}

	return (await res.json()) as { uploaded: number };
}

async function main() {
	console.log(`📦 Loading GameDB data from: ${DATA_DIR}`);
	console.log(`🎯 Target endpoint: ${ENDPOINT}`);
	console.log();

	const atoms = loadItems("atoms");
	console.log(`  atoms:   ${atoms.length} items`);

	const recipes = loadItems("recipes");
	console.log(`  recipes: ${recipes.length} items`);

	const systems = loadItems("systems");
	console.log(`  systems: ${systems.length} items`);

	const allItems = [...atoms, ...recipes, ...systems];
	console.log(`\n  total:   ${allItems.length} items\n`);

	if (allItems.length === 0) {
		console.error("❌ No items found. Check --data path.");
		process.exit(1);
	}

	let totalUploaded = 0;
	const totalBatches = Math.ceil(allItems.length / BATCH_SIZE);

	for (let i = 0; i < allItems.length; i += BATCH_SIZE) {
		const batch = allItems.slice(i, i + BATCH_SIZE);
		const batchNum = Math.floor(i / BATCH_SIZE) + 1;
		process.stdout.write(`  [${batchNum}/${totalBatches}] Ingesting ${batch.length} items... `);

		const result = await ingestBatch(batch);
		totalUploaded += result.uploaded;
		console.log(`✅ (${totalUploaded}/${allItems.length})`);

		// Brief pause between batches to avoid rate limits
		if (i + BATCH_SIZE < allItems.length) {
			await new Promise((r) => setTimeout(r, 1000));
		}
	}

	console.log(`\n🎉 Done! Ingested ${totalUploaded} items into GameDB.`);

	// Verify
	console.log("\n🔍 Verifying...");
	const healthRes = await fetch(`${ENDPOINT}/health`);
	const health = await healthRes.json();
	console.log("  Health:", JSON.stringify(health, null, 2));
}

main().catch((e) => {
	console.error("❌ Fatal error:", e);
	process.exit(1);
});
