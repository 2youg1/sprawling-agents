// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import Path from "../../parts/path.solid";
import { GalleryCase } from "../case";

export default function PathCases() {
  return (
    <>
      <GalleryCase label="path · revealable, and opening in the page">
        <div class="flex flex-col gap-tight">
          <Path path="city/lab/notes.md" />
          <Path
            path="city/lab/west/experiments/report.md"
            base="city/lab"
            onOpen={() => undefined}
          />
        </div>
      </GalleryCase>
      <GalleryCase label="path · outside the city, in a narrow column" width={240}>
        <Path path={String.raw`D:\shared\experiments\2026\07\completions-with-a-long-name.md`} />
      </GalleryCase>
    </>
  );
}
