// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import assert from "node:assert/strict";

const SAMPLE_INTERVALS = 90;
const INPUT_BUDGET_MS = 1;
const INPUT_EVENTS = new Set([
  "click",
  "keydown",
  "pointerdown",
  "pointerup",
  "touchstart",
  "touchend",
  "input",
  "wheel",
]);

function summary(values) {
  assert(values.length > 0);
  const sorted = [...values].sort((a, b) => a - b);
  const quantile = (q) => sorted[Math.max(0, Math.ceil(q * sorted.length) - 1)];
  return {
    sampleCount: values.length,
    p50: quantile(0.5),
    p95: quantile(0.95),
    max: sorted.at(-1),
  };
}

export async function animationIntervals(page) {
  const observed = await page.evaluate(
    (count) =>
      new Promise((settled) => {
        const intervals = [];
        function sample(previous) {
          requestAnimationFrame((next) => {
            intervals.push(next - previous);
            if (intervals.length === count) settled(intervals);
            else sample(next);
          });
        }
        requestAnimationFrame(sample);
      }),
    SAMPLE_INTERVALS,
  );
  const frameMs = summary(observed);
  return { frameMs, medianCallbackHz: 1000 / frameMs.p50 };
}

export async function traceInput(cdp, actuate, { scriptOrigin } = {}) {
  const complete = new Promise((resolve) => cdp.once("Tracing.tracingComplete", resolve));
  await cdp.send("Tracing.start", {
    categories: "devtools.timeline",
    transferMode: "ReturnAsStream",
  });
  try {
    await actuate();
  } finally {
    await cdp.send("Tracing.end");
  }
  const { stream } = await complete;
  assert(stream, "DevTools provided a timeline stream");
  const parts = [];
  try {
    for (;;) {
      const next = await cdp.send("IO.read", { handle: stream });
      parts.push(next.base64Encoded ? Buffer.from(next.data, "base64").toString() : next.data);
      if (next.eof) break;
    }
  } finally {
    await cdp.send("IO.close", { handle: stream });
  }
  const events = JSON.parse(parts.join("")).traceEvents;
  const dispatches = events.filter((event) => {
    const type = event.args?.data?.type;
    return (
      event.name === "EventDispatch" &&
      event.ph === "X" &&
      typeof event.dur === "number" &&
      INPUT_EVENTS.has(type)
    );
  });
  const calls =
    scriptOrigin === undefined
      ? []
      : events.filter(
          (event) =>
            event.name === "FunctionCall" &&
            event.ph === "X" &&
            typeof event.dur === "number" &&
            event.args?.data?.url?.startsWith(`${scriptOrigin}/assets/`),
        );
  const samples = dispatches.map((dispatch) => {
    const sample = { type: dispatch.args.data.type, durationMs: dispatch.dur / 1000 };
    if (scriptOrigin === undefined) return sample;
    const stop = dispatch.ts + dispatch.dur;
    const within = calls
      .filter(
        (call) =>
          call.pid === dispatch.pid &&
          call.tid === dispatch.tid &&
          call.ts >= dispatch.ts &&
          call.ts + call.dur <= stop,
      )
      .sort((a, b) => a.ts - b.ts || b.dur - a.dur);
    let scripting = 0;
    let through = dispatch.ts;
    for (const call of within) {
      const until = call.ts + call.dur;
      scripting += Math.max(0, until - Math.max(through, call.ts));
      through = Math.max(through, until);
    }
    return {
      ...sample,
      applicationScriptingMs: scripting / 1000,
      applicationCalls: within.map((call) => {
        const data = call.args.data;
        return {
          file: new URL(data.url).pathname,
          function: data.functionName,
          line: data.lineNumber,
          column: data.columnNumber,
          durationMs: call.dur / 1000,
        };
      }),
    };
  });
  const eventDispatchMs = summary(samples.map((point) => point.durationMs));
  const overBudget = samples.filter((point) => point.durationMs > INPUT_BUDGET_MS);
  return { inputBudgetMs: INPUT_BUDGET_MS, eventDispatchMs, overBudget, samples };
}
