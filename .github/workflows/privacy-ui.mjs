// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The privacy page of the served release build, on a disposable Windows
// runner (crates/sprawling/spec/Privacy.lean section 16). `lake exe
// acceptance client-ui` serves the city and hands this script its port.
//
// The page is entered the way a person enters it: Settings, the Advanced
// group, then the Advanced group's link. What the page must show is read
// from the city's own `Answer::Privacy`, taken off the socket the page
// itself opened, never from the source tables: one entry per control the
// answer names, every line not written with its reason, the host line,
// and the edition fit the host computed. Two controls are then applied
// and restored through the page, one of each scope: `start_launch_tracking`
// writes this account's settings without approval and
// `feedback_notifications` writes the machine's through an elevated child.
// Each write is read back with reg.exe, outside the city, and the
// restore must leave the value exactly as it was before, absent included.
// Every state is cropped to the element it concerns at 1440 and 1920
// wide, dark and light.

import { mkdir, writeFile, readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { join } from "node:path";
import words from "../../client/src/lang.json";
import { DEFAULT_VIEW, toFragment } from "../../client/src/core/route.ts";
import { TREE } from "../../client/src/views/settings/tree.ts";
import { HEADING } from "../../client/src/views/setup/groups.ts";
import { observeModalFocus } from "./acceptance-focus.mjs";

const [port, output] = process.argv.slice(2);
if (!/^\d+$/.test(port ?? "") || !output || !process.env.PLAYWRIGHT_MODULE || !process.env.SPRAWLING_BIN) {
  throw new Error("usage: PLAYWRIGHT_MODULE=<installed module> SPRAWLING_BIN=<binary> bun privacy-ui.mjs <port> <output>");
}
if (process.env.GITHUB_ACTIONS !== "true" || process.env.RUNNER_OS !== "Windows" || process.env.SPRAWLING_DISPOSABLE_PRIVACY !== "1") {
  throw new Error("privacy-ui.mjs writes this machine's settings and runs only on a disposable Windows runner");
}

// One reading of how long the host may take: an answer reads every
// control, the scheduled tasks through Windows PowerShell, and a machine
// write starts an elevated child first.
const HOST = 180_000;
const OPERATED = ["start_launch_tracking", "feedback_notifications"];

const english = (key) => {
  const text = words[key]?.en;
  if (!text) throw new Error(`the source has no English word for ${key}`);
  return text;
};
const filled = (key, slots) => Object.entries(slots).reduce((text, [name, value]) => text.replaceAll(`{${name}}`, value), english(key));
const title = (control) => english(`privacy_control_${control}_title`);

// What reg.exe says the value is: absent, or its type and data as reg.exe
// prints them. The city never reads this.
function registry(target) {
  const hive = target.kind === "registry_value_hkcu" ? "HKCU" : "HKLM";
  try {
    const printed = execFileSync("reg", ["query", `${hive}\\${target.path}`, "/v", target.name, "/reg:64"], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
    const line = printed.split(/\r?\n/).find((each) => each.trim().startsWith(`${target.name} `) || each.trim().startsWith(`${target.name}\t`));
    if (!line) throw new Error(`reg.exe printed no line for ${target.name}`);
    const [, type, data] = line.trim().split(/\s{2,}|\t/).filter(Boolean);
    return { type, data };
  } catch (error) {
    if (error.status === 1) return "absent";
    throw error;
  }
}
const dwordOf = (number) => ({ type: "REG_DWORD", data: `0x${number.toString(16)}` });

// The latest `Answer::Privacy` the page received, wherever the frame
// carries it.
function privacyIn(value) {
  if (value === null || typeof value !== "object") return undefined;
  if (value.privacy && typeof value.privacy === "object" && Array.isArray(value.privacy.controls)) return value.privacy;
  for (const inner of Object.values(value)) {
    const found = privacyIn(inner);
    if (found) return found;
  }
  return undefined;
}

const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);
await mkdir(output, { recursive: true });
const evidence = {
  commit: execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
  tree: execFileSync("git", ["rev-parse", "HEAD^{tree}"], { encoding: "utf8" }).trim(),
  binarySha256: createHash("sha256").update(await readFile(process.env.SPRAWLING_BIN)).digest("hex"),
  run: process.env.GITHUB_RUN_ID,
  attempt: process.env.GITHUB_RUN_ATTEMPT,
  browser: null,
  host: null,
  screens: [],
  operations: [],
  errors: [],
};
const browser = await chromium.launch({ headless: true });
evidence.browser = browser.version();
let activePage;
let firstAnswer;

async function settled(page) {
  await page.evaluate(async () => {
    await Promise.allSettled(document.getAnimations().filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity).map((animation) => animation.finished));
  });
}

async function shot(page, locator, name, record) {
  await locator.scrollIntoViewIfNeeded();
  await settled(page);
  const file = `${name}.png`;
  await locator.screenshot({ path: join(output, file) });
  record.shots.push(file);
}

