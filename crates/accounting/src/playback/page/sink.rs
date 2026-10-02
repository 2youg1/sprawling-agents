// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The sink html5ever's tree builder builds a playback page into
//! (`crates/accounting/spec/Playback/Check.lean` §8-13): one table of nodes in the order the parser
//! made them, each element with its parent and the text appended into it.

use std::borrow::Cow;
use std::cell::{Ref, RefCell};

use html5ever::tendril::StrTendril;
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, LocalName, Namespace, QualName};

use super::{Element, Node, Page};

/// The sink html5ever builds into: one table of nodes, a handle being an
/// index into it. Behind a `RefCell` because the tree builder calls the
/// sink through `&self`.
pub(super) struct Builder {
    table: RefCell<Table>,
}

/// The nodes, and the name answered for a handle that is not an element,
/// which the tree builder never asks for; it keeps the answer total and
/// lives beside the nodes so one borrow lends either.
struct Table {
    nodes: Vec<Node>,
    unnamed: QualName,
}

impl Builder {
    pub(super) fn new() -> Builder {
        Builder {
            table: RefCell::new(Table {
                nodes: vec![Node::Other],
                unnamed: QualName::new(None, Namespace::from(""), LocalName::from("")),
            }),
        }
    }

    fn push(&self, node: Node) -> usize {
        let nodes = &mut self.table.borrow_mut().nodes;
        nodes.push(node);
        nodes.len().saturating_sub(1)
    }

    fn parent(&self, of: usize) -> Option<usize> {
        match self.table.borrow().nodes.get(of) {
            Some(Node::Element(element)) => element.parent,
            Some(Node::Other) | None => None,
        }
    }

    fn set_parent(&self, child: usize, parent: Option<usize>) {
        if let Some(Node::Element(element)) = self.table.borrow_mut().nodes.get_mut(child) {
            element.parent = parent;
        }
    }

    fn add_text(&self, to: usize, text: &str) {
        if let Some(Node::Element(element)) = self.table.borrow_mut().nodes.get_mut(to) {
            element.text.push_str(text);
        }
    }

    fn hang(&self, parent: Option<usize>, child: NodeOrText<usize>) {
        match child {
            NodeOrText::AppendNode(node) => self.set_parent(node, parent),
            NodeOrText::AppendText(text) => {
                if let Some(parent) = parent {
                    self.add_text(parent, &text);
                }
            }
        }
    }
}

impl TreeSink for Builder {
    type Handle = usize;
    type Output = Page;
    type ElemName<'a> = Ref<'a, QualName>;

    fn finish(self) -> Page {
        Page {
            nodes: self.table.into_inner().nodes,
        }
    }

    /// A page with parse errors is still a page a browser shows; the
    /// checks judge what was built.
    fn parse_error(&self, _msg: Cow<'static, str>) {}

    fn get_document(&self) -> usize {
        0
    }

    fn elem_name<'a>(&'a self, target: &'a usize) -> Ref<'a, QualName> {
        Ref::map(self.table.borrow(), |table| {
            match table.nodes.get(*target) {
                Some(Node::Element(element)) => &element.name,
                Some(Node::Other) | None => &table.unnamed,
            }
        })
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> usize {
        let contents = flags.template.then(|| self.push(Node::Other));
        self.push(Node::Element(Element {
            name,
            attrs,
            text: String::new(),
            parent: None,
            contents,
        }))
    }

    fn create_comment(&self, _text: StrTendril) -> usize {
        self.push(Node::Other)
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> usize {
        self.push(Node::Other)
    }

    fn append(&self, parent: &usize, child: NodeOrText<usize>) {
        self.hang(Some(*parent), child);
    }

    fn append_based_on_parent_node(
        &self,
        element: &usize,
        prev_element: &usize,
        child: NodeOrText<usize>,
    ) {
        match self.parent(*element) {
            Some(parent) => self.hang(Some(parent), child),
            None => self.hang(Some(*prev_element), child),
        }
    }

    fn append_doctype_to_document(&self, _: StrTendril, _: StrTendril, _: StrTendril) {}

    fn get_template_contents(&self, target: &usize) -> usize {
        match self.table.borrow().nodes.get(*target) {
            Some(Node::Element(element)) => element.contents.unwrap_or(*target),
            Some(Node::Other) | None => *target,
        }
    }

    fn same_node(&self, x: &usize, y: &usize) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn append_before_sibling(&self, sibling: &usize, new_node: NodeOrText<usize>) {
        self.hang(self.parent(*sibling), new_node);
    }

    fn add_attrs_if_missing(&self, target: &usize, attrs: Vec<Attribute>) {
        if let Some(Node::Element(element)) = self.table.borrow_mut().nodes.get_mut(*target) {
            for attr in attrs {
                if !element.attrs.iter().any(|held| held.name == attr.name) {
                    element.attrs.push(attr);
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &usize) {
        self.set_parent(*target, None);
    }

    fn reparent_children(&self, node: &usize, new_parent: &usize) {
        let nodes = &mut self.table.borrow_mut().nodes;
        let moved = match nodes.get_mut(*node) {
            Some(Node::Element(element)) => std::mem::take(&mut element.text),
            Some(Node::Other) | None => String::new(),
        };
        for held in nodes.iter_mut() {
            if let Node::Element(element) = held
                && element.parent == Some(*node)
            {
                element.parent = Some(*new_parent);
            }
        }
        if let Some(Node::Element(element)) = nodes.get_mut(*new_parent) {
            element.text.push_str(&moved);
        }
    }
}
