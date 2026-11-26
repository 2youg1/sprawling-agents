// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { appendFileSync, existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { createBuilding, dispatch } from "../src/core/commands";
import { encodeFrame } from "../src/core/frames";
import { LENSES, PAGES, RUN_LENSES, SETUP_GROUPS } from "../src/core/route";
import phrases from "../src/lang.json";
import type { Tier } from "../src/core/prefs";
import type { Pane } from "../src/core/workbench";
import { layersKey } from "../src/views/edge";
import { Address } from "../src/wire";
import type { Command, Mode } from "../src/wire";
import { DevTools } from "./walk_devtools";
import type { Frame, Lighting, Outcome } from "./walk_devtools";

const ROOT = join(import.meta.dir, "..", "..");
const TARGET = resolve(ROOT, process.env.CARGO_TARGET_DIR ?? "target");
const GROUND = join(TARGET, "walk");
const BINARIES = join(TARGET, "debug");
const EXE = process.platform === "win32" ? ".exe" : "";
const AT = "127.0.0.1:8799";
export const ORIGIN = `http://${AT}`;
const DEVTOOLS_PORT = 9333;
const MODEL = "stand-in-1";

const FRAMES: readonly Frame[] = [
  { name: "1920x1080", width: 1920, height: 1080, dpr: 1, touch: false },
  { name: "2560x1600", width: 1707, height: 1067, dpr: 1.5, touch: false },
  { name: "390x844", width: 390, height: 844, dpr: 2, touch: true },
];

type Step =
  | { readonly press: keyof typeof phrases }
  | { readonly select: string }
  | { readonly tier: Tier }
  | { readonly sheet: Pane };

interface Shot {
  readonly name: string;
  readonly fragment: string;
  readonly steps: readonly Step[];
}

const started: (() => void)[] = [];
process.on("exit", () => {
  started.forEach((stop) => {
    stop();
  });
});

function fail(why: string): never {
  console.error(`walk: ${why}`);
  process.exit(1);
}

function held<T>(outcome: Outcome<T>, doing: string): T {
  return outcome.ok ? outcome.value : fail(`${doing}: ${outcome.why}`);
}

export function phrase(key: keyof typeof phrases): string {
  return phrases[key].en;
}

interface Options {
  readonly keep: boolean;
  readonly only: string;
  readonly frames: readonly Frame[];
  readonly lightings: readonly Lighting[];
  readonly webDir: string | undefined;
}

function options(args: readonly string[]): Options {
  const after = (flag: string): string | undefined => {
    const at = args.indexOf(flag);
    return at < 0 ? undefined : args[at + 1];
  };
  const frames = after("--frames")?.split(",").map(frameOf) ?? FRAMES;
  const lightings = (after("--lightings") ?? "dark,light").split(",").map((named): Lighting => {
    switch (named) {
      case "dark":
      case "light":
        return named;
      default:
        return fail(`a lighting is dark or light, not ${named}`);
    }
  });
  return {
    keep: args.includes("--keep"),
    only: after("--only") ?? "",
    frames,
    lightings,
    webDir: after("--web-dir"),
  };
}

function frameOf(spelled: string): Frame {
  const read = /^(\d+)x(\d+)@([\d.]+)(:touch)?$/.exec(spelled);
  if (read === null) return fail(`a frame is <width>x<height>@<ratio>[:touch], not ${spelled}`);
  const [, width = "", height = "", dpr = "", touch] = read;
  return {
    name: `${width}x${height}`,
    width: Number(width),
    height: Number(height),
    dpr: Number(dpr),
    touch: touch !== undefined,
  };
}

const decoder = new TextDecoder();
const binary = (name: string): string => {
  const path = join(BINARIES, `${name}${EXE}`);
  return existsSync(path) ? path : fail(`${path} is missing; \`just walk\` builds it`);
};

async function standIn(
  record: string,
): Promise<{ readonly url: string; readonly process: ReturnType<typeof Bun.spawn> }> {
  const provider = Bun.spawn(
    [binary("provider"), join(import.meta.dir, "walk_provider.json"), record],
    {
      stdout: "pipe",
      stderr: "inherit",
    },
  );
  started.push(() => {
    provider.kill();
  });
  const reader = provider.stdout.getReader();
  let said = "";
  while (!said.includes("\n")) {
    const chunk = await reader.read();
    if (chunk.done) break;
    said += decoder.decode(chunk.value);
  }
  const url = /^SPRAWLING_PROVIDER=(\S+)/m.exec(said)?.[1];
  return url === undefined
    ? fail(`the stand-in provider said ${said}`)
    : { url, process: provider };
}

