#![cfg(test)]

use std::{collections::HashMap, sync::LazyLock};

use crate::{accents, entities, fonts, macros, process, scripts, unicode_data};

type Maps = (fonts::FontMaps, HashMap<char, char>, HashMap<char, char>, HashMap<char, char>);

static MAPS: LazyLock<Maps> = LazyLock::new(|| {
    let ud = unicode_data::load().expect("UnicodeData.txt must be present");
    let font_maps = fonts::build(&ud.letters);
    (font_maps, ud.superscripts, ud.subscripts, ud.negations)
});

fn process(input: &str) -> String {
    let (font_maps, sup, sub, neg) = &*MAPS;
    let (text, _) = process::process(input, font_maps, sup, sub, neg);
    text
}

#[test]
fn entity_basic() {
    assert_eq!(entities::replace("a &rarr; b"), "a → b");
}

#[test]
fn entity_gt_always_replaced() {
    assert_eq!(entities::replace("a &gt; b"), "a > b");
}

#[test]
fn entity_lt_safe() {
    assert_eq!(entities::replace("a &lt; b"), "a < b");
}

#[test]
fn entity_lt_dangerous() {
    assert_eq!(entities::replace("&lt;div&gt;"), "&lt;div>");
}

#[test]
fn entity_lt_before_digit_preserved() {
    assert_eq!(entities::replace("&lt;3"), "&lt;3");
}

#[test]
fn entity_unknown_preserved() {
    assert_eq!(entities::replace("&foo;"), "&foo;");
}

#[test]
fn macro_basic() {
    assert_eq!(macros::replace(r"\alpha"), "α");
}

#[test]
fn macro_no_prefix_match() {
    assert_eq!(macros::replace(r"\int"), "∫");
}

#[test]
fn macro_unknown_preserved() {
    assert_eq!(macros::replace(r"\frac"), r"\frac");
}

// ---------------------------------------------------------------------------
// accents
// ---------------------------------------------------------------------------

#[test]
fn accent_braced() {
    assert_eq!(accents::replace(r"\tilde{x}"), "x\u{0303}");
}

#[test]
fn accent_bare() {
    assert_eq!(accents::replace(r"\hat x"), "x\u{0302}");
}

#[test]
fn accent_multi_char_braced_preserved() {
    assert_eq!(accents::replace(r"\tilde{xy}"), r"\tilde{xy}");
}

#[test]
fn accent_no_match_inside_longer_command() {
    assert_eq!(accents::replace(r"\vector"), r"\vector");
}

#[test]
fn accent_after_macro_pass() {
    let after_macros = macros::replace(r"\tilde\pi");
    assert_eq!(after_macros, r"\tildeπ");
    let after_accents = accents::replace(&after_macros);
    assert_eq!(after_accents, "π\u{0303}");
}

// ---------------------------------------------------------------------------
// scripts
// ---------------------------------------------------------------------------

#[test]
fn superscript_braced() {
    let (_, sup, sub, _) = &*MAPS;
    assert_eq!(scripts::replace(r"x^{2}", sup, sub), "x²");
}

#[test]
fn subscript_braced() {
    let (_, sup, sub, _) = &*MAPS;
    assert_eq!(scripts::replace(r"x_{n}", sup, sub), "xₙ");
}

#[test]
fn superscript_bare() {
    let (_, sup, sub, _) = &*MAPS;
    assert_eq!(scripts::replace(r"x^2", sup, sub), "x²");
}

#[test]
fn script_partial_failure_preserved() {
    let (_, sup, sub, _) = &*MAPS;
    assert_eq!(scripts::replace(r"x^{2!}", sup, sub), r"x^{2!}");
}

#[test]
fn script_skips_tex_remnant() {
    let (_, sup, sub, _) = &*MAPS;
    assert_eq!(scripts::replace(r"x^{\alpha}", sup, sub), r"x^{\alpha}");
}

#[test]
fn superscript_minus() {
    let (_, sup, sub, _) = &*MAPS;
    assert_eq!(scripts::replace(r"x^{-1}", sup, sub), "x⁻¹");
}

#[test]
fn superscript_braced_with_spaces() {
    let (_, sup, sub, _) = &*MAPS;
    assert_eq!(scripts::replace(r"x^{n + 1}", sup, sub), "xⁿ⁺¹");
}

// ---------------------------------------------------------------------------
// negations + pipeline
// ---------------------------------------------------------------------------

#[test]
fn negation_notin() {
    assert_eq!(process(r"\(\not\in\)"), r"\(∉\)");
}

#[test]
fn negation_unknown_preserved() {
    assert_eq!(process(r"\(\not x\)"), r"\(\not x\)");
}

#[test]
fn pipeline_inline_math() {
    assert_eq!(process(r"\(\alpha + \beta\)"), r"\(α + β\)");
}

#[test]
fn pipeline_display_math() {
    assert_eq!(process(r"\[\sum_{n=0}^{\infty} x^n\]"), r"\[∑ₙ₌₀^{∞} xⁿ\]");
}

#[test]
fn pipeline_entity_outside_math() {
    assert_eq!(process(r"a &rarr; b and \(\alpha\)"), r"a → b and \(α\)");
}

#[test]
fn pipeline_math_region_skips_html_tag() {
    let input = r"\(q<p>r\)";
    assert_eq!(process(input), input);
}

#[test]
fn pipeline_lt_in_math_is_fine() {
    assert_eq!(process(r"\(q < p > r\)"), r"\(q < p > r\)");
}

#[test]
fn pipeline_tex_outside_math_untouched() {
    let input = r"the command \alpha does something";
    assert_eq!(process(input), input);
}

#[test]
fn pipeline_mathbf() {
    assert_eq!(process(r"\(\mathbf{A}\)"), r"\(𝐀\)");
}

#[test]
fn pipeline_mathbf_greek() {
    assert_eq!(process(r"\(\mathbf{\alpha}\)"), r"\(𝛂\)");
}

#[test]
fn pipeline_mathit_planck() {
    assert_eq!(process(r"\(\mathit{h}\)"), "\\\u{28}ℎ\\\u{29}");
}

#[test]
fn pipeline_font_alias_bbb() {
    assert_eq!(process(r"\(\Bbb{R}\)"), r"\(ℝ\)");
}

#[test]
fn pipeline_font_alias_bm() {
    assert_eq!(process(r"\(\bm{A}\)"), r"\(𝑨\)");
}

#[test]
fn pipeline_font_alias_bold() {
    assert_eq!(process(r"\(\bold{A}\)"), r"\(𝐀\)");
}
