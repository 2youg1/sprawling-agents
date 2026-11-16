// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const WORKING_SHAPES: [&str; 4] = ["本机", "前端会话", "这台机器", "落地记录"];

const SWALLOWED: [&str; 2] = ["版本机", "请人裁"];

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

#[must_use]
pub(super) fn working_record(line: &str) -> Option<&'static str> {
    let swallowed: String = SWALLOWED
        .iter()
        .fold(line.to_owned(), |text, word| text.replace(word, "———"));
    if numbered_sitting(&swallowed) {
        return Some("会话 <n>");
    }
    if plan_label(line) {
        return Some("roadmap <label>");
    }
    WORKING_SHAPES
        .iter()
        .chain(ADDRESSEE_SHAPES.iter())
        .find(|shape| swallowed.contains(**shape))
        .copied()
}

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

const PLAN_WORD: &str = "roadmap";

fn plan_label(line: &str) -> bool {
    let glyphs: Vec<char> = line.chars().collect();
    (0..glyphs.len()).any(|at| {
        let inside_a_word = at
            .checked_sub(1)
            .and_then(|before| glyphs.get(before))
            .is_some_and(char::is_ascii_alphanumeric);
        !inside_a_word && glyphs.get(at..).is_some_and(opens_with_a_plan_label)
    })
}

fn opens_with_a_plan_label(rest: &[char]) -> bool {
    let (Some(word), Some(after)) = (rest.get(..PLAN_WORD.len()), rest.get(PLAN_WORD.len()..))
    else {
        return false;
    };
    let named = word
        .iter()
        .zip(PLAN_WORD.chars())
        .all(|(glyph, letter)| glyph.to_ascii_lowercase() == letter);
    let spaces = after.iter().take_while(|glyph| **glyph == ' ').count();
    named && spaces > 0 && after.get(spaces..).is_some_and(is_a_plan_label)
}

fn is_a_plan_label(label: &[char]) -> bool {
    let capitals = label
        .iter()
        .take_while(|glyph| glyph.is_ascii_uppercase())
        .count();
    label.first() == Some(&'\u{a7}')
        || ((1..=3).contains(&capitals) && label.get(capitals).is_some_and(char::is_ascii_digit))
}

#[cfg(test)]
mod tests {
    use super::working_record;

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

    #[test]
    fn a_plan_label_is_a_working_record() {
        let cited = [
            "oldest first (Roadmap M2, the pure lane wait).",
            "folded away (roadmap S1-3);",
            "read and written whole (refrain roadmap Q10, 3-9)",
            "交给 Mayor（roadmap §7）",
            "吞吐台（roadmap TP1）",
        ];
        assert_eq!(cited.map(working_record), [Some("roadmap <label>"); 5]);
    }

    #[test]
    fn the_citys_roadmap_and_a_defined_label_are_not_a_plan_label() {
        let product = [
            "an empty `Roadmap.md` has no first line",
            "the roadmap row a building holds",
            "the roadmaps M2 reads",
            "| M3 接受命令 | `Whole` |",
            "TF1 次序",
            "a Roadmap M without a number",
        ];
        assert_eq!(product.map(working_record), [None; 6]);
    }

    #[test]
    fn the_product_vocabulary_is_not_a_working_record() {

        assert_eq!(working_record("只认回环调用方"), None);

        assert_eq!(working_record("4 会话 29 ms → 32 会话 319 ms"), None);

        assert_eq!(working_record("`collab::arbitrate` 答谁来仲裁"), None);
        assert_eq!(working_record("越界前缀＝拒而不裁剪"), None);

        assert_eq!(working_record("指出路径请人裁"), None);

        assert_eq!(working_record("它们的版本机制是 schema 哈希"), None);
    }
}
