// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The files one package compiles outside a test build, and every
//! `include!`, `include_str!` and `include_bytes!` written in them.
//!
//! Module files are found by the rules of the Rust reference's "Modules"
//! chapter: a `mod` without `#[path]` sits in the declaring file's module
//! directory - the file's own directory for a crate root, a `mod.rs` or
//! a file loaded through `#[path]`, a directory named for the file
//! otherwise - and a `#[path]` is joined onto the declaring file's own
//! directory, with the names of any inline modules around it in between.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use syn::ext::IdentExt as _;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};

use super::{normal, test_only};
use crate::members::Member;
use crate::report::XtaskError;
use crate::walk;

/// The three macros that compile a file's bytes into the crate.
const INCLUDES: [&str; 3] = ["include", "include_str", "include_bytes"];

/// One call of an [`INCLUDES`] macro, where it is written and what it
/// reads.
pub(super) struct Include {
    /// Repo-relative, `/`-separated.
    pub(super) file: String,
    pub(super) line: usize,
    /// The argument as written, for the report.
    pub(super) written: String,
    pub(super) target: Target,
}

/// What an [`Include`] reads.
pub(super) enum Target {
    /// A file, its `.` and `..` resolved by spelling.
    Path(PathBuf),
    /// Under `OUT_DIR`, which the build script wrote.
    BuildOutput,
    /// An argument the gate cannot read a path out of, as written.
    Unread(String),
}

/// One module file still to be read, and the directory its own
/// file-backed `mod` declarations resolve in.
struct Module {
    file: PathBuf,
    children: PathBuf,
}

/// Every [`Include`] in the files `member` compiles when it is built
/// from its archive.
///
/// # Errors
/// When a module file cannot be read or does not parse, and when a
/// `mod` declaration names a file that is not there: the gate then
/// cannot say what the package compiles.
pub(super) fn includes(root: &Path, member: &Member) -> Result<Vec<Include>, XtaskError> {
    let package = normal(&root.join(&member.dir));
    let mut pending: Vec<Module> = member
        .roots
        .iter()
        .map(|rel| crate_root(&root.join(rel)))
        .collect();
    let mut read = BTreeSet::new();
    let mut found = Vec::new();
    while let Some(module) = pending.pop() {
        if !read.insert(module.file.clone()) {
            continue;
        }
        let rel = walk::rel(root, &module.file);
        let parsed =
            syn::parse_file(&walk::read_text(&module.file)?).map_err(|err| XtaskError::Doc {
                file: rel.clone(),
                msg: format!("this file does not parse as Rust: {err}"),
            })?;
        let mut reader = Reader {
            package: &package,
            rel,
            here: parent(&module.file),
            children: module.children,
            inline: Vec::new(),
            found: &mut found,
            pending: &mut pending,
            missing: None,
        };
        reader.visit_file(&parsed);
        if let Some(err) = reader.missing {
            return Err(err);
        }
    }
    Ok(found)
}

/// A crate root's submodules sit beside it.
fn crate_root(file: &Path) -> Module {
    Module {
        file: file.to_path_buf(),
        children: parent(file),
    }
}

fn parent(file: &Path) -> PathBuf {
    file.parent().map(Path::to_path_buf).unwrap_or_default()
}

/// One module file, read for its includes and its submodules.
struct Reader<'a> {
    package: &'a Path,
    rel: String,
    /// The directory this file is in.
    here: PathBuf,
    /// The directory its file-backed submodules resolve in.
    children: PathBuf,
    /// The inline `mod name { … }` blocks the visit is inside.
    inline: Vec<String>,
    found: &'a mut Vec<Include>,
    pending: &'a mut Vec<Module>,
    /// The first declaration whose file is not there.
    missing: Option<XtaskError>,
}

impl<'ast> Visit<'ast> for Reader<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if !test_only(attrs_of(item)) {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        if !test_only(impl_attrs_of(item)) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let name = module.ident.unraw().to_string();
        match &module.content {
            Some((_, items)) => {
                self.inline.push(name);
                for item in items {
                    self.visit_item(item);
                }
                self.inline.pop();
            }
            None => match self.declared(&module.attrs, &name) {
                Ok(next) => self.pending.push(next),
                Err(err) => {
                    self.missing.get_or_insert(err);
                }
            },
        }
    }

    fn visit_macro(&mut self, called: &'ast syn::Macro) {
        if let Some(last) = called.path.segments.last()
            && INCLUDES.iter().any(|name| last.ident == name)
        {
            self.record(last.ident.span().start().line, called.tokens.clone());
        }
        self.nested(called.tokens.clone());
    }
}

