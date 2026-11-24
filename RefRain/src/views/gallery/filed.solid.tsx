// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RegistryLine } from "../../wire";
import { Address, TimeMs } from "../../wire";
import { RegistryTable } from "../registry.solid";
import { GalleryCase } from "./case";

export default function RegistryCases() {
  function filed(at: number, addr: string, kind: string, subject: string): RegistryLine {
    return { addr: Address.make(addr), at: TimeMs.make(at), kind, subject };
  }
  const ASSETS: readonly RegistryLine[] = [
    filed(
      1_764_500_400_000,
      "lab/east",
      "screenshot",
      "the settings page at 2560 px, forced colours",
    ),
    filed(
      1_764_412_800_000,
      "hall/mayor",
      "transcript",
      "what the mayor was asked on the first day",
    ),
    filed(1_764_586_800_000, "lab/west", "release", "sprawling 0.0.6, windows-x86_64"),
  ];
  return (
    <>
      <GalleryCase label={"registry · three assets, handed over out of order"}>
        <RegistryTable assets={ASSETS} />
      </GalleryCase>
      <GalleryCase label={"registry · nothing has been filed"}>
        <RegistryTable assets={[]} />
      </GalleryCase>
      <GalleryCase label={"registry · the same three rows, compact"}>
        <div data-density={"compact"}>
          <RegistryTable assets={ASSETS} />
        </div>
      </GalleryCase>
    </>
  );
}
