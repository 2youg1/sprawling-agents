// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import words from "../../RefRain/src/lang.json";
import { toFragment } from "../../RefRain/src/core/route.ts";
import { capture, enterWorkspace } from "./acceptance-entry.mjs";

const [port, output] = process.argv.slice(2);
const { PLAYWRIGHT_MODULE, REFRAIN_PDF_BROWSER, REFRAIN_PDF_OPEN_URL_FILE } = process.env;
assert(
  /^\d+$/.test(port ?? "") && output && PLAYWRIGHT_MODULE && REFRAIN_PDF_OPEN_URL_FILE,
  "Pass the loopback port and artifact directory. Supply PLAYWRIGHT_MODULE and REFRAIN_PDF_OPEN_URL_FILE from the test city launcher.",
);
const origin = `http://127.0.0.1:${port}`;
const entry = new URL((await readFile(REFRAIN_PDF_OPEN_URL_FILE, "utf8")).trim());
assert.equal(entry.origin, origin);
const { chromium } = await import(pathToFileURL(PLAYWRIGHT_MODULE).href);
const browser = await chromium.launch({ headless: true, executablePath: REFRAIN_PDF_BROWSER });
const english = (key) => words[key].en;
const draft = "Keep this unsent draft when opening either command dialog.";
const composerSelector = "#workspace-conversation textarea";
const addressSelector = '#workspace-conversation button[aria-haspopup="dialog"]';
const evidence = { browser: browser.version(), errors: [] };
await mkdir(output, { recursive: true });

async function activeOption(input) {
  return input.evaluate((element) => {
    const id = element.getAttribute("aria-activedescendant");
    const option = id === null ? null : document.getElementById(id);
    return option === null
      ? null
      : { id, selected: option.getAttribute("aria-selected"), text: option.textContent.trim() };
  });
}

async function focusIs(page, selector) {
  await page.waitForFunction(
    (target) => document.querySelector(target) === document.activeElement,
    selector,
  );
}

