import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

export function createOntologyCommand(): Command {
  const command = new Command("ontology").description("Ontology management");

  command.addCommand(
    new Command("upload")
      .description("Upload an ontology OWL file")
      .requiredOption("--key <key>", "Ontology key to store under")
      .argument("<file>", "Path to the OWL file")
      .action(async function (file: string, options: { key: string }) {
        try {
          const ctx = createContext(this);
          printSuccess("ontology.upload", await ctx.client.uploadOntology(options.key, file), ctx);
        } catch (error) {
          printError("ontology.upload", error);
        }
      })
  );

  command.addCommand(
    new Command("list")
      .description("List uploaded ontologies")
      .action(async function () {
        try {
          const ctx = createContext(this);
          printSuccess("ontology.list", await ctx.client.listOntologies(), ctx);
        } catch (error) {
          printError("ontology.list", error);
        }
      })
  );

  return command;
}
