// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The page a sandboxed frame is given to draw (client-SPEC 4-54, 12-32).
// The frame's `sandbox` attribute already runs no script, submits no
// form, opens no window and gives the page an origin of its own; what
// it leaves open is reading and going elsewhere, and this closes both
// inside the page itself: a policy that loads nothing outside the page
// (only `data:` pictures and fonts, and the page's own styles), no
// automatic refresh, no base address pointing relative names at
// another server, and every link aimed at a new window the sandbox
// will not open. The page is parsed by the browser's own parser, which
// runs nothing while it parses, and written back with the doctype it
// came with, so a page written without one is still drawn the way a
// browser draws such a page.

const POLICY =
  "default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:; media-src data:; form-action 'none'";

// `scheme` says whose colours the page is drawn in: a page drawn as its
// author wrote it keeps the browser's default paper, and a page this
// client builds - a DOCX laid out as pages - takes the frame's scheme,
// so the space between its pages is the pane behind it.
export function framed(source: string, scheme: "authored" | "built"): string {
  const page = new DOMParser().parseFromString(source, "text/html");
  for (const element of page.querySelectorAll('meta[http-equiv="refresh" i], base')) element.remove();
  for (const element of page.querySelectorAll("[target]")) element.removeAttribute("target");
  const policy = page.createElement("meta");
  policy.httpEquiv = "Content-Security-Policy";
  policy.content = POLICY;
  const base = page.createElement("base");
  base.target = "_blank";
  page.head.prepend(policy, base);
  if (scheme === "built") {
    const colours = page.createElement("meta");
    colours.name = "color-scheme";
    colours.content = "light dark";
    page.head.append(colours);
  }
  const doctype = page.doctype === null ? "" : new XMLSerializer().serializeToString(page.doctype);
  return doctype + page.documentElement.outerHTML;
}
