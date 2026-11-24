// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";

const NOTE = "workspace-state.md";
const ORIGINAL = "The editor retains its undo history.";
const PREFIX = "Edited ";
const DRAFT = "Keep this draft across the workspace subject switch.";
const NAV = "#workspace-navigation-key";
const SUBJECT = "#workspace-subject-key";
const COMPOSER = "#workspace-conversation textarea";
const EDITOR = "#workspace-file-canvas .cm-content";
const SPLITTER = '[role="separator"][aria-controls="workspace-file-canvas"]';

async function openLayout(page, button) {
  const navigation = page.locator(NAV);
  if ((await navigation.getAttribute("aria-expanded")) !== "true") await navigation.click();
  await button(page, "shell_layout_tools").click();
}

async function showDraft(page, button) {
  if (await page.locator(COMPOSER).first().isVisible()) return;
  await openLayout(page, button);
  await button(page, "shell_draft").click();
}

export async function workspaceState(page, root, button) {
  await writeFile(join(root, NOTE), ORIGINAL + "\n");
  await page.locator(NAV).click();
  await page
    .locator("#workspace-navigation")
    .getByRole("button", { name: NOTE, exact: true })
    .first()
    .click();
  const editor = page.locator(EDITOR);
  await editor.waitFor();
  await editor.focus();
  await page.keyboard.press("Control+Home");
  await page.keyboard.type(PREFIX);
  await page.keyboard.press("Shift+Home");
  const selected = await page.evaluate(() => getSelection()?.toString());
  assert.equal(selected, PREFIX);
  await editor.evaluate((node) => {
    window.workspaceOriginalEditor = node;
  });

  await showDraft(page, button);
  const composer = page.locator(COMPOSER).first();
  await composer.fill(DRAFT);
  await composer.evaluate((node) => {
    window.workspaceOriginalComposer = node;
  });
  await page.locator(SUBJECT).click();
  await page.locator('main[data-subject="session"]').waitFor();
  assert.equal(await composer.inputValue(), DRAFT);
  assert(await composer.evaluate((node) => node === window.workspaceOriginalComposer));
  await openLayout(page, button);
  const density = button(page, "tier_panorama");
  if ((await density.getAttribute("aria-pressed")) === "true") await density.click();
  await density.click({ trial: true });
  const key = await density.boundingBox();
  assert(key);
  await page.mouse.move(key.x + key.width / 2, key.y + key.height / 2);
  await page.mouse.down();
  const activity = page.locator(".refrain-activity-inline");
  await activity.waitFor();
  await page.mouse.up();
  await activity.waitFor({ state: "hidden" });
  assert.equal(await density.getAttribute("aria-pressed"), "false");
  await density.click();
  await page.locator(".refrain-world-place").waitFor();
  assert.equal(await density.getAttribute("aria-pressed"), "true");
  assert(await composer.evaluate((node) => node === window.workspaceOriginalComposer));
  await density.click();

  await page.locator(SUBJECT).click();
  await editor.waitFor();
  assert(await editor.evaluate((node) => node === window.workspaceOriginalEditor));
  await editor.focus();
  assert.equal(await page.evaluate(() => getSelection()?.toString()), selected);
  await page.keyboard.press("Control+z");
  assert.equal((await editor.textContent()).trim(), ORIGINAL);

  const beforeSplit = await editor.evaluate(
    (node) => node.closest("#workspace-file-canvas").getBoundingClientRect().width,
  );
  await openLayout(page, button);
  await button(page, "shell_split").click();
  const separator = page.locator(SPLITTER);
  await separator.waitFor();
  const previous = Number(await separator.getAttribute("aria-valuenow"));
  const minimum = Number(await separator.getAttribute("aria-valuemin"));
  const step = previous === minimum ? 1 : -1;
  await separator.press(step === 1 ? "ArrowRight" : "ArrowLeft");
  const ratio = await separator.getAttribute("aria-valuenow");
  assert.equal(Number(ratio), previous + step);
  const splitWidth = await editor.evaluate(
    (node) => node.closest("#workspace-file-canvas").getBoundingClientRect().width,
  );
  assert(splitWidth < beforeSplit, "only explicit splitting narrows the editor");
  const coldDraft = DRAFT + " Retain the latest keystroke.";
  await composer.fill(coldDraft);
  const context = page.context();
  const entry = new URL(page.url()).origin;
  await page.close();
  const reopened = await context.newPage();
  await reopened.goto(entry);
  await reopened.locator('main[data-subject="files"]').waitFor();
  assert.equal((await reopened.locator(EDITOR).textContent()).trim(), ORIGINAL);
  assert.equal(await reopened.locator(SPLITTER).getAttribute("aria-valuenow"), ratio);
  await showDraft(reopened, button);
  const nextDraft = reopened.locator(COMPOSER).first();
  await reopened.waitForFunction(
    ({ target, value }) => document.querySelector(target)?.value === value,
    { target: COMPOSER, value: coldDraft },
  );
  assert.equal(await nextDraft.inputValue(), coldDraft);
  await openLayout(reopened, button);
  await button(reopened, "shell_draft").click();
  await reopened.waitForFunction(
    (target) => document.querySelector(target) === document.activeElement,
    COMPOSER,
  );

  const source = reopened.url();
  await reopened.locator(NAV).click();
  await button(reopened, "world_commits").click();
  await reopened.waitForFunction(
    () => document.activeElement?.id === "workspace-navigation-heading",
  );
  assert(await button(reopened, "shell_go_back").isVisible());
  await reopened.keyboard.press("Escape");
  await reopened.waitForFunction(
    (node) => node === document.activeElement,
    await button(reopened, "world_commits").elementHandle(),
  );
  assert(await button(reopened, "world_commits").isVisible());
  await reopened.keyboard.press("Escape");
  await reopened.waitForFunction(() => document.activeElement?.id === "workspace-navigation-key");
  assert.equal(reopened.url(), source);
  assert.equal(await nextDraft.inputValue(), coldDraft);
  return {
    file: NOTE,
    warmSelection: selected,
    editorIdentityRetained: true,
    composerIdentityRetained: true,
    undo: true,
    panoramaAndTransientBlend: true,
    coldSubject: "files",
    coldDividerSpan: Number(ratio),
    coldDraft,
    gitHistoryReturnPreservedSession: true,
  };
}
