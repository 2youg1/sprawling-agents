// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";
import { join } from "node:path";
import { DevTools, type Frame, type Outcome } from "./walk_devtools";

const NAV = "#workspace-navigation-key";
const MODE = "button.refrain-key[aria-pressed]";
const CANVAS = "#workspace-file-canvas";
const PAPER = `${CANVAS} .pdf-scroll`;
const ENGINES = [
  { name: "wide", width: 1280, height: 830, dpr: 1, touch: false },
  { name: "narrow", width: 390, height: 844, dpr: 1, touch: true },
] satisfies readonly Frame[];

function answer<T>(received: Outcome<T>, step: string): T {
  assert(received.ok, received.ok ? "" : `${step}: ${received.why}`);
  return received.value;
}

function locate(selector: string, label?: string): string {
  const scoped = `Array.from(document.querySelectorAll(${JSON.stringify(selector)}))`;
  return label === undefined
    ? `${scoped}[0]`
    : `${scoped}.find(node => node.textContent.trim() === ${JSON.stringify(label)})`;
}

async function ready(browser: DevTools, element: string): Promise<void> {
  const result = await browser.run(`(async () => {
    const end = performance.now() + 30_000;
    while (performance.now() < end) {
      const node = ${element};
      if (node?.getBoundingClientRect().height > 0 && node.checkVisibility({ visibilityProperty: true })) return true;
      await new Promise(requestAnimationFrame);
    }
    return false;
  })()`);
  assert.equal(answer(result, "wait for a visible control"), true, element);
}

async function coordinates(browser: DevTools, element: string): Promise<{ x: number; y: number }> {
  await ready(browser, element);
  const position = answer(
    await browser.run(`(() => {
    const node = ${element};
    node.scrollIntoView({ block: "nearest", inline: "nearest" });
    const rect = node.getBoundingClientRect();
    return {x: rect.x + rect.width / 2, y: rect.y + rect.height / 2};
  })()`),
    "locate control",
  );
  assert(typeof position === "object" && position !== null);
  assert("x" in position && typeof position.x === "number");
  assert("y" in position && typeof position.y === "number");
  return { x: position.x, y: position.y };
}

async function press(browser: DevTools, element: string, touch: boolean): Promise<void> {
  await browser.settle(0);
  const at = await coordinates(browser, element);
  if (touch) {
    answer(
      await browser.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [at] }),
      "touch down",
    );
    answer(
      await browser.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] }),
      "touch up",
    );
  } else {
    answer(
      await browser.send("Input.dispatchMouseEvent", {
        type: "mousePressed",
        button: "left",
        clickCount: 1,
        ...at,
      }),
      "pointer down",
    );
    answer(
      await browser.send("Input.dispatchMouseEvent", {
        type: "mouseReleased",
        button: "left",
        clickCount: 1,
        ...at,
      }),
      "pointer up",
    );
  }
}

function visiblePage(): string {
  return `(() => {
    const view = document.querySelector(${JSON.stringify(PAPER)});
    if (!view) return null;
    const bounds = view.getBoundingClientRect();
    const painted = [...view.querySelectorAll("canvas.pdf-original")];
    let covered = 0;
    for (const canvas of painted) {
      const box = canvas.getBoundingClientRect();
      covered += Math.max(0, Math.min(box.bottom, bounds.bottom) - Math.max(box.top, bounds.top));
    }
    return covered >= bounds.height / 2 ? painted[0] : null;
  })()`;
}

async function readFrames(browser: DevTools, touch: boolean): Promise<unknown> {
  const paper = locate(PAPER);
  answer(
    await browser.run(`(() => {
    const node = ${paper};
    const panel = document.getElementById("workspace-navigation");
    const snapshot = () => {
      const {x, y, width, height} = node.getBoundingClientRect();
      const visible = getComputedStyle(panel);
      return { geometry: [x, y, width, height, node.scrollTop], opacity: Number(visible.opacity), translate: visible.translate, at: performance.now() };
    };
    const initial = snapshot();
    window.workspaceFrames = new Promise(resolve => {
      const captured = [initial];
      function sample() {
        const frame = snapshot();
        captured.push(frame);
        const settled = panel.getAttribute("aria-hidden") === "false" && frame.opacity === 1;
        if (settled || performance.now() - initial.at >= 2500) resolve(captured);
        else requestAnimationFrame(sample);
      }
      requestAnimationFrame(sample);
    });
  })()`),
    "start collecting transition frames",
  );
  await press(browser, locate(NAV), touch);
  const frames = answer(await browser.run("window.workspaceFrames"), "capture transition frames");
  assert(Array.isArray(frames) && frames.length >= 3);
  const first: unknown = frames[0];
  const last: unknown = frames.at(-1);
  assert(typeof first === "object" && first !== null && "geometry" in first);
  assert(typeof last === "object" && last !== null && "opacity" in last);
  assert.equal(last.opacity, 1, "the navigation should be visible on the final frame");
  for (const frame of frames) {
    const observed: unknown = frame;
    assert(typeof observed === "object" && observed !== null && "geometry" in observed);
    assert.deepEqual(observed.geometry, first.geometry);
  }
  return frames;
}