// Opens Settings from the conversation, the Advanced group from the tree,
// and the privacy page from the Advanced group's link.
async function enter(page) {
  await page.goto(`http://127.0.0.1:${port}/${toFragment({ kind: "welcome" })}`);
  const putOff = page.getByRole("button", { name: english("guide_put_off_rest"), exact: true });
  await putOff.waitFor();
  await putOff.click();
  await page.waitForFunction((fragment) => location.hash === fragment, toFragment(DEFAULT_VIEW));
  const opener = page.getByRole("link", { name: english("nav_settings"), exact: true });
  await opener.waitFor();
  await opener.focus();
  await page.keyboard.press("Enter");
  const settings = page.getByRole("dialog");
  await settings.waitFor();
  const branch = TREE.find((item) => [...item.entries, ...item.more].some((entry) => entry.kind === "group" && entry.group === "advanced"));
  if (!branch) throw new Error("the settings tree has no advanced group");
  const branchButton = settings.getByRole("button", { name: english(branch.word), exact: true });
  if (await branchButton.getAttribute("aria-expanded") !== "true") await branchButton.click();
  if (branch.more.some((entry) => entry.kind === "group" && entry.group === "advanced")) {
    const more = settings.getByRole("button", { name: english("settings_more"), exact: true });
    if (await more.getAttribute("aria-expanded") !== "true") await more.click();
  }
  await settings.getByRole("button", { name: english(HEADING.advanced), exact: true }).click();
  await page.waitForFunction((fragment) => location.hash === fragment, toFragment({ kind: "setup", group: "advanced" }));
  await settings.getByRole("link", { name: english("setup_group_privacy"), exact: true }).click();
  await page.waitForFunction((fragment) => location.hash === fragment, toFragment({ kind: "setup", group: "privacy" }));
  await settings.getByRole("heading", { name: english(HEADING.privacy), exact: true }).waitFor();
}

// Every control the city answered is drawn as its own entry, and every
// line it does not write is drawn with its reason. Locators start at the
// page: the settings sheet becomes inert while a confirmation is open
// over it, and every entry is named by its title, which only this page
// draws.
async function judgeAnswer(page, answer) {
  if (!("windows" in answer.host)) throw new Error(`the runner's host was not read as Windows: ${JSON.stringify(answer.host)}`);
  const articles = page.getByRole("article");
  await articles.nth(answer.controls.length - 1).waitFor({ timeout: HOST });
  const drawn = await articles.count();
  if (drawn !== answer.controls.length) throw new Error(`the page drew ${drawn} entries for ${answer.controls.length} controls`);
  for (const entry of answer.controls) {
    const article = page.getByRole("article", { name: title(entry.control), exact: true });
    if (await article.count() !== 1) throw new Error(`${entry.control} is not drawn exactly once`);
    const fit = english(`privacy_fit_${entry.host_fit}`);
    if (await article.getByText(fit, { exact: true }).count() !== 1) throw new Error(`${entry.control} does not state its edition fit ${entry.host_fit}`);
  }
  for (const line of answer.not_written) {
    if (await page.getByText(line.line.text, { exact: true }).count() < 1) throw new Error(`the line not written ${line.line.item} is not drawn`);
    if (await page.getByText(english(`privacy_reason_${line.reason}`), { exact: true }).count() < 1) throw new Error(`the reason ${line.reason} is not drawn`);
  }
}

// Opens the confirmation for `action`, checks what it states and its
// keyboard walk, cancels it once with Escape (nothing may be sent and
// focus goes back to the press), then confirms.
async function confirm(page, article, control, action, sent, record, name) {
  const press = article.getByRole("button", { name: english(`privacy_${action}`), exact: true });
  await press.waitFor({ timeout: HOST });
  await press.focus();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: filled(`privacy_confirm_${action}`, { title: title(control) }), exact: true });
  await dialog.waitFor();
  for (const key of [`privacy_control_${control}_overlooked`, `privacy_control_${control}_undo`]) {
    if (await dialog.getByText(english(key), { exact: true }).count() !== 1) throw new Error(`the ${action} confirmation for ${control} does not state ${key}`);
  }
  await shot(page, dialog, `${name}-confirm`, record);
  const focus = { traversals: [] };
  record.focus.push({ dialog: `${control} ${action}`, focus });
  await observeModalFocus(page, dialog, focus);
  const before = sent.length;
  await page.keyboard.press("Escape");
  await dialog.waitFor({ state: "hidden" });
  if (sent.length !== before) throw new Error(`cancelling the ${action} of ${control} sent a command`);
  if (!(await press.evaluate((element) => element === document.activeElement))) throw new Error(`cancelling the ${action} of ${control} did not give focus back to its press`);
  await press.click();
  await dialog.waitFor();
  await dialog.getByRole("button", { name: english(`privacy_${action}`), exact: true }).click();
  await dialog.waitFor({ state: "hidden" });
  if (sent.length !== before + 1) throw new Error(`confirming the ${action} of ${control} sent ${sent.length - before} commands`);
}

