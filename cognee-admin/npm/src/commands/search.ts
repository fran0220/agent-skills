import { Command } from "commander";
import { configError } from "../errors.js";
import { createContext, printError, printSuccess } from "./common.js";

export function createSearchCommand(): Command {
  const command = new Command("search")
    .description("Search the knowledge base")
    .argument("[query]", "Search query unless using the history subcommand")
    .option("--search-type <type>", "Search type", "GRAPH_COMPLETION")
    .option("--top-k <count>", "Number of results", "5")
    .option("--datasets <names>", "Comma-separated dataset names to filter by")
    .option("--verbose", "Include request metadata in the response")
    .action(
      async function (
        query: string | undefined,
        options: {
          searchType: string;
          topK: string;
          datasets?: string;
          verbose?: boolean;
        }
      ) {
        try {
          const ctx = createContext(this);
          if (!query) {
            throw configError("query is required unless using `search history`");
          }

          const topK = Number.parseInt(options.topK, 10);
          if (!Number.isFinite(topK) || topK < 1) {
            throw configError("--top-k must be a positive integer");
          }

          const datasets = options.datasets
            ? options.datasets
                .split(",")
                .map((value) => value.trim())
                .filter(Boolean)
            : [];

          const response = await ctx.client.search(query, {
            search_type: options.searchType,
            top_k: topK,
            datasets,
          });

          const data = options.verbose
            ? {
                request: {
                  query,
                  search_type: options.searchType,
                  top_k: topK,
                  datasets,
                },
                response,
              }
            : response;

          printSuccess("search", data, ctx);
        } catch (error) {
          printError("search", error);
        }
      }
    );

  command.addCommand(
    new Command("history")
      .description("Fetch search history")
      .action(async function () {
        try {
          const ctx = createContext(this);
          printSuccess("search.history", await ctx.client.searchHistory(), ctx);
        } catch (error) {
          printError("search.history", error);
        }
      })
  );

  return command;
}
