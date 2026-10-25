// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The walk through the running window, photographed
// (docs/frontend-method.md, "The walk through the running window"): a
// throwaway city under `target/walk/`, answered by the stand-in provider
// (`walk_provider.json`), served the way a person serves one, paired with
// a headless browser, and every page and a few pressed states kept as PNG
// files at three windows in both lightings, with an index beside them.
// `just walk` builds what it needs and runs this. Not a gate: a person
// reads the pictures, as with `cargo xtask shots`, which photographs the
// bundle's fixtures with no city behind them.
//
// What it photographs comes from the client: the pages from
// `core/route.ts`, the names it presses from `lang.json`, the frames it
// sends from `core/commands.ts`. A page added there is walked without an
// edit here, and a renamed button fails its shot instead of passing it.
//
//   bun scripts/walk.ts [--keep] [--only <text>] [--frames <list>] [--lightings <list>] [--web-dir <dir>]
//
// `--keep` leaves the city served after the shots, for walking it by hand
// in the paired profile; `--only` takes the shots whose name holds the
// text; a frame is `<width>x<height>@<ratio>`, with `:touch` for a phone;
// `--web-dir` serves a client bundle from disk, as `serve --web-dir` does,
// so a rebuilt client is walked without rebuilding the binary.

import { appendFileSync, existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { createBuilding, dispatch } from "../src/core/commands";
import { encodeFrame } from "../src/core/frames";
import { LENSES, PAGES, RUN_LENSES, SETUP_GROUPS } from "../src/core/route";
import phrases from "../src/lang.json";
import { Address } from "../src/wire";
import type { Command, Mode } from "../src/wire";
import { DevTools } from "./walk_devtools";
import type { Frame, Lighting, Outcome } from "./walk_devtools";

const ROOT = join(import.meta.dir, "..", "..");
const GROUND = join(ROOT, "target", "walk");
const BINARIES = join(process.env.CARGO_TARGET_DIR ?? join(ROOT, "target"), "debug");
const EXE = process.platform === "win32" ? ".exe" : "";
const AT = "127.0.0.1:8799";
const ORIGIN = `http://${AT}`;
const DEVTOOLS_PORT = 9333;
// The one model the stand-in's script lists.
const MODEL = "stand-in-1";

const FRAMES: readonly Frame[] = [
  { name: "1920x1080", width: 1920, height: 1080, dpr: 1, touch: false },
  // A 2560 by 1600 laptop at the 150 % its system scales to.
  { name: "2560x1600", width: 1707, height: 1067, dpr: 1.5, touch: false },
  { name: "390x844", width: 390, height: 844, dpr: 2, touch: true },
];

// A step taken on a page before its picture: press the control a screen
// reader calls `name` (a phrase of `lang.json`), or the first element a
// selector finds where the name is built from data.
type Step = { readonly press: keyof typeof phrases } | { readonly select: string };

interface Shot {
  readonly name: string;
  readonly fragment: string;
  readonly steps: readonly Step[];
}

// What a walk started and must end however it ends: the stand-in, the
// served city and the browser hold a port each, and one left running
// keeps the next walk from starting.
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

function phrase(key: keyof typeof phrases): string {
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
  return { keep: args.includes("--keep"), only: after("--only") ?? "", frames, lightings, webDir: after("--web-dir") };
}

function frameOf(spelled: string): Frame {
  const read = /^(\d+)x(\d+)@([\d.]+)(:touch)?$/.exec(spelled);
  if (read === null) return fail(`a frame is <width>x<height>@<ratio>[:touch], not ${spelled}`);
  const [, width = "", height = "", dpr = "", touch] = read;
  return { name: `${width}x${height}`, width: Number(width), height: Number(height), dpr: Number(dpr), touch: touch !== undefined };
}

// --------------------------------------------------------------- the city

const decoder = new TextDecoder();
const binary = (name: string): string => {
  const path = join(BINARIES, `${name}${EXE}`);
  return existsSync(path) ? path : fail(`${path} is missing; \`just walk\` builds it`);
};

// Where the stand-in listens, read from the line it prints before it answers.
async function standIn(record: string): Promise<{ readonly url: string; readonly process: ReturnType<typeof Bun.spawn> }> {
  const provider = Bun.spawn([binary("provider"), join(import.meta.dir, "walk_provider.json"), record], {
    stdout: "pipe",
    stderr: "inherit",
  });
  const reader = provider.stdout.getReader();
  let said = "";
  while (!said.includes("\n")) {
    const chunk = await reader.read();
    if (chunk.done) break;
    said += decoder.decode(chunk.value);
  }
  const url = /^SPRAWLING_PROVIDER=(\S+)/m.exec(said)?.[1];
  return url === undefined ? fail(`the stand-in provider said ${said}`) : { url, process: provider };
}

// What a terminal is told about colour and the cursor, taken out before
// its words are read.
const STYLING = new RegExp(`${String.fromCharCode(27)}\\[[0-9;?]*[ -/]*[@-~]`, "g");

// Serves the city on a pseudo-terminal, because the quiet host that `up`
// shows prints the pairing code only to a terminal, and waits for it.
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
  for (let tries = 0; tries < 150; tries += 1) {
    const code = /pairing code\s+(\S+)/.exec(said.replace(STYLING, ""))?.[1];
    if (code !== undefined) return { code, process: served };
    await Bun.sleep(200);
  }
  served.kill();
  return fail(`the city printed no pairing code:\n${said.slice(-2000)}`);
}

