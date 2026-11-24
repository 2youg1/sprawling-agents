// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import words from "../../RefRain/src/lang.json";
import { DEFAULT_VIEW, toFragment } from "../../RefRain/src/core/route.ts";
import { pressAndCancel } from "./control-frames.mjs";
import { animationIntervals, traceInput } from "./workspace-input.mjs";
import { workspaceState } from "./workspace-state.mjs";

const [port, output] = process.argv.slice(2);
if (!/^\d+$/.test(port ?? "") || !output || !process.env.PLAYWRIGHT_MODULE) {
  throw new Error(
    "usage: PLAYWRIGHT_MODULE=<installed module> bun acceptance-pdf.mjs <port> <output>",
  );
}

const PAPER = "sentinel.pdf";
const SCAN = "scan.pdf";
const SCAN_PAGE = Number(process.env.REFRAIN_PDF_SCAN_PAGE ?? 1);
const ORIGIN = `http://127.0.0.1:${port}`;
const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE).href);

async function recordOpening(page) {
  await page.evaluate(() => {
    const viewport = document.querySelector(".pdf-scroll");
    const panel = document.getElementById("workspace-navigation");
    window.shellFrameLog = [];
    const record = () => {
      const { x, y, width, height } = viewport.getBoundingClientRect();
      const style = getComputedStyle(panel);
      const at = performance.now();
      window.shellFrameLog.push({
        at,
        geometry: [x, y, width, height, viewport.scrollTop],
        opacity: Number(style.opacity),
      });
      if (panel.getAttribute("aria-hidden") === "false" && Number(style.opacity) === 1) return;
      if (at - window.shellFrameLog[0].at < 5000) requestAnimationFrame(record);
    };
    record();
  });
}

async function openingFrames(page) {
  await page.waitForFunction(() => window.shellFrameLog.at(-1)?.opacity === 1);
  const frames = await page.evaluate(() => window.shellFrameLog);
  assert(frames.length >= 2);
  assert.equal(frames[0].opacity, 0, "the reading surface starts with navigation closed");
  for (const frame of frames) assert.deepEqual(frame.geometry, frames[0].geometry);
  return frames;
}

async function backToNavigationKey(page, compact) {
  await page.keyboard.press("Escape");
  await page.waitForFunction(
    (id) => document.activeElement?.id === id,
    compact ? "workspace-actions-key" : "workspace-navigation-key",
  );
  await page.waitForFunction(() => {
    const panel = document.getElementById("workspace-navigation");
    return (
      panel.getAttribute("aria-hidden") === "true" && Number(getComputedStyle(panel).opacity) === 0
    );
  });
}

const hasIntermediate = (frames) => frames.some((frame) => frame.opacity > 0 && frame.opacity < 1);

