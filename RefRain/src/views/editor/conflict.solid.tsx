// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Button from "../parts/button.solid";
import Dialog from "../parts/dialog_view";
import Look from "./conflict.look.solid";
import { short } from "./reading";
import type { Session } from "./session";

interface ConflictProps {
  readonly name: string;
  readonly session: Session;
  readonly onCompare: () => void;
}

export default function Conflict(props: ConflictProps) {
  const lang = storeValue(ui().lang);
  const state = storeValue(() => props.session.state);
  const [asking, setAsking] = createSignal(false);
  const waiting = () =>
    state()?.theirs == null ? { why: say(lang(), "refrain_conflict_unknown") } : {};
  const text = () => {
    const theirs = state()?.theirs;
    return theirs == null
      ? say(lang(), "refrain_conflict_unknown")
      : fill(say(lang(), "refrain_conflict"), { version: short(theirs.version) });
  };
  return (
    <>
      <Look
        text={text()}
        wire={{ role: "alert" }}
        actions={
          <>
            <Button
              label={say(lang(), "refrain_compare")}
              tone="quiet"
              glyph="search"
              {...waiting()}
              onPress={props.onCompare}
            />
            <Button
              label={say(lang(), "refrain_rebase")}
              tone="secondary"
              glyph="branch"
              {...waiting()}
              onPress={() => props.session.rebase()}
            />
            <Button
              label={say(lang(), "refrain_discard")}
              tone="destructive"
              glyph="cross"
              {...waiting()}
              onPress={() => {
                setAsking(true);
              }}
            />
          </>
        }
      />
      <Dialog
        open={asking()}
        title={fill(say(lang(), "refrain_discard_title"), { name: props.name })}
        detail={say(lang(), "refrain_discard_detail")}
        confirmLabel={say(lang(), "refrain_discard")}
        cancelLabel={say(lang(), "part_cancel")}
        destructive
        onConfirm={() => {
          setAsking(false);
          props.session.discard();
        }}
        onCancel={() => {
          setAsking(false);
        }}
      />
    </>
  );
}
