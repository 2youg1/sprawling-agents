// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { Schema } from "effect";
import { EventRecord } from "../src/wire";
import { ORIGIN, phrase, prepareCity, type PreparedCity } from "./walk";
import type { DevTools, Outcome } from "./walk_devtools";

const ROOM = "garden/sensors";
const CURRENT = `#/talk/${ROOM}`;
const THREAD = "[data-thread]";
const FORM = `form[aria-label=${JSON.stringify(phrase("region_composer"))}]`;
const BOX = `${FORM} textarea`;
const SEND = `${FORM} button[data-up=send][type=submit]:not([aria-disabled=true])`;
const CONTINUE = "Which of those sensors should we install first?";
const BRANCH = "Let's plan just the temperature readings.";
const CONTINUED_REPLY = "Start with the soil moisture sensors in the north bed.";
const BRANCH_REPLY = "Log the temperature in each bed at dawn and dusk.";
const BUNDLE = join(import.meta.dir, "..", "..", "Sprawling", "app", "web-dist");
const FRAME = { name: "1280x540", width: 1280, height: 540, dpr: 1, touch: false };

function value<T>(outcome: Outcome<T>): T {
  assert(outcome.ok, outcome.ok ? "" : outcome.why);
  return outcome.value;
}

async function waitFor<T>(
  description: string,
  read: () => T | Promise<T>,
): Promise<NonNullable<T>> {
  const deadline = performance.now() + 15_000;
  do {
    const found = await read();
    if (found) return found;
    await Bun.sleep(50);
  } while (performance.now() < deadline);
  assert.fail(`Timed out waiting for ${description}`);
}

const query = (selector: string): string => `document.querySelector(${JSON.stringify(selector)})`;

async function dom(tools: DevTools, expression: string): Promise<unknown> {
  return value(await tools.run(expression));
}

async function shown(tools: DevTools, description: string, expression: string): Promise<void> {
  await waitFor(description, async () => (await dom(tools, expression)) === true);
}

async function point(tools: DevTools, selector: string): Promise<{ x: number; y: number }> {
  const found = await waitFor(selector, () =>
    dom(
      tools,
      `(() => {
      const element = ${query(selector)};
      if (!element?.getClientRects().length) return null;
      element.scrollIntoView({ block: "center", inline: "nearest", behavior: "instant" });
      const rect = element.getBoundingClientRect();
      const x = rect.x + rect.width / 2;
      const y = rect.y + rect.height / 2;
      return element.contains(document.elementFromPoint(x, y)) ? { x, y } : null;
    })()`,
    ),
  );
  assert(typeof found === "object" && found !== null);
  assert("x" in found && typeof found.x === "number");
  assert("y" in found && typeof found.y === "number");
  return { x: found.x, y: found.y };
}

async function click(tools: DevTools, selector: string): Promise<void> {
  const hover = await point(tools, selector);
  value(await tools.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...hover }));
  await tools.settle(0);
  const at = await point(tools, selector);
  for (const type of ["mousePressed", "mouseReleased"]) {
    value(
      await tools.send("Input.dispatchMouseEvent", { type, button: "left", clickCount: 1, ...at }),
    );
  }
}

async function write(tools: DevTools, words: string): Promise<void> {
  await shown(
    tools,
    "the composer ready for typing",
    `(() => {
    const box = ${query(BOX)};
    box?.focus({ preventScroll: true });
    return box !== null && box === document.activeElement;
  })()`,
  );
  await tools.raw("a", "KeyA", 65, process.platform === "darwin" ? 4 : 2);
  await tools.type(words);
  assert.equal(
    await dom(tools, `${query(BOX)}?.value`),
    words,
    "The composer must receive the draft",
  );
}

async function pair(tools: DevTools, code: string): Promise<void> {
  const input = 'input[autocomplete="one-time-code"]';
  await tools.frame(FRAME, "dark");
  await tools.load(ORIGIN, 0);
  await click(tools, input);
  await tools.type(code);
  await click(tools, "main form button[type=submit]");
  await shown(tools, "a paired browser", `!${query(input)}`);
}