impl Reader<'_> {
    /// The file a `mod name;` declaration loads.
    fn declared(&self, attrs: &[syn::Attribute], name: &str) -> Result<Module, XtaskError> {
        let inner = self
            .inline
            .iter()
            .fold(self.children.clone(), |dir, segment| dir.join(segment));
        let candidates = match path_attribute(attrs) {
            // A path outside any inline block is the declaring file's
            // own directory's; inside one, the inline names come first.
            Some(path) if self.inline.is_empty() => vec![self.here.join(path)],
            Some(path) => vec![inner.join(path)],
            None => vec![
                inner.join(format!("{name}.rs")),
                inner.join(name).join("mod.rs"),
            ],
        };
        let file = candidates
            .into_iter()
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| XtaskError::Doc {
                file: self.rel.clone(),
                msg: format!("`mod {name};` names no file this gate can find"),
            })?;
        // A file loaded through `#[path]` resolves its own submodules
        // beside itself, as a `mod.rs` does; any other file in a
        // directory named for it.
        let children = match path_attribute(attrs) {
            Some(_) => parent(&file),
            None => inner.join(name),
        };
        Ok(Module { file, children })
    }

    /// Every include call nested in a macro's tokens, which the syntax
    /// tree leaves unparsed: `format!("{}", include_str!(…))`.
    fn nested(&mut self, tokens: TokenStream) {
        let trees: Vec<TokenTree> = tokens.into_iter().collect();
        for tree in &trees {
            if let TokenTree::Group(group) = tree {
                self.nested(group.stream());
            }
        }
        for window in trees.windows(3) {
            if let [
                TokenTree::Ident(ident),
                TokenTree::Punct(bang),
                TokenTree::Group(args),
            ] = window
                && bang.as_char() == '!'
                && INCLUDES.iter().any(|name| ident == name)
            {
                self.record(ident.span().start().line, args.stream());
            }
        }
    }

    fn record(&mut self, line: usize, args: TokenStream) {
        let written = args.to_string();
        let target = target(args, &self.here, self.package);
        self.found.push(Include {
            file: self.rel.clone(),
            line,
            written,
            target,
        });
    }
}

/// What an include call's argument reads: a literal is joined onto the
/// directory of the file it is written in, `concat!(env!("OUT_DIR"), …)`
/// is the build script's output, and `concat!(env!("CARGO_MANIFEST_DIR"),
/// …)` is joined onto the package's directory.
fn target(args: TokenStream, here: &Path, package: &Path) -> Target {
    let shown = args.to_string();
    let Ok(argument) = syn::parse2::<syn::Expr>(args) else {
        return Target::Unread(shown);
    };
    if let Some(path) = literal(&argument) {
        return Target::Path(normal(&here.join(path)));
    }
    if let syn::Expr::Macro(called) = &argument
        && called.mac.path.is_ident("concat")
        && let Ok(parts) = called
            .mac
            .parse_body_with(Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
    {
        let mut parts = parts.into_iter();
        let base = parts.next().as_ref().and_then(environment);
        let rest: Option<String> = parts.map(|part| literal(&part)).collect();
        return match (base.as_deref(), rest) {
            (Some("OUT_DIR"), _) => Target::BuildOutput,
            (Some("CARGO_MANIFEST_DIR"), Some(tail)) => {
                Target::Path(normal(&package.join(tail.trim_start_matches(['/', '\\']))))
            }
            (Some(_) | None, Some(_) | None) => Target::Unread(shown),
        };
    }
    Target::Unread(shown)
}

/// The value of a string literal expression.
fn literal(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(found) = expr
        && let syn::Lit::Str(text) = &found.lit
    {
        return Some(text.value());
    }
    None
}

/// The variable an `env!("NAME")` expression reads.
fn environment(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Macro(called) = expr
        && called.mac.path.is_ident("env")
        && let Ok(name) = called.mac.parse_body::<syn::LitStr>()
    {
        return Some(name.value());
    }
    None
}

/// The value of a `#[path = "…"]` attribute.
fn path_attribute(attrs: &[syn::Attribute]) -> Option<String> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("path"))
        .find_map(|attr| match &attr.meta {
            syn::Meta::NameValue(pair) => literal(&pair.value),
            syn::Meta::Path(_) | syn::Meta::List(_) => None,
        })
}

fn attrs_of(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(it) => &it.attrs,
        syn::Item::Enum(it) => &it.attrs,
        syn::Item::ExternCrate(it) => &it.attrs,
        syn::Item::Fn(it) => &it.attrs,
        syn::Item::ForeignMod(it) => &it.attrs,
        syn::Item::Impl(it) => &it.attrs,
        syn::Item::Macro(it) => &it.attrs,
        syn::Item::Mod(it) => &it.attrs,
        syn::Item::Static(it) => &it.attrs,
        syn::Item::Struct(it) => &it.attrs,
        syn::Item::Trait(it) => &it.attrs,
        syn::Item::TraitAlias(it) => &it.attrs,
        syn::Item::Type(it) => &it.attrs,
        syn::Item::Union(it) => &it.attrs,
        syn::Item::Use(it) => &it.attrs,
        _ => &[],
    }
}

fn impl_attrs_of(item: &syn::ImplItem) -> &[syn::Attribute] {
    match item {
        syn::ImplItem::Const(it) => &it.attrs,
        syn::ImplItem::Fn(it) => &it.attrs,
        syn::ImplItem::Type(it) => &it.attrs,
        syn::ImplItem::Macro(it) => &it.attrs,
        _ => &[],
    }
}
