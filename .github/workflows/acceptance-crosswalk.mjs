// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { enterWorkspace } from "./acceptance-entry.mjs";
const [port, output, variant = process.env.REFRAIN_BASELINE_VARIANT] = process.argv.slice(2);
const { PLAYWRIGHT_MODULE, REFRAIN_PDF_BROWSER, REFRAIN_PDF_OPEN_URL_FILE } = process.env;
assert(/^\d+$/.test(port ?? "") && output && PLAYWRIGHT_MODULE && REFRAIN_PDF_OPEN_URL_FILE);
assert(variant === "baseline" || variant === "candidate");
const { chromium } = await import(pathToFileURL(PLAYWRIGHT_MODULE).href);
const browser = await chromium.launch({ headless: true, executablePath: REFRAIN_PDF_BROWSER });
const origin = `http://127.0.0.1:${port}`;
const oldFixed = variant === "baseline";
const report = {
  commit: oldFixed ? "8dbb84b48a11" : null,
  browser: browser.version(),
  locale: "en-US",
  viewport: [1280, 830],
  probes: {},
  pageErrors: [],
};
function safeError(error) {
  return String(error.message)
    .split("\n")[0]
    .replace(/#open=[^\s'"<>]+/g, "#open");
}
const evidenceLabels = [
  "welcome",
  "session",
  "settings",
  "record-ledger",
  "record-archive",
  "mcp",
  "city",
];
await mkdir(output, { recursive: true });
try {
  const page = await browser.newPage({
    viewport: { width: 1280, height: 830 },
    locale: "en-US",
    colorScheme: "dark",
  });
  page.setDefaultTimeout(10000);
  page.on("pageerror", (e) => report.pageErrors.push(safeError(e)));
  async function snapshot(name) {
    await paint();
    const accessibility = await page.locator("body").ariaSnapshot();
    const snap = await page.evaluate(() => ({
      fragment: location.hash.startsWith("#open=") ? "#" : location.hash,
      active: {
        role: document.activeElement?.getAttribute("role"),
        tag: document.activeElement?.tagName,
        name: document.activeElement?.getAttribute("aria-label"),
      },
      controls: [...document.querySelectorAll("button,a[href],input,select,textarea,[role=tab]")]
        .filter(
          (n) =>
            n.checkVisibility({
              opacityProperty: true,
              visibilityProperty: true,
              contentVisibilityAuto: true,
            }) && !n.closest("[inert],[aria-hidden=true]"),
        )
        .map((n) => ({
          type: n.getAttribute("role") ?? n.tagName,
          name:
            n.getAttribute("aria-label") || n.getAttribute("placeholder") || n.textContent.trim(),
          enabled: n.disabled !== true && n.getAttribute("aria-disabled") !== "true",
          expanded: n.getAttribute("aria-expanded"),
          pressed: n.getAttribute("aria-pressed"),
          selected: n.getAttribute("aria-selected"),
        })),
    }));
    snap.controls = snap.controls.filter((x) => x.name.length > 0);
    report.probes[name] = { ...snap, accessibility, status: "observed" };
    if (process.env.REFRAIN_SCREENSHOTS !== "0" && evidenceLabels.includes(name)) {
      try {
        await page.screenshot({ path: join(output, name + ".png") });
      } catch (error) {
        (report.captureWarnings ??= {})[name] = safeError(error);
      }
    }
  }
  async function probe(name, run) {
    try {
      await run();
      await snapshot(name);
    } catch (e) {
      const visibleMain = await page
        .locator("main")
        .first()
        .ariaSnapshot({ timeout: 1000 })
        .catch(() => null);
      report.probes[name] = { status: "unverified", detail: safeError(e), visibleMain };
    }
  }
  async function route(hash, text, ready) {
    await page.goto(origin + "/" + hash);
    await page.waitForFunction((fragment) => location.hash === fragment, hash);
    if (ready) await ready();
    else await page.locator("main").filter({ hasText: text }).first().waitFor({ timeout: 30_000 });
  }
  async function paint() {
    await page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
  }
  const code = (await readFile(REFRAIN_PDF_OPEN_URL_FILE, "utf8")).trim();
  assert.equal(new URL(code).origin, origin);
  report.entry = await enterWorkspace(page, code, { observeGuide: () => snapshot("welcome") });
  const composer = page.locator("main textarea").first();
  await snapshot("session");
  await probe("palette-normal", async () => {
    await composer.fill("crosswalk draft");
    await page.keyboard.press("Control+/");
    const prompt = page.locator("[role=combobox]").first();
    await prompt.waitFor();
    await prompt.press("ArrowDown");
    assert(await prompt.getAttribute("aria-activedescendant"));
    await snapshot("palette-arrows");
    await prompt.press("Escape");
    assert.equal(await composer.inputValue(), "crosswalk draft");
    assert(await composer.evaluate((element) => document.activeElement === element));
  });
  await probe("mailbox", async () => {
    const opener = page.getByRole("button", { name: /^mailbox/i }).first();
    await opener.click();
    await page
      .getByRole("complementary", { name: /^mailbox/i })
      .first()
      .waitFor();
    await snapshot("mailbox-open");
    await page.keyboard.press("Escape");
    await page
      .getByRole("complementary", { name: /^mailbox/i })
      .first()
      .waitFor({ state: "hidden" });
  });
  await probe("settings", async () => {
    const settings = oldFixed
      ? page.getByRole("link", { name: /^settings$/i }).first()
      : page.locator("#workspace-settings-key");
    await settings.click();
    await page.locator("dialog[open]").first().waitFor();
  });
  await probe("settings-appearance", async () => {
    const panel = page.locator("dialog[open]").first();
    const prefs = panel.getByRole("button", { name: /^preferences$/i }).first();
    if (await prefs.isVisible()) await prefs.click();
    const appearance = panel.getByRole("button", { name: /^appearance$/i }).first();
    await appearance.click();
    await page.waitForFunction(() => location.hash.includes("appearance"));
    await snapshot("appearance-content");
    await page.keyboard.press("Escape");
    await page.locator("dialog[open]").first().waitFor({ state: "hidden" });
  });
  if (await page.locator("dialog[open]").first().isVisible()) await page.keyboard.press("Escape");
  if (oldFixed)
    await probe("old-tier-action", async () => {
      const tier = page.getByRole("button", { name: /^layers.*zen/i });
      await tier.click();
      await page
        .getByRole("button", { name: /^layers.*(blend|panorama)/i })
        .first()
        .waitFor({ timeout: 3500 });
    });
  for (const [name, hash, text] of [
    ["city", "#/city", /city/i],
    ["building", "#/building/hall", /hall/i],
    ["raise", "#/raise", /build|workspace/i],
    ["record-ledger", "#/record", /record|timeline/i],
    ["mcp", "#/mcp", /MCP/i],
    ["cost", "#/cost", /cost/i],
    ["registry", "#/registry", /registry/i],
    ["monitor", "#/monitor", /monitor|performance/i],
    ["building-catalog", "#/building/hall/catalog", /catalog/i],
  ])
    await probe(name, () => route(hash, text));
  await probe("record-archive", async () => {
    await route("#/record", /timeline/i);
    const archive = page.getByRole("tab", { name: /archive/i });
    await archive.click();
    await page.waitForFunction(() => location.hash === "#/record/archive");
    await paint();
  });
  await probe("record-bin-arrow", async () => {
    const archive = page.getByRole("tab", { name: /archive/i });
    await archive.press("ArrowRight");
    await page.waitForFunction(() => location.hash === "#/record/bin");
    await paint();
  });
  for (const group of [
    "you",
    "accounts",
    "agents",
    "network",
    "remote",
    "devices",
    "run",
    "rules",
    "automation",
    "skills",
    "tools",
    "performance",
    "appearance",
    "colours",
    "keys",
    "advanced",
    "privacy",
    "about",
  ])
    await probe("setup-" + group, async () => {
      await page.goto(origin + "/#/setup/" + group);
      await page.locator("dialog[open]").first().waitFor();
      assert.equal(new URL(page.url()).hash, "#/setup/" + group);
    });
  if (!oldFixed)
    await probe("rapid-navigation", async () => {
      await route("#/city", /city/i);
      await page.evaluate(() => {
        location.hash = "#/record";
        location.hash = "#/registry";
        location.hash = "#/cost";
      });
      await page.waitForFunction(() => location.hash === "#/cost");
      await page.locator("main").filter({ hasText: /cost/i }).first().waitFor();
      await paint();
    });
  if (!oldFixed)
    await probe("single-main-semantic", async () => {
      for (const [hash, text] of [
        ["#/", /the Mayor/i],
        ["#/city", /city/i],
        ["#/record/archive", /archive/i],
        ["#/building/hall/catalog", /catalog/i],
      ]) {
        await route(hash, text);
        assert.equal(await page.locator("#main").count(), 1);
        const link = page.getByRole("link", { name: /skip to the page/i });
        await link.press("Enter");
        assert.equal(new URL(page.url()).hash, hash);
        await page.waitForFunction(() => Boolean(document.activeElement?.closest("main#main")));
      }
    });
} catch (e) {
  report.fatal = safeError(e);
} finally {
  await browser.close();
  await writeFile(join(output, "access.json"), JSON.stringify(report, null, 2));
}
console.log(
  JSON.stringify({
    statuses: Object.entries(report.probes).map(([name, row]) => [name, row.status]),
    errors: report.pageErrors,
    fatal: report.fatal,
  }),
);

assert.equal(report.fatal, undefined);
assert.deepEqual(report.pageErrors, []);
assert(Object.values(report.probes).every((probe) => probe.status === "observed"));