function ledgerOf({ env }: PreparedCity): string {
  assert(env.USERPROFILE, "The walk must supply a private home");
  const workspaces = join(env.USERPROFILE, ".sprawling", "w");
  const registered = readdirSync(workspaces, { withFileTypes: true }).filter((entry) =>
    entry.isDirectory(),
  );
  const [workspace] = registered;
  assert(registered.length === 1 && workspace, "The walk must register exactly one workspace");
  return join(workspaces, workspace.name, ".sprawling", "ledger");
}

function records(ledger: string): readonly EventRecord[] {
  return readdirSync(ledger)
    .filter((name) => /^ledger-\d+\.jsonl$/.test(name))
    .sort()
    .flatMap((name) => readFileSync(join(ledger, name), "utf8").split("\n").slice(0, -1))
    .map((line) => Schema.decodeUnknownSync(EventRecord)(JSON.parse(line)));
}

function opened(ledger: string): readonly EventRecord[] {
  return records(ledger).filter((each) => each.addr === ROOM && each.kind === "session_opened");
}

async function completed(ledger: string, task: string): Promise<EventRecord> {
  const run = await waitFor(`a run for ${task}`, () =>
    records(ledger).find(
      (each) => each.addr === ROOM && each.kind === "run_started" && each.data.task === task,
    ),
  );
  await waitFor(`run ${run.run} to finish`, () =>
    records(ledger).find((each) => each.kind === "run_frozen" && each.run === run.run),
  );
  return run;
}

function inheritedFrom(ledger: string, run: string): readonly unknown[] {
  return records(ledger)
    .filter((each) => each.kind === "run_forked" && each.run === run)
    .map(({ data }) => ({ run: data.from, at_seq: data.at_seq }));
}

function sessionComposer(route: string, link: "talk_current" | "fork_back"): string {
  return `location.hash === ${JSON.stringify(route)} && Boolean(${query(BOX)}) &&
    [...document.querySelectorAll("a")].some((anchor) => anchor.textContent?.trim() === ${JSON.stringify(phrase(link))})`;
}

async function openHistory(tools: DevTools, historical: string): Promise<void> {
  await click(tools, "#workspace-navigation-key[aria-expanded=false]");
  await click(tools, `#workspace-navigation a[href=${JSON.stringify(historical)}]`);
  await shown(
    tools,
    "the mounted historical composer",
    sessionComposer(historical, "talk_current"),
  );
}

async function scrollBack(tools: DevTools): Promise<void> {
  const scroller = query(THREAD);
  await shown(
    tools,
    "a scrollable transcript at its end",
    `(() => {
    const thread = ${scroller};
    return thread && thread.scrollTop > 24 && thread.scrollHeight - thread.clientHeight - thread.scrollTop < 4;
  })()`,
  );
  const at = await point(tools, THREAD);
  value(
    await tools.send("Input.dispatchMouseEvent", {
      type: "mouseWheel",
      deltaX: 0,
      deltaY: -180,
      ...at,
    }),
  );
  await shown(
    tools,
    "the transcript scrolled away from its end",
    `(() => {
    const thread = ${scroller};
    return thread.scrollHeight - thread.clientHeight - thread.scrollTop > 24;
  })()`,
  );
}

const runSection = (run: string): string => `section[aria-label=${JSON.stringify(run)}]`;

async function showsReply(tools: DevTools, run: string, reply: string): Promise<void> {
  await shown(
    tools,
    `the reply in run ${run}`,
    `${query(runSection(run))}?.innerText.includes(${JSON.stringify(reply)})`,
  );
}

