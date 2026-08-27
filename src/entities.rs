use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use phf::{phf_map, phf_set};
use regex::Regex;

pub static ENTITIES: phf::Map<&str, char> = phf_map! {
    "&Alpha;" => 'Α',
    "&Beta;" => 'Β',
    "&Chi;" => 'Χ',
    "&Delta;" => 'Δ',
    "&Epsilon;" => 'Ε',
    "&Eta;" => 'Η',
    "&Gamma;" => 'Γ',
    "&Iota;" => 'Ι',
    "&Kappa;" => 'Κ',
    "&Lambda;" => 'Λ',
    "&Mu;" => 'Μ',
    "&Nu;" => 'Ν',
    "&Omega;" => 'Ω',
    "&Omicron;" => 'Ο',
    "&Phi;" => 'Φ',
    "&Pi;" => 'Π',
    "&Psi;" => 'Ψ',
    "&Rho;" => 'Ρ',
    "&Sigma;" => 'Σ',
    "&Tau;" => 'Τ',
    "&Theta;" => 'Θ',
    "&Upsilon;" => 'Υ',
    "&Xi;" => 'Ξ',
    "&Zeta;" => 'Ζ',
    "&alpha;" => 'α',
    "&and;" => '∧',
    "&asymp;" => '≈',
    "&beta;" => 'β',
    "&bull;" => '•',
    "&cap;" => '∩',
    "&chi;" => 'χ',
    "&cup;" => '∪',
    "&darr;" => '↓',
    "&deg;" => '°',
    "&delta;" => 'δ',
    "&divide;" => '÷',
    "&ell;" => 'ℓ',
    "&empty;" => '∅',
    "&epsilon;" => 'ε',
    "&equiv;" => '≡',
    "&eta;" => 'η',
    "&exist;" => '∃',
    "&forall;" => '∀',
    "&gamma;" => 'γ',
    "&ge;" => '≥',
    "&gg;" => '≫',
    "&hArr;" => '⇔',
    "&harr;" => '↔',
    "&hellip;" => '…',
    "&in;" => '∈',
    "&infin;" => '∞',
    "&int;" => '∫',
    "&iota;" => 'ι',
    "&kappa;" => 'κ',
    "&lArr;" => '⇐',
    "&ldquo;" => '“',
    "&lambda;" => 'λ',
    "&larr;" => '←',
    "&le;" => '≤',
    "&lsquo;" => '‘',
    "&ll;" => '≪',
    "&map;" => '↦',
    "&mapsto;" => '↦',
    "&mdash;" => '—',
    "&micro;" => 'µ',
    "&middot;" => '·',
    "&minus;" => '−',
    "&mu;" => 'μ',
    "&nabla;" => '∇',
    "&nbsp;" => '\u{00A0}',
    "&ndash;" => '–',
    "&ne;" => '≠',
    "&not;" => '¬',
    "&notin;" => '∉',
    "&nu;" => 'ν',
    "&ocirc;" => 'ô',
    "&omega;" => 'ω',
    "&or;" => '∨',
    "&para;" => '¶',
    "&part;" => '∂',
    "&phi;" => 'φ',
    "&pi;" => 'π',
    "&plusmn;" => '±',
    "&prime;" => '′',
    "&prod;" => '∏',
    "&prop;" => '∝',
    "&psi;" => 'ψ',
    "&rArr;" => '⇒',
    "&radic;" => '√',
    "&rarr;" => '→',
    "&rho;" => 'ρ',
    "&rdquo;" => '”',
    "&rsquo;" => '’',
    "&sdot;" => '⋅',
    "&sect;" => '§',
    "&sigma;" => 'σ',
    "&sub;" => '⊂',
    "&sube;" => '⊆',
    "&sum;" => '∑',
    "&sup;" => '⊃',
    "&sup2;" => '²',
    "&sup3;" => '³',
    "&supe;" => '⊇',
    "&tau;" => 'τ',
    "&theta;" => 'θ',
    "&times;" => '×',
    "&trade;" => '™',
    "&uarr;" => '↑',
    "&upsilon;" => 'υ',
    "&xi;" => 'ξ',
    "&zeta;" => 'ζ',
};

pub static STRUCTURAL: phf::Set<&str> =
    phf_set!["&shy;", "&ensp;", "&emsp;", "&thinsp;", "&lt;", "&gt;", "&amp;"];

static DECODE_LT_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new("&lt;(?![a-zA-Z0-9/!-])").unwrap());
static DECODE_AMP_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new("&amp;(?![a-zA-Z0-9#])").unwrap());
static ENCODE_LT_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new("<(?=[a-zA-Z0-9/!-])").unwrap());
static ENCODE_AMP_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new("&(?=[a-zA-Z0-9#])").unwrap());

static ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("&[a-zA-Z0-9]+;").unwrap());

pub fn decode_basic(text: &str) -> String {
    let text = text.replace("&gt;", ">");
    let text = DECODE_LT_RE.replace_all(&text, "<");
    DECODE_AMP_RE.replace_all(&text, "&").into_owned()
}

pub fn encode_basic(text: &str) -> String {
    let text = ENCODE_AMP_RE.replace_all(text, "&amp;");
    ENCODE_LT_RE.replace_all(&text, "&lt;").into_owned()
}

pub fn decode_basic_unconditional(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

pub fn replace(text: &str) -> String {
    let text = ENTITY_RE.replace_all(text, |caps: &regex::Captures| {
        let entity = caps.get(0).unwrap().as_str();
        if STRUCTURAL.contains(entity) {
            return entity.to_owned();
        }
        ENTITIES.get(entity).map_or_else(|| entity.to_owned(), |&c| c.to_string())
    });
    decode_basic(&text)
}
