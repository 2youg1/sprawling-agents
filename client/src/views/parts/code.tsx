// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A file the way a tool handed it over: which file it was, which line
// each row is, and enough colour to tell a comment from a string.
//
// **Read-only by construction.** There is no editor here, no minimap
// and no language server: this view answers "what did that call
// produce", and every one of those three answers a question about
// editing a project instead.
//
// **The colouring knows words, never structure.** It walks a line and
// names what it passes - a comment, a string, a number, one of the
// words the family below lists - and it never builds a tree. A file
// whose extension is not in the table is drawn in one ink, which is
// the honest reading of "this build cannot tell". A tokenizer
// dependency would buy the difference for about two hundred kilobytes
// of a bundle that exists to show a few dozen lines at a time.

import { For, Show, createMemo } from "solid-js";

import { useSay } from "../../ui";

// What a stretch of a line is, as far as a reader without a grammar
// can tell. Four classes and a fifth for everything else, because the
// theme offers four inks a reader can hold apart at note size.
type Ink = "plain" | "comment" | "string" | "number" | "word";

// Every colour comes from `theme.css`; this file states no value.
const PAINT: Record<Ink, string> = {
  plain: "",
  comment: "text-text-disabled",
  string: "text-alert",
  number: "text-accent",
  word: "text-text",
};

interface Piece {
  readonly ink: Ink;
  readonly text: string;
}

// What one family of files is made of, lexically. Not a language
// definition: it is the shortest description under which a comment, a
// string and a keyword can be told apart on one line.
interface Syntax {
  // What opens a comment that runs to the end of the line.
  readonly line: string;
  // What opens and closes a comment that may cross lines, when the
  // family has one.
  readonly block: readonly [string, string] | null;
  // Every character that both opens and closes a string. A backslash
  // inside one escapes the character after it.
  readonly quotes: readonly string[];
  // The words drawn as words. Short on purpose: a reader scanning a
  // diff wants the shape of the line, and a complete keyword list for
  // six languages would be a table nobody maintains.
  readonly words: readonly string[];
}

const BRACES: Syntax = {
  line: "//",
  block: ["/*", "*/"],
  quotes: ['"', "'", "`"],
  words: [
    "async", "await", "break", "case", "const", "continue", "else", "enum",
    "export", "fn", "for", "func", "function", "if", "impl", "import",
    "interface", "let", "match", "mut", "pub", "return", "struct", "switch",
    "trait", "type", "use", "var", "while",
  ],
};

const HASHES: Syntax = {
  line: "#",
  block: null,
  quotes: ['"', "'"],
  words: [
    "async", "await", "break", "case", "class", "def", "elif", "else",
    "esac", "fi", "for", "from", "if", "import", "in", "lambda", "return",
    "then", "while", "with",
  ],
};

// Which family a file belongs to, by the last part of its name. A name
// this table does not hold is drawn without colour rather than guessed
// at, because a wrong comment marker hides a line instead of dimming
// it.
const SYNTAX = new Map<string, Syntax>([
  ["c", BRACES],
  ["cpp", BRACES],
  ["go", BRACES],
  ["h", BRACES],
  ["java", BRACES],
  ["js", BRACES],
  ["json", BRACES],
  ["jsx", BRACES],
  ["rs", BRACES],
  ["ts", BRACES],
  ["tsx", BRACES],
  ["zig", BRACES],
  ["bash", HASHES],
  ["py", HASHES],
  ["sh", HASHES],
  ["toml", HASHES],
  ["yaml", HASHES],
  ["yml", HASHES],
]);

const WORD_START = /[A-Za-z_$]/;
const WORD_PART = /[A-Za-z0-9_$]/;
const DIGIT = /[0-9]/;

// Where the file's name stops and its family begins. A name with no
// dot, or a dotfile whose only dot starts it, has no extension.
function extensionOf(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  return dot <= 0 ? "" : name.slice(dot + 1).toLowerCase();
}

// Where the string opened at `from` ends, one past its closing quote,
// or the end of the line when it does not close there. A string that
// runs past a line end is coloured on the line it opened on and no
// further: carrying that state would make a stray quote recolour the
// rest of a file, which is louder than the mistake it reports.
function closes(line: string, from: number, quote: string): number {
  let at = from + 1;
  while (at < line.length) {
    const here = line.charAt(at);
    if (here === "\\") {
      at += 2;
      continue;
    }
    if (here === quote) return at + 1;
    at += 1;
  }
  return line.length;
}

