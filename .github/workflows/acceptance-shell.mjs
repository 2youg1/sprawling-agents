// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import words from "../../RefRain/src/lang.json";
import { capture, enterWorkspace } from "./acceptance-entry.mjs";
import { animationIntervals, traceInput } from "./workspace-input.mjs";

const [port, output] = process.argv.slice(2);
const { PLAYWRIGHT_MODULE, REFRAIN_PDF_BROWSER, REFRAIN_PDF_OPEN_URL_FILE, REFRAIN_PDF_SCAN } =
  process.env;
assert(
  /^\d+$/.test(port ?? "") &&
    output &&
    PLAYWRIGHT_MODULE &&
    REFRAIN_PDF_OPEN_URL_FILE &&
    REFRAIN_PDF_SCAN,
);
const { chromium } = await import(pathToFileURL(PLAYWRIGHT_MODULE).href);
const origin = `http://127.0.0.1:${port}`;
const entry = new URL((await readFile(REFRAIN_PDF_OPEN_URL_FILE, "utf8")).trim());
assert.equal(entry.origin, origin);
const browser = await chromium.launch({ headless: true, executablePath: REFRAIN_PDF_BROWSER });
const root = await mkdtemp(join(tmpdir(), "refrain-shell-fixture-"));
const evidence = { browser: browser.version(), errors: [] };
const english = (key) => words[key].en;
const launch = "workspace-actions-key";
const settings = "workspace-settings-key";
const navigation = "workspace-navigation-key";
const mailbox = "workspace-mailbox-key";
const subject = "workspace-subject-key";
const actions = [settings, navigation, mailbox, subject];
const paper = "layout-paper.pdf";
const drawer = (page) => page.locator("#workspace-navigation");
const key = (page, id) => page.locator(`#${id}`);
const navButton = (page, name) =>
  drawer(page).getByRole("button", { name: english(name), exact: true });
const splitter = (page) =>
  page.locator('[role="separator"][aria-controls="workspace-file-canvas"]');
const paths = Array.from({ length: 6 }, (_unused, i) => `level-${i + 1}`);

await mkdir(output, { recursive: true });
await copyFile(REFRAIN_PDF_SCAN, join(root, paper));
for (let depth = 0; depth < paths.length; depth++) {
  const folder = join(root, ...paths.slice(0, depth + 1));
  await mkdir(folder, { recursive: true });
  await writeFile(join(folder, `visible-at-${depth + 1}.md`), `# File at ${depth + 1}\n`);
}

async function tap(page, cdp, locator) {
  await locator.waitFor();
  const r = await locator.boundingBox();
  assert(r);
  await cdp.send("Input.dispatchTouchEvent", {
    type: "touchStart",
    touchPoints: [{ x: r.x + r.width / 2, y: r.y + r.height / 2 }],
  });
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
}

async function focused(page, id) {
  try {
    await page.waitForFunction((anchor) => document.activeElement?.id === anchor, id, {
      timeout: 8000,
    });
  } catch (error) {
    evidence.focus = await page.evaluate((wanted) => {
      const layer = document.getElementById("workspace-navigation");
      const target = document.getElementById(wanted);
      return {
        wanted,
        active: document.activeElement?.id,
        name: document.activeElement?.tagName,
        layer: layer?.dataset.layer,
        hidden: layer?.getAttribute("aria-hidden"),
        inert: layer?.inert,
        visibility: target ? getComputedStyle(target).visibility : null,
      };
    }, id);
    throw error;
  }
}

