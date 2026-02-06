//! Character code constants used by the scanner.
//! Matches TypeScript's CharacterCodes enum.

#![allow(dead_code)]

// ASCII control characters
pub const NULL_CHARACTER: char = '\0';
pub const MAX_ASCII_CHARACTER: u32 = 0x7F;
pub const LINE_FEED: char = '\n';
pub const CARRIAGE_RETURN: char = '\r';
pub const LINE_SEPARATOR: char = '\u{2028}';
pub const PARAGRAPH_SEPARATOR: char = '\u{2029}';

// ASCII characters
pub const SPACE: char = ' ';
pub const EXCLAMATION: char = '!';
pub const DOUBLE_QUOTE: char = '"';
pub const HASH: char = '#';
pub const DOLLAR_SIGN: char = '$';
pub const PERCENT: char = '%';
pub const AMPERSAND: char = '&';
pub const SINGLE_QUOTE: char = '\'';
pub const OPEN_PAREN: char = '(';
pub const CLOSE_PAREN: char = ')';
pub const ASTERISK: char = '*';
pub const PLUS: char = '+';
pub const COMMA: char = ',';
pub const MINUS: char = '-';
pub const DOT: char = '.';
pub const SLASH: char = '/';
pub const _0: char = '0';
pub const _1: char = '1';
pub const _7: char = '7';
pub const _8: char = '8';
pub const _9: char = '9';
pub const COLON: char = ':';
pub const SEMICOLON: char = ';';
pub const LESS_THAN: char = '<';
pub const EQUALS: char = '=';
pub const GREATER_THAN: char = '>';
pub const QUESTION: char = '?';
pub const AT: char = '@';

pub const A_UPPER: char = 'A';
pub const B_UPPER: char = 'B';
pub const E_UPPER: char = 'E';
pub const F_UPPER: char = 'F';
pub const N_UPPER: char = 'N';
pub const O_UPPER: char = 'O';
pub const X_UPPER: char = 'X';
pub const Z_UPPER: char = 'Z';

pub const OPEN_BRACKET: char = '[';
pub const BACKSLASH: char = '\\';
pub const CLOSE_BRACKET: char = ']';
pub const CARET: char = '^';
pub const UNDERSCORE: char = '_';
pub const BACKTICK: char = '`';

pub const A_LOWER: char = 'a';
pub const B_LOWER: char = 'b';
pub const E_LOWER: char = 'e';
pub const F_LOWER: char = 'f';
pub const N_LOWER: char = 'n';
pub const O_LOWER: char = 'o';
pub const R_LOWER: char = 'r';
pub const T_LOWER: char = 't';
pub const U_LOWER: char = 'u';
pub const V_LOWER: char = 'v';
pub const X_LOWER: char = 'x';
pub const Z_LOWER: char = 'z';

pub const OPEN_BRACE: char = '{';
pub const BAR: char = '|';
pub const CLOSE_BRACE: char = '}';
pub const TILDE: char = '~';

/// Check if a character is a line terminator.
#[inline]
pub fn is_line_break(ch: char) -> bool {
    ch == LINE_FEED
        || ch == CARRIAGE_RETURN
        || ch == LINE_SEPARATOR
        || ch == PARAGRAPH_SEPARATOR
}

/// Check if a character is whitespace (not line break).
#[inline]
pub fn is_white_space_single_line(ch: char) -> bool {
    matches!(
        ch,
        ' ' | '\t'
            | '\u{000B}' // vertical tab
            | '\u{000C}' // form feed
            | '\u{00A0}' // no-break space
            | '\u{1680}' // ogham space mark
            | '\u{2000}'..='\u{200A}' // various spaces
            | '\u{202F}' // narrow no-break space
            | '\u{205F}' // medium mathematical space
            | '\u{3000}' // ideographic space
            | '\u{FEFF}' // BOM / zero-width no-break space
    )
}

/// Check if a character is a decimal digit.
#[inline]
pub fn is_digit(ch: char) -> bool {
    ch.is_ascii_digit()
}

/// Check if a character is an octal digit (0-7).
#[inline]
pub fn is_octal_digit(ch: char) -> bool {
    matches!(ch, '0'..='7')
}

/// Check if a character is a hex digit.
#[inline]
pub fn is_hex_digit(ch: char) -> bool {
    ch.is_ascii_hexdigit()
}