const STYLING = new RegExp(`${String.fromCharCode(27)}\\[[0-9;?]*[ -/]*[@-~]`, "g");

async function serve(
  city: string,
  env: Record<string, string | undefined>,
  webDir: string | undefined,
): Promise<{ readonly code: string; readonly process: ReturnType<typeof Bun.spawn> }> {
  let said = "";
  const disk = webDir === undefined ? [] : ["--web-dir", webDir];
  const served = Bun.spawn([binary("sprawling"), "up", city, AT, "--no-open", ...disk], {
    env,
    terminal: {
      cols: 120,
      rows: 40,
      data: (_terminal, bytes) => {
        said += decoder.decode(bytes);
      },
    },
  });
  started.push(() => {
    served.kill();
  });
  let asked = false;
  for (let tries = 0; tries < 150; tries += 1) {
    const screen = said.replace(STYLING, "");
    if (!asked && screen.includes("C: show a pairing code")) {
      const terminal = served.terminal ?? fail("the city has no terminal");
      terminal.write("c");
      asked = true;
    }
    const code = /pairing code[ \t]+([a-f0-9]{32})\b/i.exec(screen)?.[1];
    if (code !== undefined) return { code, process: served };
    await Bun.sleep(200);
  }
  served.kill();
  return fail("the city terminal did not issue a pairing code");
}

function call(
  command: Command,
  env: Record<string, string | undefined>,
  until: string,
): Outcome<readonly unknown[]> {
  const ran = Bun.spawnSync(
    [
      binary("sprawling"),
      "call",
      encodeFrame({ command }),
      "--at",
      AT,
      "--quiet-ms",
      "5000",
      "--until",
      until,
    ],
    { env },
  );
  const frames = ran.stdout
    .toString()
    .split("\n")
    .filter((line) => line.trim() !== "")
    .map((line): unknown => JSON.parse(line));
  const refusal = frames.find(
    (frame) => typeof frame === "object" && frame !== null && "refusal" in frame,
  );
  if (refusal !== undefined) return { ok: false, why: JSON.stringify(refusal) };
  if (ran.exitCode !== 0) return { ok: false, why: ran.stderr.toString() };
  return { ok: true, value: frames };
}

function runStarted(frames: readonly unknown[]): string {
  for (const frame of frames) {
    if (typeof frame !== "object" || frame === null || !("event" in frame)) continue;
    const event: unknown = frame.event;
    if (
      typeof event === "object" &&
      event !== null &&
      "kind" in event &&
      event.kind === "run_started" &&
      "run" in event &&
      typeof event.run === "string"
    ) {
      return event.run;
    }
  }
  return fail("a dispatch started no run");
}

export function settle(env: Record<string, string | undefined>, city: string): string {
  for (const building of ["atlas", "garden"]) {
    held(
      call(createBuilding(Address.make(building), "minimal", null), env, "building_created"),
      `raise ${building}`,
    );
  }
  const ran = (room: string, mode: Mode, task: string, goal: string): string =>
    runStarted(
      held(
        call(
          dispatch({
            addr: Address.make(room),
            task,
            goal,
            effort: null,
            policy: { mode, write: "full", admit: "standing", landing: "ordinary" },
          }),
          env,
          "run_frozen",
        ),
        `run in ${room}`,
      ),
    );
  const first = ran(
    "atlas/notes",
    "work",
    "Write a short README for the Atlas field notes: what the folders hold and the next three steps.",
    "a README a new team member can read in one minute",
  );
  ran(
    "atlas/review",
    "chat",
    "Review the building: where does each room stand, and what would you change in the README?",
    "",
  );
  ran(
    "garden/sensors",
    "chat",
    "Sketch how the garden sensors should be read and stored. Do not write files yet.",
    "",
  );
  mkdirSync(join(city, "lab-notes"), { recursive: true });
  writeFileSync(
    join(city, "lab-notes", "notes.md"),
    "# Lab notes\n\nLoose notes from the bench, not yet a building.\n",
  );
  return first;
}