async function operate(page, answer, control, sent, record, prefix) {
  const entry = answer.controls.find((each) => each.control === control);
  if (!entry) throw new Error(`the answer has no ${control}`);
  if (entry.written?.value !== "dword") throw new Error(`${control} does not write a REG_DWORD on this host: ${JSON.stringify(entry.written)}`);
  // A runner image that already holds the written value offers no apply,
  // and this run would then prove nothing about a write; say so instead of
  // timing out on a press that is rightly absent.
  if (JSON.stringify(entry.current?.read?.value) === JSON.stringify(entry.written)) {
    throw new Error(`${control} already holds its written value on this runner, so the page offers no apply`);
  }
  const article = page.getByRole("article", { name: title(control), exact: true });
  const result = (key) => article.locator('[role="status"]', { hasText: english(key) }).waitFor({ timeout: HOST });
  const original = registry(entry.target);
  const operation = { control, scope: entry.scope, original, applied: null, restored: null };
  evidence.operations.push(operation);
  await shot(page, article, `${prefix}-${control}-before`, record);

  await confirm(page, article, control, "apply", sent, record, `${prefix}-${control}-apply`);
  await result("privacy_result_applied");
  operation.applied = registry(entry.target);
  if (JSON.stringify(operation.applied) !== JSON.stringify(dwordOf(entry.written.number))) {
    throw new Error(`${control} reads ${JSON.stringify(operation.applied)} after apply, not ${entry.written.number}`);
  }
  await shot(page, article, `${prefix}-${control}-applied`, record);

  await confirm(page, article, control, "restore", sent, record, `${prefix}-${control}-restore`);
  await result("privacy_result_restored");
  operation.restored = registry(entry.target);
  if (JSON.stringify(operation.restored) !== JSON.stringify(original)) {
    throw new Error(`${control} reads ${JSON.stringify(operation.restored)} after restore, not ${JSON.stringify(original)}`);
  }
  await article.getByRole("button", { name: english("privacy_apply"), exact: true }).waitFor({ timeout: HOST });
  await shot(page, article, `${prefix}-${control}-restored`, record);
}

try {
  for (const width of [1440, 1920]) {
    for (const colorScheme of ["dark", "light"]) {
      const prefix = `${width}-${colorScheme}`;
      const record = { width, colorScheme, shots: [], focus: [] };
      evidence.screens.push(record);
      const context = await browser.newContext({ viewport: { width, height: 1080 }, colorScheme, locale: "en-US" });
      const page = await context.newPage();
      activePage = page;
      page.on("pageerror", (error) => evidence.errors.push(error.message));
      let answer;
      const sent = [];
      page.on("websocket", (socket) => {
        socket.on("framereceived", ({ payload }) => {
          if (typeof payload !== "string" || !payload.includes('"privacy"')) return;
          const found = privacyIn(JSON.parse(payload));
          if (found) answer = found;
        });
        socket.on("framesent", ({ payload }) => {
          if (typeof payload === "string" && payload.includes("privacy_operation")) sent.push(payload);
        });
      });
      await enter(page);
      const deadline = Date.now() + HOST;
      while (answer === undefined && Date.now() < deadline) await page.waitForTimeout(250);
      if (answer === undefined) throw new Error("the city sent no privacy answer");
      firstAnswer ??= answer;
      evidence.host ??= answer.host;
      await judgeAnswer(page, answer);
      await shot(page, page.getByRole("region", { name: english("privacy_host_title"), exact: true }), `${prefix}-host`, record);
      const ignored = answer.controls.find((entry) => entry.host_fit === "ignored");
      if (ignored) await shot(page, page.getByRole("article", { name: title(ignored.control), exact: true }), `${prefix}-ignored-${ignored.control}`, record);
      if (answer.not_written.length > 0) {
        await shot(page, page.getByRole("region", { name: english("privacy_not_written_title"), exact: true }), `${prefix}-not-written`, record);
      }
      for (const control of OPERATED) await operate(page, answer, control, sent, record, prefix);
      await context.close();
      activePage = undefined;
    }
  }
  if (evidence.errors.length) throw new Error("the served client reported runtime JavaScript errors");
  evidence.result = "success";
} catch (error) {
  evidence.result = "failure";
  evidence.failure = error.message;
  if (activePage && !activePage.isClosed()) {
    evidence.failureRoute = new URL(activePage.url()).hash;
    await activePage.screenshot({ path: join(output, "failure.png") });
    await writeFile(join(output, "failure.html"), await activePage.content());
  }
  throw error;
} finally {
  if (firstAnswer) await writeFile(join(output, "answer.json"), JSON.stringify(firstAnswer, null, 2));
  await writeFile(join(output, "privacy-ui.json"), JSON.stringify(evidence, null, 2));
  await browser.close();
}
