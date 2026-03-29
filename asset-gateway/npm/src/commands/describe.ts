import { Command } from "commander";
import { CLI_DESCRIPTION, CLI_NAME, CLI_VERSION } from "../meta.js";
import { output, success } from "../output.js";
import type { GlobalOptions } from "./common.js";

export const SCHEMAS: Record<string, object> = {
  auth: {
    description: "Credential management",
    subcommands: {
      set: {
        description: "Save token and gateway URL locally",
        params: { token: { type: "string", required: true, description: "API token (agk_ prefix for admin, or any API key)" } },
      },
      status: { description: "Show current authentication status" },
      clear: { description: "Remove saved credentials" },
    },
  },
  generate: {
    description: "Generate assets via the gateway",
    subcommands: {
      image: {
        description: "Generate an image from a text prompt",
        params: {
          "--prompt": { type: "string", required: true, description: "Image description prompt" },
          "--provider": { type: "string", required: false, description: "Provider to use" },
          "--transparent": { type: "bool", default: false, description: "Request transparent background" },
          "--model": { type: "string", required: false, description: "Model to use" },
          "--size": { type: "string", required: false, description: "Image size (e.g. 1024x1024)" },
          "--output-dir": { type: "string", default: ".", description: "Directory to save output" },
        },
      },
      video: {
        description: "Generate a video from a text prompt",
        params: {
          "--prompt": { type: "string", required: true, description: "Video description prompt" },
          "--provider": { type: "string", required: false, description: "Provider to use" },
          "--output-dir": { type: "string", default: ".", description: "Directory to save output" },
        },
      },
      audio: {
        description: "Generate audio from a text prompt",
        params: {
          "--prompt": { type: "string", required: true, description: "Audio description prompt" },
          "--type": { type: "string", required: false, description: "Audio type: bgm or sfx" },
          "--duration": { type: "number", required: false, description: "Duration in seconds" },
          "--output-dir": { type: "string", default: ".", description: "Directory to save output" },
        },
      },
      model: {
        description: "Generate a 3D model",
        params: {
          "--image": { type: "string", required: false, description: "Reference image URL" },
          "--prompt": { type: "string", required: false, description: "Model description prompt" },
          "--output-dir": { type: "string", default: ".", description: "Directory to save output" },
        },
      },
      text: {
        description: "Generate text via LLM",
        params: {
          "--prompt": { type: "string", required: true, description: "Text prompt" },
          "--model": { type: "string", required: false, description: "Model to use" },
          "--max-tokens": { type: "number", required: false, description: "Maximum tokens" },
          "--output-dir": { type: "string", default: ".", description: "Directory to save output" },
        },
      },
    },
  },
  provider: {
    description: "Provider management",
    subcommands: {
      list: { description: "List available providers" },
      health: {
        description: "Check provider health",
        params: { name: { type: "string", required: false, description: "Specific provider name" } },
      },
    },
  },
  job: {
    description: "Job management",
    subcommands: {
      list: {
        description: "List jobs",
        params: {
          "--status": { type: "string", required: false, description: "Filter by status" },
          "--limit": { type: "number", required: false, description: "Maximum number of jobs to return" },
        },
      },
      status: {
        description: "Get job status",
        params: { id: { type: "string", required: true, description: "Job ID" } },
      },
      cancel: {
        description: "Cancel a job",
        params: { id: { type: "string", required: true, description: "Job ID" } },
      },
    },
  },
  describe: {
    description: "Self-describe available commands (JSON Schema)",
    params: {
      command: { type: "string", required: false, description: "Specific command to describe" },
    },
  },
};

export function createDescribeCommand(): Command {
  return new Command("describe")
    .description("Self-describe available commands (JSON Schema)")
    .argument("[command]", "Specific command to describe")
    .action(function (commandArg?: string) {
      const globals = this.optsWithGlobals<GlobalOptions>();
      if (!commandArg) {
        output(
          success("describe", {
            name: CLI_NAME,
            version: CLI_VERSION,
            description: CLI_DESCRIPTION,
            commands: SCHEMAS,
          }),
          globals.human
        );
        return;
      }

      const schema = SCHEMAS[commandArg];
      if (!schema) {
        output(
          success("describe", {
            error: `Unknown command: ${commandArg}`,
            available: Object.keys(SCHEMAS),
          }),
          globals.human
        );
        return;
      }

      output(success("describe", { command: commandArg, schema }), globals.human);
    });
}