export interface PreparedCity {
  readonly city: string;
  readonly env: Record<string, string | undefined>;
  readonly run: string | null;
  readonly tools: DevTools;
  readonly served: { readonly process: ReturnType<typeof Bun.spawn> };
  readonly close: () => Promise<void>;
}

export async function prepareCity(
  webDir: string | undefined,
  populated: boolean,
  pairWith: (tools: DevTools, code: string) => Promise<void> = pair,
  workspace: "walk" | "send" = "walk",
): Promise<PreparedCity> {
  const ground = join(TARGET, workspace);
  rmSync(ground, { recursive: true, force: true });
  const city = join(ground, "city");
  const home = join(ground, "home");
  mkdirSync(home, { recursive: true });
  const provider = await standIn(join(ground, "exchanges.jsonl"));
  const env = {
    ...process.env,
    HOME: home,
    USERPROFILE: home,
    LOCALAPPDATA: home,
    TMP: home,
    TEMP: home,
    TMPDIR: home,
    XDG_RUNTIME_DIR: home,
    SPRAWLING_MODEL_URL: provider.url,
    SPRAWLING_MODEL: populated ? MODEL : undefined,
  };
  const raised = Bun.spawnSync([binary("sprawling"), "init", city], { env });
  if (raised.exitCode !== 0) fail(`init: ${raised.stderr.toString()}`);
  const served = await serve(city, env, webDir);
  const run = populated ? settle(env, city) : null;
  const tools = held(
    await DevTools.open(join(ground, "profile"), DEVTOOLS_PORT),
    "open the browser",
  );
  started.push(() => {
    void tools.close();
  });
  let closed: Promise<void> | undefined;
  const close = (): Promise<void> => {
    if (closed !== undefined) return closed;
    const browserExited = tools.close();
    provider.process.kill();
    served.process.kill();
    closed = Promise.all([browserExited, provider.process.exited, served.process.exited]).then(
      () => undefined,
    );
    return closed;
  };
  await pairWith(tools, served.code);
  return { city, env, run, tools, served, close };
}

function shots(run: string): readonly Shot[] {
  const at = (name: string, fragment: string, ...steps: Step[]): Shot => ({
    name,
    fragment,
    steps,
  });
  const palette: Step = { select: "form button[aria-haspopup=dialog]" };
  const shown = new Set(["talk", "city", "raise", "setup", "record", "gallery"]);
  return [
    at("talk-empty-room", "#/talk/hall/mayor"),
    at("talk-work", "#/talk/atlas/notes"),
    at("talk-chat", "#/talk/atlas/review"),
    at("talk-code", "#/talk/garden/sensors"),
    at("talk-layers", "#/talk/atlas/review", { tier: "blend" }),
    at("talk-panorama", "#/talk/atlas/notes", { tier: "panorama" }),
    at("talk-panorama-bench", "#/talk/atlas/review", { tier: "panorama" }, { sheet: "session" }),
    at("talk-file-beside", "#/talk/atlas/notes?building=atlas&path=notes/README.md", {
      tier: "zen",
    }),
    at("talk-palette", "#/talk/atlas/notes", palette),
    at("talk-palette-carry", "#/talk/atlas/notes", palette, { press: "palette_carry" }),
    at("city", "#/city"),
    at("building", "#/building/atlas"),
    at("building-prompts", "#/building/atlas", { press: "bld_prompts" }),
    at("building-capabilities", "#/building/atlas", { press: "bld_catalog" }),
    at("raise", "#/raise"),
    at("raise-adopt", "#/raise/lab-notes"),
    at("run", `#/run/${run}`),
    ...RUN_LENSES.map((lens) => at(`run-${lens}`, `#/run/${run}/${lens}`)),
    at("record", "#/record"),
    ...LENSES.filter((lens) => lens !== "ledger").map((lens) =>
      at(`record-${lens}`, `#/record/${lens}`),
    ),
    ...PAGES.filter((page) => !shown.has(page)).map((page) => at(page, `#/${page}`)),
    ...SETUP_GROUPS.map((group) => at(`setup-${group}`, `#/setup/${group}`)),
  ];
}

const PRESS = `(name) => {
  const all = [...document.querySelectorAll("button, a, [role=button], [role=option], [role=tab], [role=menuitem], summary")];
  const said = (e) => [(e.getAttribute("aria-label") || "").trim(), (e.textContent || "").trim()];
  const found = all.find((e) => said(e).includes(name)) ?? all.find((e) => said(e).some((s) => s.startsWith(name)));
  if (found === undefined) throw new Error("nothing on the page is called " + name);
  found.click();
}`;

