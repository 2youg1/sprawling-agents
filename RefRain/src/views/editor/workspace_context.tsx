// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";

import { say, type Lang } from "../../core/lang";
import { createContextTools, type ContextTool } from "../parts/context_tools";
import type { ContextSelection, Editing, TextRange } from "./editing";
import { QUOTE_FORMAT } from "./markdown_editing";
import { workspaceSelectionTools } from "./workspace_document_tools";
import { WorkspaceIndent, WorkspaceUndo } from "./workspace_history";
import { WorkspaceSearch } from "./workspace_search";
import "./workspace_edit_panel.css";

interface Commands {
  readonly lang: () => Lang;
  readonly editing: () => Editing | null;
  readonly revision: () => number;
  readonly save: () => void;
  readonly canSave: () => boolean;
  readonly attach: ((range: TextRange) => void) | undefined;
  readonly properties: (() => JSX.Element) | undefined;
}

export function createWritingTools(options: Commands) {
  const controller = createContextTools();
  const restore = () => options.editing()?.focus();

  function search(editor: Editing): ContextTool {
    return {
      id: "search",
      glyph: "search",
      label: say(options.lang(), "writing_search"),
      shortcut: { accel: true, shift: false, key: "f" },
      panel: () => <WorkspaceSearch view={editor.view} lang={options.lang()} />,
    };
  }

  return {
    controller,
    find(anchor: { readonly x: number; readonly y: number }): boolean {
      const editor = options.editing();
      if (editor === null) return false;
      const tool = search(editor);
      const bounds = editor.view.scrollDOM.getBoundingClientRect();
      return controller.open(
        { tools: [tool], panel: tool.panel, label: tool.label, restore },
        { kind: "keyboard", x: Math.max(anchor.x, bounds.right), y: bounds.top },
      );
    },
    context({ range, event, x, y }: ContextSelection): void {
      const editor = options.editing();
      if (editor === null) return;
      const lang = options.lang();
      const readonly = () => editor.view.state.readOnly;
      const tools: ContextTool[] = [
        {
          id: "undo",
          label: say(lang, "writing_undo"),
          glyph: "refresh",
          openOnHover: true,
          disabled: readonly,
          panel: () => (
            <WorkspaceUndo view={editor.view} lang={options.lang()} revision={options.revision} />
          ),
        },
        search(editor),
        {
          id: "save",
          label: say(lang, "refrain_save"),
          glyph: "check",
          disabled: () => !options.canSave(),
          shortcut: { accel: true, shift: false, key: "s" },
          run: options.save,
        },
        {
          id: "indent",
          label: say(lang, "writing_complete_indent"),
          glyph: "record",
          disabled: readonly,
          panel: (controls) => (
            <WorkspaceIndent
              view={editor.view}
              lang={options.lang()}
              revision={options.revision}
              done={controls.close}
            />
          ),
        },
      ];
      const properties = options.properties;
      if (properties !== undefined)
        tools.push({
          id: "properties",
          label: say(lang, "shell_file_details"),
          glyph: "record",
          panel: properties,
        });
      if (range !== null)
        tools.push(
          ...workspaceSelectionTools(
            lang,
            readonly() ? undefined : editor.format,
            options.attach === undefined ? undefined : () => options.attach?.(range),
          ),
        );
      else
        tools.push({
          id: QUOTE_FORMAT.id,
          label: say(lang, QUOTE_FORMAT.label),
          disabled: readonly,
          run: () => editor.format(QUOTE_FORMAT),
        });
      const kind =
        event instanceof KeyboardEvent
          ? "keyboard"
          : event instanceof PointerEvent && event.pointerType === "touch"
            ? "touch"
            : "pointer";
      controller.open({ tools, restore }, { kind, event, x, y });
    },
  };
}
