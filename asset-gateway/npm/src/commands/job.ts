import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

export function createJobCommand(): Command {
  const command = new Command("job").description("Job management");

  command.addCommand(
    new Command("list")
      .description("List jobs")
      .option("--status <status>", "Filter by status")
      .option("--limit <n>", "Maximum number of jobs to return")
      .action(async function (options: { status?: string; limit?: string }) {
        try {
          const ctx = createContext(this);
          const params = new URLSearchParams();
          if (options.status) params.set("status", options.status);
          if (options.limit) params.set("limit", options.limit);
          const query = params.toString();
          const path = query ? `/api/jobs?${query}` : "/api/jobs";
          const data = await ctx.client.get(path);
          printSuccess("job.list", data, ctx);
        } catch (error) {
          printError("job.list", error);
        }
      })
  );

  command.addCommand(
    new Command("status")
      .description("Get job status")
      .argument("<id>", "Job ID")
      .action(async function (id: string) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.get(`/api/jobs/${encodeURIComponent(id)}`);
          printSuccess("job.status", data, ctx);
        } catch (error) {
          printError("job.status", error);
        }
      })
  );

  command.addCommand(
    new Command("cancel")
      .description("Cancel a job")
      .argument("<id>", "Job ID")
      .action(async function (id: string) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post(`/api/jobs/${encodeURIComponent(id)}/cancel`);
          printSuccess("job.cancel", data, ctx);
        } catch (error) {
          printError("job.cancel", error);
        }
      })
  );

  return command;
}
