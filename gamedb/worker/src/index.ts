interface Env {
	VECTORIZE: VectorizeIndex;
	R2: R2Bucket;
	AI: Ai;
	GAMEDB_API_KEY: string;
}

interface SearchResult {
	id: string;
	score: number;
	metadata: Record<string, string>;
}

const CORS_HEADERS: Record<string, string> = {
	"Access-Control-Allow-Origin": "*",
	"Access-Control-Allow-Methods": "GET, POST, OPTIONS",
	"Access-Control-Allow-Headers": "Content-Type, Authorization",
};

const EMBED_MODEL = "@cf/baai/bge-large-en-v1.5" as const;

function json(data: unknown, status = 200): Response {
	return new Response(JSON.stringify(data), {
		status,
		headers: { "Content-Type": "application/json", ...CORS_HEADERS },
	});
}

function err(message: string, status = 400): Response {
	return json({ ok: false, error: message }, status);
}

function requireAuth(request: Request, env: Env): boolean {
	const auth = request.headers.get("Authorization");
	return auth === `Bearer ${env.GAMEDB_API_KEY}`;
}

async function embed(text: string | string[], env: Env): Promise<number[][]> {
	const input = Array.isArray(text) ? text : [text];
	const result = await env.AI.run(EMBED_MODEL, { text: input });
	return (result as { data: number[][] }).data;
}

function buildEmbedText(id: string, data: Record<string, unknown>): string {
	const parts: string[] = [];
	if (data.name_en) parts.push(data.name_en as string);
	if (data.name_cn) parts.push(data.name_cn as string);
	if (data.definition) parts.push(data.definition as string);

	const mda = data.mda as Record<string, unknown> | undefined;
	if (mda) {
		if (typeof mda.mechanics === "string") parts.push(mda.mechanics);
		if (Array.isArray(mda.mechanics)) parts.push((mda.mechanics as string[]).join(". "));
		if (Array.isArray(mda.aesthetics)) parts.push((mda.aesthetics as string[]).join(", "));
	}

	if (Array.isArray(data.tags)) parts.push((data.tags as string[]).join(", "));
	if (Array.isArray(data.atoms)) {
		const atomIds = (data.atoms as { atom_id: string }[]).map((a) => a.atom_id);
		parts.push(atomIds.join(", "));
	}
	if (Array.isArray(data.feedback_loops)) {
		for (const loop of data.feedback_loops as { description: string }[]) {
			if (loop.description) parts.push(loop.description);
		}
	}

	// Truncate to ~500 tokens (~2000 chars) for bge-large max 512 tokens
	const full = parts.join("\n");
	return full.slice(0, 2000);
}

function detectType(id: string): string {
	if (id.startsWith("atom_")) return "atom";
	if (id.startsWith("sys_")) return "system";
	return "recipe";
}

function detectCategory(id: string, data: Record<string, unknown>): string {
	return (data.category as string) || detectType(id);
}

// --- Route handlers ---

async function handleSearch(request: Request, env: Env): Promise<Response> {
	const url = new URL(request.url);
	const q = url.searchParams.get("q");
	if (!q) return err("Missing ?q= parameter");

	const topK = Math.min(parseInt(url.searchParams.get("top_k") || "10"), 50);
	const type = url.searchParams.get("type");
	const category = url.searchParams.get("category");

	const vectors = await embed(q, env);

	const filter: VectorizeVectorMetadataFilter = {};
	if (type) filter["type"] = { $eq: type };
	if (category) filter["category"] = { $eq: category };

	const results = await env.VECTORIZE.query(vectors[0], {
		topK,
		returnMetadata: "all",
		...(Object.keys(filter).length > 0 ? { filter } : {}),
	});

	const matches: SearchResult[] = results.matches.map((m) => ({
		id: m.id,
		score: m.score,
		metadata: (m.metadata || {}) as Record<string, string>,
	}));

	return json({ ok: true, query: q, count: matches.length, results: matches });
}

async function handleGet(id: string, env: Env): Promise<Response> {
	const jsonObj = await env.R2.get(`json/${id}.json`);
	const mdObj = await env.R2.get(`md/${id}.md`);

	if (!jsonObj && !mdObj) return err(`Not found: ${id}`, 404);

	const data: Record<string, unknown> = {};
	if (jsonObj) data.json = await jsonObj.json();
	if (mdObj) data.markdown = await mdObj.text();

	return json({ ok: true, id, ...data });
}