const TIERED = `async (wanted, called) => {
  const key = () => {
    const found = [...document.querySelectorAll("button[aria-label]")].find((e) => e.getAttribute("aria-label").startsWith(called));
    if (found === undefined) throw new Error("nothing on the page is called " + called);
    return found;
  };
  const first = key().getAttribute("aria-label");
  for (let said = first; said !== wanted; said = key().getAttribute("aria-label")) {
    key().click();
    await new Promise((done) => setTimeout(done, 400));
    if (key().getAttribute("aria-label") === first) return;
  }
}`;

const NO_HANDS = { press: () => undefined, release: () => undefined, click: () => undefined };

function stepScript(step: Step): string {
  if ("press" in step) return `(${PRESS})(${JSON.stringify(phrase(step.press))})`;
  if ("select" in step) return `document.querySelector(${JSON.stringify(step.select)}).click()`;
  if ("sheet" in step)
    return `document.querySelector(${JSON.stringify(`[role=tab][id$="-tab-${step.sheet}"]`)})?.click()`;
  const wanted =
    layersKey(step.tier, "en", (words) => words, NO_HANDS).key.wire["aria-label"] ?? "";
  return `(${TIERED})(${JSON.stringify(wanted)}, ${JSON.stringify(phrase("edge_layers"))})`;
}

async function pair(tools: DevTools, code: string): Promise<void> {
  const first = FRAMES[0] ?? fail("no frame");
  await tools.frame(first, "dark");
  await tools.load(`${ORIGIN}/`, 2500);
  held(await tools.run(`document.querySelector("input").focus()`), "find the pairing code's box");
  await tools.type(code);
  held(await tools.run(`(${PRESS})(${JSON.stringify(phrase("pair_action"))})`), "press pair");
  await tools.settle(3000);
  const still = await tools.run(
    `document.body.innerText.includes(${JSON.stringify(phrase("pair_title"))})`,
  );
  if (!still.ok || still.value === true) fail("the browser is still on the pairing page");
}

async function photograph(tools: DevTools, wanted: Options, run: string): Promise<number> {
  const out = join(GROUND, "out");
  const index = join(out, "index.md");
  mkdirSync(out, { recursive: true });
  writeFileSync(index, "| frame | lighting | shot | fragment | picture |\n|---|---|---|---|---|\n");
  let missed = 0;
  const taken = shots(run).filter((shot) => shot.name.includes(wanted.only));
  if (taken.length === 0) fail(`no shot is named with "${wanted.only}"`);
  for (const frame of wanted.frames) {
    for (const lighting of wanted.lightings) {
      await tools.frame(frame, lighting);
      await tools.load(`${ORIGIN}/`, 2000);
      for (const shot of taken) {
        await tools.run(`location.hash = ${JSON.stringify(shot.fragment)}`);
        await tools.settle(1500);
        let why = "";
        for (const step of shot.steps) {
          const pressed = await tools.run(stepScript(step));
          if (!pressed.ok) why = pressed.why;
          await tools.settle(900);
        }
        const file = `${frame.name}-${lighting}/${shot.name}.png`;
        const kept = why === "" ? await tools.keep(join(out, file)) : { ok: false as const, why };
        appendFileSync(
          index,
          `| ${frame.name} | ${lighting} | ${shot.name} | \`${shot.fragment}\` | ${kept.ok ? `[${file}](${file})` : kept.why} |\n`,
        );
        if (!kept.ok) {
          missed += 1;
          console.error(`walk: ${file}: ${kept.why}`);
        }
        await tools.key("Escape", "Escape", 27);
      }
    }
  }
  console.log(`walk: pictures and index.md in ${out}`);
  return missed;
}

if (import.meta.main) {
  const wanted = options(Bun.argv.slice(2));
  const { run, tools, served } = await prepareCity(wanted.webDir, true);
  if (run === null) fail("a populated walk started no run");
  const missed = await photograph(tools, wanted, run);
  if (wanted.keep) {
    console.log(`walk: ${ORIGIN} stays served; Ctrl-C ends it`);
    await served.process.exited;
  }
  process.exit(missed === 0 ? 0 : 1);
}