async function rail(page, expected) {
  await page.waitForFunction(
    ({ ids, count }) =>
      ids.filter((id) => document.getElementById(id)?.getClientRects().length).length === count,
    { ids: [launch, ...actions], count: expected.length },
  );
  const positions = await page.evaluate(
    (ids) =>
      ids.flatMap((id) => {
        const node = document.getElementById(id);
        if (node === null || node.getClientRects().length === 0) return [];
        const rect = node.getBoundingClientRect();
        return [
          {
            id,
            x: rect.left,
            bottom: window.innerHeight - rect.bottom,
            width: rect.width,
            height: rect.height,
            label: node.getAttribute("aria-label"),
            glyph: node.querySelector("svg")?.innerHTML,
            shadow: getComputedStyle(node).boxShadow,
            visibleCaption: node.querySelector(".refrain-rail-caption")?.getClientRects().length
              ? node.textContent.trim()
              : undefined,
          },
        ];
      }),
    [launch, ...actions],
  );
  assert.deepEqual(
    positions.map((p) => p.id),
    expected,
  );
  for (const [i, pos] of positions.entries()) {
    assert(Math.abs(pos.x - 16) < 1, JSON.stringify(pos));
    assert(Math.abs(pos.bottom - (16 + i * 52)) < 1, JSON.stringify(pos));
    assert(pos.width >= 44 - 0.25 && pos.height >= 44 - 0.25 && pos.label, JSON.stringify(pos));
  }
  return positions;
}

async function docGeometry(page) {
  return page.locator(".pdf-scroll").evaluate((node) => {
    const r = node.getBoundingClientRect();
    return [r.x, r.y, r.width, r.height, node.scrollTop];
  });
}

async function desktopGrid(page, width) {
  await page.setViewportSize({ width, height: 830 });
  await rail(page, actions);
  const grid = await page.locator(".frame").evaluate((frame) => {
    const styles = getComputedStyle(frame);
    const margin = Number.parseFloat(styles.paddingLeft);
    const gutter = Number.parseFloat(styles.columnGap);
    const tracks = styles.gridTemplateColumns.split(" ").map(Number.parseFloat);
    const lines = [];
    let x = frame.getBoundingClientRect().left + margin - gutter / 2;
    for (const track of tracks) {
      lines.push(x);
      x += track + gutter;
    }
    return { margin, gutter, tracks, lines };
  });
  assert.equal(grid.margin, 32);
  assert.equal(grid.gutter, 24);
  assert.equal(grid.tracks.length, 12);
  const silver = width / (2 + Math.SQRT2);
  assert(Math.abs(grid.lines[3] - silver) < 1);
  assert(Math.abs(grid.lines[9] - (width - silver)) < 1);

  const full = await docGeometry(page);
  await key(page, navigation).click();
  await navButton(page, "shell_layout_tools").click();
  assert.deepEqual(await docGeometry(page), full);
  await navButton(page, "shell_split").click();
  const divider = splitter(page);
  await divider.waitFor();
  assert.equal(await divider.getAttribute("aria-orientation"), "vertical");
  await divider.press("Enter");
  const reset = await docGeometry(page);
  assert(reset[2] < full[2]);
  const hit = await divider.boundingBox();
  assert(hit);
  const y = hit.y + hit.height / 2;
  await page.mouse.move(hit.x + hit.width / 2, y);
  await page.mouse.down();
  await page.mouse.move(grid.lines[8], y, { steps: 4 });
  await page.mouse.up();
  await page.waitForFunction(
    () =>
      document
        .querySelector('[aria-controls="workspace-file-canvas"]')
        ?.getAttribute("aria-valuenow") === "7",
  );
  const dragged = await docGeometry(page);
  assert(dragged[2] > reset[2]);
  assert(dragged[2] < full[2]);
  await divider.press("ArrowLeft");
  assert.equal(await divider.getAttribute("aria-valuenow"), "6");
  assert.equal((await docGeometry(page))[2], reset[2]);
  await divider.press("ArrowRight");
  assert.equal(await divider.getAttribute("aria-valuenow"), "7");
  await key(page, navigation).click();
  await navButton(page, "shell_layout_tools").click();
  await navButton(page, "shell_split").click();
  await divider.waitFor({ state: "hidden" });
  assert.deepEqual(await docGeometry(page), full);
  return {
    viewport: width,
    grid,
    fullWidth: full[2],
    resetWidth: reset[2],
    draggedWidth: dragged[2],
  };
}

