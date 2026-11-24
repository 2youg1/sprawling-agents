// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";

import { animationIntervals, traceInput } from "./workspace-input.mjs";
import { controlInventory, pressAndCancel, spaceAndCancel } from "./control-frames.mjs";
import words from "../../RefRain/src/lang.json";
import { DEFAULT_VIEW, toFragment } from "../../RefRain/src/core/route.ts";

function drafts(page) {
  return page.evaluate(() =>
    Object.keys(localStorage)
      .filter((key) => key.startsWith("sprawling.draft."))
      .sort()
      .map((key) => [key, localStorage.getItem(key)]),
  );
}

async function verifySeparateStands(page) {
  const starting = page.getByRole("region", {
    name: "palette · places, the cursor on the first row",
  });
  const pasted = page.getByRole("region", {
    name: "palette · a pasted address becomes one direct row",
  });
  const history = page.getByRole("region", {
    name: "palette · opened from the address line: this room's sessions first",
  });
  const field = starting.getByRole("combobox");
  const other = pasted.getByRole("combobox");
  await field.waitFor();
  const example = await other.inputValue();
  assert.equal(example, "hall/new-room");
  const session = history.getByRole("option", { name: /release notes/ });
  await session.waitFor();
  await field.fill("gallery/specimen");
  assert.equal(await field.inputValue(), "gallery/specimen");
  assert.equal(await other.inputValue(), example);
  await session.click();
  assert.equal(new URL(page.url()).hash, toFragment({ kind: "gallery" }));

  const historyFixture = page.getByRole("region", {
    name: "workbench · a picked commit's identity in the commits pane",
  });
  const initial = historyFixture.getByRole("button", { name: /edit document\.rs/ });
  const selected = historyFixture.getByRole("button", { name: /read the specification/ });
  assert.equal(await initial.getAttribute("aria-expanded"), "true");
  await selected.click();
  assert.equal(await selected.getAttribute("aria-expanded"), "true");
  assert.equal(await initial.getAttribute("aria-expanded"), "false");
  assert.equal(new URL(page.url()).hash, toFragment({ kind: "gallery" }));

  const occupied = page.getByRole("region", { name: "shell · a working room, in zen" });
  const vacant = page.getByRole("region", { name: "shell · an empty room, in zen" });
  const railKey = (sample) =>
    sample.getByRole("button", { name: words.world_sessions.en, exact: true });
  const openList = railKey(occupied);
  const otherList = railKey(vacant);
  const panelId = await openList.getAttribute("aria-controls");
  assert.ok(panelId);
  assert.notEqual(await otherList.getAttribute("aria-controls"), panelId);
  const shown = occupied.locator(`[id="${panelId}"]`);
  await openList.press("Enter");
  await shown.getByRole("heading", { name: words.world_sessions.en, exact: true }).waitFor();
  assert.equal(await otherList.getAttribute("aria-expanded"), "false");
  await shown.getByRole("button", { name: words.shell_close_list.en, exact: true }).press("Enter");
  await page.waitForFunction(
    (node) => node === document.activeElement,
    await openList.elementHandle(),
  );
  assert.equal(await openList.getAttribute("aria-expanded"), "false");
}