async function handleList(request: Request, env: Env): Promise<Response> {
	const url = new URL(request.url);
	const type = url.searchParams.get("type");

	const indexObj = await env.R2.get("index.json");
	if (!indexObj) return err("Index not found. Run ingest first.", 404);

	const index = (await indexObj.json()) as { id: string; type: string; name_en: string; name_cn: string; category?: string }[];
	const filtered = type ? index.filter((i) => i.type === type) : index;

	return json({ ok: true, count: filtered.length, items: filtered });
}

async function handleIngest(request: Request, env: Env): Promise<Response> {
	if (!requireAuth(request, env)) return err("Unauthorized", 401);

	const body = (await request.json()) as {
		items: { id: string; json: Record<string, unknown>; markdown?: string; code?: string }[];
	};

	if (!body.items?.length) return err("Missing items array");

	const index: { id: string; type: string; name_en: string; name_cn: string; category?: string }[] = [];
	const vectors: VectorizeVector[] = [];
	let uploaded = 0;

	// Batch embed: collect all texts first
	const texts: string[] = [];
	const metas: { id: string; type: string; category: string; name_en: string; name_cn: string }[] = [];

	for (const item of body.items) {
		const { id, json: data, markdown, code } = item;
		const type = detectType(id);
		const category = detectCategory(id, data);

		// Store files to R2
		await env.R2.put(`json/${id}.json`, JSON.stringify(data));
		if (markdown) await env.R2.put(`md/${id}.md`, markdown);
		if (code) await env.R2.put(`rs/${id}.rs`, code);

		texts.push(buildEmbedText(id, data));
		metas.push({
			id,
			type,
			category,
			name_en: (data.name_en as string) || id,
			name_cn: (data.name_cn as string) || "",
		});

		index.push({
			id,
			type,
			name_en: (data.name_en as string) || id,
			name_cn: (data.name_cn as string) || "",
			category,
		});

		uploaded++;
	}

	// Batch embed all texts at once
	const embeddings = await embed(texts, env);

	for (let i = 0; i < metas.length; i++) {
		vectors.push({
			id: metas[i].id,
			values: embeddings[i],
			metadata: {
				type: metas[i].type,
				category: metas[i].category,
				name_en: metas[i].name_en,
				name_cn: metas[i].name_cn,
			},
		});
	}

	// Upsert vectors (max 1000 per batch)
	for (let i = 0; i < vectors.length; i += 1000) {
		await env.VECTORIZE.upsert(vectors.slice(i, i + 1000));
	}

	// Merge with existing index or create new
	const existingObj = await env.R2.get("index.json");
	let fullIndex = index;
	if (existingObj) {
		const existing = (await existingObj.json()) as typeof index;
		const existingMap = new Map(existing.map((e) => [e.id, e]));
		for (const item of index) {
			existingMap.set(item.id, item);
		}
		fullIndex = Array.from(existingMap.values());
	}
	await env.R2.put("index.json", JSON.stringify(fullIndex));

	return json({ ok: true, uploaded, total: body.items.length });
}

async function handleHealth(env: Env): Promise<Response> {
	const indexObj = await env.R2.get("index.json");
	const count = indexObj ? ((await indexObj.json()) as unknown[]).length : 0;

	return json({
		ok: true,
		service: "gamedb",
		version: "1.0.0",
		vectors: count,
		embedding_model: EMBED_MODEL,
		dimensions: 1024,
		storage: "cloudflare-r2",
		index: "cloudflare-vectorize",
	});
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		if (request.method === "OPTIONS") {
			return new Response(null, { status: 204, headers: CORS_HEADERS });
		}

		const url = new URL(request.url);
		const path = url.pathname;

		try {
			if (path === "/" || path === "/health") return handleHealth(env);
			if (path === "/search") return handleSearch(request, env);
			if (path === "/list") return handleList(request, env);
			if (path === "/ingest" && request.method === "POST") return handleIngest(request, env);

			const getMatch = path.match(/^\/get\/(.+)$/);
			if (getMatch) return handleGet(getMatch[1], env);

			return err("Not found", 404);
		} catch (e) {
			const message = e instanceof Error ? e.message : "Internal error";
			return err(message, 500);
		}
	},
};
