// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { LinkState } from "./link";

export type Mark = "quiet" | "live" | "waiting" | "untold";

export interface Standing {
  readonly waiting: number;
  readonly working: boolean;
  readonly link: LinkState["kind"];
}

export function markOf(standing: Standing): Mark {
  if (standing.link !== "live") return "untold";
  if (standing.waiting > 0) return "waiting";
  return standing.working ? "live" : "quiet";
}

const TOKEN: Readonly<Record<Mark, string>> = {
  quiet: "--color-g5",
  live: "--color-accent",
  waiting: "--color-alert",
  untold: "--color-g5",
};

const FIGURE: Readonly<Record<Mark, (ink: string) => string>> = {
  quiet: (ink) => `<circle cx="16" cy="16" r="6" fill="none" stroke="${ink}" stroke-width="3"/>`,
  live: (ink) => `<circle cx="16" cy="16" r="7" fill="${ink}"/>`,
  waiting: (ink) => `<path d="M16 6 26 16 16 26 6 16Z" fill="${ink}"/>`,
  untold: (ink) => `<rect x="7" y="14" width="18" height="4" rx="2" fill="${ink}"/>`,
};

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

export function markSvg(mark: Mark, ground: string, ink: string): string {
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">` +
    `<rect width="32" height="32" rx="9" fill="${ground}"/>` +
    `${FIGURE[mark](ink)}</svg>`
  );
}
