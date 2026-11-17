import fs from "fs";
import path from "path";
import { selectApi } from "../../src/api/select";

const ORIGINAL = process.env.EXPO_PUBLIC_API_MODE;
afterEach(() => {
  process.env.EXPO_PUBLIC_API_MODE = ORIGINAL;
  delete (globalThis as { __micoldMock?: unknown }).__micoldMock;
});

function sources(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(dir, e.name);
    return e.isDirectory() ? sources(p) : /\.tsx?$/.test(e.name) ? [p] : [];
  });
}

describe("selectApi", () => {
  it("selects the mock in mock mode and exposes its control hook", async () => {
    process.env.EXPO_PUBLIC_API_MODE = "mock";
    const api = selectApi();
    expect((await api.orchestrator.getSnapshot()).sessions).toHaveLength(60);
    expect((globalThis as { __micoldMock?: unknown }).__micoldMock).toBeDefined();
  });

  it("selects the real client in real mode and connects to the configured address, never the mock", () => {
    const urls: string[] = [];
    class FakeSocket {
      constructor(url: string) {
        urls.push(url);
      }
      send() {}
      close() {}
    }
    const original = globalThis.WebSocket;
    globalThis.WebSocket = FakeSocket as unknown as typeof WebSocket;
    const url = process.env.EXPO_PUBLIC_ORCHESTRATOR_URL;
    try {
      process.env.EXPO_PUBLIC_API_MODE = "real";
      delete process.env.EXPO_PUBLIC_ORCHESTRATOR_URL;
      const api = selectApi();
      expect(api.orchestrator.connection.get()).toEqual({ kind: "connecting" });
      process.env.EXPO_PUBLIC_ORCHESTRATOR_URL = "ws://example.test:9/api/ws";
      selectApi();
      expect(urls).toEqual(["ws://127.0.0.1:8730/api/ws", "ws://example.test:9/api/ws"]);
      expect((globalThis as { __micoldMock?: unknown }).__micoldMock).toBeUndefined();
    } finally {
      globalThis.WebSocket = original;
      if (url === undefined) delete process.env.EXPO_PUBLIC_ORCHESTRATOR_URL;
      else process.env.EXPO_PUBLIC_ORCHESTRATOR_URL = url;
    }
  });

  it("is the only app module that names the mock or a socket", () => {
    const root = path.join(__dirname, "../..");
    const offenders = [...sources(path.join(root, "app")), ...sources(path.join(root, "src"))]
      .filter((f) => !f.endsWith(path.join("src", "api", "select.ts")))
      .filter((f) => /@micold\/protocol\/mock|protocol\/src|WebSocket/.test(fs.readFileSync(f, "utf8")))
      .map((f) => path.relative(root, f));
    expect(offenders).toEqual([]);
  });
});
