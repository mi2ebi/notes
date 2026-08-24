#[derive(Debug)]
pub struct ClassInfo {
    pub key: char,
    pub name: &'static str,
    pub bg: (u8, u8, u8),
}

macro_rules! class {
    ($key:literal, $name:literal, $color:expr) => {
        ClassInfo {
            key: $key,
            name: $name,
            bg: (
                (($color >> 16) & 0xff) as u8,
                (($color >> 8) & 0xff) as u8,
                ($color & 0xff) as u8,
            ),
        }
    };
}

pub const CLASSES: &[ClassInfo] = &[
    class!('b', "builtin", 0x_feacd0),
    class!('c', "constant", 0x_b6a0ff),
    class!('d', "docstring", 0x_9ac8e0),
    class!('f', "function", 0x_f78fe7),
    class!('k', "keyword", 0x_79a8ff),
    class!('m', "docmarkup", 0x_caa6df),
    class!('p', "preprocessor", 0x_ff5f87),
    class!('v', "variable", 0x_4ae2f0),
    class!('s', "string", 0x_2fafff),
    class!('t', "type", 0x_11c777),
    class!(';', "comment", 0x_ef8386),
];

pub const RAINBOW: &[ClassInfo] = &[
    class!('0', "rainbow0", 0x_ffffff),
    class!('1', "rainbow1", 0x_ff66ff),
    class!('2', "rainbow2", 0x_00eff0),
    class!('3', "rainbow3", 0x_ff6b55),
    class!('4', "rainbow4", 0x_efef00),
    class!('5', "rainbow5", 0x_b6a0ff),
    class!('6', "rainbow6", 0x_44df44),
    class!('7', "rainbow7", 0x_79a8ff),
    class!('8', "rainbow8", 0x_f78fe7),
];

pub fn by_key(key: char) -> Option<&'static ClassInfo> { CLASSES.iter().find(|c| c.key == key) }

pub fn rainbow_by_key(key: char) -> Option<&'static ClassInfo> {
    RAINBOW.iter().find(|c| c.key == key)
}

pub fn by_name(name: &str) -> Option<&'static ClassInfo> {
    CLASSES.iter().chain(RAINBOW).find(|c| c.name == name)
}
