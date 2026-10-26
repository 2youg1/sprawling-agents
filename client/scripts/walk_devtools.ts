// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A headless Chromium-family browser driven over the DevTools protocol,
// for `walk.ts`: open a page at a window size and a lighting, run a
// script in it, and keep what it drew as a PNG. Every answer comes back
// as a value that says whether it failed, so the walk decides what a
// failure costs.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

// The doctor's Chromium family, the table `xtask render` reads too: one
// brand per line, its program, and where each platform installs it.
const FAMILY = join(
  import.meta.dir,
  "..",
  "..",
  "crates",
  "sprawling",
  "src",
  "doctor",
  "family",
  "chromium.tsv",
);

// The engine to drive: the one `SPRAWLING_BROWSER` names, as for the
// render gate, or else the first of the family installed here, in the
// family's order; `null` when there is none.
export function engine(): string | null {
  const named = process.env.SPRAWLING_BROWSER;
  if (named !== undefined && existsSync(named)) return named;
  const column = process.platform === "win32" ? 2 : process.platform === "darwin" ? 3 : 4;
  for (const line of readFileSync(FAMILY, "utf8").split(/\r?\n/)) {
    if (line.startsWith("#") || line.trim() === "") continue;
    const fields = line.split("\t");
    const placed = (fields[column] ?? "")
      .split("|")
      .find((place) => place !== "" && existsSync(place));
    const found = placed ?? Bun.which(fields[1] ?? "");
    if (found !== null) return found;
  }
  return null;
}

// One window a walk photographs: the size in CSS pixels, the device
// pixel ratio, and whether it is a phone (touch, coarse pointer).
export interface Frame {
  readonly name: string;
  readonly width: number;
  readonly height: number;
  readonly dpr: number;
  readonly touch: boolean;
}

export type Lighting = "dark" | "light";

export type Outcome<T> =
  { readonly ok: true; readonly value: T } | { readonly ok: false; readonly why: string };

const ok = <T>(value: T): Outcome<T> => ({ ok: true, value });
const failed = <T>(why: string): Outcome<T> => ({ ok: false, why });
const sleep = (ms: number): Promise<void> =>
  new Promise((done) => {
    setTimeout(done, ms);
  });

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function text(value: unknown): string {
  return typeof value === "string" ? value : JSON.stringify(value);
}

export class DevTools {
  private next = 0;
  private readonly pending = new Map<number, (answer: Outcome<Record<string, unknown>>) => void>();
  private waiting: readonly { readonly method: string; readonly heard: () => void }[] = [];

  private constructor(
    private readonly socket: WebSocket,
    private readonly browser: ReturnType<typeof Bun.spawn>,
  ) {
    socket.onmessage = (event: MessageEvent<string>) => {
      const message: unknown = JSON.parse(event.data);
      if (!isRecord(message)) return;
      if (typeof message.id === "number") {
        const answer = isRecord(message.error)
          ? failed<Record<string, unknown>>(text(message.error.message))
          : ok(isRecord(message.result) ? message.result : {});
        this.pending.get(message.id)?.(answer);
        this.pending.delete(message.id);
        return;
      }
      const method = message.method;
      const heard = this.waiting.filter((waiter) => waiter.method === method);
      this.waiting = this.waiting.filter((waiter) => waiter.method !== method);
      heard.forEach((waiter) => {
        waiter.heard();
      });
    };
  }

