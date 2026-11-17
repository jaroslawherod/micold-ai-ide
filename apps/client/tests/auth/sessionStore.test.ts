import fs from "fs";
import path from "path";
import { createMockApi } from "@micold/protocol/mock";
import { createSessionStore } from "../../src/auth/sessionStore";

describe("session store", () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  it("holds the session in memory and writes nothing to web storage or cookies", async () => {
    const { api } = createMockApi();
    const store = createSessionStore();
    const session = await api.auth.signIn("github");
    if ("failure" in session) throw new Error("sign-in failed");
    store.getState().setSession(session);
    expect(store.getState().session?.accessToken).toBe(session.accessToken);
    const stored = [
      ...Object.keys(window.localStorage).map((k) => window.localStorage.getItem(k)),
      ...Object.keys(window.sessionStorage).map((k) => window.sessionStorage.getItem(k)),
      document.cookie,
    ].join("|");
    expect(stored).not.toContain(session.accessToken);
    expect(stored).not.toMatch(/token/i);
    expect(JSON.stringify(store.getState())).not.toContain(session.accessToken);
  });

  it("keeps two stores (two tabs) apart", async () => {
    const tabA = createSessionStore();
    const tabB = createSessionStore();
    const a = await createMockApi().api.auth.signIn("github");
    const b = await createMockApi().api.auth.signIn("google");
    if ("failure" in a || "failure" in b) throw new Error("sign-in failed");
    tabA.getState().setSession(a);
    expect(tabB.getState().session).toBeNull();
    tabB.getState().setSession(b);
    expect(tabA.getState().session?.accessToken).not.toBe(tabB.getState().session?.accessToken);
    tabA.getState().clear();
    expect(tabA.getState().session).toBeNull();
    expect(tabB.getState().session).not.toBeNull();
  });

  it("no auth module touches web storage, IndexedDB or cookies", () => {
    const dir = path.join(__dirname, "../../src/auth");
    for (const f of fs.readdirSync(dir)) {
      expect([f, /localStorage|sessionStorage|indexedDB|document\.cookie|persist\(/.test(fs.readFileSync(path.join(dir, f), "utf8"))]).toEqual([f, false]);
    }
  });
});
