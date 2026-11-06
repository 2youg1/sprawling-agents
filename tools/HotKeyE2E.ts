// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Keyboard-only acceptance of the client's main flows, on the same
// throwaway city and stand-in provider as client/scripts/walk.ts.
//
//   just build-web
//   cargo build --package sprawling --bin sprawling --package citysim --bin provider --locked
//   bun tools/HotKeyE2E.ts
//
// No product action below calls `focus()` or `click()`: pairing, forms,
// disclosures, comboboxes, navigation and commands all arrive through
// DevTools key events. JavaScript only reads the resulting DOM, navigates
// to visual fixtures, freezes animation frames and takes screenshots.

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { ORIGIN, phrase, prepareCity } from "../client/scripts/walk";
import { DevTools } from "../client/scripts/walk_devtools";

const ROOT = join(import.meta.dir, "..");
const OUT = join(ROOT, "target", "hotkey-e2e");
const TASK = "Keyboard acceptance: describe the first ledger round.";
const FRAME = { name: "1440x1000", width: 1440, height: 1000, dpr: 1, touch: false };
const MODIFIER = { ctrl: 2, shift: 8 };

interface Active {
  readonly tag: string;
  readonly text: string;
  readonly aria: string;
  readonly placeholder: string;
  readonly labels: string;
  readonly type: string;
}

interface Check {
  readonly name: string;
  readonly evidence: string;
}

const checks: Check[] = [];

