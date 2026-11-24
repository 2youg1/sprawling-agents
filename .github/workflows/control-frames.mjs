// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";

function painted(control) {
  return control.evaluate(
    (node) =>
      new Promise((read) =>
        requestAnimationFrame((at) => {
          const css = getComputedStyle(node);
          read({
            at,
            hover: node.matches(":hover"),
            active: node.matches(":active"),
            holding: node.hasAttribute("data-control-holding"),
            keydown: node.hasAttribute("data-control-keydown"),
            focused: node === document.activeElement,
            pressed: node.getAttribute("aria-pressed"),
            selected: node.getAttribute("aria-selected") ?? node.getAttribute("aria-checked"),
            expanded: node.getAttribute("aria-expanded"),
            busy: node.getAttribute("aria-busy"),
            refused: node.getAttribute("aria-disabled"),
            state: node.getAttribute("data-control-state"),
            scale: css.scale,
            duration: css.transitionDuration,
            background: css.backgroundColor,
            color: css.color,
            outline: css.outline,
            outlineOffset: css.outlineOffset,
            shadow: css.boxShadow,
            glyph: node.querySelector("svg")?.innerHTML ?? null,
          });
        }),
      ),
  );
}

function present(value) {
  return [value.pressed, value.selected, value.expanded, value.state];
}

export async function pressAndCancel(page, control, { held, canHold = false } = {}) {
  await control.scrollIntoViewIfNeeded();
  const area = await control.boundingBox();
  const viewport = page.viewportSize();
  assert.ok(area && viewport);
  const x = area.x + area.width / 2;
  const y = area.y + area.height / 2;
  const away =
    area.y >= 14 ? area.y - 12 : Math.min(viewport.height - 1, area.y + area.height + 12);
  await page.mouse.move(x, away);
  const rest = await painted(control);
  await page.mouse.move(x, y);
  const hover = await painted(control);
  await page.mouse.down();
  try {
    const down = await painted(control);
    await held?.();
    const holding = await painted(control);
    await page.mouse.move(x, away, { steps: 4 });
    const canceled = await painted(control);
    await page.mouse.up();
    const up = await painted(control);
    await control.evaluate(async (node) => {
      await Promise.allSettled(
        node
          .getAnimations()
          .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
          .map((animation) => animation.finished),
      );
    });
    const settled = await painted(control);
    assert.ok(hover.hover && down.active && holding.active && !up.active);
    assert.deepEqual(present(up), present(rest));
    assert.deepEqual(present(settled), present(rest));
    if (canHold) {
      assert.ok(holding.holding);
      assert.ok(!canceled.holding && !up.holding);
    }
    return { rest, hover, down, holding, canceled, up, settled };
  } catch (error) {
    await page.mouse.up();
    throw error;
  }
}

export async function spaceAndCancel(page, control) {
  await control.focus();
  const rest = await painted(control);
  await page.keyboard.down("Space");
  let held;
  let canceled;
  try {
    held = await painted(control);
    assert.ok(held.keydown && held.focused);
    await control.evaluate((node) => node.blur());
    canceled = await painted(control);
    assert.ok(!canceled.keydown);
  } finally {
    await page.keyboard.up("Space");
  }
  const up = await painted(control);
  assert.ok(!up.keydown);
  assert.deepEqual(present(up), present(rest));
  return { rest, held, canceled, up };
}

export async function controlInventory(page) {
  return page.locator("main#main").evaluate((main) =>
    [...main.querySelectorAll('button, a[href], input, select, [role="tab"], [role="menuitem"]')]
      .filter((node) => node.getClientRects().length > 0)
      .map((node) => ({
        specimen: node.closest("section[aria-label]")?.getAttribute("aria-label") ?? null,
        label: node.getAttribute("aria-label") ?? node.textContent?.trim() ?? null,
        role: node.getAttribute("role") ?? node.localName,
        disabled: node.matches(':disabled, [aria-disabled="true"]'),
        contract: node.hasAttribute("data-control"),
        glyph: node.querySelector("svg") !== null,
      })),
  );
}
