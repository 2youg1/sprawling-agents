// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";

export function HomeSide(props: { readonly onHome: () => void }) {
  const lang = storeValue(ui().lang);
  return (
    <>
      <button
        data-control=""
        type="button"
        class="home-side col-start-1 row-start-2 narrow:hidden"
        aria-label={say(lang(), "page_home")}
        onClick={() => props.onHome()}
      />
      <button
        data-control=""
        type="button"
        class="home-side col-start-12 row-start-2 narrow:hidden"
        aria-label={say(lang(), "page_home")}
        onClick={() => props.onHome()}
      />
    </>
  );
}
