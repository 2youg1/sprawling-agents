// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const spoken = new Set([
  "title",
  "alt",
  "placeholder",
  "aria-label",
  "aria-description",
  "aria-placeholder",
  "aria-roledescription",
  "aria-valuetext",
]);

export default {
  meta: { name: "refrain" },
  rules: {
    "effect-failure": {
      meta: {
        type: "problem",
        schema: [],
        messages: {
          throw: "Effect owns failure: return an Effect or an Either.",
          try: "Effect owns failure: use Effect.try or Effect.tryPromise.",
        },
      },
      create(context) {
        return {
          ThrowStatement(node) {
            context.report({ node, messageId: "throw" });
          },
          TryStatement(node) {
            context.report({ node, messageId: "try" });
          },
        };
      },
    },
    wording: {
      meta: {
        type: "problem",
        schema: [],
        messages: { literal: "Source reader-facing text from the phrase table or the document." },
      },
      create(context) {
        function report(node, value) {
          if (/\p{Script=Han}|\p{L}{2}/u.test(value)) {
            context.report({ node, messageId: "literal" });
          }
        }
        function literal(node) {
          if (!node) return;
          if (node.type === "Literal" && typeof node.value === "string") report(node, node.value);
          if (node.type === "TemplateLiteral") {
            for (const part of node.quasis) report(part, part.value.cooked ?? "");
          }
          if (node.type === "ConditionalExpression") {
            literal(node.consequent);
            literal(node.alternate);
          }
        }
        return {
          JSXText(node) {
            report(node, node.value);
          },
          JSXAttribute(node) {
            if (!spoken.has(node.name.name)) return;
            literal(
              node.value?.type === "JSXExpressionContainer" ? node.value.expression : node.value,
            );
          },
          JSXExpressionContainer(node) {
            if (node.parent.type === "JSXElement" || node.parent.type === "JSXFragment") {
              literal(node.expression);
            }
          },
        };
      },
    },
  },
};
