import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

export function createLibraryCommand(): Command {
  const command = new Command("library").description(
    "Search and manage the asset library"
  );

  command.addCommand(
    new Command("search")
      .description("Search the asset library")
      .argument("[query]", "Search query")
      .option("-t, --type <type>", "Filter by asset type (audio, music, image, etc.)")
      .option("--tags <tags>", "Filter by tags (comma-separated)")
      .option("--source <source>", "Filter by source (manual, generated, imported)")
      .option("-n, --limit <limit>", "Max results", "20")
      .option("--offset <offset>", "Offset for pagination", "0")
      .action(async function (query: string | undefined) {
        const ctx = createContext(this);
        const opts = this.opts();
        try {
          const params = new URLSearchParams();
          if (query) params.set("q", query);
          if (opts.type) params.set("type", opts.type);
          if (opts.tags) params.set("tags", opts.tags);
          if (opts.source) params.set("source", opts.source);
          params.set("limit", opts.limit);
          params.set("offset", opts.offset);

          const data = await ctx.client.get(`/api/library/search?${params}`);
          printSuccess("library.search", data, ctx);
        } catch (error) {
          printError("library.search", error, ctx.human);
        }
      })
  );

  command.addCommand(
    new Command("add")
      .description("Add an item to the asset library")
      .requiredOption("--name <name>", "Display name")
      .requiredOption("--url <url>", "File URL (public)")
      .requiredOption("-t, --type <type>", "Asset type (audio, music, image, etc.)")
      .option("-d, --description <desc>", "Description")
      .option("--tags <tags>", "Comma-separated tags")
      .option("--duration <seconds>", "Duration in seconds (for audio/video)")
      .option("--source <source>", "Source (manual, generated, imported)", "manual")
      .action(async function () {
        const ctx = createContext(this);
        const opts = this.opts();
        try {
          const body: Record<string, unknown> = {
            asset_type: opts.type,
            name: opts.name,
            file_url: opts.url,
            source: opts.source,
          };
          if (opts.description) body.description = opts.description;
          if (opts.tags)
            body.tags = opts.tags.split(",").map((t: string) => t.trim());
          if (opts.duration)
            body.duration_seconds = parseFloat(opts.duration);

          const data = await ctx.client.post("/api/library", body);
          printSuccess("library.add", data, ctx);
        } catch (error) {
          printError("library.add", error, ctx.human);
        }
      })
  );

  command.addCommand(
    new Command("get")
      .description("Get a library item by ID")
      .argument("<id>", "Library item ID")
      .action(async function (id: string) {
        const ctx = createContext(this);
        try {
          const data = await ctx.client.get(`/api/library/${encodeURIComponent(id)}`);
          printSuccess("library.get", data, ctx);
        } catch (error) {
          printError("library.get", error, ctx.human);
        }
      })
  );

  command.addCommand(
    new Command("delete")
      .description("Delete a library item (admin only)")
      .argument("<id>", "Library item ID")
      .action(async function (id: string) {
        const ctx = createContext(this);
        try {
          const data = await ctx.client.delete(`/api/library/${encodeURIComponent(id)}`);
          printSuccess("library.delete", data, ctx);
        } catch (error) {
          printError("library.delete", error, ctx.human);
        }
      })
  );

  command.addCommand(
    new Command("catalog-jobs")
      .description("Catalog completed generation jobs into the library (admin)")
      .option("-t, --type <type>", "Filter jobs by asset type")
      .option("--provider <id>", "Filter jobs by provider ID")
      .option("--after <date>", "Only catalog jobs after this date")
      .option("--dry-run", "Just count, don't actually catalog")
      .action(async function () {
        const ctx = createContext(this);
        const opts = this.opts();
        try {
          const body: Record<string, unknown> = {};
          if (opts.type) body.asset_type = opts.type;
          if (opts.provider) body.provider_id = opts.provider;
          if (opts.after) body.after = opts.after;
          if (opts.dryRun) body.dry_run = true;

          const data = await ctx.client.post("/api/library/catalog-jobs", body);
          printSuccess("library.catalog_jobs", data, ctx);
        } catch (error) {
          printError("library.catalog_jobs", error, ctx.human);
        }
      })
  );

  return command;
}
