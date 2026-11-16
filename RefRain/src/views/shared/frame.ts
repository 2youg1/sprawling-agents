// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type Columns = "one" | "twelve";

export function watchColumns(element: Element, onColumns: (columns: Columns) => void): () => void {
  const report = (): void => {
    onColumns(
      getComputedStyle(element).getPropertyValue("--shell-columns").trim() === "1"
        ? "one"
        : "twelve",
    );
  };
  report();
  const observer = new ResizeObserver(report);
  observer.observe(element);
  return () => {
    observer.disconnect();
  };
}

export function followViewport(frame: HTMLElement, viewport: VisualViewport): () => void {
  const follow = (): void => {
    frame.style.setProperty("--viewport-block", `${String(viewport.height * viewport.scale)}px`);
    frame.style.setProperty(
      "--viewport-top",
      `${String(viewport.scale > 1 ? 0 : viewport.offsetTop)}px`,
    );
  };
  follow();
  viewport.addEventListener("resize", follow);
  viewport.addEventListener("scroll", follow);
  return () => {
    viewport.removeEventListener("resize", follow);
    viewport.removeEventListener("scroll", follow);
    frame.style.removeProperty("--viewport-block");
    frame.style.removeProperty("--viewport-top");
  };
}