async function dragPhoneDivider(page, cdp) {
  const divider = splitter(page);
  await divider.waitFor();
  assert.equal(await divider.getAttribute("aria-orientation"), "horizontal");
  assert.equal(await divider.getAttribute("aria-valuenow"), "50");
  const before = await docGeometry(page);
  const r = await divider.boundingBox();
  assert(r && r.height >= 44);
  const x = r.x + r.width / 2;
  const y = r.y + r.height / 2;
  const hit = await divider.evaluate(
    (node, [x, y]) => {
      const target = document.elementFromPoint(x, y);
      return { received: node.contains(target), target: target?.className };
    },
    [x, y],
  );
  assert(hit.received, JSON.stringify(hit));
  const point = (at) => [{ x, y: at }];
  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: point(y) });
  for (const offset of [30, 65, 110]) {
    await cdp.send("Input.dispatchTouchEvent", {
      type: "touchMove",
      touchPoints: point(y + offset),
    });
  }
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  const saved = Number(await divider.getAttribute("aria-valuenow"));
  const after = await docGeometry(page);
  assert(saved > 55 && saved <= 75, JSON.stringify({ saved, before, after }));
  assert.equal(after[2], before[2]);
  assert(after[3] > before[3]);
  return { saved, before, after };
}

async function waitForPdf(page) {
  await page.waitForFunction(() => {
    const root = document.querySelector(".pdf-scroll");
    if (!root) return false;
    const view = root.getBoundingClientRect();
    const coverage = [...root.querySelectorAll(".pdf-original")].reduce((total, node) => {
      const range = node.getBoundingClientRect();
      return (
        total + Math.max(0, Math.min(view.bottom, range.bottom) - Math.max(view.top, range.top))
      );
    }, 0);
    return coverage > view.height / 3;
  });
}

async function frameTrace(page, cdp, action) {
  const record = () =>
    page.evaluate(() => {
      const scroller = document.querySelector(".pdf-scroll");
      const panel = document.getElementById("workspace-navigation");
      const frames = [];
      window.refrainRailFrames = frames;
      const observe = () => {
        const rect = scroller.getBoundingClientRect();
        const opacity = Number(getComputedStyle(panel).opacity);
        const time = performance.now();
        frames.push({
          time,
          opacity,
          fixed: [rect.x, rect.y, rect.width, rect.height, scroller.scrollTop],
        });
        if (opacity === 1 && panel.getAttribute("aria-hidden") === "false") return;
        if (time - frames[0].time < 5000) requestAnimationFrame(observe);
      };
      observe();
    });
  const frames = async () => {
    await page.waitForFunction(() => window.refrainRailFrames?.at(-1)?.opacity === 1);
    const captured = await page.evaluate(() => window.refrainRailFrames);
    assert.equal(captured[0].opacity, 0);
    for (const frame of captured) assert.deepEqual(frame.fixed, captured[0].fixed);
    return captured;
  };
  await page.evaluate(() => performance.clearMeasures("sprawling:shell_input_handler"));
  await record();
  const input = await traceInput(cdp, action, { scriptOrigin: origin });
  const normal = await frames();
  const layerToNextFrameMs = await page.evaluate(
    () => performance.getEntriesByName("sprawling:layer_drawn", "measure").at(-1)?.duration,
  );
  let slowed;
  const middle = (samples) => samples.some((frame) => frame.opacity > 0 && frame.opacity < 1);
  if (!middle(normal)) {
    await navButton(page, "shell_close_list").click();
    await page.waitForFunction(
      () => Number(getComputedStyle(document.getElementById("workspace-navigation")).opacity) === 0,
    );
    await cdp.send("Animation.enable");
    await cdp.send("Animation.setPlaybackRate", { playbackRate: 0.125 });
    try {
      await record();
      await action();
      slowed = await frames();
      assert(middle(slowed));
    } finally {
      await cdp.send("Animation.setPlaybackRate", { playbackRate: 1 });
      await cdp.send("Animation.disable");
    }
  }
  return { normal, slowed, input, layerToNextFrameMs, ownHandlers: await shellHandlers(page) };
}

async function shellHandlers(page) {
  const samples = await page.evaluate(() =>
    performance
      .getEntriesByName("sprawling:shell_input_handler", "measure")
      .map(({ duration, detail }) => ({ action: detail, durationMs: duration })),
  );
  assert(samples.some((sample) => sample.action === "list"));
  return samples;
}

