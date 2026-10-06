// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { mkdir, writeFile, readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { join } from "node:path";
import words from "../../client/src/lang.json";
import { DEFAULT_VIEW, toFragment } from "../../client/src/core/route.ts";
import { TREE } from "../../client/src/views/settings/tree.ts";
import { HEADING } from "../../client/src/views/setup/groups.ts";

function advanceSettingsTraversal(state, sample) {
  if (!sample.modalOpen || !sample.stable) throw new Error("modal closed, replaced, or its Tab inventory changed");
  if (!sample.inside) {
    if (sample.documentHasFocus || !sample.bodyActive || sample.tag !== "BODY") throw new Error("Tab reached the background document");
    if (state.chromePending) throw new Error("Tab did not return from browser chrome on the next step");
    state.chromePending = true;
    return;
  }
  if (!sample.documentHasFocus) throw new Error("modal element is stale while browser chrome holds focus");
  if (!Number.isInteger(sample.stop) || sample.stop < 0 || sample.stop >= state.total) throw new Error("Tab reached an unlisted modal element");
  state.chromePending = false;
  if (sample.stop === state.start) {
    if (state.seen.size !== state.total) throw new Error("Tab returned before visiting every control");
    state.complete = true;
    return;
  }
  if (state.seen.has(sample.stop)) throw new Error("Tab repeated a control before returning to the start");
  state.seen.add(sample.stop);
}

async function observeSettingsFocus(page, dialog, focus) {
  const scope = await dialog.evaluateHandle((element) => {
    const collect = () => {
      const descendants = [...element.querySelectorAll("*")];
      if (descendants.some((node) => node.shadowRoot || node.tagName.includes("-") || node.matches("iframe, object, embed, input[type=radio], audio[controls], video[controls], img[usemap]") || (!(node instanceof HTMLElement) && node.tabIndex >= 0))) {
        throw new Error("unsupported sequential focus scope in modal inventory");
      }
      return descendants.filter((node) => {
        if (!(node instanceof HTMLElement) || node.matches(":disabled") || node.closest("[inert]")) return false;
        const style = getComputedStyle(node);
        if (style.visibility !== "visible" || !node.checkVisibility() || node.getClientRects().length === 0) return false;
        return node.tabIndex >= 0 || (node.isContentEditable && !node.hasAttribute("tabindex") && !node.parentElement?.isContentEditable);
      });
    };
    const stops = collect();
    return {
      inventory: stops.map((node, stop) => ({ stop, tag: node.tagName, role: node.getAttribute("role"), name: node.getAttribute("aria-label") ?? node.textContent?.trim() })),
      read: () => {
        const current = collect();
        const active = document.activeElement;
        return {
          tag: active?.tagName,
          role: active?.getAttribute("role"),
          name: active?.getAttribute("aria-label") ?? active?.textContent?.trim(),
          inside: Boolean(active && element.contains(active)),
          stop: stops.indexOf(active),
          documentHasFocus: document.hasFocus(),
          modalOpen: element.isConnected && element.matches("dialog:modal") && document.querySelector("dialog:modal") === element,
          bodyActive: active === document.body,
          stable: current.length === stops.length && current.every((node, index) => node === stops[index]),
        };
      },
    };
  });
  try {
    focus.inventory = await scope.evaluate((value) => value.inventory);
    focus.initial = await scope.evaluate((value) => value.read());
    if (!focus.inventory.length || !focus.initial.modalOpen || !focus.initial.stable || !focus.initial.inside || !focus.initial.documentHasFocus || focus.initial.stop < 0) {
      throw new Error("traversal needs a focused modal control and a nonempty stable Tab inventory");
    }
    for (const key of ["Tab", "Shift+Tab"]) {
      const traversal = { key, samples: [], complete: false };
      focus.traversals.push(traversal);
      const state = { total: focus.inventory.length, start: focus.initial.stop, seen: new Set([focus.initial.stop]), chromePending: false, complete: false };
      // One visit per control, the return to the start, and at most one chrome stop between each pair.
      for (let step = 0; step < state.total * 2 && !state.complete; step += 1) {
        await page.keyboard.press(key);
        const sample = await scope.evaluate((value) => value.read());
        traversal.samples.push(sample);
        advanceSettingsTraversal(state, sample);
      }
      traversal.visited = [...state.seen];
      traversal.complete = state.complete;
      if (!state.complete || state.chromePending) throw new Error(`${key} did not complete the modal traversal`);
    }
  } finally {
    await scope.dispose();
  }
}

