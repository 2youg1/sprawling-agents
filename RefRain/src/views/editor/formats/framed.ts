// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const POLICY =
  "default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:; media-src data:; form-action 'none'";

export function framed(source: string, scheme: "authored" | "built"): string {
  const page = new DOMParser().parseFromString(source, "text/html");
  for (const element of page.querySelectorAll('meta[http-equiv="refresh" i], base'))
    element.remove();
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