async function repeatedNavigation(page, cdp, open) {
  const settled = () =>
    page.waitForFunction(() => {
      const panel = document.getElementById("workspace-navigation");
      return (
        panel?.getAttribute("aria-hidden") === "false" &&
        getComputedStyle(panel).translate === "none"
      );
    });
  const closed = () =>
    page.waitForFunction(() => {
      const panel = document.getElementById("workspace-navigation");
      return (
        panel?.getAttribute("aria-hidden") === "true" &&
        Number(getComputedStyle(panel).opacity) === 0
      );
    });
  await page.evaluate(() => performance.clearMeasures("sprawling:shell_input_handler"));
  const input = await traceInput(
    cdp,
    async () => {
      await navButton(page, "shell_close_list").click();
      await closed();
      for (let turn = 0; turn < 5; turn++) {
        await open();
        await settled();
        if (turn < 4) {
          await navButton(page, "shell_close_list").click();
          await closed();
        }
      }
    },
    { scriptOrigin: origin },
  );
  return { ...input, ownHandlers: await shellHandlers(page) };
}

async function reportFirstRail(page) {
  await page.addInitScript(
    (ids) => {
      performance.mark("sprawling:measure-shell-input");
      function first() {
        const keys = ids.filter((id) => document.getElementById(id)?.getClientRects().length);
        if (keys.length > 0) {
          window.refrainFirstRail = { keys, navigationToFirstRailMs: performance.now() };
        } else requestAnimationFrame(first);
      }
      requestAnimationFrame(first);
    },
    [launch, ...actions],
  );
}

async function firstRail(page) {
  await page.waitForFunction(() => window.refrainFirstRail !== undefined);
  const observed = await page.evaluate(() => window.refrainFirstRail);
  assert.deepEqual(observed.keys, [launch]);
  return observed;
}

async function useSkipLink(page) {
  const fragment = new URL(page.url()).hash;
  assert.equal(await page.locator("#main").count(), 1);
  const skip = page.getByRole("link", { name: english("skip_main"), exact: true });
  await skip.focus();
  await skip.press("Enter");
  await page.waitForFunction(() => document.activeElement?.closest("main#main") !== null);
  assert.equal(new URL(page.url()).hash, fragment);
}

