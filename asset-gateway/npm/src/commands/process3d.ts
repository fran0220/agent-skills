import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

function infer3dExtension(format?: string): string {
  const map: Record<string, string> = {
    fbx: "fbx",
    usdz: "usdz",
    obj: "obj",
    stl: "stl",
    gltf: "gltf",
    "3mf": "3mf",
    glb: "glb",
  };
  return map[String(format ?? "glb").toLowerCase()] ?? "glb";
}

async function saveProcess3dOutput(
  data: Record<string, unknown>,
  operation: string,
  outputDir: string,
  format?: string
): Promise<string | null> {
  if (!data.output_url) {
    return null;
  }

  const ext = infer3dExtension(format);
  const timestamp = Date.now();
  const filePath = join(outputDir, `${operation}_${timestamp}.${ext}`);
  mkdirSync(outputDir, { recursive: true });

  const response = await fetch(String(data.output_url));
  if (!response.ok) {
    return null;
  }

  const buffer = Buffer.from(await response.arrayBuffer());
  writeFileSync(filePath, buffer);
  return filePath;
}

export function createProcess3dCommand(): Command {
  const command = new Command("process3d").description("3D model post-processing via Tripo pipeline");

  command.addCommand(
    new Command("convert")
      .description("Convert 3D model format (FBX/USDZ/OBJ/STL/GLTF/3MF)")
      .requiredOption("--task-id <id>", "Tripo task ID from generate model")
      .requiredOption("--format <fmt>", "Target format: FBX, USDZ, OBJ, STL, GLTF, 3MF")
      .option("--quad", "Enable quad remeshing")
      .option("--face-limit <n>", "Max face count")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const params: Record<string, unknown> = {
            format: options.format,
          };
          if (options.quad) params.quad = true;
          if (options.faceLimit) params.face_limit = Number(options.faceLimit);

          const data = await ctx.client.post("/api/process3d", {
            task_id: options.taskId,
            operation: "convert",
            params,
          }) as Record<string, unknown>;

          const localPath = await saveProcess3dOutput(data, "convert", options.outputDir, options.format);
          if (localPath) data.local_path = localPath;
          printSuccess("process3d.convert", data, ctx);
        } catch (error) {
          printError("process3d.convert", error);
        }
      })
  );

  command.addCommand(
    new Command("texture")
      .description("Re-texture a 3D model with new materials")
      .requiredOption("--task-id <id>", "Tripo task ID")
      .option("--prompt <text>", "Texture description prompt")
      .option("--pbr", "Enable PBR materials")
      .option("--quality <q>", "Texture quality: standard or detailed")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const params: Record<string, unknown> = {};
          if (options.prompt) params.prompt = options.prompt;
          if (options.pbr) params.pbr = true;
          if (options.quality) params.quality = options.quality;

          const data = await ctx.client.post("/api/process3d", {
            task_id: options.taskId,
            operation: "texture",
            params,
          }) as Record<string, unknown>;

          const localPath = await saveProcess3dOutput(data, "texture", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process3d.texture", data, ctx);
        } catch (error) {
          printError("process3d.texture", error);
        }
      })
  );

  command.addCommand(
    new Command("rig")
      .description("Auto-rig a 3D model (add skeleton for animation)")
      .requiredOption("--task-id <id>", "Tripo task ID")
      .option("--format <fmt>", "Output format: glb or fbx", "glb")
      .option("--spec <spec>", "Rig spec: mixamo or tripo", "mixamo")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process3d", {
            task_id: options.taskId,
            operation: "rig",
            params: {
              format: options.format,
              spec: options.spec,
            },
          }) as Record<string, unknown>;

          const localPath = await saveProcess3dOutput(data, "rig", options.outputDir, options.format);
          if (localPath) data.local_path = localPath;
          printSuccess("process3d.rig", data, ctx);
        } catch (error) {
          printError("process3d.rig", error);
        }
      })
  );

  command.addCommand(
    new Command("animate")
      .description("Apply preset animation to a rigged model")
      .requiredOption("--task-id <id>", "Tripo task ID (from rig step)")
      .requiredOption("--animation <preset>", "Animation preset (e.g. preset:walk, preset:idle, preset:run)")
      .option("--format <fmt>", "Output format: glb or fbx", "glb")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process3d", {
            task_id: options.taskId,
            operation: "animate",
            params: {
              animation: options.animation,
              format: options.format,
            },
          }) as Record<string, unknown>;

          const localPath = await saveProcess3dOutput(data, "animate", options.outputDir, options.format);
          if (localPath) data.local_path = localPath;
          printSuccess("process3d.animate", data, ctx);
        } catch (error) {
          printError("process3d.animate", error);
        }
      })
  );

  command.addCommand(
    new Command("reduce")
      .description("Reduce polygon count (high-poly to low-poly)")
      .requiredOption("--task-id <id>", "Tripo task ID")
      .option("--face-limit <n>", "Target face count")
      .option("--quad", "Use quad topology")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const params: Record<string, unknown> = {};
          if (options.faceLimit) params.face_limit = Number(options.faceLimit);
          if (options.quad) params.quad = true;

          const data = await ctx.client.post("/api/process3d", {
            task_id: options.taskId,
            operation: "reduce",
            params,
          }) as Record<string, unknown>;

          const localPath = await saveProcess3dOutput(data, "reduce", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process3d.reduce", data, ctx);
        } catch (error) {
          printError("process3d.reduce", error);
        }
      })
  );

  command.addCommand(
    new Command("stylize")
      .description("Apply artistic style to a model")
      .requiredOption("--task-id <id>", "Tripo task ID")
      .requiredOption("--style <style>", "Style: lego, voxel, voronoi, minecraft")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process3d", {
            task_id: options.taskId,
            operation: "stylize",
            params: {
              style: options.style,
            },
          }) as Record<string, unknown>;

          const localPath = await saveProcess3dOutput(data, "stylize", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process3d.stylize", data, ctx);
        } catch (error) {
          printError("process3d.stylize", error);
        }
      })
  );

  command.addCommand(
    new Command("segment")
      .description("Segment mesh into logical parts")
      .requiredOption("--task-id <id>", "Tripo task ID")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process3d", {
            task_id: options.taskId,
            operation: "segment",
            params: {},
          }) as Record<string, unknown>;

          const localPath = await saveProcess3dOutput(data, "segment", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process3d.segment", data, ctx);
        } catch (error) {
          printError("process3d.segment", error);
        }
      })
  );

  command.addCommand(
    new Command("prerigcheck")
      .description("Check if a model can be rigged")
      .requiredOption("--task-id <id>", "Tripo task ID")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process3d", {
            task_id: options.taskId,
            operation: "prerigcheck",
            params: {},
          }) as Record<string, unknown>;

          const localPath = await saveProcess3dOutput(data, "prerigcheck", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process3d.prerigcheck", data, ctx);
        } catch (error) {
          printError("process3d.prerigcheck", error);
        }
      })
  );

  return command;
}
