// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { neutralTones } from "./pdf_tones";

globalThis.addEventListener(
  "message",
  (event: MessageEvent<{ bitmap: ImageBitmap; scan: boolean }>) => {
    const { bitmap, scan } = event.data;
    const raster = new OffscreenCanvas(bitmap.width, bitmap.height);
    const context = raster.getContext("2d");
    if (context === null) {
      bitmap.close();
      globalThis.postMessage(null);
      return;
    }
    context.drawImage(bitmap, 0, 0);
    bitmap.close();
    const pixels = context.getImageData(0, 0, raster.width, raster.height);
    neutralTones(pixels, scan);
    context.putImageData(pixels, 0, 0);
    const tones = raster.transferToImageBitmap();
    globalThis.postMessage(tones, { transfer: [tones] });
  },
);