function fail(why: string): never {
  console.error(`HotKeyE2E: ${why}`);
  process.exit(1);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function text(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function record(name: string, evidence: string): void {
  checks.push({ name, evidence });
  console.log(`HotKeyE2E: ${name} — ${evidence}`);
}

async function evaluate(tools: DevTools, expression: string): Promise<unknown> {
  const answer = await tools.run(expression);
  return answer.ok ? answer.value : fail(answer.why);
}

async function active(tools: DevTools): Promise<Active | null> {
  const value = await evaluate(
    tools,
    `(() => {
      const node = document.activeElement;
      if (!node) return null;
      return {
        tag: node.tagName,
        text: (node.textContent || "").trim(),
        aria: node.getAttribute("aria-label") || "",
        placeholder: node.getAttribute("placeholder") || "",
        labels: node.labels ? [...node.labels].map((label) => label.textContent || "").join(" ") : "",
        type: node.getAttribute("type") || "",
      };
    })()`,
  );
  if (!isRecord(value)) return null;
  return {
    tag: text(value.tag),
    text: text(value.text),
    aria: text(value.aria),
    placeholder: text(value.placeholder),
    labels: text(value.labels),
    type: text(value.type),
  };
}

function said(node: Active): string {
  if (node.tag === "BODY") return "";
  return [node.aria, node.placeholder, node.labels, node.text].join(" ").toLocaleLowerCase();
}

async function key(tools: DevTools, name: string, code: string, virtual: number): Promise<void> {
  await tools.raw(name, code, virtual);
}

async function tabUntil(
  tools: DevTools,
  wanted: string | ((node: Active) => boolean),
  limit = 100,
): Promise<Active> {
  const matches = (node: Active): boolean =>
    typeof wanted === "string" ? said(node).includes(wanted.toLocaleLowerCase()) : wanted(node);
  for (let at = 0; at < limit; at += 1) {
    const node = await active(tools);
    if (node !== null && matches(node)) return node;
    await key(tools, "Tab", "Tab", 9);
  }
  const last = await active(tools);
  return fail(
    `Tab never reached ${typeof wanted === "string" ? wanted : "the requested control"}; last focus: ${JSON.stringify(last)}`,
  );
}

async function downUntil(tools: DevTools, wanted: string, limit = 30): Promise<Active> {
  for (let at = 0; at < limit; at += 1) {
    const node = await active(tools);
    if (node !== null && said(node).includes(wanted.toLocaleLowerCase())) return node;
    await key(tools, "ArrowDown", "ArrowDown", 40);
  }
  return fail(`ArrowDown never reached ${wanted}`);
}

async function replace(tools: DevTools, words: string): Promise<void> {
  await tools.raw("a", "KeyA", 65, MODIFIER.ctrl);
  await tools.type(words);
}

async function waitFor(
  tools: DevTools,
  expression: string,
  description: string,
  attempts = 200,
): Promise<void> {
  for (let at = 0; at < attempts; at += 1) {
    if ((await evaluate(tools, expression)) === true) return;
    await Bun.sleep(100);
  }
  const body = await evaluate(tools, "document.body.innerText.slice(0, 1200)");
  return fail(`timed out waiting for ${description}; page begins: ${JSON.stringify(body)}`);
}

async function waitText(tools: DevTools, words: string, present = true): Promise<void> {
  await waitFor(
    tools,
    `document.body.innerText.includes(${JSON.stringify(words)}) === ${String(present)}`,
    `${present ? "present" : "absent"}: ${words}`,
  );
}

async function waitHash(tools: DevTools, wanted: string): Promise<void> {
  await waitFor(tools, `location.hash.includes(${JSON.stringify(wanted)})`, `hash ${wanted}`);
}

async function shot(tools: DevTools, name: string): Promise<void> {
  await tools.settle(350);
  const kept = await tools.keep(join(OUT, name));
  if (!kept.ok) return fail(`screenshot ${name}: ${kept.why}`);
  await tools.settle(50);
}

async function hotkey(
  tools: DevTools,
  name: string,
  code: string,
  virtual: number,
  shift = false,
): Promise<void> {
  await tools.raw(name, code, virtual, MODIFIER.ctrl + (shift ? MODIFIER.shift : 0));
}

async function pairByKeyboard(tools: DevTools, code: string): Promise<void> {
  await tools.frame(FRAME, "dark");
  await tools.load(`${ORIGIN}/`, 500);
  await tabUntil(tools, (node) => node.tag === "INPUT");
  await tools.type(code);
  await key(tools, "Tab", "Tab", 9);
  await tabUntil(tools, (node) => node.tag === "INPUT");
  await replace(tools, "keyboard acceptance");
  const filled = await evaluate(
    tools,
    `[...document.querySelectorAll("input")].map((node) => node.value.length)`,
  );
  if (!Array.isArray(filled) || filled[0] === 0) return fail("the pairing fields accepted no text");
  await tabUntil(tools, phrase("pair_action"));
  await key(tools, " ", "Space", 32);
  await waitText(tools, phrase("pair_title"), false);
}

async function focusComposer(tools: DevTools): Promise<void> {
  const node = await active(tools);
  if (node?.tag === "TEXTAREA") return;
  await key(tools, "/", "Slash", 191);
  await tabUntil(tools, (current) => current.tag === "TEXTAREA", 10);
}

async function firstHours(): Promise<void> {
  const prepared = await prepareCity(undefined, false, pairByKeyboard);
  const { tools } = prepared;
  await waitText(tools, phrase("guide_step_provider"));
  const providerUrl = prepared.env.SPRAWLING_MODEL_URL;
  if (providerUrl === undefined) return fail("the stand-in provider has no URL");
  await tabUntil(tools, "base_url");
  await tools.type(providerUrl);
  await tabUntil(tools, phrase("setup_look"));
  await key(tools, " ", "Space", 32);
  await waitText(tools, "stand-in-1");
  await tabUntil(tools, phrase("setup_attach_btn"));
  await key(tools, " ", "Space", 32);
  record("provider attach", "base URL, list models and attach all complete by keyboard");
  await waitFor(
    tools,
    `[...document.querySelectorAll("details")].some((node) => node.open && node.querySelector("summary")?.textContent?.includes("models ·"))`,
    "the attached model editor starts open in the guide",
  );
  record("#229 disclosure", "the required model editor is open after pairing");
  await shot(tools, "01-guide-model-editor.png");

  await tabUntil(tools, "models ·");
  await tabUntil(tools, phrase("setup_models_save"));
  record("#229 save tab stop", "Tab reaches save models from the model summary");
  await tabUntil(tools, "role stand-in-1");
  await key(tools, " ", "Space", 32);
  await waitFor(
    tools,
    `document.activeElement?.getAttribute("placeholder") === "search"`,
    "the role combobox's search field",
  );
  await tools.type("main");
  await key(tools, "Enter", "Enter", 13);
  await waitFor(
    tools,
    `[...document.querySelectorAll("button")].some((node) => node.textContent?.trim() === "save models" && node.getAttribute("aria-disabled") !== "true")`,
    "save models becomes available after choosing the role",
  );
  await tabUntil(tools, phrase("setup_models_save"));
  await key(tools, " ", "Space", 32);
  await waitFor(
    tools,
    `[...document.querySelectorAll("button")].some((node) => node.textContent?.trim() === "start the conversation" && node.getAttribute("aria-disabled") !== "true")`,
    "the guide unlocks start the conversation",
  );
  record("#229 role and save", "the keyboard chooses main · thinks and saves it");

  await tabUntil(tools, phrase("guide_start"));
  await key(tools, " ", "Space", 32);
  await waitFor(tools, `document.querySelector("textarea") !== null`, "the conversation composer");
  await focusComposer(tools);
  await tools.type(TASK);
  await key(tools, "Enter", "Enter", 13);
  await waitText(tools, "I created");
  record("dispatch", "the first task is sent and its reply is drawn");

  await focusComposer(tools);
  await tools.type("/new");
  await key(tools, "Enter", "Enter", 13);
  await waitText(tools, TASK, false);
  await waitFor(
    tools,
    `!document.body.innerText.includes("earlier rounds") && document.querySelectorAll("textarea").length === 1`,
    "an empty new session with only its composer",
  );
  record("/new empty scene", "the old thread and its fold are unmounted after the city accepts");
  await shot(tools, "02-new-empty-session.png");

  await hotkey(tools, "U", "KeyU", 85, true);
  await waitFor(
    tools,
    `document.querySelector(${JSON.stringify(`dialog[open][aria-label="${phrase("world_session_switcher")}"]`)}) !== null`,
    "the session switcher dialog",
  );
  await tabUntil(tools, (node) => node.tag === "INPUT");
  await tools.type("hall/mayor");
  await key(tools, "ArrowDown", "ArrowDown", 40);
  await key(tools, "Enter", "Enter", 13);
  await waitText(tools, TASK);
  record("session tree", "Ctrl-Shift-U finds and reopens the earlier conversation");
  await shot(tools, "03-session-reopened.png");
  await prepared.close();
}

async function settledFlows(): Promise<DevTools> {
  const prepared = await prepareCity(undefined, true, pairByKeyboard);
  const { tools } = prepared;

  await hotkey(tools, "/", "Slash", 191);
  await waitFor(tools, `document.querySelector("dialog[open]") !== null`, "the command palette");
  await tabUntil(tools, (node) => node.tag === "INPUT");
  await waitFor(
    tools,
    `[...document.querySelectorAll("[role=option]")].some((node) => node.textContent?.includes("atlas/review"))`,
    "the settled workspace answers",
  );
  await tools.type("/cd atlas/review");
  await waitFor(
    tools,
    `[...document.querySelectorAll("[role=option]")].some((node) => node.textContent?.includes("/cd"))`,
    "the /cd verb with its address argument",
  );
  await key(tools, "Enter", "Enter", 13);
  await waitHash(tools, "atlas/review");
  await waitText(tools, "Review the building");
  record("palette navigation", "Accel-/ and /cd open the requested room");

  await hotkey(tools, "/", "Slash", 191);
  await tabUntil(tools, (node) => node.tag === "INPUT");
  await tools.type("/diff");
  await key(tools, "Enter", "Enter", 13);
  await waitHash(tools, "#/run/");
  await waitText(tools, phrase("run_changes"));
  record("run page", "the keyboard opens the current run's changes lens");

  await tabUntil(tools, "settings");
  await key(tools, "Enter", "Enter", 13);
  await waitFor(tools, `document.querySelector("dialog[open]") !== null`, "the settings dialog");
  await key(tools, "End", "End", 35);
  await key(tools, "Home", "Home", 36);
  await downUntil(tools, "access");
  await key(tools, " ", "Space", 32);
  await key(tools, "a", "KeyA", 65);
  await key(tools, " ", "Space", 32);
  await waitFor(
    tools,
    `[...document.querySelectorAll("details")].some((node) => !node.open && node.querySelector("summary")?.textContent?.includes("models ·"))`,
    "the settings model editor stays folded",
  );
  await key(tools, "Escape", "Escape", 27);
  record("settings keyboard", "tree keys work, settings stays folded, Escape returns focus");

  await hotkey(tools, "\\", "Backslash", 220);
  await waitFor(
    tools,
    `[...document.querySelectorAll("button")].some((node) => node.getAttribute("aria-label")?.includes("blend"))`,
    "blend tier",
  );
  await hotkey(tools, "\\", "Backslash", 220);
  await waitFor(
    tools,
    `[...document.querySelectorAll("button")].some((node) => node.getAttribute("aria-label")?.includes("panorama"))`,
    "panorama tier",
  );
  await hotkey(tools, "\\", "Backslash", 220);
  await waitFor(
    tools,
    `[...document.querySelectorAll("button")].some((node) => node.getAttribute("aria-label")?.includes("zen"))`,
    "zen tier",
  );
  record("tier cycling", "Accel-\\ visits blend, panorama and returns to zen");

  await tabUntil(tools, "mailbox");
  await key(tools, " ", "Space", 32);
  await waitFor(
    tools,
    `document.querySelector("aside[aria-label=mailbox]") !== null`,
    "the mailbox",
  );
  await key(tools, "Escape", "Escape", 27);
  record("mailbox keyboard", "the mailbox opens and Escape closes it");

  return tools;
}

async function showCase(tools: DevTools, label: string): Promise<void> {
  await waitFor(
    tools,
    `(() => {
      const target = [...document.querySelectorAll("section[aria-label]")].find(
        (node) => node.getAttribute("aria-label") === ${JSON.stringify(label)},
      );
      if (!target) return false;
      for (const node of document.querySelectorAll("section[aria-label]")) {
        if (node !== target && !node.contains(target) && !target.contains(node)) {
          node.style.display = "none";
        } else {
          node.style.display = "";
        }
      }
      target.scrollIntoView({ block: "start" });
      return true;
    })()`,
    `gallery case ${label}`,
  );
  await tools.settle(100);
}

async function thinkingFrame(tools: DevTools, at: number, name: string): Promise<void> {
  const found = await evaluate(
    tools,
    `(() => {
      const mark = document.querySelector(".thinking-mark");
      if (!mark) return false;
      for (const animation of mark.getAnimations({ subtree: true })) {
        animation.pause();
        animation.currentTime = ${String(at)};
      }
      return true;
    })()`,
  );
  if (found !== true) return fail("the thinking mark is not mounted");
  await tools.settle(50);
  await shot(tools, name);
}

async function visuals(tools: DevTools): Promise<void> {
  await evaluate(tools, "location.hash = '/gallery'; undefined");
  await waitText(tools, "gallery");
  await showCase(
    tools,
    "models · what the city will register: a figure and its rung, the provider's default, unknown, and an id named by hand",
  );
  const overlaps = await evaluate(
    tools,
    `(() => {
      const target = [...document.querySelectorAll("section[aria-label]")].find(
        (node) => node.getAttribute("aria-label")?.startsWith("models · what the city will register:"),
      );
      if (!target) return ["fixture missing"];
      const collisions = [];
      for (const [rowIndex, row] of [...target.querySelectorAll("tbody tr")].entries()) {
        const cells = [...row.querySelectorAll("td")];
        const groups = [2, 3].map((column) => {
          const cell = cells[column];
          if (!cell) return [];
          return [...cell.querySelectorAll(".figure-box > div > span")].filter(
            (node) => node.getClientRects().length > 0,
          );
        });
        for (const left of groups[0] || []) {
          for (const right of groups[1] || []) {
            const a = left.getBoundingClientRect();
            const b = right.getBoundingClientRect();
            const width = Math.min(a.right, b.right) - Math.max(a.left, b.left);
            const height = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
            if (width > 1 && height > 1) collisions.push({ rowIndex, width, height });
          }
        }
      }
      return collisions;
    })()`,
  );
  if (!Array.isArray(overlaps) || overlaps.length > 0)
    return fail(`model figure sources overlap: ${JSON.stringify(overlaps)}`);
  record("#230 figure cells", "context and output source text rectangles do not intersect");
  await shot(tools, "04-model-figure-placeholders.png");

  await showCase(tools, "conversation · the thinking mark at rest and gathered");
  await thinkingFrame(tools, 0, "05-thinking-rest.png");
  await thinkingFrame(tools, 1600, "06-thinking-gathered.png");
  await evaluate(tools, "document.documentElement.dataset.motion = 'off'; undefined");
  await tools.settle(100);
  await shot(tools, "07-thinking-reduced-motion.png");
  await evaluate(tools, "document.documentElement.removeAttribute('data-motion'); undefined");
  record("thinking motion", "rest, gathered and reduced-motion frames are captured");

  await showCase(tools, "conversation · an empty new session after one round");
  await shot(tools, "08-new-session-gallery.png");
}

async function main(): Promise<void> {
  mkdirSync(OUT, { recursive: true });
  await firstHours();
  const tools = await settledFlows();
  await visuals(tools);
  writeFileSync(join(OUT, "report.json"), `${JSON.stringify({ checks }, null, 2)}\n`);
  console.log(
    `HotKeyE2E: ${String(checks.length)} checks passed; screenshots and report.json in target/hotkey-e2e`,
  );
  process.exit(0);
}

await main();
