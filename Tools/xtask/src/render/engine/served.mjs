// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { mkdir } from "node:fs/promises";
import { join } from "node:path";

const POLL = 100;
const STARTUP = 25_000;
const DRAW = 35_000;

function clean(message) {
  return String(message).replace(/#open=[A-Za-z0-9_-]+/g, "#open=<redacted>");
}

async function until(read, budget, problem) {
  const deadline = performance.now() + budget;
  while (performance.now() < deadline) {
    const result = await read();
    if (result) return result;
    await Bun.sleep(POLL);
  }
  throw new Error(problem);
}

function connect(endpoint) {
  const address = new URL(endpoint);
  if (address.protocol !== "ws:" || address.hostname !== "127.0.0.1")
    throw new Error("Chromium did not advertise a loopback DevTools endpoint");
  const socket = new WebSocket(address);
  const outstanding = new Map();
  let serial = 0;
  const opened = new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", () => reject(new Error("cannot connect to Chromium")), {
      once: true,
    });
  });
  socket.addEventListener("message", (event) => {
    const frame = JSON.parse(event.data);
    const reply = outstanding.get(frame.id);
    if (!reply) return;
    outstanding.delete(frame.id);
    if (frame.error) reply.reject(new Error(frame.error.message));
    else reply.resolve(frame.result);
  });
  socket.addEventListener("close", () => {
    for (const reply of outstanding.values())
      reply.reject(new Error("Chromium closed its DevTools connection"));
    outstanding.clear();
  });
  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++serial;
      outstanding.set(id, { resolve, reject });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async function read(expression) {
    const frame = await send("Runtime.evaluate", { expression, returnByValue: true });
    if (frame.exceptionDetails)
      throw new Error(frame.exceptionDetails.exception?.description ?? frame.exceptionDetails.text);
    return frame.result.value;
  }
  return { opened, send, read, close: () => socket.close() };
}

async function run(input) {
  const admission = new URL((await Bun.file(input.entry).text()).trim());
  if (
    admission.protocol !== "http:" ||
    admission.hostname !== "127.0.0.1" ||
    !admission.hash.startsWith("#open=")
  )
    throw new Error("the render entry must be the one-use URL of a disposable loopback city");
  const target = new URL(admission.origin + "/#/" + input.route);
  const environment = { ...process.env };
  for (const [kind, key] of [
    ["config", "XDG_CONFIG_HOME"],
    ["data", "XDG_DATA_HOME"],
    ["cache", "XDG_CACHE_HOME"],
    ["runtime", "XDG_RUNTIME_DIR"],
  ]) {
    const dir = join(input.profile, kind);
    await mkdir(dir, { mode: 0o700 });
    environment[key] = dir;
  }
  const profile = join(input.profile, "browser");
  const browser = Bun.spawn(
    [
      input.browser,
      "--headless=new",
      "--no-sandbox",
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-background-networking",
      "--disable-component-update",
      "--disable-dev-shm-usage",
      "--disable-crash-reporter",
      "--disable-sync",
      "--hide-scrollbars",
      "--use-mock-keychain",
      "--remote-debugging-address=127.0.0.1",
      "--remote-debugging-port=0",
      "--user-data-dir=" + profile,
      "about:blank",
    ],
    { stdout: "ignore", stderr: "ignore", env: environment },
  );
  let page;
  try {
    const port = await until(
      async () => {
        if (browser.exitCode !== null) throw new Error("Chromium exited before exposing a page");
        const sent = await Bun.file(join(profile, "DevToolsActivePort"))
          .text()
          .catch(() => "");
        return Number(sent.split("\n")[0]);
      },
      STARTUP,
      "Chromium did not expose a DevTools port",
    );
    const site = await until(
      async () => {
        const sites = await (
          await fetch(`http://127.0.0.1:${port}/json/list`, {
            signal: AbortSignal.timeout(STARTUP),
          })
        ).json();
        return sites.find((target) => target.type === "page");
      },
      STARTUP,
      "Chromium did not expose a page",
    );
    page = connect(site.webSocketDebuggerUrl);
    await page.opened;
    await page.send("Page.enable");
    await page.send("Page.addScriptToEvaluateOnNewDocument", {
      source: `window.__renderProblems = [];
      window.addEventListener('error', event => __renderProblems.push(event.message || 'uncaught'));
      window.addEventListener('unhandledrejection', event => __renderProblems.push(event.reason?.stack || String(event.reason)));`,
    });
    const admit = await page.send("Page.navigate", { url: admission.href });
    if (admit.errorText) throw new Error(admit.errorText);
    await until(
      () =>
        page.read(`(() => {
          if (location.origin !== ${JSON.stringify(admission.origin)}) return false;
          const steady = location.hash !== '' && location.hash !== '#/welcome' &&
            !location.hash.startsWith('#open=');
          if (steady && (document.querySelector('#workspace-subject-key') ||
              document.querySelector('main textarea'))) return true;
          const defer = [...document.querySelectorAll('main button')].find(button =>
            ${JSON.stringify(input.defer)}.includes(button.textContent.trim()) && button.getClientRects().length > 0);
          if (defer) defer.click();
          return false;
        })()`),
      STARTUP,
      "the served client never reached the paired entrance",
    );

    const ready =
      input.route === "gallery"
        ? `!!document.querySelector('main section[aria-label^="kbd ·"]')`
        : "!!document.querySelector('main')";
    const results = [];
    for (const pass of input.passes) {
      await page.send("Emulation.setDeviceMetricsOverride", {
        width: pass.width,
        height: input.height,
        mobile: false,
        deviceScaleFactor: 1,
      });
      await page.send("Emulation.setEmulatedMedia", {
        features: [{ name: "forced-colors", value: pass.forced ? "active" : "none" }],
      });
      if (results.length === 0) {
        await page.read(`location.hash = ${JSON.stringify(target.hash)}`);
      } else {
        await page.send("Page.reload");
      }
      await until(
        () =>
          page.read(
            `location.hash === ${JSON.stringify(target.hash)} && innerWidth === ${pass.width} && ${ready}`,
          ),
        STARTUP,
        `the served fixture did not open at ${pass.width} CSS pixels`,
      );
      await page.read(`(() => {
        const script = document.createElement('template');
        script.innerHTML = ${JSON.stringify(pass.probe)};
        for (const item of script.content.children) {
          if (item.tagName !== 'SCRIPT') document.body.append(item.cloneNode(true));
          else new Function(item.textContent)();
        }
        return true;
      })()`);
      const snapshot = await until(
        () =>
          page.read(`(() => {
        const out = (id) => document.getElementById(id)?.innerHTML ?? '';
        const failed = out('sprawling-render-failed') || window.__renderProblems?.[0];
        if (failed) return { failed };
        const conditions = out('sprawling-render-conditions');
        return conditions ? { conditions, sink: out('sprawling-render'), declared: out('sprawling-render-declared') } : null;
      })()`),
        DRAW,
        `the render probe produced no reading at ${pass.width} CSS pixels`,
      );
      if (snapshot.failed)
        throw new Error(
          `the served page failed before the probe could measure it: ${snapshot.failed}`,
        );
      results.push(snapshot);
    }
    return { results };
  } finally {
    page?.close();
    if (browser.exitCode === null) browser.kill();
    await browser.exited;
  }
}

try {
  const input = await Bun.file(process.argv.at(-1)).json();
  console.log(JSON.stringify(await run(input)));
} catch (error) {
  console.log(JSON.stringify({ error: clean(error?.message ?? error) }));
}