async function shellCapture(page, output) {
  await page.bringToFront();
  const sizes = [
    { width: 1280, height: 830, touch: false },
    { width: 390, height: 844, touch: true },
  ];
  const reports = [];
  const cdp = await page.context().newCDPSession(page);
  const initial = page.viewportSize();
  assert.ok(initial, "the browser has a sized viewport");
  for (const v of sizes) {
    await page.setViewportSize({ width: v.width, height: v.height });
    await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: v.touch });
    const scroll = page.locator(".pdf-scroll");
    await scroll.evaluate((e) => (e.scrollTop = 0));
    await page.waitForFunction(() => {
      const viewport = document.querySelector(".pdf-scroll");
      if (!viewport) return false;
      const bounds = viewport.getBoundingClientRect();
      const medium = [...viewport.querySelectorAll(".pdf-original")].reduce((coverage, pixels) => {
        const rect = pixels.getBoundingClientRect();
        return (
          coverage +
          Math.max(0, Math.min(rect.bottom, bounds.bottom) - Math.max(rect.top, bounds.top))
        );
      }, 0);
      return medium > bounds.height / 2;
    });
    const nav = navigation(page);
    const actuate = async () => {
      await showRail(page, nav);
      if (v.touch) {
        const b = await nav.boundingBox();
        assert.ok(b, "the navigation key lies in the touchscreen viewport");
        await cdp.send("Input.dispatchTouchEvent", {
          type: "touchStart",
          touchPoints: [{ x: b.x + b.width / 2, y: b.y + b.height / 2 }],
        });
        await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
      } else await nav.click();
    };
    await recordOpening(page);
    const input = await traceInput(cdp, actuate);
    const frames = await openingFrames(page);
    await page.screenshot({ path: join(output, "shell-" + (v.touch ? "touch" : "wide") + ".png") });
    await backToNavigationKey(page, v.touch);
    let controlledFrames;
    if (!hasIntermediate(frames)) {
      await cdp.send("Animation.enable");
      await cdp.send("Animation.setPlaybackRate", { playbackRate: 0.125 });
      try {
        await recordOpening(page);
        await actuate();
        controlledFrames = await openingFrames(page);
        assert(
          hasIntermediate(controlledFrames),
          "the slowed transition has an intermediate frame",
        );
        assert.deepEqual(controlledFrames[0].geometry, frames[0].geometry);
      } finally {
        await cdp.send("Animation.setPlaybackRate", { playbackRate: 1 });
        await cdp.send("Animation.disable");
      }
      await backToNavigationKey(page, v.touch);
    }
    reports.push({
      viewport: v,
      frames,
      controlledFrames,
      input,
      animation: await animationIntervals(page),
    });
  }
  await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: false });
  await cdp.detach();
  await page.setViewportSize(initial);
  await writeFile(join(output, "shell-frames.json"), JSON.stringify(reports, null, 2));
}
async function enter(page) {
  const link = process.env.REFRAIN_PDF_OPEN_URL_FILE;
  const code = process.env.REFRAIN_PDF_PAIR_CODE_FILE;
  if (link !== undefined) {
    const opening = new URL((await readFile(link, "utf8")).trim());
    assert.equal(
      opening.origin,
      ORIGIN,
      "the opening URL belongs to the test city's loopback port",
    );
    await page.goto(opening.href);
  } else {
    await page.goto(`${ORIGIN}/${toFragment({ kind: "welcome" })}`);
  }
  const welcome = button(page, "guide_put_off_rest");
  const pairing = page.getByLabel(english("pair_code"), { exact: true });
  if (link === undefined) {
    assert.ok(code, "pass the test city's terminal pairing code using REFRAIN_PDF_PAIR_CODE_FILE");
    await pairing.waitFor();
    await pairing.fill((await readFile(code, "utf8")).trim());
    await button(page, "pair_action").click();
  }
  await welcome.click();
  await page.waitForFunction((hash) => location.hash === hash, toFragment(DEFAULT_VIEW));
}