const [port, output] = process.argv.slice(2);
if (!/^\d+$/.test(port ?? "") || !output || !process.env.PLAYWRIGHT_MODULE) {
  throw new Error("usage: PLAYWRIGHT_MODULE=<installed module> bun acceptance-ui.mjs <port> <output>");
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
  screens: [],
  errors: [],
};
const browser = await chromium.launch({ headless: true });
let activePage;
evidence.browser = browser.version();
const english = (key) => {
  const text = words[key]?.en;
  if (!text) throw new Error(`the source has no English word for ${key}`);
  return text;
};
try {
  for (const width of [1440, 1920]) {
    for (const colorScheme of ["dark", "light"]) {
      const context = await browser.newContext({ viewport: { width, height: 1080 }, colorScheme, locale: "en-US" });
      const page = await context.newPage();
      activePage = page;
      page.on("pageerror", (error) => evidence.errors.push(error.message));
      await page.goto(`http://127.0.0.1:${port}/${toFragment({ kind: "welcome" })}`);
      const putOff = page.getByRole("button", { name: english("guide_put_off_rest"), exact: true });
      await putOff.waitFor();
      await putOff.click();
      await page.waitForFunction((fragment) => location.hash === fragment, toFragment(DEFAULT_VIEW));
      await page.locator("main textarea").waitFor();
      const opener = page.getByRole("link", { name: english("nav_settings"), exact: true });
      await opener.waitFor();
      const openerElement = await opener.elementHandle();
      if (openerElement === null) throw new Error("the rendered conversation has no settings opener");
      await openerElement.focus();
      await page.keyboard.press("Enter");
      const dialog = page.getByRole("dialog");
      await dialog.waitFor();
      for (const group of ["you", "accounts", "advanced"]) {
        const branch = TREE.find((item) => [...item.entries, ...item.more].some((entry) => entry.kind === "group" && entry.group === group));
        if (!branch) throw new Error(`the settings tree has no ${group} group`);
        const branchButton = dialog.getByRole("button", { name: english(branch.word), exact: true });
        if (await branchButton.getAttribute("aria-expanded") !== "true") await branchButton.click();
        if (branch.more.some((entry) => entry.kind === "group" && entry.group === group)) {
          const more = dialog.getByRole("button", { name: english("settings_more"), exact: true });
          if (await more.getAttribute("aria-expanded") !== "true") await more.click();
        }
        const entry = dialog.getByRole("button", { name: english(HEADING[group]), exact: true });
        await entry.focus();
        await page.keyboard.press("Enter");
        await page.waitForFunction((fragment) => location.hash === fragment, toFragment({ kind: "setup", group }));
        await dialog.getByRole("heading", { name: english(HEADING[group]), exact: true }).waitFor();
        await page.evaluate(async () => {
          await Promise.allSettled(document.getAnimations().filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity).map((animation) => animation.finished));
        });
        const focus = { traversals: [] };
        const screenshot = `${group}-${width}-${colorScheme}.png`;
        const screen = { group, route: new URL(page.url()).hash, width, colorScheme, screenshot, focus };
        evidence.screens.push(screen);
        await observeSettingsFocus(page, dialog, focus);
        await page.screenshot({ path: join(output, screenshot) });
        const geometry = await dialog.evaluate((element) => {
          const box = element.getBoundingClientRect();
          return { left: box.left, right: box.right, width: window.innerWidth, overflow: document.documentElement.scrollWidth > window.innerWidth };
        });
        screen.geometry = geometry;
        if (geometry.left < 0 || geometry.right > geometry.width || geometry.overflow) throw new Error(`${group}: settings overflow the viewport`);
      }
      await page.keyboard.press("Escape");
      await dialog.waitFor({ state: "hidden" });
      await page.waitForFunction((element) => element?.isConnected && element === document.activeElement, openerElement);
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
  await writeFile(join(output, "production-ui.json"), JSON.stringify(evidence, null, 2));
  await browser.close();
}
