// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const CONTROLLED = /^\s*(?:---|%%\s*\{)/mu;
const LIMIT = 25_000;
let nextId = 0;
let pending: Promise<unknown> = Promise.resolve();

export function drawMermaid(source: string): Promise<string | null> {
  if (source.length > LIMIT || CONTROLLED.test(source)) return Promise.resolve(null);
  const render = pending.then(async () => {
    const { default: mermaid } = await import("mermaid");
    mermaid.initialize({
      securityLevel: "strict",
      startOnLoad: false,
      htmlLabels: false,
      maxTextSize: LIMIT,
      maxEdges: 500,
      suppressErrorRendering: true,
    });
    const id = `refrain-diagram-${String(++nextId)}`;
    const { svg } = await mermaid.render(id, source);
    return svg;
  });
  pending = render.catch(() => null);
  return render.catch(() => null);
}
