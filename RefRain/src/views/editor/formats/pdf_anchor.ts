// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface PdfRegion {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export interface PdfReference {
  readonly page: number;
  readonly region: PdfRegion;
  readonly version: string;
}

export interface PdfProgress {
  readonly page: number;
  readonly offset: number;
}

export type PdfSelection = PdfReference &
  (
    | { readonly kind: "pdf-text"; readonly text: string }
    | { readonly kind: "pdf-region"; readonly image: Blob }
  );

export interface Point {
  readonly x: number;
  readonly y: number;
}

export function unit(value: number): number {
  return Math.min(1, Math.max(0, value));
}

export function pointAt(bounds: DOMRect, x: number, y: number): Point {
  return {
    x: unit((x - bounds.left) / bounds.width),
    y: unit((y - bounds.top) / bounds.height),
  };
}

export function regionBetween(first: Point, second: Point): PdfRegion {
  return {
    x: Math.min(first.x, second.x),
    y: Math.min(first.y, second.y),
    width: Math.abs(first.x - second.x),
    height: Math.abs(first.y - second.y),
  };
}

export function readableRegion(region: PdfRegion): boolean {
  return (
    Number.isFinite(region.x) &&
    Number.isFinite(region.y) &&
    region.x >= 0 &&
    region.y >= 0 &&
    region.width > 0 &&
    region.height > 0 &&
    region.x + region.width <= 1 &&
    region.y + region.height <= 1
  );
}

export function cropPixels(region: PdfRegion, width: number, height: number): PdfRegion | null {
  if (!readableRegion(region)) return null;
  const x = Math.ceil(region.x * width);
  const y = Math.ceil(region.y * height);
  const right = Math.floor((region.x + region.width) * width);
  const bottom = Math.floor((region.y + region.height) * height);
  if (right <= x || bottom <= y) return null;
  return { x, y, width: right - x, height: bottom - y };
}

export function textWithin(page: HTMLElement): { text: string; region: PdfRegion } | null {
  const selection = page.ownerDocument.getSelection();
  if (selection === null || selection.isCollapsed || selection.rangeCount !== 1) return null;
  const range = selection.getRangeAt(0);
  const layer = page.querySelector(".pdf-text-layer");
  if (!layer?.contains(range.startContainer) || !layer.contains(range.endContainer)) return null;
  const text = range.toString().trim();
  if (text === "") return null;
  const bounds = page.getBoundingClientRect();
  const picked = range.getBoundingClientRect();
  const region = regionBetween(
    pointAt(bounds, picked.left, picked.top),
    pointAt(bounds, picked.right, picked.bottom),
  );
  return readableRegion(region) ? { text, region } : null;
}

export async function cropPdf(canvas: HTMLCanvasElement, region: PdfRegion): Promise<Blob | null> {
  const area = cropPixels(region, canvas.width, canvas.height);
  if (area === null) return null;
  const output = canvas.ownerDocument.createElement("canvas");
  output.width = area.width;
  output.height = area.height;
  const context = output.getContext("2d");
  if (context === null) return null;
  context.drawImage(canvas, area.x, area.y, area.width, area.height, 0, 0, area.width, area.height);
  return new Promise((resolve) => {
    output.toBlob((image) => {
      output.width = 0;
      output.height = 0;
      resolve(image);
    }, "image/png");
  });
}

export function regionStyle(region: PdfRegion): string {
  return `--pdf-x: ${String(region.x * 100)}%; --pdf-y: ${String(region.y * 100)}%; --pdf-width: ${String(region.width * 100)}%; --pdf-height: ${String(region.height * 100)}%`;
}