try {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, locale: "en-US" });
  page.setDefaultTimeout(60_000);
  page.on("pageerror", (error) => {
    evidence.errors.push({ stage: evidence.stage, message: error.message, stack: error.stack });
  });
  await reportFirstRail(page);
  const landed = await enterWorkspace(page, entry.href);
  evidence.freshProfile = { ...(await firstRail(page)), ...landed };
  await page.setViewportSize({ width: 1280, height: 830 });
  await useSkipLink(page);
  evidence.desktopRail = await rail(page, actions);
  const mode = key(page, subject);
  await mode.hover();
  const modeBox = await mode.boundingBox();
  assert(modeBox);
  await page.mouse.move(modeBox.x + modeBox.width / 2, modeBox.y + modeBox.height / 2);
  await page.mouse.down();
  await page.waitForFunction(
    () => getComputedStyle(document.getElementById("workspace-subject-key")).scale === "0.96",
  );
  assert.equal(await mode.getAttribute("aria-pressed"), "false");
  await page.mouse.up();
  await page.locator('#main[data-subject="files"]').waitFor();
  await useSkipLink(page);
  assert.equal(await mode.getAttribute("aria-pressed"), "true");
  await page.waitForFunction(
    () => getComputedStyle(document.getElementById("workspace-subject-key")).scale === "none",
  );
  await page.mouse.down();
  await page.waitForFunction(() => {
    const control = document.getElementById("workspace-subject-key");
    return (
      control?.getAttribute("aria-pressed") === "true" &&
      getComputedStyle(control).scale === "0.96" &&
      getComputedStyle(control, "::after").borderTopStyle === "solid"
    );
  });
  await page.mouse.move(80, modeBox.y + modeBox.height / 2);
  await page.mouse.up();
  assert.equal(await mode.getAttribute("aria-pressed"), "true");
  await page.waitForFunction(
    () => getComputedStyle(document.getElementById("workspace-subject-key")).scale === "none",
  );
  assert.deepEqual(
    (await rail(page, actions)).map((pos) => [pos.x, pos.bottom]),
    evidence.desktopRail.map((pos) => [pos.x, pos.bottom]),
  );
  evidence.mousePress = {
    commitOnRelease: true,
    pressedAndSelected: true,
    releasedOutsideDoesNotCommit: true,
    stableAnchors: true,
  };

  evidence.stage = "desktop navigation and registration";
  await key(page, navigation).press("Enter");
  await focused(page, "workspace-navigation-heading");
  await navButton(page, "shell_library_tools").click();
  await focused(page, "workspace-navigation-heading");
  await drawer(page).getByLabel(english("shell_workspace_root"), { exact: true }).fill(root);
  await navButton(page, "shell_register_workspace").click();
  await drawer(page).getByRole("button", { name: paper, exact: true }).click();
  await waitForPdf(page);
  const desktopPDF = await docGeometry(page);
  await page.keyboard.press("Control+\\");
  assert.equal(await drawer(page).getAttribute("aria-hidden"), "true");
  assert.deepEqual(await docGeometry(page), desktopPDF);
  evidence.densityShortcutLeavesFileNavClosed = true;
  const cdp = await page.context().newCDPSession(page);
  evidence.desktopFrames = await frameTrace(page, cdp, () => key(page, navigation).click());
  evidence.desktopFrames.warm = await repeatedNavigation(page, cdp, () =>
    key(page, navigation).click(),
  );
  evidence.desktopAnimationIntervals = await animationIntervals(page);
  assert.deepEqual(await docGeometry(page), desktopPDF);
  await capture(page, join(output, "desktop-navigation.png"));
  await navButton(page, "shell_layout_tools").click();
  await focused(page, "workspace-navigation-heading");
  await page.keyboard.press("Escape");
  await page.waitForFunction(
    (name) => document.activeElement?.textContent.trim() === name,
    english("shell_layout_tools"),
  );
  await navButton(page, "shell_close_list").click();
  await focused(page, navigation);
  await page.waitForFunction(
    () => Number(getComputedStyle(document.getElementById("workspace-navigation")).opacity) === 0,
  );
  await capture(page, join(output, "desktop-four-actions.png"));
  evidence.stage = "desktop settings return";
  await key(page, settings).press("Enter");
  const settingsPanel = page
    .locator("dialog[open]")
    .filter({ has: page.getByRole("navigation") })
    .first();
  await settingsPanel.waitFor();
  await page.keyboard.press("Escape");
  await focused(page, settings);

  evidence.stage = "wide grid and divider movement";
  evidence.desktopGrids = [];
  for (const width of [1280, 1440, 1920]) {
    evidence.desktopGrids.push(await desktopGrid(page, width));
  }
  await page.setViewportSize({ width: 1280, height: 830 });

  evidence.stage = "desktop Blend holds the same target";
  await mode.click();
  await key(page, navigation).click();
  await navButton(page, "shell_layout_tools").click();
  const desktopDensity = navButton(page, "tier_panorama");
  await desktopDensity.click({ trial: true });
  const initialDensity = await desktopDensity.boundingBox();
  assert(initialDensity);
  await page.mouse.move(
    initialDensity.x + initialDensity.width / 2,
    initialDensity.y + initialDensity.height / 2,
  );
  await page.mouse.down();
  const activity = drawer(page).locator(".refrain-activity-inline");
  await activity.waitFor();
  const blendColors = () =>
    activity.evaluate((node) => {
      const style = getComputedStyle(node);
      return { backing: style.backgroundColor, text: style.color };
    });
  const beforeBlend = await blendColors();
  const heldDensity = await desktopDensity.boundingBox();
  assert(heldDensity);
  assert(
    Math.abs(
      initialDensity.y + initialDensity.height / 2 - heldDensity.y - heldDensity.height / 2,
    ) < 1,
  );
  await page.mouse.up();
  await activity.waitFor({ state: "hidden" });
  assert.equal(await desktopDensity.getAttribute("aria-pressed"), "false");
  await key(page, navigation).click();

  evidence.stage = "saved Blend appearance changes the activity backing";
  await key(page, settings).press("Enter");
  await settingsPanel.waitFor();
  const preferences = settingsPanel.getByRole("button", { name: /^preferences$/i }).first();
  if ((await preferences.getAttribute("aria-expanded")) !== "true") await preferences.click();
  await settingsPanel
    .getByRole("button", { name: /^appearance$/i })
    .first()
    .click();
  const blend = settingsPanel.getByRole("slider", {
    name: english("appearance_blend"),
    exact: true,
  });
  const previousPercent = Number(await blend.inputValue());
  await blend.press(previousPercent >= 90 ? "ArrowLeft" : "ArrowRight");
  assert.notEqual(Number(await blend.inputValue()), previousPercent);
  await page.keyboard.press("Escape");
  await focused(page, settings);
  await key(page, navigation).click();
  await navButton(page, "shell_layout_tools").click();
  await desktopDensity.click({ trial: true });
  const newTarget = await desktopDensity.boundingBox();
  assert(newTarget);
  await page.mouse.move(newTarget.x + newTarget.width / 2, newTarget.y + newTarget.height / 2);
  await page.mouse.down();
  await activity.waitFor();
  const afterBlend = await blendColors();
  assert.notEqual(afterBlend.backing, beforeBlend.backing);
  assert.equal(afterBlend.text, beforeBlend.text);
  evidence.blendAppearance = { before: beforeBlend, after: afterBlend };
  await page.mouse.up();
  await activity.waitFor({ state: "hidden" });
  await key(page, navigation).click();
  await mode.click();

  await page.setViewportSize({ width: 390, height: 844 });
  evidence.stage = "compact rail disclosure";
  await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: true });
  const compact = await rail(page, [launch]);
  await tap(page, cdp, key(page, launch));
  const mobile = await rail(page, actions);
  assert(mobile.every((p) => !p.visibleCaption && p.glyph && p.shadow === "none"));
  assert.equal(new Set(mobile.map((p) => p.glyph)).size, actions.length);
  const menu = page.getByRole("group", { name: english("shell_edge_actions"), exact: true });
  assert(await menu.evaluate((node) => getComputedStyle(node).outlineStyle === "solid"));
  await focused(page, settings);
  await capture(page, join(output, "phone-actions.png"));
  for (const next of [navigation, mailbox, subject]) {
    await page.keyboard.press("Tab");
    await focused(page, next);
  }
  await page.keyboard.press("Escape");
  await focused(page, launch);
  assert.deepEqual(
    (await rail(page, [launch])).map(({ id, x, bottom }) => [id, x, bottom]),
    compact.map(({ id, x, bottom }) => [id, x, bottom]),
  );
  await waitForPdf(page);
  const phonePDF = await docGeometry(page);
  const openNavigation = async () => {
    await tap(page, cdp, key(page, launch));
    await tap(page, cdp, key(page, navigation));
  };
  evidence.phoneFrames = await frameTrace(page, cdp, openNavigation);
  evidence.phoneFrames.warm = await repeatedNavigation(page, cdp, openNavigation);
  evidence.phoneAnimationIntervals = await animationIntervals(page);
  evidence.stage = "direct children at depth six";
  await rail(page, [launch]);
  evidence.drawerDuringLastFrame = await drawer(page).boundingBox();
  await page.waitForFunction(() => {
    const drawer = document.getElementById("workspace-navigation");
    return drawer !== null && getComputedStyle(drawer).translate === "none";
  });
  const panel = await drawer(page).boundingBox();
  evidence.drawerSettledRect = panel;
  assert(panel && panel.height >= 600 && Math.abs(panel.x - 16) < 0.2, JSON.stringify(panel));
  for (const [at, next] of paths.entries()) {
    await tap(page, cdp, drawer(page).getByRole("button", { name: next, exact: true }));
    await drawer(page)
      .locator(`[data-folder-path="${paths.slice(0, at + 1).join("/")}"]`)
      .waitFor();
    await drawer(page)
      .getByRole("button", { name: `visible-at-${at + 1}.md`, exact: true })
      .waitFor();
    const children = await drawer(page)
      .locator("[data-file-kind]")
      .evaluateAll((rows) => rows.map((row) => row.dataset.filePath));
    assert(children.every((path) => path.split("/").length === at + 2));
    assert.equal(children.length, at === paths.length - 1 ? 1 : 2);
  }
  const deep = paths.join("/");
  await capture(page, join(output, "phone-depth-six.png"));
  await tap(page, cdp, navButton(page, "shell_parent_folder"));
  await page.waitForFunction((path) => document.activeElement?.dataset.filePath === path, deep);
  assert.deepEqual(await docGeometry(page), phonePDF);
  await tap(page, cdp, navButton(page, "shell_close_list"));
  await focused(page, launch);
  assert.deepEqual(await docGeometry(page), phonePDF);

  evidence.stage = "touch cancel and Blend hold";
  await tap(page, cdp, key(page, launch));
  const phoneMode = await mode.boundingBox();
  assert(phoneMode);
  await cdp.send("Input.dispatchTouchEvent", {
    type: "touchStart",
    touchPoints: [{ x: phoneMode.x + phoneMode.width / 2, y: phoneMode.y + phoneMode.height / 2 }],
  });
  assert.equal(await mode.getAttribute("aria-pressed"), "true");
  await cdp.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] });
  await rail(page, actions);
  assert.equal(await mode.getAttribute("aria-pressed"), "true");
  await tap(page, cdp, mode);
  await page.locator('#main[data-subject="session"]').waitFor();
  await focused(page, launch);
  await rail(page, [launch]);
  await openNavigation();
  await tap(page, cdp, navButton(page, "shell_layout_tools"));
  const density = navButton(page, "tier_panorama");
  await density.waitFor();
  const phoneDensity = await density.boundingBox();
  assert(phoneDensity);
  await cdp.send("Input.dispatchTouchEvent", {
    type: "touchStart",
    touchPoints: [
      { x: phoneDensity.x + phoneDensity.width / 2, y: phoneDensity.y + phoneDensity.height / 2 },
    ],
  });
  await drawer(page).locator("[data-control-holding]").waitFor();
  assert.equal(await density.getAttribute("aria-pressed"), "false");
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await drawer(page).locator("[data-control-holding]").waitFor({ state: "hidden" });
  assert.equal(await density.getAttribute("aria-pressed"), "false");
  await tap(page, cdp, navButton(page, "shell_go_back"));
  await tap(page, cdp, navButton(page, "shell_close_list"));
  await tap(page, cdp, key(page, launch));
  await tap(page, cdp, mode);
  await page.locator('#main[data-subject="files"]').waitFor();
  assert.deepEqual(await docGeometry(page), phonePDF);

  evidence.stage = "outside collapse";
  await tap(page, cdp, key(page, launch));
  await cdp.send("Input.dispatchTouchEvent", {
    type: "touchStart",
    touchPoints: [{ x: 365, y: 210 }],
  });
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await rail(page, [launch]);
  await capture(page, join(output, "phone-compact.png"));
  evidence.stage = "phone split drag";
  await openNavigation();
  await tap(page, cdp, navButton(page, "shell_layout_tools"));
  await tap(page, cdp, navButton(page, "shell_split"));
  evidence.stacked = await dragPhoneDivider(page, cdp);
  await cdp.detach();
  evidence.stage = "phone cold initial frame";
  await page.goto(origin);
  evidence.reopenedProfile = await firstRail(page);
  const again = splitter(page);
  await again.waitFor();
  assert.equal(await again.getAttribute("aria-valuenow"), String(evidence.stacked.saved));
  await page.setViewportSize({ width: 1440, height: 830 });
  await page.waitForFunction(
    () =>
      document
        .querySelector('[aria-controls="workspace-file-canvas"]')
        ?.getAttribute("aria-orientation") === "vertical",
  );
  assert.equal(await again.getAttribute("aria-valuenow"), "7");
  evidence.stacked.preservedAcrossReload = true;
  evidence.stacked.independentDesktopLine = 9;
  await useSkipLink(page);
  assert.deepEqual(evidence.errors, []);
  evidence.phone = {
    compact,
    expanded: mobile,
    drawer: panel,
    deepestFolder: deep,
    focusReturned: true,
    nativeTouch: true,
    canceledPressDidNotSwitch: true,
    longPressDidNotSwitchDensity: true,
  };
  delete evidence.stage;
  evidence.result = "success";
} catch (error) {
  evidence.result = "failure";
  evidence.failure = error.message;
  throw error;
} finally {
  await writeFile(join(output, "production-shell.json"), JSON.stringify(evidence, null, 2));
  await browser.close();
  await rm(root, { recursive: true, force: true });
}
