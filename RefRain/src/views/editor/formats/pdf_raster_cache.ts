// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const RASTER_BYTES = 48 * 1024 ** 2;

export interface PdfRaster {
  readonly original: HTMLCanvasElement;
  readonly full: boolean;
}

function weight(raster: PdfRaster): number {
  return 4 * raster.original.width * raster.original.height;
}

export function releaseRaster(raster: PdfRaster): void {
  raster.original.width = raster.original.height = 0;
}

export class PdfRasterCache {
  private readonly entries = new Map<string, PdfRaster>();
  private bytes = 0;
  private closed = false;

  take(page: number, width: number): PdfRaster | undefined {
    const key = `${String(page)}:${String(width)}`;
    const raster = this.entries.get(key);
    if (raster !== undefined) {
      this.entries.delete(key);
      this.bytes -= weight(raster);
    }
    return raster;
  }

  keep(page: number, width: number, raster: PdfRaster): void {
    const size = weight(raster);
    if (this.closed || size > RASTER_BYTES) {
      releaseRaster(raster);
      return;
    }
    const prior = this.take(page, width);
    if (prior !== undefined) releaseRaster(prior);
    this.entries.set(`${String(page)}:${String(width)}`, raster);
    this.bytes += size;
    this.prune(false);
    this.prune(true);
  }

  private prune(full: boolean): void {
    for (const [key, oldest] of this.entries) {
      if (this.bytes <= RASTER_BYTES) break;
      if (oldest.full !== full) continue;
      this.entries.delete(key);
      this.bytes -= weight(oldest);
      releaseRaster(oldest);
    }
  }

  close(): void {
    this.closed = true;
    for (const raster of this.entries.values()) releaseRaster(raster);
    this.entries.clear();
    this.bytes = 0;
  }
}
