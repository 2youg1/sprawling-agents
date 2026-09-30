// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Ship the decision, never the occasion.
//!
//! A working record is noise to a contributor and to a model, and signal
//! to an attacker: what was tried and abandoned, which machine could not
//! verify what, which account ran out of quota, and the words a decision
//! arrived in. A reader skips them; somebody looking for a way in reads
//! them closely.
//!
//! What can be checked mechanically is collected into two closed tables
//! of literal shapes and one rule about a digit. What cannot is said out
//! loud at the bottom of this file rather than left for somebody to
//! discover.

/// The spellings that turn a product document into a working record.
///
/// A closed table of literal shapes, the way [`HOME_SHAPES`] is, and for
/// the same reason: a rule that tried to recognise working context by
/// meaning would be a rule nobody could predict. These four are what the
/// leak is made of when it has a shape at all.
///
/// **`本机` and `这台机器` both point at a machine only the author can
/// see.** What they carry is a toolchain version, a measurement, or a
/// capability one person's laptop happens not to have, and every one of
/// those is a fact about somebody rather than about this code. A
/// measurement names the class of machine it came from; a running
/// machine is named by what it is, `运行中的机器`, and a machine under
/// test is `测试机`. The socket's own word, `回环`, is unaffected.
///
/// **What this table cannot catch is said out loud** rather than left
/// for somebody to discover: working context written as ordinary prose
/// has no shape. *"only a third could be verified"*, a quotation of a
/// conversation, an account that ran out of quota - the gate reads each
/// of those as a sentence. That half is held by review, and the same
/// admission is in `xtask secret`'s own rustdoc for the same reason.
const WORKING_SHAPES: [&str; 4] = ["本机", "前端会话", "这台机器", "落地记录"];

/// The compounds that contain a shape and are not one.
///
/// Chinese writes no space between words, so a table of literal shapes
/// needs a table of the words that swallow one. `版本机制` - a
/// versioning mechanism - is the one this project's vocabulary has, and
/// it is here rather than in a cleverer matcher because a closed list of
/// two words is checkable and a word segmenter is not. `请人裁` is the
/// second: it contains `人裁` and is a product behaviour, the system
/// escalating to the person inside a refusal's own text.
const SWALLOWED: [&str; 2] = ["版本机", "请人裁"];

/// The spellings that carry a ruling's occasion rather than a ruling.
///
/// A decision belongs in the tree; who made it, when, and in what words
/// does not. These three are the shapes that failed: an addressee makes
/// the sentence a turn in a conversation a reader is not in, and it goes
/// stale the moment the ruling lands - which is worse than never having
/// been written, because it tells every later reader that a settled
/// matter is still open.
///
/// **One author's habitual wording is itself a working record.** A rule
/// is fixed by the SPEC, the code and the comments around it, so a
/// sentence that also says who settled it, when, or against which
/// earlier wording adds nothing a reader can use - while the habit it is
/// written in identifies the person who wrote it. So the markers go and
/// the rule stays: an acceptance criterion is `验收标准`, a decision is
/// `决定`, and a superseded wording is simply absent.
///
/// **`裁` and `判` alone are not the defect.** `仲裁` is the domain word
/// for arbitration, `裁剪` is truncation, `判定` is a verdict a function
/// returns, and an authority ladder names the person on purpose. Only
/// these spellings are.
const ADDRESSEE_SHAPES: [&str; 10] = [
    "待人裁",
    "立场归人",
    "这一步我不做",
    "裁决",
    "裁定",
    "人裁",
    "自裁",
    "改判",
    "原判",
    "判据",
];

/// Whether a line writes one machine's working record into a product
/// document, or hands a ruling to somebody by name.
///
/// One function for both tables because a caller acts on them the same
/// way, and the report names which shape it found so the person reading
/// it does not have to guess.
#[must_use]
pub(super) fn working_record(line: &str) -> Option<&'static str> {
    let swallowed: String = SWALLOWED
        .iter()
        .fold(line.to_owned(), |text, word| text.replace(word, "———"));
    if numbered_sitting(&swallowed) {
        return Some("会话 <n>");
    }
    WORKING_SHAPES
        .iter()
        .chain(ADDRESSEE_SHAPES.iter())
        .find(|shape| swallowed.contains(**shape))
        .copied()
}

/// Whether a line numbers a sitting: `会话` and one digit.
///
/// **A session number is an index into a document this tree does not
/// have.** It tells a reader that a record of the work exists, and it
/// leaves this document unreadable without it.
///
/// One digit and no second one, because `会话` is also this project's
/// word for the product's own sessions, and a measurement counts them:
/// `4 会话 29 ms` is a reading off a probe. A sitting was never numbered
/// past nine, and a count rarely stops at one digit, so the line between
/// them is where the digits end.
fn numbered_sitting(line: &str) -> bool {
    let glyphs: Vec<char> = line.chars().collect();
    glyphs
        .windows(4)
        .any(|window| matches!(window, ['会', '话', ' ', digit] if digit.is_ascii_digit()))
        && !glyphs.windows(5).any(|window| {
            matches!(window, ['会', '话', ' ', first, second]
            if first.is_ascii_digit() && second.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::working_record;

    /// A product document states the decision, never the occasion.
    ///
    /// The three families the shape table exists for: a fact about the
    /// author's own machine, an index into a record of the work, and a
    /// ruling handed to somebody by name.
    #[test]
    fn a_working_record_in_a_product_document_is_caught() {
        assert_eq!(working_record("本机实测 22 秒"), Some("本机"));
        assert_eq!(working_record("前端会话 3 实测修正"), Some("会话 <n>"));
        assert_eq!(working_record("（会话 4，2026-09-11）"), Some("会话 <n>"));
        assert_eq!(working_record("这件事待人裁"), Some("待人裁"));
        assert_eq!(working_record("立场归人"), Some("立场归人"));
        assert_eq!(working_record("这台机器实测 22 秒"), Some("这台机器"));
        assert_eq!(working_record("（用户裁定）本地恒不轮询"), Some("裁定"));
        assert_eq!(working_record("判据：`cargo xtask gates` 绿"), Some("判据"));
    }

    /// The four families the rule must not eat, each of which is either a
    /// product concept or a word that swallows a shape.
    #[test]
    fn the_product_vocabulary_is_not_a_working_record() {
        // The socket's own word for the local caller.
        assert_eq!(working_record("只认回环调用方"), None);
        // A count of the product's own sessions, off a probe.
        assert_eq!(working_record("4 会话 29 ms → 32 会话 319 ms"), None);
        // The domain words that contain a banned glyph without being one.
        assert_eq!(working_record("`collab::arbitrate` 答谁来仲裁"), None);
        assert_eq!(working_record("越界前缀＝拒而不裁剪"), None);
        // The product behaviour of escalating to the person.
        assert_eq!(working_record("指出路径请人裁"), None);
        // A compound that contains a shape and is not one.
        assert_eq!(working_record("它们的版本机制是 schema 哈希"), None);
    }
}
