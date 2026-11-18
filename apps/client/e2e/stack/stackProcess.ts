// Starts and stops `scripts/local-stack.sh` (process mode) for the stack spec (#1732, T038). The
// stack runs in its own process group (`detached`), so a stop reaches all three processes.
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

export const STACK_API_PORT = Number(process.env.MICOLD_LOCAL_API_PORT ?? 8730);
export const STACK_WEB_PORT = Number(process.env.MICOLD_LOCAL_WEB_PORT ?? 8731);
const ROOT = resolve(__dirname, "../../../..");

export interface Stack {
  dir: string;
  stop(opts: { removeDir: boolean }): Promise<void>;
}

function need(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`${name} is not set: run \`mise run client-e2e-stack\``);
  // CI and mise may pass a path relative to the repository; Playwright runs in apps/client.
  return resolve(ROOT, v);
}

/** Whether something accepts a TCP connection on 127.0.0.1:<port>. */
function listening(port: number): Promise<boolean> {
  return new Promise((done) => {
    const sock = connect(port, "127.0.0.1");
    sock.once("connect", () => (sock.destroy(), done(true)));
    sock.once("error", () => done(false));
  });
}

/** Waits for both the web server and the orchestrator's API port. */
async function stackUp(child: ChildProcess, log: () => string): Promise<void> {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`local-stack.sh exited ${child.exitCode}:\n${log()}`);
    if (log().includes("local-stack: web client")) {
      try {
        if ((await fetch(`http://127.0.0.1:${STACK_WEB_PORT}/`)).ok && (await listening(STACK_API_PORT))) return;
      } catch {
        // not listening yet
      }
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`the stack did not come up in 60 s:\n${log()}`);
}

/** Starts the stack on `dir` (a fresh directory when unset) and waits for the web server and the API. */
export async function startStack(dir = mkdtempSync(join(tmpdir(), "micold-e2e-stack-"))): Promise<Stack> {
  let out = "";
  const child = spawn(join(ROOT, "scripts/local-stack.sh"), [], {
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      MICOLD_LOCAL_MODE: "process",
      MICOLD_LOCAL_DIR: dir,
      MICOLD_LOCAL_API_PORT: String(STACK_API_PORT),
      MICOLD_LOCAL_WEB_PORT: String(STACK_WEB_PORT),
      MICOLD_LOCAL_DAEMON_BIN: need("MICOLD_LOCAL_DAEMON_BIN"),
      MICOLD_LOCAL_WEB_DIR: need("MICOLD_LOCAL_WEB_DIR"),
    },
  });
  child.stdout?.on("data", (d) => (out += d));
  child.stderr?.on("data", (d) => (out += d));
  const exited = new Promise<void>((r) => child.once("exit", () => r()));
  await stackUp(child, () => out);
  return {
    dir,
    async stop({ removeDir }) {
      if (child.exitCode === null && child.pid) {
        try {
          process.kill(-child.pid, "SIGTERM");
        } catch {
          // already gone
        }
        await exited;
      }
      if (removeDir) rmSync(dir, { recursive: true, force: true });
    },
  };
}
