// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { say } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { galleryUi, ui, UiScope } from "../ui";
import { PRODUCT } from "./gallery/cases_product";
import { CONTROLS } from "./gallery/cases_controls";

const FIXTURES = [...PRODUCT, ...CONTROLS] as const;

function fixtureLink(event: MouseEvent): void {
  if (event.target instanceof Element && event.target.closest('a[href^="#/"]') !== null) {
    event.preventDefault();
  }
}

export default function Gallery() {
  const real = ui();
  const lang = storeValue(real.lang);
  const fixture = galleryUi(real);
  return (
    <UiScope value={fixture}>
      <div class="w-full min-w-0 px-pane py-pane" onClick={fixtureLink}>
        <h1 class="mb-wide text-heading text-text">{say(lang(), "gallery_title")}</h1>
        <For each={FIXTURES}>{(Fixture) => <Fixture />}</For>
      </div>
    </UiScope>
  );
}
