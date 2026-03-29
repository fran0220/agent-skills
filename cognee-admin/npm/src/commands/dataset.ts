import { Command } from "commander";
import { configError } from "../errors.js";
import { createContext, printError, printSuccess } from "./common.js";

export function createDatasetCommand(): Command {
  const command = new Command("dataset").description("Dataset management");

  command
    .addCommand(
      new Command("list")
        .description("List all datasets")
        .action(async function () {
          try {
            const ctx = createContext(this);
            printSuccess("dataset.list", await ctx.client.datasets(), ctx);
          } catch (error) {
            printError("dataset.list", error);
          }
        })
    )
    .addCommand(
      new Command("create")
        .description("Create a new dataset")
        .argument("<name>", "Dataset name")
        .action(async function (name: string) {
          try {
            const ctx = createContext(this);
            printSuccess("dataset.create", await ctx.client.createDataset(name), ctx);
          } catch (error) {
            printError("dataset.create", error);
          }
        })
    )
    .addCommand(
      new Command("delete")
        .description("Delete a dataset")
        .argument("<id>", "Dataset ID")
        .action(async function (id: string) {
          try {
            const ctx = createContext(this);
            printSuccess("dataset.delete", await ctx.client.deleteDataset(id), ctx);
          } catch (error) {
            printError("dataset.delete", error);
          }
        })
    )
    .addCommand(
      new Command("delete-all")
        .description("Delete every dataset")
        .option("--yes", "Confirm destructive delete-all operation")
        .action(async function (options: { yes?: boolean }) {
          try {
            const ctx = createContext(this);
            if (!options.yes) {
              throw configError("refusing to delete all datasets: use --yes to confirm");
            }
            printSuccess("dataset.delete-all", await ctx.client.deleteAllDatasets(), ctx);
          } catch (error) {
            printError("dataset.delete-all", error);
          }
        })
    )
    .addCommand(
      new Command("status")
        .description("Show dataset processing status")
        .action(async function () {
          try {
            const ctx = createContext(this);
            printSuccess("dataset.status", await ctx.client.datasetStatus(), ctx);
          } catch (error) {
            printError("dataset.status", error);
          }
        })
    )
    .addCommand(
      new Command("graph")
        .description("Get dataset knowledge graph")
        .argument("<id>", "Dataset ID")
        .action(async function (id: string) {
          try {
            const ctx = createContext(this);
            printSuccess("dataset.graph", await ctx.client.datasetGraph(id), ctx);
          } catch (error) {
            printError("dataset.graph", error);
          }
        })
    );

  return command;
}
