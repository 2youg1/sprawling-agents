// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { WidgetType } from "@codemirror/view";
import type { EditorView } from "@codemirror/view";
import type { SyntaxNode } from "@lezer/common";
import { drawMath, formulaOf } from "./formats/math";
import { drawMermaid } from "./formats/mermaid";

export type PreviewResource = (relative: string) => Promise<{ bytes: Uint8Array; name: string }>;

const RASTER = /^data:image\/(?:png|jpeg|gif|webp|avif);base64,[a-z\d+/=]+$/iu;
const RELATIVE = /^(?![\s/#])[^:\\\p{Cc}?#]+$/u;
const WEB = /^(?:https?|mailto):[^\p{Cc}\s]/iu;

export function webLink(text: string): string | null {
  return WEB.test(text) ? text : null;
}

export function imageType(name: string): string | null {
  const extension = name.toLowerCase().split(".").at(-1) ?? "";
  switch (extension) {
    case "apng":
    case "avif":
    case "bmp":
    case "gif":
    case "png":
    case "webp":
      return `image/${extension}`;
    case "jpg":
    case "jpeg":
      return "image/jpeg";
    default:
      return null;
  }
}

export function diagramUrl(svg: string): string {
  return URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
}

export class MathWidget extends WidgetType {
  constructor(readonly source: string) {
    super();
  }

  override eq(other: MathWidget): boolean {
    return this.source === other.source;
  }

  toDOM(): HTMLElement {
    const formula = formulaOf(this.source);
    const dom = document.createElement("span");
    dom.className = "cm-markdown-formula";
    dom.textContent = this.source;
    if (formula !== null) {
      dom.classList.toggle("cm-markdown-display", formula.placed === "display");
      const equation = document.createElement("span");
      void drawMath(formula.tex, equation, formula.placed, () => dom.isConnected).then(
        (ok) => {
          if (ok) dom.replaceChildren(equation);
        },
        () => undefined,
      );
    }
    return dom;
  }
}

export class ImageWidget extends WidgetType {
  constructor(
    readonly source: string,
    readonly alt: string,
    readonly read?: PreviewResource,
  ) {
    super();
  }

  override eq(other: ImageWidget): boolean {
    return this.source === other.source && this.alt === other.alt && this.read === other.read;
  }

  toDOM(): HTMLElement {
    const dom = document.createElement("span");
    dom.className = "cm-markdown-image";
    dom.textContent = this.alt;
    const image = dom.ownerDocument.createElement("img");
    image.alt = this.alt;
    image.addEventListener(
      "error",
      () => {
        dom.textContent = this.alt;
      },
      { once: true },
    );
    if (RASTER.test(this.source)) {
      image.src = this.source;
      dom.replaceChildren(image);
    } else if (this.read !== undefined && RELATIVE.test(this.source)) {
      void this.read(this.source).then(
        ({ bytes, name }) => {
          const type = imageType(name);
          if (type === null || !dom.isConnected) return;
          const url = URL.createObjectURL(new Blob([bytes.slice()], { type }));
          image.addEventListener(
            "load",
            () => {
              URL.revokeObjectURL(url);
            },
            { once: true },
          );
          image.addEventListener(
            "error",
            () => {
              URL.revokeObjectURL(url);
            },
            { once: true },
          );
          image.src = url;
          dom.replaceChildren(image);
        },
        () => undefined,
      );
    }
    return dom;
  }
}

export class TableWidget extends WidgetType {
  constructor(
    readonly rows: readonly (readonly string[])[],
    readonly source: string,
  ) {
    super();
  }

  override eq(other: TableWidget): boolean {
    return this.source === other.source;
  }

  override get estimatedHeight(): number {
    return this.rows.length * 32;
  }

  toDOM(): HTMLElement {
    const dom = document.createElement("div");
    dom.className = "cm-markdown-table";
    const table = document.createElement("table");
    dom.append(table);
    const head = table.createTHead();
    const body = table.createTBody();
    this.rows.forEach((cells, row) => {
      const line = (row === 0 ? head : body).insertRow();
      for (const cell of cells) {
        const element = document.createElement(row === 0 ? "th" : "td");
        element.textContent = cell;
        line.append(element);
      }
    });
    return dom;
  }

  override ignoreEvent(): boolean {
    return false;
  }
}

export class DiagramWidget extends WidgetType {
  constructor(
    readonly source: string,
    readonly alt: string,
  ) {
    super();
  }

  override eq(other: DiagramWidget): boolean {
    return this.source === other.source && this.alt === other.alt;
  }

  toDOM(view: EditorView): HTMLElement {
    const dom = document.createElement("div");
    dom.className = "cm-markdown-diagram";
    dom.textContent = this.source;
    void drawMermaid(this.source).then((svg) => {
      if (svg === null || !dom.isConnected) return;
      const image = document.createElement("img");
      image.alt = this.alt;
      const url = diagramUrl(svg);
      image.addEventListener(
        "load",
        () => {
          URL.revokeObjectURL(url);
          if (dom.isConnected) {
            dom.replaceChildren(image);
            view.requestMeasure();
          }
        },
        { once: true },
      );
      image.addEventListener(
        "error",
        () => {
          URL.revokeObjectURL(url);
        },
        { once: true },
      );
      image.src = url;
    });
    return dom;
  }

  override ignoreEvent(): boolean {
    return false;
  }
}

export function* children(node: SyntaxNode): Iterable<SyntaxNode> {
  for (let child = node.firstChild; child !== null; child = child.nextSibling) yield child;
}