interface Scanned {
  readonly pieces: readonly Piece[];
  // Whether a block comment is still open when this line ends.
  readonly open: boolean;
}

function scan(line: string, syntax: Syntax, opened: boolean): Scanned {
  const pieces: Piece[] = [];
  let plain = "";
  let at = 0;
  let open = opened;
  const keep = (ink: Ink, text: string) => {
    if (plain !== "") {
      pieces.push({ ink: "plain", text: plain });
      plain = "";
    }
    pieces.push({ ink, text });
  };
  while (at < line.length) {
    const block = syntax.block;
    if (open && block !== null) {
      const ends = line.indexOf(block[1], at);
      const upto = ends === -1 ? line.length : ends + block[1].length;
      keep("comment", line.slice(at, upto));
      open = ends === -1;
      at = upto;
      continue;
    }
    if (block !== null && line.startsWith(block[0], at)) {
      open = true;
      continue;
    }
    if (line.startsWith(syntax.line, at)) {
      keep("comment", line.slice(at));
      at = line.length;
      continue;
    }
    const here = line.charAt(at);
    if (syntax.quotes.includes(here)) {
      const upto = closes(line, at, here);
      keep("string", line.slice(at, upto));
      at = upto;
      continue;
    }
    if (DIGIT.test(here) || WORD_START.test(here)) {
      let upto = at;
      while (upto < line.length && WORD_PART.test(line.charAt(upto))) upto += 1;
      const run = line.slice(at, upto);
      if (DIGIT.test(here)) {
        keep("number", run);
      } else if (syntax.words.includes(run)) {
        keep("word", run);
      } else {
        plain += run;
      }
      at = upto;
      continue;
    }
    plain += here;
    at += 1;
  }
  if (plain !== "") pieces.push({ ink: "plain", text: plain });
  return { pieces, open };
}

// The whole artifact as one run of pieces, newlines included, so the
// column that draws it is a flat list rather than a box per line.
function painted(text: string, syntax: Syntax | null): readonly Piece[] {
  if (syntax === null) return [{ ink: "plain", text }];
  const pieces: Piece[] = [];
  let open = false;
  const lines = text.split("\n");
  for (const [at, line] of lines.entries()) {
    const said = scan(line, syntax, open);
    pieces.push(...said.pieces);
    open = said.open;
    if (at < lines.length - 1) pieces.push({ ink: "plain", text: "\n" });
  }
  return pieces;
}

export interface CodeProps {
  // The file this came from, as the tool named it. An empty string
  // when the call named none: the trail is then not drawn and nothing
  // is coloured.
  readonly path: string;
  readonly text: string;
}

// Line numbers start at one because the wire hands over the head of a
// result and says nothing about where in the file it began. The day a
// call carries that offset, this becomes a prop rather than a fact
// decided here (client-SPEC 4-26).
export function Code(props: CodeProps) {
  const say = useSay();
  const pieces = createMemo(() => painted(props.text, SYNTAX.get(extensionOf(props.path)) ?? null));
  const trail = createMemo(() => props.path.split("/").filter((part) => part !== ""));
  const gutter = createMemo(() =>
    props.text
      .split("\n")
      .map((_line, at) => String(at + 1))
      .join("\n"),
  );
  return (
    <div class="flex min-h-0 min-w-0 flex-col">
      <Show when={trail().length > 0}>
        <nav
          class="flex flex-wrap items-center gap-tight border-b border-g2 px-snug py-tight text-note"
          aria-label={say("code_crumbs")}
        >
          <For each={trail()}>
            {(part, at) => (
              <>
                <Show when={at() > 0}>
                  <span class="text-text-disabled" aria-hidden="true">/</span>
                </Show>
                <span class={at() === trail().length - 1 ? "text-text-quiet" : "text-text-faint"}>
                  {part}
                </span>
              </>
            )}
          </For>
        </nav>
      </Show>
      <div class="min-h-0 flex-1 overflow-auto">
        <div class="flex min-w-max font-mono text-note leading-relaxed">
          <pre class="sticky left-0 shrink-0 select-none bg-g1 px-snug text-right text-text-disabled" aria-hidden="true">
            {gutter()}
          </pre>
          <pre class="px-snug text-text-quiet">
            <For each={pieces()}>{(piece) => <span class={PAINT[piece.ink]}>{piece.text}</span>}</For>
          </pre>
        </div>
      </div>
    </div>
  );
}
