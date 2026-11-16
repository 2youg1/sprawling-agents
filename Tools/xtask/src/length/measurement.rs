// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use syn::spanned::Spanned;

pub(crate) struct Body {
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) lines: usize,
}

pub(crate) struct Found {
    pub(crate) body: Body,

    pub(crate) args: usize,
}

pub(super) fn measure(items: &[syn::Item]) -> Vec<Found> {
    let mut out = Vec::new();
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                if skipped(&function.attrs) {
                    continue;
                }
                out.push(found(&function.sig, function.block.span()));
            }
            syn::Item::Mod(module) => {
                if skipped(&module.attrs) {
                    continue;
                }
                if let Some((_, items)) = &module.content {
                    out.extend(measure(items));
                }
            }
            syn::Item::Impl(block) => {
                if skipped(&block.attrs) {
                    continue;
                }
                for member in &block.items {
                    let syn::ImplItem::Fn(function) = member else {
                        continue;
                    };
                    if skipped(&function.attrs) {
                        continue;
                    }
                    out.push(found(&function.sig, function.block.span()));
                }
            }
            syn::Item::Trait(declared) => {
                if skipped(&declared.attrs) {
                    continue;
                }
                for member in &declared.items {
                    let syn::TraitItem::Fn(function) = member else {
                        continue;
                    };
                    let Some(body) = &function.default else {
                        continue;
                    };
                    if skipped(&function.attrs) {
                        continue;
                    }
                    out.push(found(&function.sig, body.span()));
                }
            }
            _ => {}
        }
    }
    out
}

fn found(signature: &syn::Signature, body: proc_macro2::Span) -> Found {
    let start = signature.fn_token.span().start().line;
    let end = body.end().line;
    Found {
        body: Body {
            name: signature.ident.to_string(),
            line: start,
            lines: end.saturating_sub(start).saturating_add(1),
        },

        args: signature
            .inputs
            .iter()
            .filter(|input| matches!(input, syn::FnArg::Typed(_)))
            .count(),
    }
}

fn skipped(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        attr.meta
            .require_list()
            .is_ok_and(|list| list.tokens.to_string().contains("test"))
    })
}