  // Starts the browser on `profile`, a directory that keeps what the
  // page stored (the device key a pairing leaves) from one walk to the
  // next, and attaches to its first page.
  static async open(profile: string, port: number): Promise<Outcome<DevTools>> {
    const program = engine();
    if (program === null) {
      return failed(
        "no Chromium-family browser is installed here; install one, or point SPRAWLING_BROWSER at one",
      );
    }
    mkdirSync(profile, { recursive: true });
    const browser = Bun.spawn(
      [
        program,
        "--headless=new",
        `--remote-debugging-port=${String(port)}`,
        `--user-data-dir=${profile}`,
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-extensions",
        "--hide-scrollbars",
        "--force-color-profile=srgb",
        "--lang=en-US",
        "about:blank",
      ],
      { stdout: "ignore", stderr: "ignore" },
    );
    for (let tries = 0; tries < 100; tries += 1) {
      await sleep(200);
      const listed: unknown = await fetch(`http://127.0.0.1:${String(port)}/json/list`).then(
        (answer) => answer.json(),
        () => null,
      );
      const targets: readonly unknown[] = Array.isArray(listed) ? listed : [];
      const page = targets.find((target) => isRecord(target) && target.type === "page");
      if (isRecord(page) && typeof page.webSocketDebuggerUrl === "string") {
        const socket = new WebSocket(page.webSocketDebuggerUrl);
        await new Promise((opened) => {
          socket.onopen = opened;
        });
        const tools = new DevTools(socket, browser);
        await tools.send("Page.enable");
        await tools.send("Runtime.enable");
        return ok(tools);
      }
    }
    browser.kill();
    return failed(
      "the browser never offered a page to attach to; set SPRAWLING_BROWSER to a Chromium-family browser",
    );
  }

  close(): void {
    this.socket.close();
    this.browser.kill();
  }

  send(
    method: string,
    params: Record<string, unknown> = {},
  ): Promise<Outcome<Record<string, unknown>>> {
    this.next += 1;
    const id = this.next;
    this.socket.send(JSON.stringify({ id, method, params }));
    return new Promise((answered) => {
      this.pending.set(id, answered);
    });
  }

  // Resolves when the page says `method`, or after `patience` ms.
  private heard(method: string, patience: number): Promise<void> {
    return Promise.race([
      new Promise<void>((heard) => {
        this.waiting = [...this.waiting, { method, heard }];
      }),
      sleep(patience),
    ]);
  }

  async frame(frame: Frame, lighting: Lighting): Promise<void> {
    await this.send("Emulation.setDeviceMetricsOverride", {
      width: frame.width,
      height: frame.height,
      deviceScaleFactor: frame.dpr,
      mobile: frame.touch,
    });
    await this.send(
      "Emulation.setTouchEmulationEnabled",
      frame.touch ? { enabled: true, maxTouchPoints: 5 } : { enabled: false },
    );
    await this.send("Emulation.setEmulatedMedia", {
      features: [{ name: "prefers-color-scheme", value: lighting }],
    });
  }

  async load(url: string, settle: number): Promise<void> {
    const loaded = this.heard("Page.loadEventFired", 15_000);
    await this.send("Page.navigate", { url });
    await loaded;
    await this.settle(settle);
  }

  // Waits `ms`, then for the fonts and two frames, so what is kept is
  // what the page drew once it was still.
  async settle(ms: number): Promise<void> {
    await sleep(ms);
    await this.run(
      "document.fonts.ready.then(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))))",
    );
  }

  // Runs `expression` in the page and waits for the promise it returns;
  // a page that threw comes back as a failure naming what it said.
  async run(expression: string): Promise<Outcome<unknown>> {
    const answer = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (!answer.ok) return answer;
    const thrown = answer.value.exceptionDetails;
    if (isRecord(thrown)) {
      const said = isRecord(thrown.exception) ? thrown.exception.description : thrown.text;
      return failed(text(said));
    }
    return ok(isRecord(answer.value.result) ? answer.value.result.value : undefined);
  }

  async type(words: string): Promise<void> {
    await this.send("Input.insertText", { text: words });
  }

  async key(key: string, code: string, virtual: number): Promise<void> {
    await this.send("Input.dispatchKeyEvent", {
      type: "keyDown",
      key,
      code,
      windowsVirtualKeyCode: virtual,
    });
    await this.send("Input.dispatchKeyEvent", {
      type: "keyUp",
      key,
      code,
      windowsVirtualKeyCode: virtual,
    });
  }

  async keep(path: string): Promise<Outcome<string>> {
    const shot = await this.send("Page.captureScreenshot", { format: "png" });
    if (!shot.ok) return shot;
    const data = shot.value.data;
    if (typeof data !== "string") return failed("the browser sent no picture");
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, Buffer.from(data, "base64"));
    return ok(path);
  }
}
