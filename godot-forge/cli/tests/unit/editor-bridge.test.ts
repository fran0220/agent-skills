import { describe, it, expect, afterEach } from "vitest";
import { createServer, type Server } from "node:net";
import { sendEditorCommand, queryEditor, DEFAULT_EDITOR_PORT } from "../../src/utils/editor-bridge.js";
import { mkdtempSync, rmSync, existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const TEST_PORT = 23686;

function setTestPort(port: number): void {
  process.env.GODOT_FORGE_PORT = String(port);
}

function clearTestPort(): void {
  delete process.env.GODOT_FORGE_PORT;
}

/**
 * Create a mock ForgeSync TCP server that echoes back success responses.
 */
function createMockServer(port: number): Promise<{ server: Server; received: string[] }> {
  const received: string[] = [];
  return new Promise((resolve) => {
    const server = createServer((socket) => {
      socket.on("error", () => {}); // suppress ECONNRESET from fire-and-forget
      let buffer = "";
      socket.on("data", (chunk) => {
        buffer += chunk.toString();
        const newlineIdx = buffer.indexOf("\n");
        if (newlineIdx !== -1) {
          const line = buffer.slice(0, newlineIdx);
          received.push(line);
          buffer = buffer.slice(newlineIdx + 1);

          try {
            const req = JSON.parse(line);
            const response = JSON.stringify({ id: req.id, ok: true, data: { action: req.action } }) + "\n";
            socket.write(response);
          } catch {
            socket.write(JSON.stringify({ id: "unknown", ok: false, error: "parse error" }) + "\n");
          }
        }
      });
    });
    trackConnections(server);
    server.listen(port, "127.0.0.1", () => {
      resolve({ server, received });
    });
  });
}

function closeServer(server: Server): Promise<void> {
  return new Promise((resolve) => {
    // Destroy all active connections so the server can close immediately
    server.close(() => resolve());
    server.emit("close-connections");
  });
}

function trackConnections(server: Server): void {
  const sockets = new Set<import("node:net").Socket>();
  server.on("connection", (socket) => {
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
  });
  server.on("close-connections", () => {
    for (const socket of sockets) {
      socket.destroy();
    }
  });
}

describe("editor-bridge TCP", () => {
  let mockServer: Server | null = null;

  afterEach(async () => {
    clearTestPort();
    if (mockServer) {
      await closeServer(mockServer);
      mockServer = null;
    }
  });

  describe("sendEditorCommand", () => {
    it("sends command via TCP when server is available", async () => {
      setTestPort(TEST_PORT);
      const { server, received } = await createMockServer(TEST_PORT);
      mockServer = server;

      sendEditorCommand("/tmp/test-project", { action: "reload_scene" });

      await new Promise((r) => setTimeout(r, 200));

      expect(received.length).toBe(1);
      const msg = JSON.parse(received[0]);
      expect(msg.action).toBe("reload_scene");
      expect(msg.id).toBeDefined();
      expect(msg.params).toBeDefined();
    });

    it("sends command with path param", async () => {
      setTestPort(TEST_PORT);
      const { server, received } = await createMockServer(TEST_PORT);
      mockServer = server;

      sendEditorCommand("/tmp/test-project", { action: "open_scene", path: "res://scenes/main.tscn" });

      await new Promise((r) => setTimeout(r, 200));

      expect(received.length).toBe(1);
      const msg = JSON.parse(received[0]);
      expect(msg.action).toBe("open_scene");
      expect(msg.params.path).toBe("res://scenes/main.tscn");
    });

    it("falls back to file when TCP is unavailable", async () => {
      setTestPort(23699);
      const tmpDir = mkdtempSync(join(tmpdir(), "godot-forge-test-"));

      sendEditorCommand(tmpDir, { action: "reload_scene" });

      await new Promise((r) => setTimeout(r, 500));

      const commandsPath = join(tmpDir, ".godot-forge", "commands.json");
      expect(existsSync(commandsPath)).toBe(true);
      const content = JSON.parse(readFileSync(commandsPath, "utf-8"));
      expect(content.action).toBe("reload_scene");

      rmSync(tmpDir, { recursive: true, force: true });
    });
  });

  describe("queryEditor", () => {
    it("returns response from TCP server", async () => {
      setTestPort(TEST_PORT);
      const { server } = await createMockServer(TEST_PORT);
      mockServer = server;

      const response = await queryEditor({ action: "scan" });

      expect(response).not.toBeNull();
      expect(response!.ok).toBe(true);
      expect(response!.data?.action).toBe("scan");
    });

    it("returns null when server is unavailable", async () => {
      setTestPort(23699);

      const response = await queryEditor({ action: "scan" }, 500);

      expect(response).toBeNull();
    });

    it("returns null on timeout", async () => {
      setTestPort(TEST_PORT + 1);
      const silentServer = createServer((socket) => {
        socket.on("error", () => {}); // suppress ECONNRESET
      });
      trackConnections(silentServer);
      await new Promise<void>((resolve) => {
        silentServer.listen(TEST_PORT + 1, "127.0.0.1", () => resolve());
      });
      mockServer = silentServer;

      const response = await queryEditor({ action: "scan" }, 500);
      expect(response).toBeNull();
    });
  });

  describe("DEFAULT_EDITOR_PORT", () => {
    it("is 23685", () => {
      expect(DEFAULT_EDITOR_PORT).toBe(23685);
    });
  });
});