function fixture() {
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Symbol >>",
  ];
  const pages = [];
  for (let number = 1; number <= 8; number++) {
    const page = objects.length + 1;
    pages.push(`${page} 0 R`);
    const columns = Array.from({ length: 29 }, (_unused, index) => {
      const row = 720 - index * 22;
      return [
        `BT /F1 10 Tf 42 ${row} Td (Left column page ${number} line ${index}) Tj ET`,
        `BT /F1 10 Tf 335 ${row} Td (Right column page ${number} line ${index}) Tj ET`,
      ].join("\n");
    }).join("\n");
    const stream = [
      columns,
      "0 0 1 rg 70 575 100 25 re f",
      "1 0 0 rg 173 575 110 25 re f",
      "1 1 1 rg BT /F2 13 Tf 76 583 Td (a) Tj ET",
      "BT /F1 10 Tf 87 583 Td ( = 1) Tj ET",
      "BT /F1 10 Tf 179 583 Td (OUTSIDE SENTINEL) Tj ET",
    ].join("\n");
    objects.push(
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents ${page + 1} 0 R >>`,
      `<< /Length ${stream.length} >>\nstream\n${stream}\nendstream`,
    );
  }
  objects[1] = `<< /Type /Pages /Kids [${pages.join(" ")}] /Count ${pages.length} >>`;
  let source = "%PDF-1.4\n";
  const xref = ["0000000000 65535 f \n"];
  objects.forEach((value, index) => {
    xref.push(`${String(source.length).padStart(10, "0")} 00000 n \n`);
    source += `${index + 1} 0 obj\n${value}\nendobj\n`;
  });
  const end = source.length;
  source += `xref\n0 ${xref.length}\n${xref.join("")}trailer\n<< /Size ${xref.length} /Root 1 0 R >>\nstartxref\n${end}\n%%EOF\n`;
  return source;
}

const english = (key) => words[key].en;
const button = (page, key) => page.getByRole("button", { name: english(key), exact: true });
const navigation = (page) => page.locator("#workspace-navigation-key");
const subject = (page) => page.locator("#workspace-subject-key");
const surface = (page) => page.locator(".pdf-scroll");
const sheet = (page, number) => page.locator(`.pdf-sheet[data-page="${number}"]`);

async function sampledPaper(page, capture) {
  const paper = await sheet(page, 1).boundingBox();
  assert.ok(paper);
  return page.evaluate(
    async ({ paper, bytes }) => {
      const image = await createImageBitmap(
        new Blob([new Uint8Array(bytes)], { type: "image/png" }),
      );
      const canvas = new OffscreenCanvas(image.width, image.height);
      const ctx = canvas.getContext("2d");
      ctx.drawImage(image, 0, 0);
      const scale = image.width / innerWidth;
      const pixel = (x, y) =>
        [
          ...ctx.getImageData(
            Math.round((paper.x + (x / 612) * paper.width) * scale),
            Math.round((paper.y + (y / 792) * paper.height) * scale),
            1,
            1,
          ).data,
        ].slice(0, 3);
      image.close();
      return { margin: pixel(12, 20), blue: pixel(155, 201), red: pixel(270, 215) };
    },
    { paper, bytes: [...capture] },
  );
}

async function showRail(page, key) {
  if (!(await key.isVisible())) await page.locator("#workspace-actions-key").click();
  await key.waitFor();
}

async function openAttachments(page) {
  const drawer = page
    .locator("#workspace-conversation summary")
    .filter({ hasText: english("talk_attachments") });
  await drawer.waitFor();
  if (!(await drawer.evaluate((node) => node.parentElement.open))) await drawer.click();
}

async function attachmentState(page) {
  return page
    .locator("#workspace-conversation summary")
    .filter({ hasText: english("talk_attachments") })
    .evaluate((summary) => ({
      open: summary.parentElement.open,
      images: summary.parentElement.querySelectorAll("img").length,
    }));
}

async function position(page) {
  return surface(page).evaluate((scroller) => {
    const row =
      [...scroller.querySelectorAll(".pdf-sheet")]
        .reverse()
        .find((candidate) => candidate.offsetTop <= scroller.scrollTop) ??
      scroller.querySelector(".pdf-sheet");
    return {
      page: Number(row.dataset.page),
      offset: Math.max(0, (scroller.scrollTop - row.offsetTop) / row.offsetHeight),
      width: row.getBoundingClientRect().width,
      screenTop: row.getBoundingClientRect().top,
    };
  });
}

const objectTools = (page) => page.locator("[data-context-tools]");
const pdfTool = (page, key) => objectTools(page).getByText(english(key), { exact: true });
const toolRow = (page, id) => objectTools(page).locator(`[data-context-item="${id}"]`);

async function keyboardTools(page) {
  await page.locator(".pdf-scroll").focus();
  await page.keyboard.press("Shift+F10");
  await objectTools(page).getByRole("menu").waitFor();
}

const selectedText = (page) => page.evaluate(() => window.getSelection()?.toString());

async function selectPaperText(page) {
  const line = sheet(page, 1)
    .locator(".pdf-text-layer span")
    .filter({ hasText: /^Right column page 1 line 0$/ })
    .first();
  const bounds = await line.boundingBox();
  assert.ok(bounds, "the PDF exposes the paper's original text spans");
  const y = bounds.y + bounds.height / 2;
  await page.mouse.move(bounds.x + 1, y);
  await page.mouse.down();
  await page.mouse.move(bounds.x + bounds.width - 1, y, { steps: 8 });
  await page.mouse.up();
  const text = await selectedText(page);
  assert.equal(text, "Right column page 1 line 0");
  return { text, point: { x: bounds.x + bounds.width / 2, y } };
}

async function keyboardSelection(page) {
  await surface(page).focus();
  const { text } = await selectPaperText(page);
  await page.keyboard.press("Shift+F10");
  await objectTools(page).getByRole("menu").waitFor();
  await pdfTool(page, "pdf_attach").waitFor();
  await page.keyboard.press("Escape");
  await objectTools(page).waitFor({ state: "hidden" });
  assert.equal(await selectedText(page), text, "closing the menu preserves the native range");
  assert.ok(await surface(page).evaluate((node) => node === document.activeElement));
  return text;
}

async function keyboardRegion(page) {
  await keyboardTools(page);
  await pdfTool(page, "pdf_select_region").click();
  const reader = surface(page);
  assert.ok(await reader.evaluate((node) => node === document.activeElement));
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Shift+ArrowDown");
  const region = sheet(page, 1).locator(".pdf-region");
  const bounds = await region.boundingBox();
  assert.ok(bounds && bounds.width > 0 && bounds.height > 0);
  await page.keyboard.press("Enter");
  await pdfTool(page, "pdf_attach").waitFor();
  await page.keyboard.press("Escape");
  await objectTools(page).waitFor({ state: "hidden" });
  assert.deepEqual(await region.boundingBox(), bounds);
  await page.keyboard.press("Escape");
  return bounds;
}

async function originalControls(page) {
  const results = {};
  for (const mode of ["normal", "reduced", "forced"]) {
    await page.emulateMedia({
      reducedMotion: mode === "reduced" ? "reduce" : "no-preference",
      forcedColors: mode === "forced" ? "active" : "none",
    });
    await keyboardTools(page);
    const original = objectTools(page).getByRole("menuitem", { name: english("pdf_original") });
    const show = () => page.locator(".pdf-view").getAttribute("data-original");
    const cycle = await pressAndCancel(page, original, {
      canHold: true,
      held: async () => {
        assert.equal(await show(), "true");
      },
    });
    assert.equal(await show(), "false");
    assert.ok(cycle.rest.glyph);
    if (mode === "reduced") assert.equal(cycle.holding.duration, "0s");
    if (mode === "forced") assert.match(cycle.holding.outline, /dashed/);
    await original.focus();
    await page.keyboard.down("Space");
    assert.equal(await show(), "true");
    await page.keyboard.up("Space");
    assert.equal(await show(), "false");
    await page.keyboard.press("Escape");
    await objectTools(page).waitFor({ state: "hidden" });
    assert.ok(await surface(page).evaluate((node) => node === document.activeElement));
    results[mode] = cycle;
  }
  await page.emulateMedia({ forcedColors: "none" });
  return results;
}

async function readingControls(page, names) {
  const results = {};
  for (const [mode, media] of [
    ["normal", {}],
    ["reduced", { reducedMotion: "reduce" }],
    ["forced", { forcedColors: "active" }],
  ]) {
    await page.emulateMedia({ reducedMotion: "no-preference", forcedColors: "none", ...media });
    await keyboardTools(page);
    results[mode] = {};
    for (const id of names) {
      const cycle = await pressAndCancel(page, toolRow(page, id));
      assert.ok(cycle.rest.glyph, `${id} has a visible icon`);
      if (mode === "reduced") assert.equal(cycle.holding.duration, "0s");
      results[mode][id] = cycle;
    }
    assert.equal(
      new Set(Object.values(results[mode]).map((cycle) => cycle.rest.glyph)).size,
      names.length,
    );
    await page.keyboard.press("Escape");
    await objectTools(page).waitFor({ state: "hidden" });
    assert.ok(await surface(page).evaluate((node) => node === document.activeElement));
  }
  await page.emulateMedia({ reducedMotion: "no-preference", forcedColors: "none" });
  return results;
}

async function touchTools(page) {
  const initial = page.viewportSize();
  assert.ok(initial);
  const viewport = { width: 390, height: 844 };
  await page.setViewportSize(viewport);
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: true });
  try {
    const reader = page.locator(".pdf-scroll");
    await page.waitForFunction(() => {
      const canvas = document.querySelector('.pdf-sheet[data-page="1"] .pdf-original');
      return canvas instanceof HTMLCanvasElement && canvas.width < 800;
    });
    const bounds = await reader.boundingBox();
    const paper = await sheet(page, 1).boundingBox();
    assert.ok(bounds && paper);
    const point = { x: paper.x + 6, y: bounds.y + bounds.height / 3 };
    assert.ok(
      await page.evaluate(({ x, y }) => {
        const target = document.elementFromPoint(x, y);
        return (
          target?.closest(".pdf-sheet") != null && target.closest(".pdf-text-layer span") === null
        );
      }, point),
      "the press targets a blank part of the page, leaving native word selection alone",
    );
    const before = await position(page);
    await cdp.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [point],
    });
    await objectTools(page).getByRole("menu").waitFor();
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    const menuBounds = await objectTools(page).boundingBox();
    assert.ok(
      menuBounds && menuBounds.x <= 40 && menuBounds.y + menuBounds.height >= viewport.height * 0.7,
    );
    assert.deepEqual(await position(page), before);
    await pdfTool(page, "pdf_details").click();
    const panel = objectTools(page).getByRole("dialog");
    await panel.waitFor();
    const back = panel.getByRole("button", { name: /back/i });
    const backBounds = await back.boundingBox();
    assert.ok(backBounds && backBounds.x <= 56);
    await cdp.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [
        { x: backBounds.x + backBounds.width / 2, y: backBounds.y + backBounds.height / 2 },
      ],
    });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await objectTools(page).getByRole("menu").waitFor();
    await page.keyboard.press("Escape");
    await objectTools(page).waitFor({ state: "hidden" });
    assert.ok(await reader.evaluate((node) => node === document.activeElement));
    assert.deepEqual(await position(page), before);

    await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    const selection = await selectPaperText(page);
    await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: true });
    await cdp.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [selection.point],
    });
    await pdfTool(page, "pdf_attach").waitFor();
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await page.keyboard.press("Escape");
    await objectTools(page).waitFor({ state: "hidden" });
    assert.equal(await selectedText(page), selection.text);

    await keyboardTools(page);
    await pdfTool(page, "pdf_select_region").click();
    const normalizedPoint = (x, y) => ({
      x: paper.x + (paper.width * x) / 612,
      y: paper.y + (paper.height * y) / 792,
    });
    await cdp.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [normalizedPoint(70, 192)],
    });
    await cdp.send("Input.dispatchTouchEvent", {
      type: "touchMove",
      touchPoints: [normalizedPoint(170, 218)],
    });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await pdfTool(page, "pdf_attach").waitFor();
    const region = sheet(page, 1).locator(".pdf-region");
    const outline = await region.boundingBox();
    assert.ok(outline && outline.width > 30 && outline.height > 7);
    await page.keyboard.press("Escape");
    await objectTools(page).waitFor({ state: "hidden" });
    assert.deepEqual(await region.boundingBox(), outline);
    await page.keyboard.press("Escape");
    await reader.evaluate((node) => node.ownerDocument.getSelection()?.removeAllRanges());
    assert.deepEqual(await position(page), before);
    return { viewport, surface: menuBounds, before, text: selection.text, outline };
  } finally {
    await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    await cdp.detach();
    await page.setViewportSize(initial);
  }
}

async function openDocument(page, name, root) {
  await showRail(page, navigation(page));
  if ((await navigation(page).getAttribute("aria-expanded")) !== "true")
    await navigation(page).click();
  if (root !== undefined) {
    await button(page, "shell_library_tools").click();
    const rootPath = page.getByLabel(english("shell_workspace_root"), { exact: true });
    const registration = page
      .locator("#workspace-navigation summary")
      .filter({ hasText: english("shell_register_workspace") });
    await rootPath.or(registration).first().waitFor();
    if (!(await rootPath.isVisible())) {
      await registration.click();
    }
    await rootPath.fill(root);
    await button(page, "shell_register_workspace").click();
  }
  const entry = page.getByRole("button", { name, exact: true }).first();
  await entry.evaluate((node) => {
    node.addEventListener(
      "click",
      () => {
        window.refrainPdfOpenedAt = performance.now();
      },
      {
        capture: true,
        once: true,
      },
    );
  });
  await entry.click();
  await sheet(page, 1).locator(".pdf-original").waitFor();
}

async function crop(page) {
  const current = sheet(page, 1);
  await page.waitForFunction(() => {
    const sheet = document.querySelector('.pdf-sheet[data-page="1"]');
    return sheet?.querySelector(".pdf-original")?.width > sheet?.clientWidth;
  });
  await keyboardTools(page);
  await pdfTool(page, "pdf_select_region").click();
  const box = await current.boundingBox();
  assert.ok(box, "the first PDF page is displayed");
  const left = box.x + (box.width * 70) / 612;
  const top = box.y + (box.height * 192) / 792;
  const right = box.x + (box.width * 170) / 612;
  const bottom = box.y + (box.height * 218) / 792;
  await page.mouse.move(left, top);
  await page.mouse.down();
  await page.mouse.move(right, bottom, { steps: 8 });
  await page.mouse.up();
  const selected = current.locator(".pdf-region");
  const outline = await selected.boundingBox();
  assert.ok(outline);
  await page.mouse.click(right, bottom, { button: "right" });
  await pdfTool(page, "pdf_attach").waitFor();
  await page.keyboard.press("Escape");
  await objectTools(page).waitFor({ state: "hidden" });
  assert.deepEqual(await selected.boundingBox(), outline, "Back preserves the selected region");
  await page.mouse.click(right, bottom, { button: "right" });
  await pdfTool(page, "pdf_attach").click();
  const target = page.getByRole("dialog", { name: english("shell_choose_target") });
  await target.getByRole("button", { name: english("shell_use_session"), exact: true }).click();
  await openAttachments(page);
  const preview = page.getByRole("img", { name: english("talk_attachment_region") });
  await preview.waitFor();
  return preview.evaluate(async (image) => {
    await image.decode();
    const { naturalWidth: width, naturalHeight: height } = image;
    const source = new OffscreenCanvas(width, height);
    const context = source.getContext("2d");
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, width, height).data;
    let blue = 0;
    let red = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      const [r, g, b] = pixels.subarray(index, index + 3);
      if (r < 30 && g < 30 && b > 220) blue++;
      if (r > 220 && g < 30 && b < 30) red++;
    }
    return { width, height, blue, red };
  });
}

const root = await mkdtemp(join(tmpdir(), "refrain-pdf-"));
await mkdir(output, { recursive: true });
await writeFile(join(root, PAPER), fixture(), "ascii");
const scan =
  process.env.REFRAIN_PDF_SCAN === undefined ? null : await readFile(process.env.REFRAIN_PDF_SCAN);
if (scan !== null) await writeFile(join(root, SCAN), scan);
const executablePath = process.env.REFRAIN_PDF_BROWSER;
const browser = await chromium.launch({ headless: true, executablePath });
const evidence = { browser: browser.version(), errors: [], foreignRequests: [], failedAssets: [] };
try {
  const context = await browser.newContext({
    viewport: { width: 1280, height: 960 },
    deviceScaleFactor: 1.25,
    locale: "en-US",
  });
  context.setDefaultTimeout(60_000);
  context.on("response", (response) => {
    const asset = new URL(response.url()).pathname;
    if (asset.startsWith("/assets/") && response.status() >= 400) {
      evidence.failedAssets.push({ asset, status: response.status() });
    }
  });
  context.on("request", (request) => {
    const url = new URL(request.url());
    if (/^https?:$/.test(url.protocol) && url.origin !== ORIGIN) {
      evidence.foreignRequests.push(url.origin);
    }
  });
  await context.addInitScript(() => {
    const frames = [];
    window.refrainPdfPaints = frames;
    new MutationObserver((changes) => {
      for (const change of changes) {
        for (const node of change.addedNodes) {
          if (node instanceof HTMLCanvasElement && node.closest('[data-page="1"]')) {
            frames.push({ time: performance.now(), width: node.width, kind: node.className });
          }
        }
      }
    }).observe(document, { subtree: true, childList: true });
  });
  const page = await context.newPage();
  page.on("pageerror", (error) => evidence.errors.push(error.message));
  await enter(page);
  await context.route("**/*", (route) => {
    const url = new URL(route.request().url());
    return /^https?:$/.test(url.protocol) && url.origin !== ORIGIN
      ? route.abort()
      : route.continue();
  });
  await showRail(page, subject(page));
  await subject(page).click();
  await openDocument(page, PAPER, root);
  const dark = await page.screenshot({ path: join(output, "paper-page-width.png") });
  evidence.paper = await sampledPaper(page, dark);
  const [r, g, b] = evidence.paper.margin;
  assert.ok(r < g && g < b && b < 42, "the default paper is cool and dark");
  assert.ok(evidence.paper.blue[2] >= 247 && evidence.paper.blue[0] <= 8);
  assert.ok(evidence.paper.red[0] >= 247 && evidence.paper.red[2] <= 8);
  await keyboardTools(page);
  await toolRow(page, "original").focus();
  await page.keyboard.down("Space");
  const raw = await sampledPaper(page, await page.screenshot());
  assert.ok(
    raw.margin.every((light) => light >= 245),
    "original hold exposes the source paper",
  );
  await page.keyboard.up("Space");
  await page.keyboard.press("Escape");
  await objectTools(page).waitFor({ state: "hidden" });
  evidence.originalPaper = raw.margin;
  evidence.stage = "original image hold with pointer and keyboard";
  evidence.original = await originalControls(page);
  evidence.stage = "PDF tool visual states";
  evidence.toolControls = await readingControls(page, [
    "region",
    "magnify",
    "brightness",
    "details",
  ]);

  evidence.stage = "document details";
  const before = await position(page);
  assert.equal(await page.getByText(english("pdf_zoom"), { exact: true }).isVisible(), false);
  const metadata = page.locator(".pdf-metadata");
  assert.equal(await metadata.isVisible(), false);
  await keyboardTools(page);
  const details = pdfTool(page, "pdf_details");
  await details.click();
  await metadata.waitFor();
  assert.deepEqual(await position(page), before, "document details leave the page geometry fixed");
  await page.waitForFunction(() => document.activeElement?.hasAttribute("data-context-back"));
  evidence.detailsBack = await pressAndCancel(
    page,
    objectTools(page).locator("[data-context-back]"),
  );
  assert.ok(evidence.detailsBack.rest.glyph);
  await page.keyboard.press("Escape");
  await metadata.waitFor({ state: "hidden" });
  evidence.stage = "focus after leaving details";
  await page.waitForFunction(
    (node) => node?.closest("button") === document.activeElement,
    await details.elementHandle(),
  );
  evidence.stage = "brightness preview and raster reuse";
  await pdfTool(page, "wallpaper_brightness").click();
  const brightness = objectTools(page).getByRole("slider", {
    name: english("wallpaper_brightness"),
    exact: true,
  });
  await brightness.waitFor();
  const raster = await sheet(page, 1).locator(".pdf-original").elementHandle();
  assert.ok(raster);
  const mapped = await sheet(page, 1).locator(".pdf-clear").elementHandle();
  assert.ok(mapped);
  const oldFilter = await page.locator(".pdf-tone-map feFuncR").getAttribute("tableValues");
  const previousLight = await brightness.inputValue();
  await brightness.press("End");
  assert.notEqual(await brightness.inputValue(), previousLight);
  await page.waitForFunction(
    (initial) =>
      document.querySelector(".pdf-tone-map feFuncR")?.getAttribute("tableValues") !== initial,
    oldFilter,
  );
  assert.ok(
    await raster.evaluate(
      (canvas) => canvas === document.querySelector('.pdf-sheet[data-page="1"] .pdf-original'),
    ),
  );
  assert.ok(
    await mapped.evaluate(
      (canvas) => canvas === document.querySelector('.pdf-sheet[data-page="1"] .pdf-clear'),
    ),
  );
  const apply = objectTools(page).getByRole("button", { name: english("wallpaper_apply") });
  evidence.brightnessApply = await pressAndCancel(page, apply);
  assert.ok(evidence.brightnessApply.rest.glyph);
  await page.keyboard.press("Escape");
  const brightnessAction = pdfTool(page, "wallpaper_brightness");
  await page.waitForFunction(
    (node) => node?.closest("button") === document.activeElement,
    await brightnessAction.elementHandle(),
  );
  await page.keyboard.press("Escape");
  await objectTools(page).waitFor({ state: "hidden" });
  assert.deepEqual(
    await position(page),
    before,
    "closing document details leaves the page geometry fixed",
  );
  evidence.details = "preserved the reading anchor";
  evidence.stage = "native text selection and keyboard tools";
  evidence.textSelection = await keyboardSelection(page);
  evidence.attachmentControl = await readingControls(page, ["attach"]);
  evidence.stage = "touch tools";
  evidence.touchTools = await touchTools(page);
  evidence.stage = "keyboard region controls";
  evidence.keyboardRegion = await keyboardRegion(page);
  await navigation(page).click();
  const covered = await position(page);
  assert.deepEqual(covered, before, "the file list leaves the page width and screen line fixed");
  await navigation(page).click();

  evidence.stage = "right-click draft crop";
  evidence.crop = await crop(page);
  assert.ok(evidence.crop.blue > 800, "the selected formula should have its blue background");
  assert.equal(evidence.crop.red, 0, "the chart outside the selection must never enter the draft");
  assert.deepEqual(await position(page), before, "the draft leaves the page geometry fixed");
  const composer = page.locator("#workspace-conversation textarea").first();
  const prompt = "Explain the selected formula in the paper.";
  await composer.fill(prompt);

  const stored = page.waitForResponse((response) => {
    const request = response.request();
    if (
      new URL(request.url()).pathname !== "/workspace" ||
      request.method() !== "POST" ||
      !response.ok()
    )
      return false;
    const data = request.postDataJSON();
    return data.kind === "remember" && data.changes?.some((change) => change.progress?.page === 3);
  });
  await sheet(page, 3).evaluate((row) => {
    const scroller = row.closest(".pdf-scroll");
    scroller.scrollTop = row.offsetTop + row.offsetHeight * 0.24;
  });
  await stored;
  evidence.anchor = await position(page);
  assert.equal(evidence.anchor.page, 3);

  evidence.stage = "temporary magnification";
  await keyboardTools(page);
  await pdfTool(page, "pdf_zoom").click();
  const zoom = page.getByRole("dialog", { name: english("pdf_zoom") });
  await zoom.waitFor();
  const zoomBack = zoom.getByRole("button", { name: english("pdf_close_zoom") });
  evidence.zoomBack = await pressAndCancel(page, zoomBack);
  assert.ok(evidence.zoomBack.rest.glyph);
  const pan = zoom.locator(".pdf-zoom-scroll");
  const panBounds = await pan.boundingBox();
  assert.ok(panBounds);
  const initialPan = await pan.evaluate((node) => ({
    x: node.scrollLeft,
    y: node.scrollTop,
    east: node.scrollWidth - node.clientWidth - node.scrollLeft,
    south: node.scrollHeight - node.clientHeight - node.scrollTop,
  }));
  const start = {
    x: panBounds.x + panBounds.width / 2,
    y: panBounds.y + panBounds.height / 2,
  };
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  try {
    await page.mouse.move(
      start.x + (initialPan.east >= 80 ? -80 : 80),
      start.y + (initialPan.south >= 80 ? -80 : 80),
      { steps: 6 },
    );
  } finally {
    await page.mouse.up();
  }
  evidence.panned = await pan.evaluate((node) => ({ x: node.scrollLeft, y: node.scrollTop }));
  assert.notEqual(evidence.panned.x, initialPan.x);
  assert.notEqual(evidence.panned.y, initialPan.y);
  await page.keyboard.press("Escape");
  await zoom.waitFor({ state: "hidden" });
  assert.deepEqual(
    await position(page),
    evidence.anchor,
    "closing zoom leaves the original page and line fixed",
  );

  evidence.stage = "session and file subject switch";
  const mode = subject(page);
  await openAttachments(page);
  const beforeMode = await attachmentState(page);
  await mode.click();
  assert.equal(await mode.getAttribute("aria-pressed"), "false");
  assert.equal(await composer.inputValue(), prompt);
  evidence.attachmentAfterSubjectSwitch = await attachmentState(page);
  assert.deepEqual(evidence.attachmentAfterSubjectSwitch, beforeMode);
  assert.equal(await page.getByRole("img", { name: english("talk_attachment_region") }).count(), 1);
  await mode.click();
  assert.equal(await mode.getAttribute("aria-pressed"), "true");
  assert.deepEqual(
    await position(page),
    evidence.anchor,
    "switching subjects leaves the reading position fixed",
  );

  const restored = await context.newPage();
  restored.on("pageerror", (error) => evidence.errors.push(error.message));
  await restored.goto(`${ORIGIN}/`);
  await sheet(restored, 3).locator(".pdf-original").waitFor();
  evidence.restored = await position(restored);
  assert.equal(evidence.restored.page, evidence.anchor.page);
  assert.equal(evidence.restored.width, evidence.anchor.width);
  assert.ok(Math.abs(evidence.restored.offset - evidence.anchor.offset) < 0.002);
  evidence.frames = await page.evaluate(() =>
    window.refrainPdfPaints.map((frame) => ({
      ...frame,
      elapsedAfterOpen: frame.time - window.refrainPdfOpenedAt,
    })),
  );
  evidence.liveCanvasBytes = await page
    .locator(".pdf-pixels canvas")
    .evaluateAll((nodes) => nodes.reduce((sum, node) => sum + 4 * node.width * node.height, 0));
  assert.ok(evidence.liveCanvasBytes <= 160 * 1024 ** 2, "resident page rasters are bounded");

  evidence.stage = "versioned source references";
  const reference = page.locator("#workspace-conversation").getByRole("button", {
    name: PAPER,
    exact: true,
  });
  await reference.click();
  await sheet(page, 1).locator(".pdf-reference").waitFor();
  await sheet(page, 2).evaluate((row) => {
    row.closest(".pdf-scroll").scrollTop = row.offsetTop;
  });
  evidence.referenceControl = await readingControls(page, ["reference"]);
  await keyboardTools(page);
  await pdfTool(page, "pdf_reference").click();
  assert.equal((await position(page)).page, 1);
  await writeFile(join(root, PAPER), `${fixture()}% source changed after drafting\n`, "ascii");
  await reference.click();
  await page.getByText(english("shell_reference_changed"), { exact: true }).waitFor();
  assert.equal(await page.locator(".pdf-reference").count(), 0);
  evidence.staleReference = "refused";

  if (scan !== null) {
    evidence.stage = "real scan and shell geometry";
    assert.ok(Number.isSafeInteger(SCAN_PAGE) && SCAN_PAGE > 0, "choose a one-based scan page");
    await openDocument(restored, SCAN);
    const scanned = sheet(restored, SCAN_PAGE);
    await scanned.evaluate((row) => {
      row.closest(".pdf-scroll").scrollTop = row.offsetTop;
    });
    await scanned.locator(".pdf-clear").waitFor();
    evidence.scan = {
      sha256: createHash("sha256").update(scan).digest("hex"),
      position: await position(restored),
      textSpans: await scanned.locator(".pdf-text-layer span").count(),
      graphics: await restored.locator(".pdf-view").evaluate((view) => ({
        webgl2: view.dataset.webgl2,
        webgpu: view.dataset.webgpu,
      })),
    };
    await restored.screenshot({ path: join(output, "scan-page-width.png") });
    await keyboardTools(restored);
    await toolRow(restored, "original").focus();
    await restored.keyboard.down("Space");
    try {
      assert.equal(await restored.locator(".pdf-view").getAttribute("data-original"), "true");
      await restored.screenshot({ path: join(output, "scan-original-held.png") });
    } finally {
      await restored.keyboard.up("Space");
    }
    await restored.keyboard.press("Escape");
    await objectTools(restored).waitFor({ state: "hidden" });
    assert.deepEqual(await position(restored), evidence.scan.position);
    await shellCapture(restored, output);
    evidence.stage = "workspace editing, density and recovery";
    evidence.workspaceState = await workspaceState(restored, root, button);
  }
  assert.deepEqual(evidence.errors, []);
  assert.deepEqual(evidence.foreignRequests, []);
  assert.deepEqual(evidence.failedAssets, []);
  delete evidence.stage;
  evidence.result = "success";
} catch (error) {
  evidence.result = "failure";
  evidence.failure = error.message;
  throw error;
} finally {
  await writeFile(join(output, "production-pdf.json"), JSON.stringify(evidence, null, 2));
  await browser.close();
  await rm(root, { recursive: true, force: true });
}
