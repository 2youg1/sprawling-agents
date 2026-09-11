// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How long a function is, and how many parameters it takes, read
//! from the parsed item rather than from the text around it.

use syn::spanned::Spanned;

/// One function, as the gate sees it.
pub(crate) struct Found {
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) lines: usize,
    /// Parameters, not counting a receiver. `self` is what the method is
    /// for, never an argument somebody had to decide to pass.
    pub(crate) args: usize,
}

/// Walks items, skipping what is not measured, and measures the rest.
///
/// Recurses into `mod` and `impl` blocks because that is where most
/// functions in this repository live; a nested `#[cfg(test)] mod tests`
/// therefore drops out at its own level rather than by file position.
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
                        continue; // a signature is not a function body
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
        name: signature.ident.to_string(),
        line: start,
        lines: end.saturating_sub(start).saturating_add(1),
        // A receiver is not a parameter. `&self` is what makes the
        // function a method, not a value somebody chose to thread
        // through it, and counting it would charge every method one
        // argument it never had a say in.
        args: signature
            .inputs
            .iter()
            .filter(|input| matches!(input, syn::FnArg::Typed(_)))
            .count(),
    }
}

/// Whether this item is the one kind the gate does not measure.
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
