// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tab's icon, painted from the theme rather than shipped: a hidden
// tab shows whether the city is idle, working, or waiting for the
// person, in the same three colours the page uses. No colour is spelled
// here - each is read back from the stylesheet through an element, so
// the coloured tokens' chroma coefficient resolves the way it does on
// the page.

export type Mark = "quiet" | "live" | "waiting";

// What the page knows, reduced to the one word the tab can say.
export interface Standing {
  readonly waiting: number;
  readonly working: boolean;
}

export function markOf(standing: Standing): Mark {
  if (standing.waiting > 0) return "waiting";
  return standing.working ? "live" : "quiet";
}

const TOKEN: Readonly<Record<Mark, string>> = {
  quiet: "--color-g5",
  live: "--color-accent",
  waiting: "--color-alert",
};

// The resolved colour of one token, as the engine would paint it.
function resolved(doc: Document, token: string): string {
  const probe = doc.createElement("span");
  probe.style.color = `var(${token})`;
  doc.body.append(probe);
  const colour = getComputedStyle(probe).color;
  probe.remove();
  return colour;
}

export function paintMark(doc: Document, mark: Mark): void {
  const svg = markSvg(mark, resolved(doc, "--color-g2"), resolved(doc, TOKEN[mark]));
  let link = doc.querySelector<HTMLLinkElement>('link[rel="icon"]');
  if (link === null) {
    link = doc.createElement("link");
    link.rel = "icon";
    link.type = "image/svg+xml";
    doc.head.append(link);
  }
  link.href = `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

// The icon itself, given the two colours it is painted in.
export function markSvg(mark: Mark, ground: string, ink: string): string {
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">` +
    `<rect width="32" height="32" rx="9" fill="${ground}"/>` +
    `<circle cx="16" cy="16" r="7" fill="${ink}"/></svg>`
  );
}