try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 830 }, locale: "en-US" });
  page.setDefaultTimeout(60_000);
  page.on("pageerror", (error) => evidence.errors.push(error.message));
  evidence.entry = await enterWorkspace(page, entry.href);
  const room = toFragment({ kind: "talk", address: "seed/lobby" });
  await page.goto(`${origin}/${room}`);

  const composer = page.locator(composerSelector).first();
  await composer.fill(draft);
  await composer.evaluate((node) => {
    window.paletteTestComposer = node;
  });
  const palette = page.getByRole("dialog", { name: english("nav_palette"), exact: true });
  const commands = palette.getByRole("combobox");
  await page.keyboard.press("Control+/");
  await palette.waitFor();
  assert(await commands.evaluate((input) => input === document.activeElement));
  const first = await activeOption(commands);
  await commands.press("ArrowDown");
  const second = await activeOption(commands);
  assert(first && second && first.id !== second.id && second.selected === "true");

  await commands.fill("/sto");
  await commands.press("Tab");
  assert.equal(await commands.inputValue(), "/stop ");
  const stopped = palette.getByRole("option").filter({ hasText: "/stop" });
  assert.equal(await stopped.getAttribute("aria-disabled"), "true");
  await commands.press("Enter");
  assert(await palette.isVisible());
  await commands.press("Escape");
  await palette.waitFor({ state: "hidden" });
  await focusIs(page, composerSelector);
  assert.equal(await composer.inputValue(), draft);
  assert(await composer.evaluate((node) => node === window.paletteTestComposer));
  evidence.paletteKeyboard = { first, second, completion: "/stop ", refusedStayedOpen: true };

  await page.locator("#workspace-subject-key").click();
  await page.locator("#workspace-navigation-key").click();
  await page.getByRole("button", { name: english("shell_layout_tools"), exact: true }).click();
  await page.locator("#workspace-draft-key").click();
  await focusIs(page, composerSelector);
  await page.keyboard.press("Control+/");
  await commands.fill(english("keys_finder"));
  await palette
    .getByRole("option")
    .filter({ hasText: english("keys_finder") })
    .click();
  await palette.waitFor({ state: "hidden" });
  const finder = page.getByRole("dialog", {
    name: english("finder_title").replace("{building}", "seed"),
    exact: true,
  });
  const files = finder.getByRole("combobox");
  assert(await files.evaluate((input) => input === document.activeElement));
  await files.fill("fixture");
  await finder.getByRole("option").filter({ hasText: "fixture-omega.md" }).waitFor();
  const foundFirst = await activeOption(files);
  await files.press("ArrowDown");
  const foundSecond = await activeOption(files);
  assert(
    foundFirst &&
      foundSecond &&
      foundFirst.id !== foundSecond.id &&
      foundSecond.selected === "true",
  );
  await files.press("Escape");
  await finder.waitFor({ state: "hidden" });
  await focusIs(page, composerSelector);
  assert(await page.locator(".refrain-workspace[data-draft]").isVisible());
  assert.equal(await composer.inputValue(), draft);
  evidence.finderKeyboard = { foundFirst, foundSecond, draftStillOpen: true };

  await page.keyboard.press("Control+/");
  await commands.fill(english("keys_finder"));
  await commands.press("Enter");
  await files.fill("fixture-omega");
  await finder.getByRole("option").filter({ hasText: "fixture-omega.md" }).waitFor();
  await files.press("Enter");
  await finder.waitFor({ state: "hidden" });
  const inspector = page.locator(".refrain-file-inspector");
  await inspector.getByText("fixture-omega.md", { exact: true }).first().waitFor();
  assert.equal(new URL(page.url()).hash, room);
  assert.equal(await composer.inputValue(), draft);
  evidence.opened = "fixture-omega.md";
  await inspector.getByRole("button", { name: english("inspect_close"), exact: true }).click();
  await inspector.waitFor({ state: "hidden" });

  await composer.focus();
  await page.keyboard.press("Control+/");
  await commands.fill("/cd");
  await commands.press("Enter");
  await page.waitForFunction((hash) => location.hash === hash, toFragment({ kind: "city" }));
  await page.goBack();
  await page.waitForFunction((hash) => location.hash === hash, room);
  await page.waitForFunction(
    ({ selector, text }) => document.querySelector(selector)?.value === text,
    { selector: composerSelector, text: draft },
  );
  evidence.slashDispatch = { command: "/cd", route: "city", draftedRoomRetained: true };

  await page.goto(origin);
  await page.waitForFunction(
    ({ selector, text }) => document.querySelector(selector)?.value === text,
    { selector: composerSelector, text: draft },
  );
  const settingsKey = "#workspace-settings-key";
  await page.locator(settingsKey).press("Enter");
  const settings = page
    .locator("dialog[open]")
    .filter({ has: page.getByRole("navigation") })
    .first();
  const group = settings.getByRole("button", { name: /^preferences$/i }).first();
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  await settings
    .getByRole("button", { name: /^appearance$/i })
    .first()
    .click();
  await page.waitForFunction(() => location.hash === "#/setup/appearance");
  await page.keyboard.press("Escape");
  await settings.waitFor({ state: "hidden" });
  await focusIs(page, settingsKey);
  assert.equal(new URL(page.url()).hash, "");
  assert.equal(await composer.inputValue(), draft);
  evidence.settingsBareRoute = { sameDraft: true, focusReturned: true };

  await page.locator("#workspace-subject-key").click();
  await composer.fill("");
  await page.setViewportSize({ width: 390, height: 844 });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: true });
  await page.waitForFunction(() => matchMedia("(pointer: coarse)").matches);
  const address = page.locator(addressSelector).first();
  await address.waitFor();
  const location = await address.boundingBox();
  assert(location);
  assert(
    await address.evaluate((node) => {
      const rect = node.getBoundingClientRect();
      const hit = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2);
      return hit !== null && node.contains(hit);
    }),
  );
  await cdp.send("Input.dispatchTouchEvent", {
    type: "touchStart",
    touchPoints: [{ x: location.x + location.width / 2, y: location.y + location.height / 2 }],
  });
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await palette.waitFor();
  assert(await palette.evaluate((panel) => panel === document.activeElement));
  await page.keyboard.press("Escape");
  await palette.waitFor({ state: "hidden" });
  assert(await address.evaluate((node) => node === document.activeElement));
  await cdp.detach();
  evidence.paletteTouch = { inputNotFocused: true, returned: true };
  await capture(page, join(output, "palette-return.png"));
  assert.deepEqual(evidence.errors, []);
  evidence.result = "success";
} catch (error) {
  evidence.result = "failure";
  evidence.failure = error.message;
  throw error;
} finally {
  await writeFile(join(output, "production-palette.json"), JSON.stringify(evidence, null, 2));
  await browser.close();
}
