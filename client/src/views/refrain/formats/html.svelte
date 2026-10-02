<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // An HTML document's preview: its text drawn by this browser in a
  // sandboxed frame (client-SPEC 4-54). The text is the one the editor
  // held when the reading opened, the draft included, because this
  // browser draws HTML itself and needs no answer from the city; the
  // line above says whether a draft is in it; RefRain's head names the
  // version. HTML is text, so its diff
  // is RefRain's own between versions of that text.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Frame from "./frame.svelte";
  import Label from "./label.svelte";

  interface Props {
    readonly text: string;
    readonly drafted: boolean;
    readonly name: string;
  }

  const { text, drafted, name }: Props = $props();

  const lang = ui().lang;
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <Label
    format="html"
    tool={say($lang, "format_tool_browser")}
    settings={[say($lang, "format_html_settings")]}
    version={null}
    notes={drafted ? [say($lang, "format_from_draft")] : []}
  />
  <Frame page={text} scheme="authored" title={name} />
</div>