// Sends one command through `sprawling call` and hands back every frame
// that came back, or the city's refusal.
function call(command: Command, env: Record<string, string | undefined>, until: string): Outcome<readonly unknown[]> {
  const ran = Bun.spawnSync(
    [binary("sprawling"), "call", encodeFrame({ command }), "--at", AT, "--quiet-ms", "5000", "--until", until],
    { env },
  );
  const frames = ran.stdout
    .toString()
    .split("\n")
    .filter((line) => line.trim() !== "")
    .map((line): unknown => JSON.parse(line));
  const refusal = frames.find((frame) => typeof frame === "object" && frame !== null && "refusal" in frame);
  if (refusal !== undefined) return { ok: false, why: JSON.stringify(refusal) };
  if (ran.exitCode !== 0) return { ok: false, why: ran.stderr.toString() };
  return { ok: true, value: frames };
}

function runStarted(frames: readonly unknown[]): string {
  for (const frame of frames) {
    if (typeof frame !== "object" || frame === null || !("event" in frame)) continue;
    const event: unknown = frame.event;
    if (typeof event === "object" && event !== null && "kind" in event && event.kind === "run_started" && "run" in event && typeof event.run === "string") {
      return event.run;
    }
  }
  return fail("a dispatch started no run");
}

// The city a walk photographs: two buildings, three rooms that each ran
// once, an empty Mayor's room and a folder nobody adopted. The runs take
// the stand-in's replies in this order.
function settle(env: Record<string, string | undefined>, city: string): string {
  for (const building of ["atlas", "garden"]) {
    held(call(createBuilding(Address.make(building), "minimal", null), env, "building_created"), `raise ${building}`);
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
  ran("atlas/review", "chat", "Review the building: where does each room stand, and what would you change in the README?", "");
  ran("garden/sensors", "chat", "Sketch how the garden sensors should be read and stored. Do not write files yet.", "");
  mkdirSync(join(city, "lab-notes"), { recursive: true });
  writeFileSync(join(city, "lab-notes", "notes.md"), "# Lab notes\n\nLoose notes from the bench, not yet a building.\n");
  return first;
}

// ------------------------------------------------------------- the shots

function shots(run: string): readonly Shot[] {
  const at = (name: string, fragment: string, ...steps: Step[]): Shot => ({ name, fragment, steps });
  const palette: Step = { select: "header button[aria-haspopup=dialog]" };
  const shown = new Set(["talk", "city", "raise", "setup", "record", "gallery"]);
  return [
    at("talk-empty-room", "#/talk/hall/mayor"),
    at("talk-work", "#/talk/atlas/notes"),
    at("talk-chat", "#/talk/atlas/review"),
    at("talk-code", "#/talk/garden/sensors"),
    at("talk-file-beside", "#/talk/atlas/notes?building=atlas&path=notes/README.md"),
    at("talk-palette", "#/talk/atlas/notes", palette),
    at("talk-palette-carry", "#/talk/atlas/notes", palette, { press: "palette_carry" }),
    at("talk-layers", "#/talk/atlas/notes", { press: "edge_layers" }),
    at("city", "#/city"),
    at("building", "#/building/atlas"),
    at("building-prompts", "#/building/atlas", { press: "bld_prompts" }),
    at("building-capabilities", "#/building/atlas", { press: "bld_catalog" }),
    at("raise", "#/raise"),
    at("raise-adopt", "#/raise/lab-notes"),
    at("run", `#/run/${run}`),
    ...RUN_LENSES.map((lens) => at(`run-${lens}`, `#/run/${run}/${lens}`)),
    at("record", "#/record"),
    ...LENSES.filter((lens) => lens !== "ledger").map((lens) => at(`record-${lens}`, `#/record/${lens}`)),
    ...PAGES.filter((page) => !shown.has(page)).map((page) => at(page, `#/${page}`)),
    ...SETUP_GROUPS.map((group) => at(`setup-${group}`, `#/setup/${group}`)),
  ];
}

// Presses what a person would: the control whose accessible name or text
// is the phrase, the whole of it before any that start with it.
const PRESS = `(name) => {
  const all = [...document.querySelectorAll("button, a, [role=button], [role=option], [role=tab], [role=menuitem], summary")];
  const said = (e) => [(e.getAttribute("aria-label") || "").trim(), (e.textContent || "").trim()];
  const found = all.find((e) => said(e).includes(name)) ?? all.find((e) => said(e).some((s) => s.startsWith(name)));
  if (found === undefined) throw new Error("nothing on the page is called " + name);
  found.click();
}`;

function stepScript(step: Step): string {
  return "press" in step
    ? `(${PRESS})(${JSON.stringify(phrase(step.press))})`
    : `document.querySelector(${JSON.stringify(step.select)}).click()`;
}

async function pair(tools: DevTools, code: string): Promise<void> {
  const first = FRAMES[0] ?? fail("no frame");
  await tools.frame(first, "dark");
  await tools.load(`${ORIGIN}/`, 2500);
  held(await tools.run(`document.querySelector("input").focus()`), "find the pairing code's box");
  await tools.type(code);
  held(await tools.run(`(${PRESS})(${JSON.stringify(phrase("pair_action"))})`), "press pair");
  await tools.settle(3000);
  const still = await tools.run(`document.body.innerText.includes(${JSON.stringify(phrase("pair_title"))})`);
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
        appendFileSync(index, `| ${frame.name} | ${lighting} | ${shot.name} | \`${shot.fragment}\` | ${kept.ok ? `[${file}](${file})` : kept.why} |\n`);
        if (!kept.ok) {
          missed += 1;
          console.error(`walk: ${file}: ${kept.why}`);
        }
        // A state that opened a dialog or a menu closes before the next.
        await tools.key("Escape", "Escape", 27);
      }
    }
  }
  console.log(`walk: pictures and index.md in ${out}`);
  return missed;
}

