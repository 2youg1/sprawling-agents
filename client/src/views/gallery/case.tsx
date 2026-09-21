// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One fixture, under the words naming the state it is in.
//
// Every fixture on this route is one of these, so the name is drawn
// once and announced once: the label is the caption a person reads and
// the region name `xtask render` reads, and two fixtures that cannot be
// told apart by name are two unnamed regions in a screen reader's jump
// list.
//
// **The label is English written at the call site, not a phrase from
// `lang.json`.** This route is an instrument for the people who build
// the client, and the name of a state here is the name the source gives
// that state; a translated caption would be a second spelling of it.
//
// It lives in a file of its own rather than in the index beside it
// because every section imports it and the index imports every section:
// putting it in the index would close that ring.

import type { JSX } from "solid-js";

export function Case(props: { readonly label: string; readonly children: JSX.Element }) {
  return (
    <section class="mb-wide" aria-label={props.label}>
      <div class="mb-tight text-note text-text-disabled">{props.label}</div>
      {props.children}
    </section>
  );
}
