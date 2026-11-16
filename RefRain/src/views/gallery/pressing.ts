// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export function pressing(selector: string): (node: HTMLElement) => () => void {
  return (node) => {
    const seen = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      const button = node.querySelector<HTMLButtonElement>(selector);
      const held = button?.getAttribute("aria-pressed") ?? button?.getAttribute("aria-expanded");
      if (held !== "true") button?.click();
    });
    seen.observe(node);
    return () => {
      seen.disconnect();
    };
  };
}
