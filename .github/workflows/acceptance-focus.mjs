// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The keyboard walk through one native modal dialog, shared by the
// settings acceptance (acceptance-ui.mjs) and the privacy acceptance
// (privacy-ui.mjs); what it requires is written in
// tools/adversary/spec/Acceptance.lean. The modal judged is the topmost
// one: a dialog opened from inside another is later in document order,
// so the last `dialog:modal` is the one the keyboard is held in.

function advanceModalTraversal(state, sample) {
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

export async function observeModalFocus(page, dialog, focus) {
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
          modalOpen: element.isConnected && element.matches("dialog:modal") && [...document.querySelectorAll("dialog:modal")].at(-1) === element,
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
        advanceModalTraversal(state, sample);
      }
      traversal.visited = [...state.seen];
      traversal.complete = state.complete;
      if (!state.complete || state.chromePending) throw new Error(`${key} did not complete the modal traversal`);
    }
  } finally {
    await scope.dispose();
  }
}
