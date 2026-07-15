#[derive(Debug)]
pub struct ClassInfo {
    pub key: char,
    pub name: &'static str,
    pub bg: (u8, u8, u8),
}

macro_rules! class {
    ($key:literal, $name:literal, $r:literal $g:literal $b:literal) => {
        ClassInfo { key: $key, name: $name, bg: ($r, $g, $b) }
    };
}

pub const CLASSES: &[ClassInfo] = &[
    class!('b', "builtin", 0xfe 0xac 0xd0),
    class!('c', "constant", 0xb6 0xa0 0xff),
    class!('d', "docstring", 0x9a 0xc8 0xe0),
    class!('f', "function", 0xf7 0x8f 0xe7),
    class!('k', "keyword", 0x79 0xa8 0xff),
    class!('m', "docmarkup", 0xca 0xa6 0xdf),
    class!('p', "preprocessor", 0xff 0x5f 0x87),
    class!('v', "variable", 0x4a 0xe2 0xf0),
    class!('s', "string", 0x2f 0xaf 0xff),
    class!('t', "type", 0x11 0xc7 0x77),
    class!(';', "comment", 0xef 0x83 0x86),
];

pub const RAINBOW: &[ClassInfo] = &[
    class!('0', "rainbow0", 0xff 0xff 0xff),
    class!('1', "rainbow1", 0xff 0x66 0xff),
    class!('2', "rainbow2", 0x00 0xef 0xf0),
    class!('3', "rainbow3", 0xff 0x6b 0x55),
    class!('4', "rainbow4", 0xef 0xef 0x00),
    class!('5', "rainbow5", 0xb6 0xa0 0xff),
    class!('6', "rainbow6", 0x44 0xdf 0x44),
    class!('7', "rainbow7", 0x79 0xa8 0xff),
    class!('8', "rainbow8", 0xf7 0x8f 0xe7),
];

pub const HEADINGS: &[ClassInfo] = &[
    class!('1', "heading-1", 0xff 0xff 0xff),
    class!('2', "heading-2", 0xd2 0xb5 0x80),
    class!('3', "heading-3", 0x82 0xb0 0xec),
    class!('4', "heading-4", 0xfe 0xac 0xd0),
    class!('5', "heading-5", 0x88 0xca 0x9f),
    class!('6', "heading-6", 0xef 0x83 0x86),
];

pub fn by_key(key: char) -> Option<&'static ClassInfo> { CLASSES.iter().find(|c| c.key == key) }

pub fn rainbow_by_key(key: char) -> Option<&'static ClassInfo> {
    RAINBOW.iter().find(|c| c.key == key)
}

pub fn heading_by_key(key: char) -> Option<&'static ClassInfo> {
    HEADINGS.iter().find(|c| c.key == key)
}

pub fn by_name(name: &str) -> Option<&'static ClassInfo> {
    CLASSES.iter().chain(RAINBOW).chain(HEADINGS).find(|c| c.name == name)
}
