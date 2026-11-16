import fs from "fs";
import path from "path";
import { REAL_CLIENT_MISSING, selectApi } from "../../src/api/select";

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

  it("throws in real mode, never falling back to the mock", () => {
    process.env.EXPO_PUBLIC_API_MODE = "real";
    expect(() => selectApi()).toThrow(REAL_CLIENT_MISSING);
    expect(REAL_CLIENT_MISSING).toBe("real client not built (#1732)");
    expect((globalThis as { __micoldMock?: unknown }).__micoldMock).toBeUndefined();
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
