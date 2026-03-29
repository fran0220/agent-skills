import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

export function createUploadCommand(): Command {
  const command = new Command("upload").description("Upload and manage assets");

  command.addCommand(
    new Command("file")
      .description("Upload a file and get a public URL")
      .argument("<path>", "Path to file to upload")
      .action(async function (filePath: string) {
        const ctx = createContext(this);
        try {
          const data = await ctx.client.uploadFile(filePath);
          printSuccess("asset.upload", data, ctx);
        } catch (error) {
          printError("asset.upload", error, ctx.human);
        }
      })
  );

  command.addCommand(
    new Command("list")
      .description("List uploaded assets")
      .action(async function () {
        const ctx = createContext(this);
        try {
          const data = await ctx.client.get("/api/assets");
          printSuccess("asset.list", data, ctx);
        } catch (error) {
          printError("asset.list", error, ctx.human);
        }
      })
  );

  command.addCommand(
    new Command("delete")
      .description("Delete an uploaded asset (admin only)")
      .argument("<filename>", "Filename to delete")
      .action(async function (filename: string) {
        const ctx = createContext(this);
        try {
          const data = await ctx.client.delete(`/api/assets/${encodeURIComponent(filename)}`);
          printSuccess("asset.delete", data, ctx);
        } catch (error) {
          printError("asset.delete", error, ctx.human);
        }
      })
  );

  return command;
}
