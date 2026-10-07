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
import { observeModalFocus } from "./acceptance-focus.mjs";


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
        await observeModalFocus(page, dialog, focus);
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
