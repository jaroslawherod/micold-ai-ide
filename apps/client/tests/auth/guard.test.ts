import { guardRedirect, returnTarget, SIGN_IN_PATH } from "../../src/auth/guard";

describe("route guard", () => {
  it("sends a signed-out visitor to sign-in, carrying the asked-for route", () => {
    expect(guardRedirect("/sessions/s1", false)).toBe("/sign-in?returnTo=%2Fsessions%2Fs1");
    expect(guardRedirect("/", false)).toBe(SIGN_IN_PATH);
  });

  it("lets sign-in itself and every route of a signed-in person render", () => {
    expect(guardRedirect(SIGN_IN_PATH, false)).toBeNull();
    expect(guardRedirect("/sessions/s1", true)).toBeNull();
  });

  it("recognises sign-in whatever its case, escapes or trailing slash", () => {
    for (const path of ["/Sign-In", "/sign-in/", "/sign%2Din", "/SIGN%2dIN/"]) expect(guardRedirect(path, false)).toBeNull();
    expect(guardRedirect("/%E0%A4%A", false)).toBe("/sign-in?returnTo=%2F%25E0%25A4%25A");
  });

  it("returns to the asked-for route after sign-in", () => {
    expect(returnTarget("/sessions/s1")).toBe("/sessions/s1");
    expect(returnTarget(["/a", "/b"])).toBe("/a");
    expect(returnTarget(decodeURIComponent("%2Fsessions%2Fs1"))).toBe("/sessions/s1");
    expect(returnTarget("/sessions/s1?tab=diff#top")).toBe("/sessions/s1?tab=diff#top");
  });

  it("never returns off-site, to sign-in, or to nothing", () => {
    for (const bad of [undefined, "", "https://evil.example", "//evil.example", "/\\evil", "/sign-in", "/sign-in?returnTo=/x", "/sign-in/", "/Sign-In", "/sign%2din", "/sign-in#x", "/\tx", "/a\nb", "/%2F/evil.example"]) {
      expect(returnTarget(bad)).toBe("/");
    }
  });
});