// ------------------------------------------------------------------ main

const wanted = options(Bun.argv.slice(2));
rmSync(GROUND, { recursive: true, force: true });
const city = join(GROUND, "city");
const home = join(GROUND, "home");
mkdirSync(home, { recursive: true });
const provider = await standIn(join(GROUND, "exchanges.jsonl"));
started.push(() => {
  provider.process.kill();
});
const env = {
  ...process.env,
  HOME: home,
  USERPROFILE: home,
  SPRAWLING_MODEL_URL: provider.url,
  SPRAWLING_MODEL: MODEL,
};
const raised = Bun.spawnSync([binary("sprawling"), "init", city], { env });
if (raised.exitCode !== 0) fail(`init: ${raised.stderr.toString()}`);
const served = await serve(city, env, wanted.webDir);
started.push(() => {
  served.process.kill();
});
const run = settle(env, city);
const tools = held(await DevTools.open(join(GROUND, "profile"), DEVTOOLS_PORT), "open the browser");
started.push(() => {
  tools.close();
});
await pair(tools, served.code);
const missed = await photograph(tools, wanted, run);
if (wanted.keep) {
  console.log(`walk: ${ORIGIN} stays served; Ctrl-C ends it`);
  await served.process.exited;
}
process.exit(missed === 0 ? 0 : 1);