async function verifyWorkspace(page) {
  const root = await mkdtemp(join(tmpdir(), "refrain-gallery-workspace-"));
  const name = "gallery-return.txt";
  try {
    await writeFile(join(root, name), "The desktop directory remains available.\n");
    const mode = page.locator("#workspace-subject-key");
    await mode.click();
    assert.equal(await mode.getAttribute("aria-pressed"), "true");
    await page.locator("#workspace-navigation-key").click();
    await page.getByRole("button", { name: words.shell_library_tools.en, exact: true }).click();
    const registration = page.locator("#workspace-navigation summary").filter({
      hasText: words.shell_register_workspace.en,
    });
    const field = page.getByLabel(words.shell_workspace_root.en, { exact: true });
    await field.or(registration).first().waitFor();
    if (!(await field.isVisible())) await registration.click();
    await field.fill(root);
    await page
      .getByRole("button", { name: words.shell_register_workspace.en, exact: true })
      .click();
    await page.getByRole("button", { name, exact: true }).first().click();
    await page
      .locator("#workspace-file-canvas")
      .getByText("The desktop directory remains available.")
      .waitFor();
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

async function nativePress(page, control) {
  assert.notEqual(await control.getAttribute("data-control"), null);
  const pointer = await pressAndCancel(page, control, {
    held: async () =>
      page.waitForFunction(
        (node) => Number(getComputedStyle(node).scale) < 1,
        await control.elementHandle(),
      ),
  });
  assert.notEqual(pointer.holding.scale, pointer.settled.scale);
  return { pointer, keyboard: await spaceAndCancel(page, control) };
}

async function verifyControls(page, evidence) {
  const row = page.getByRole("region", { name: "row · a walked list, the second row chosen" });
  const first = row.getByRole("button", { name: /openai\/gpt-nucleus-6/ });
  await first.scrollIntoViewIfNeeded();
  evidence.nativePress = { row: await nativePress(page, first) };
  await first.focus();
  await page.keyboard.press("ArrowDown");
  const chosen = row.getByRole("button", { name: /anthropic\/claude-lattice/ });
  assert.ok(await chosen.evaluate((node) => node === document.activeElement));
  assert.equal(await chosen.locator("..").getAttribute("aria-current"), "true");
  await page.keyboard.press("End");
  assert.ok(await chosen.evaluate((node) => node === document.activeElement));
  await page.keyboard.press("Home");
  assert.ok(await first.evaluate((node) => node === document.activeElement));
  const check = row.getByRole("button", { name: words.mcp_check.en, exact: true });
  await page.keyboard.press("Tab");
  assert.ok(await check.evaluate((node) => node === document.activeElement));
  await page.keyboard.press("Tab");
  assert.ok(await chosen.evaluate((node) => node === document.activeElement));

  const segmented = page.getByRole("region", { name: "segmented · a cell that cannot be chosen" });
  const previous = segmented.getByRole("radio", { name: "chat" });
  const disabled = segmented.getByRole("radio", { name: "responses" });
  const next = segmented.getByRole("radio", { name: "messages" });
  await previous.scrollIntoViewIfNeeded();
  evidence.nativePress.segmented = await nativePress(page, previous);
  await page.emulateMedia({ reducedMotion: "reduce" });
  const reduced = await spaceAndCancel(page, previous);
  assert.match(reduced.held.shadow, /inset/);
  assert.notEqual(reduced.held.shadow, reduced.rest.shadow);
  assert.equal(reduced.held.scale, reduced.rest.scale);
  await page.emulateMedia({ forcedColors: "active" });
  const forced = await spaceAndCancel(page, previous);
  assert.match(forced.held.outline, /dashed/);
  assert.ok(Number.parseFloat(forced.held.outlineOffset) > 0);
  assert.doesNotMatch(forced.rest.outline, /dashed/);
  evidence.nativePress.static = { reduced, forced };
  await page.emulateMedia({ reducedMotion: "no-preference", forcedColors: "none" });
  const cdp = await page.context().newCDPSession(page);
  await previous.focus();
  evidence.segmentedInput = await traceInput(cdp, async () => {
    for (let step = 0; step < 6; step++) {
      await page.keyboard.press("ArrowRight");
      await page.keyboard.press("ArrowLeft");
    }
    await page.keyboard.press("ArrowRight");
  });
  await cdp.detach();
  assert.equal(await next.getAttribute("aria-checked"), "true");
  assert.ok(await next.evaluate((node) => node === document.activeElement));
  assert.equal(await disabled.getAttribute("aria-disabled"), "true");
  await disabled.focus();
  const hint = await disabled.getAttribute("aria-describedby");
  assert.ok(hint);
  const tip = page.locator(`[id="${hint}"]`);
  await tip.waitFor();
  await page.keyboard.press("Escape");
  await tip.waitFor({ state: "hidden" });
  assert.ok(await disabled.evaluate((node) => node === document.activeElement));
  await page.keyboard.press("Space");
  assert.equal(await next.getAttribute("aria-checked"), "true");

  const bars = page
    .getByRole("region", { name: "progress · part of the way, at its end, and with no end yet" })
    .getByRole("progressbar");
  const [partial, complete, pending] = await bars.all();
  assert.equal(await partial.getAttribute("aria-valuenow"), "3");
  assert.equal(await partial.getAttribute("aria-valuemax"), "12");
  assert.equal(await complete.getAttribute("aria-valuenow"), "12");
  assert.equal(await pending.getAttribute("aria-busy"), "true");
  const pendingStill = page
    .getByRole("region", { name: "progress · with no end yet, motion off" })
    .getByRole("progressbar");
  assert.equal(await pendingStill.getAttribute("aria-busy"), "true");
  await pending.scrollIntoViewIfNeeded();
  const pulse = await pending.evaluate((node) => {
    const animation = node.getAnimations()[0];
    const effect = animation?.effect;
    return effect
      ? { name: animation.animationName, ms: effect.getComputedTiming().duration }
      : null;
  });
  const motionOff = await pendingStill.evaluate((node) => node.getAnimations().length);
  assert.ok(pulse?.ms > 0);
  assert.equal(motionOff, 0);
  evidence.progress = { pulse, motionOff, environment: await animationIntervals(page) };
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.waitForFunction(() =>
    [...document.querySelectorAll('[role="progressbar"].pulse')].every(
      (node) => node.getAnimations().length === 0,
    ),
  );
  await page.emulateMedia({ reducedMotion: "no-preference" });

  const field = page.getByRole("region", {
    name: "field · help, the city's error, the browser's user-invalid, mono with fixed parts",
  });
  const host = field.getByRole("textbox", { name: words.setup_base_url.en, exact: false }).first();
  await host.fill("api.second.test");
  await page.keyboard.press("End");
  await page.keyboard.type(".org");
  assert.equal(await host.inputValue(), "api.second.test.org");
  assert.ok(await host.evaluate((node) => node === document.activeElement));
  const badge = page.getByRole("region", {
    name: "badge · counts on a key's corner: a few, past the cap, and new without a number",
  });
  assert.equal(await badge.getByText("99+", { exact: true }).getAttribute("aria-hidden"), "true");
  evidence.controls = "row, segmented, progress, field, badge";
}

async function verifyTabsAndCopy(page, evidence) {
  const tabs = page.getByRole("region", {
    name: "tabs · three lenses, a control in the panel and a count beside a name",
  });
  const changes = tabs.getByRole("tab", { name: words.run_changes.en });
  const modes = [
    ["normal", {}],
    ["reduced", { reducedMotion: "reduce" }],
    ["forced", { forcedColors: "active" }],
  ];
  evidence.tabs = {};
  for (const [name, media] of modes) {
    await page.emulateMedia({ reducedMotion: "no-preference", forcedColors: "none", ...media });
    const trace = await pressAndCancel(page, changes);
    if (name === "reduced") assert.equal(trace.holding.duration, "0s");
    evidence.tabs[name] = trace;
  }
  await page.emulateMedia({ forcedColors: "none" });
  await changes.click();
  assert.equal(await changes.getAttribute("aria-selected"), "true");
  await changes.focus();
  await page.keyboard.press("Home");
  const time = tabs.getByRole("tab", { name: words.run_time.en });
  assert.equal(await time.getAttribute("aria-selected"), "true");
  assert.ok(await time.evaluate((node) => node === document.activeElement));
  await page.waitForFunction(
    (panel) => panel?.tabIndex === 0,
    await tabs.getByRole("tabpanel").elementHandle(),
  );
  await page.keyboard.press("Tab");
  assert.ok(await tabs.getByRole("tabpanel").evaluate((node) => node === document.activeElement));

  const examples = page.getByRole("region", {
    name: "copy · beside a command, at rest, copied, refused, and with no clipboard",
  });
  const copy = examples.getByRole("button", { name: words.setup_copy.en, exact: true }).first();
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  const copied = await copy.elementHandle();
  assert.ok(copied);
  await copy.click();
  assert.equal(
    await page.evaluate(() => navigator.clipboard.readText()),
    "cargo install sprawling --locked",
  );
  evidence.copy = {
    action: "clipboard received the fixture command",
    focus: await copied.evaluate((node) => node === document.activeElement),
  };
  assert.ok(evidence.copy.focus);
  evidence.galleryInventory = await controlInventory(page);
}

async function verifyPanels(page, evidence) {
  const tipExample = page.getByRole("region", {
    name: "tip · a hint on a control named by its own text",
  });
  const tipButton = tipExample.getByRole("button", { name: words.part_save.en, exact: true });
  await tipButton.focus();
  const tip = tipExample.getByRole("tooltip");
  await tip.waitFor();
  const hint = await tip.getAttribute("id");
  assert.equal(await tipButton.getAttribute("aria-describedby"), hint);
  await page.keyboard.press("Escape");
  await tip.waitFor({ state: "hidden" });
  assert.ok(await tipButton.evaluate((node) => node === document.activeElement));

  const example = page.getByRole("region", { name: "dialog · confirming what cannot be undone" });
  const opener = example.getByRole("button", { name: words.setup_remove.en, exact: true });
  const dialog = example.getByRole("dialog", { name: words.part_remove_endpoint.en });
  const canceledPress = await pressAndCancel(page, opener);
  assert.equal(await dialog.isVisible(), false);
  await opener.click();
  await dialog.waitFor();
  const cancel = dialog.getByRole("button", { name: words.part_cancel.en, exact: true });
  await cancel.focus();
  await page.keyboard.press("Escape");
  await dialog.waitFor({ state: "hidden" });
  assert.ok(await opener.evaluate((node) => node === document.activeElement));
  await opener.press("Enter");
  await dialog.getByRole("button", { name: words.part_delete.en, exact: true }).click();
  await dialog.waitFor({ state: "hidden" });
  assert.ok(await opener.evaluate((node) => node === document.activeElement));

  const box = page.getByRole("region", { name: "combobox · open on click, nothing chosen" });
  const digest = box.getByRole("button", { name: words.setup_digest.en, exact: true });
  const field = box.getByRole("combobox", { name: words.setup_digest.en, exact: true });
  await digest.press("Enter");
  await field.waitFor();
  const id = await field.getAttribute("aria-controls");
  assert.ok(id);
  const options = page.locator(`[id="${id}"]`);
  const absent = "no-such-reflection-model";
  await field.fill(absent);
  await options.getByText(words.part_no_match.en, { exact: true }).waitFor();
  await field.press("Enter");
  assert.equal(await digest.getAttribute("aria-expanded"), "true");
  await field.fill("");
  const first = options.getByRole("option").first();
  await first.waitFor();
  const selected = await first.locator("span").first().textContent();
  await first.click();
  assert.ok((await digest.textContent()).startsWith(selected));
  assert.equal(await digest.getAttribute("aria-expanded"), "false");
  assert.ok(await digest.evaluate((node) => node === document.activeElement));

  const table = page
    .getByRole("region", {
      name: "table · partial selection, sortable, corrected in place",
    })
    .getByRole("table");
  const all = table.getByRole("checkbox", { name: words.part_select_all.en });
  assert.ok(await all.evaluate((node) => node.indeterminate));
  await all.check();
  assert.ok(await all.isChecked());
  assert.ok(await all.evaluate((node) => !node.indeterminate));
  const sort = table.getByRole("button", { name: words.part_model_id.en });
  evidence.nativePress.sort = await nativePress(page, sort);
  await sort.click();
  assert.equal(
    await sort.evaluate((node) => node.closest("th")?.getAttribute("aria-sort")),
    "ascending",
  );
  const cell = table.getByRole("textbox").first();
  await cell.fill("8192");
  await cell.press("Tab");
  assert.equal(await cell.inputValue(), "8192");
  evidence.panels = {
    tip: "describes the focused control and dismisses on Escape without changing focus",
    dialog: canceledPress,
    combo: { refused: absent, selected },
    table: "selected, sorted and edited",
  };
}

async function verifyDocumentPress(page, evidence) {
  const documentActions = page.getByRole("region", {
    name: "document actions · draft, citation and diagnostics",
  });
  await documentActions.getByText(words.talk_attachments.en, { exact: true }).click();
  await documentActions.getByText(words.diagnostic_details.en, { exact: true }).click();
  const action = (name) => documentActions.getByRole("button", { name, exact: true });
  for (const [kind, name] of [
    ["attachment", words.talk_remove_attachment.en.replace("{file}", "paper.md")],
    ["citation", words.talk_open_citation.en.replace("{file}", "paper.md")],
    ["diagnostic", words.diagnostic_copy.en],
  ]) {
    evidence.nativePress[kind] = await nativePress(page, action(name));
    assert.equal(new URL(page.url()).hash, toFragment({ kind: "gallery" }));
  }
}

async function verifyFormats(page, evidence) {
  const docx = page.getByRole("region", {
    name: "formats · a DOCX laid out as pages, its tracked change and comment counted",
  });
  await docx.getByText(words.format_docx_revisions_one.en, { exact: false }).waitFor();
  await docx.getByText(words.format_docx_comments_one.en, { exact: false }).waitFor();
  const docxFrame = docx.locator("iframe");
  assert.equal(await docxFrame.getAttribute("sandbox"), "");
  await docxFrame.contentFrame().getByText("Release plan", { exact: true }).waitFor();

  const html = page.getByRole("region", {
    name: "formats · an HTML page in RefRain's preview, its script and outside picture held back",
  });
  const htmlFrame = html.locator("iframe");
  assert.equal(await htmlFrame.getAttribute("sandbox"), "");
  assert.equal(await htmlFrame.getAttribute("referrerpolicy"), "no-referrer");
  await htmlFrame.contentFrame().getByRole("heading", { name: "Opening hours" }).waitFor();

  const overLimit = page.getByRole("region", {
    name: "formats · a DOCX that unpacks past the bound, refused unopened",
  });
  assert.equal(await overLimit.locator("iframe").count(), 0);
  const bounded = words.format_zip_too_large.en.split("{limit}")[0];
  assert.ok(bounded.length > 0);
  await overLimit.getByText(bounded, { exact: false }).waitFor();

  const receipt = page.getByRole("region", {
    name: "refrain · the head line, every word a save leaves on it",
  });
  const states = await receipt.getByRole("status").allTextContents();
  for (const state of ["draft", "saving", "pending", "saved", "conflict", "refused"])
    assert.ok(states.includes(words[`refrain_receipt_${state}`].en));
  evidence.formats = {
    docx: "opened with comments and changes",
    html: "isolated",
    receipt: states,
  };
}

const [port, output] = process.argv.slice(2);
const admissionFile = process.env.REFRAIN_OPEN_URL_FILE ?? process.env.REFRAIN_PDF_OPEN_URL_FILE;
const playwright = process.env.PLAYWRIGHT_MODULE;
if (!/^\d+$/.test(port ?? "") || !output || !playwright || !admissionFile) {
  throw new Error(
    "usage: PLAYWRIGHT_MODULE=<module> REFRAIN_OPEN_URL_FILE=<opening> bun acceptance-gallery.mjs <port> <output>",
  );
}
const origin = `http://127.0.0.1:${port}`;
const opening = new URL((await readFile(admissionFile, "utf8")).trim());
assert.ok(
  opening.origin === origin && opening.hash.startsWith("#open="),
  "use the one-use URL from the disposable city",
);
await mkdir(output, { recursive: true });
const { chromium } = await import(pathToFileURL(playwright).href);
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.REFRAIN_BROWSER_BINARY ?? process.env.REFRAIN_PDF_BROWSER,
});
const page = await browser.newPage({
  viewport: { width: 1280, height: 960 },
  deviceScaleFactor: 1.25,
  locale: "en-US",
});
page.setDefaultTimeout(70000);
const evidence = {
  browser: browser.version(),
  errors: [],
  cmaps: [],
  fixtureCommands: 0,
  externalRequests: [],
};
let watchingFixtures = false;
const conceal = (text) => text.replaceAll(/#open=[a-z\d_-]+/gi, "#open=<redacted>");
page.on("pageerror", (error) => {
  const stack = conceal(error.stack ?? error.message);
  if (!evidence.errors.includes(stack)) evidence.errors.push(stack);
});
page.on("response", (response) => {
  const url = new URL(response.url());
  if (url.origin === origin && url.pathname.endsWith(".bcmap"))
    evidence.cmaps.push({ path: url.pathname, code: response.status() });
});
page.on("request", (request) => {
  const url = new URL(request.url());
  if (watchingFixtures && /^https?:$/.test(url.protocol) && url.origin !== origin)
    evidence.externalRequests.push(url.origin);
});
page.on("websocket", (socket) => {
  socket.on("framesent", ({ payload }) => {
    if (watchingFixtures && typeof payload === "string" && payload.startsWith('{"command":'))
      evidence.fixtureCommands++;
  });
});
try {
  await page.goto(opening.href, { waitUntil: "domcontentloaded" });
  const putOff = page.getByRole("button", { name: words.guide_put_off_rest.en, exact: true });
  await putOff.focus();
  await page.keyboard.press("Enter");
  await page.waitForFunction((route) => location.hash === route, toFragment(DEFAULT_VIEW));
  await page.locator("#main textarea").first().waitFor();
  await page.evaluate(async () => {
    await Promise.allSettled(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished),
    );
  });
  const before = await drafts(page);
  watchingFixtures = true;
  await page.evaluate(
    (fragment) => {
      location.hash = fragment;
    },
    toFragment({ kind: "gallery" }),
  );
  await verifySeparateStands(page);
  const keys = page.getByRole("region", {
    name: "kbd · a chord, a line move and a first letter at a row's end",
  });
  await keys.scrollIntoViewIfNeeded();
  await keys.locator("kbd").first().waitFor();
  evidence.keys = await keys.locator("kbd").allTextContents();
  assert.deepEqual(evidence.keys, ["Ctrl", "/", "↓", "D"]);
  await keys.screenshot({ path: join(output, "native-kbd.png") });

  const fixture = page.getByRole("region", {
    name: "formats · a PDF, its pages drawn by pdf.js, Chinese through a CMap",
  });
  await fixture.scrollIntoViewIfNeeded();
  const scroll = fixture.locator(".pdf-scroll");
  const choices = page.locator("[data-context-tools]");
  const openTools = async () => {
    await scroll.focus();
    await page.keyboard.press("Shift+F10");
    await choices.getByRole("menu").waitFor();
  };
  await fixture.locator('[data-page="1"] .pdf-original').waitFor();
  await openTools();
  const original = choices.getByRole("menuitem", { name: words.pdf_original.en });
  await original.focus();
  await page.keyboard.down("Space");
  assert.equal(await fixture.locator(".pdf-view").getAttribute("data-original"), "true");
  await page.keyboard.up("Space");
  assert.equal(await fixture.locator(".pdf-view").getAttribute("data-original"), "false");
  await page.keyboard.press("Escape");
  await scroll.focus();
  await page.keyboard.press("End");
  const chinese = fixture.locator('[data-page="2"] .pdf-text-layer');
  await chinese.getByText("中文排版", { exact: false }).waitFor();
  await fixture.locator('[data-page="2"] .pdf-original').waitFor();
  evidence.page = await fixture.locator(".pdf-sheet[data-page='2']").evaluate((el) => {
    const box = el.getBoundingClientRect();
    return { width: box.width, height: box.height, top: box.top };
  });
  const anchor = await scroll.evaluate((el) => el.scrollTop);
  await openTools();
  const details = choices.getByRole("menuitem", { name: words.pdf_details.en, exact: true });
  await details.focus();
  await page.keyboard.press("Enter");
  await choices.locator(".pdf-metadata").waitFor();
  assert.equal(await scroll.evaluate((el) => el.scrollTop), anchor);
  await page.waitForFunction(() => document.activeElement?.hasAttribute("data-context-back"));
  await page.keyboard.press("Escape");
  await choices.locator(".pdf-metadata").waitFor({ state: "hidden" });
  await page.waitForFunction(
    (node) => node === document.activeElement,
    await details.elementHandle(),
  );
  const zoom = choices.getByRole("menuitem", { name: words.pdf_zoom.en, exact: true });
  await zoom.focus();
  await page.keyboard.press("Enter");
  const layer = fixture.getByRole("dialog", { name: "Magnify page", exact: true });
  await layer.waitFor();
  await page.keyboard.press("Escape");
  await layer.waitFor({ state: "hidden" });
  assert.equal(await scroll.evaluate((el) => el.scrollTop), anchor);
  assert.ok(await scroll.evaluate((el) => document.activeElement === el));
  assert.ok(evidence.cmaps.some((item) => item.code === 200));
  await fixture.screenshot({ path: join(output, "native-pdf-gallery.png") });
  await verifyFormats(page, evidence);
  await verifyControls(page, evidence);
  await verifyTabsAndCopy(page, evidence);
  await verifyPanels(page, evidence);
  await verifyDocumentPress(page, evidence);
  assert.deepEqual(await drafts(page), before, "fixture drafts stay in the Gallery");
  assert.equal(evidence.fixtureCommands, 0, "no fixture transmits a real command");
  assert.deepEqual(evidence.externalRequests, [], "the authored fixture sends no external request");
  watchingFixtures = false;
  await page.goBack();
  await page.waitForFunction((fragment) => location.hash === fragment, toFragment(DEFAULT_VIEW));
  await verifyWorkspace(page);
  evidence.isolation =
    "independent stand input, commit and rail, unchanged drafts, real directory after back";
  assert.deepEqual(evidence.errors, []);
  evidence.result = "success";
} catch (error) {
  evidence.result = "failure";
  evidence.failure = conceal(error.message);
  evidence.location = new URL(page.url()).hash.split("#open=")[0];
  evidence.visibleHeadings = await page.locator("h1:visible,h2:visible").allTextContents();
  throw new Error(evidence.failure);
} finally {
  await writeFile(join(output, "native-gallery.json"), JSON.stringify(evidence, null, 2));
  await browser.close();
}