async function exercise(prepared: PreparedCity): Promise<void> {
  const { tools } = prepared;
  const ledger = ledgerOf(prepared);
  const initial = records(ledger);
  const first = initial.find((each) => each.addr === ROOM && each.kind === "run_started");
  assert(first, "The walk must seed a conversation in garden/sensors");
  const firstCall = initial.find((each) => each.kind === "model_called" && each.run === first.run);
  assert(firstCall, "The original user message must have a reply to branch before");
  const oldTail = initial
    .filter((each) => each.kind === "model_returned" && each.run === first.run)
    .at(-1);
  assert(oldTail, "The historical session must have a completed turn");
  const priorOpens = opened(ledger);
  const original = priorOpens.at(-1)?.seq ?? first.seq;
  const historical = `${CURRENT}:${original}`;
  const origin = { run: first.run, at_seq: firstCall.seq - 1 };
  await dom(tools, `location.hash = ${JSON.stringify(CURRENT)}`);
  await shown(tools, "the original conversation", `Boolean(${query(runSection(first.run))})`);

  await write(tools, "/new");
  await tools.key("Enter", "Enter", 13);
  const newer = await waitFor("a newer session in the same room", () => {
    const record = opened(ledger).at(-1);
    return record !== undefined && record.seq > original ? record : undefined;
  });
  await shown(tools, "the current session's empty composer", `${query(BOX)}?.value === ""`);

  await openHistory(tools, historical);
  await scrollBack(tools);
  await write(tools, CONTINUE);
  await click(tools, SEND);
  const resumed = await completed(ledger, CONTINUE);
  assert.equal(resumed.data.session_began, original, "Send must append to the historical session");
  assert.deepEqual(
    inheritedFrom(ledger, resumed.run),
    [{ run: first.run, at_seq: oldTail.seq }],
    "Reading back must not move where the next run inherits the old conversation",
  );
  assert.deepEqual(opened(ledger), [...priorOpens, newer], "Ordinary send must not open a session");
  assert.equal(await dom(tools, "location.hash"), historical, "Continue in the original view");
  await showsReply(tools, resumed.run, CONTINUED_REPLY);
  console.log(`Historical send appended to session #${original} while #${newer.seq} was current`);

  await click(tools, `${runSection(first.run)} [data-fork] button`);
  const branch = await waitFor("the explicit branch to open", () => {
    const record = opened(ledger).at(-1);
    return record !== undefined && record.seq > newer.seq ? record : undefined;
  });
  assert.deepEqual(
    branch.data.from,
    origin,
    "Branch must start before the selected old user message",
  );
  await shown(tools, "the mounted branch composer", sessionComposer(CURRENT, "fork_back"));
  assert.equal(
    records(ledger).some(
      (each) => each.kind === "run_started" && each.addr === ROOM && each.seq > branch.seq,
    ),
    false,
    "Opening the branch must not send a message",
  );

  await write(tools, BRANCH);
  await click(tools, SEND);
  const forked = await completed(ledger, BRANCH);
  assert(
    forked.seq > branch.seq && forked.data.session_began === undefined,
    "The run must start in the current branch",
  );
  assert.deepEqual(
    inheritedFrom(ledger, forked.run),
    [{ run: first.run, at_seq: first.seq }],
    "Before the first turn the sent run inherits only the old run's opening",
  );
  assert.deepEqual(
    opened(ledger),
    [...priorOpens, newer, branch],
    "Sending in the branch must use the session just opened",
  );
  assert.equal(await dom(tools, "location.hash"), CURRENT);
  await showsReply(tools, forked.run, BRANCH_REPLY);
  console.log(
    `Branch #${branch.seq} sent its first run from the selected message in session #${original}`,
  );
}

const prepared = await prepareCity(BUNDLE, true, pair, "send");
await exercise(prepared)
  .catch(async (error: unknown) => {
    const page = await prepared.tools.run(`({
      route: location.hash,
      visibleText: document.body.innerText.slice(-1600),
      draft: ${query(BOX)}?.value,
      sendState: ${query(`${FORM} .talk-coin`)}?.getAttribute("data-control-state"),
    })`);
    if (page.ok) console.error("Session send page at failure", page.value);
    return Promise.reject(error);
  })
  .finally(prepared.close);