async function exercise(
  browser: DevTools,
  launch: string,
  folder: string,
  file: string,
  output: string,
  fileLink: string | undefined,
): Promise<void> {
  const wide = ENGINES[0];
  assert(wide !== undefined);
  await browser.frame(wide, "light");
  await browser.load(launch, 1000);
  const skip = locate("button", "put off the optional steps");
  if (!new URL(launch).hash.includes("workspace=")) {
    await ready(browser, `(${skip}) ?? (${locate(MODE, "File mode")})`);
  }
  if (answer(await browser.run(`Boolean(${skip})`), "check first visit"))
    await press(browser, skip, false);
  if (fileLink !== undefined) {
    const linked = new URL(fileLink);
    assert.equal(linked.origin, new URL(launch).origin);
    answer(
      await browser.run(`location.hash = ${JSON.stringify(linked.hash)}`),
      "follow registered file link",
    );
  } else if (!new URL(launch).hash.includes("workspace=")) {
    if (
      !answer(
        await browser.run('Boolean(document.querySelector("[data-subject=files]"))'),
        "check file mode",
      )
    ) {
      await press(browser, locate(MODE, "File mode"), false);
    }
    await press(browser, locate(NAV), false);
    const chosen = locate("#workspace-navigation [data-file-index]", file);
    await browser.settle(350);
    if (!answer(await browser.run(`Boolean(${chosen})`), "check registered workspace")) {
      const input = locate("#workspace-navigation input[required]");
      const detail = locate("#workspace-navigation details summary", "Add workspace");
      if (!answer(await browser.run(`(${input})?.checkVisibility()`), "find folder path")) {
        await press(browser, detail, false);
      }
      await ready(browser, input);
      answer(await browser.run(`(${input}).focus()`), "focus folder path");
      await browser.type(folder);
      await press(
        browser,
        locate("#workspace-navigation form button[type=submit]", "Add workspace"),
        false,
      );
    }
    await press(browser, chosen, false);
  }
  await ready(browser, visiblePage());

  const reports = [];
  for (const viewport of ENGINES) {
    await browser.frame(viewport, "light");
    await browser.settle(100);
    answer(await browser.run(`(${locate(PAPER)}).focus()`), "focus PDF scroll area");
    await browser.raw("Home", "Home", 36);
    await browser.settle(100);
    const at = await coordinates(browser, locate(PAPER));
    answer(
      await browser.send("Input.dispatchMouseEvent", {
        type: "mouseWheel",
        deltaX: 0,
        deltaY: 640,
        ...at,
      }),
      "scroll paper",
    );
    await browser.settle(100);
    await ready(browser, visiblePage());
    const frames = await readFrames(browser, viewport.touch);
    answer(
      await browser.keep(join(output, `workspace-${viewport.name}.png`)),
      "capture screenshot",
    );
    await browser.raw("Escape", "Escape", 27);
    assert.equal(
      answer(
        await browser.run(`(${locate(NAV)}) === document.activeElement`),
        "restore navigation focus",
      ),
      true,
    );
    await press(browser, locate(MODE, "File mode"), viewport.touch);
    await press(browser, locate(MODE, "File mode"), viewport.touch);
    await ready(browser, locate(PAPER));
    reports.push({ viewport, frames });
  }
  await Bun.write(join(output, "workspace-frames.json"), JSON.stringify(reports, null, 2) + "\n");
}

const [redirect, folder, file, output, rawPort, fileLink] = Bun.argv.slice(2);
assert(
  redirect && folder && file && output && rawPort,
  "Usage: bun scripts/workspace_revisit.ts REDIRECT_OR_URL WORKSPACE_ROOT PDF_NAME OUTPUT_DIRECTORY DEVTOOLS_PORT [FILE_LINK]",
);
const redirection = URL.canParse(redirect) ? redirect : await Bun.file(redirect).text();
const launch = URL.canParse(redirect)
  ? redirect
  : /https?:[^ "]+#open=[^ "]+/.exec(redirection)?.[0];
assert(launch !== undefined, "Use the private redirect from the running city's trusted launch.");
const browser = answer(
  await DevTools.open(join(output, "browser"), Number(rawPort)),
  "start browser",
);
await exercise(browser, launch, folder, file, output, fileLink)
  .catch(async (error: unknown) => {
    await browser.keep(join(output, "workspace-failure.png"));
    return Promise.reject(error instanceof Error ? error : new Error(String(error)));
  })
  .finally(() => browser.close());
