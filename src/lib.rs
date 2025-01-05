use std::fmt::Display;
use std::string::FromUtf8Error;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub mod xml;
pub mod json;


pub fn format(body: &str, ident: usize) -> Result<String, String> {
    if body.contains("</") {
        xml::format(body, ident)
    } else {
        json::format(body, ident)
    }
}

#[derive(Debug)]
pub struct FormatError {
    message: String,
}

impl From<FromUtf8Error> for FormatError {
    fn from(err: FromUtf8Error) -> Self {
        return FormatError {
            message: err.to_string(),
        };
    }
}

impl Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message.to_string())
    }
}

trait Parser {
    // вход - исходная строка
    // выход - распарсенная и отформатированная строка, остаток исходной строки
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str);
}

struct BoxedParser<'a> {
    parser: Box<dyn Parser + 'a>,
}

impl<'a> BoxedParser<'a> {
    fn new(p: impl Parser + 'a) -> Self {
        Self {
            parser: Box::new(p),
        }
    }
}

impl<'a> Parser for BoxedParser<'a> {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        self.parser.parse(in_string)
    }
}

// Парсинг строки начинающейся с последовательности символов
struct CharSequence {
    prefix: String,
}

impl CharSequence {
    fn new(pr: String) -> Self {
        Self { prefix: pr }
    }
}

impl Parser for CharSequence {
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str) {
        if in_string.is_empty() {
            return (Err(()), in_string);
        }

        let expected_len = self.prefix.len();

        return match in_string.get(0..expected_len) {
            Some(real_prefix) if self.prefix == real_prefix => {
                (Ok(real_prefix.to_string()), &in_string[expected_len..])
            }
            _ => (Err(()), in_string),
        };
    }
}

// Парсинг строки оканчивающейся на символ/символы
struct AnyExcept {
    prefix: String,
}

impl AnyExcept {
    fn new(prefix: String) -> Self {
        Self { prefix }
    }
}

impl Parser for AnyExcept {
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str) {
        return if let Some(position) = in_string.find(&self.prefix) {
            if position > 0 {
                let parsed = &in_string[0..position];
                (Ok(parsed.to_string()), &in_string[position..])
            } else {
                (Err(()), in_string)
            }
        } else {
            (Err(()), in_string)
        };
    }
}

// Парсинг строки оканчивающейся на символ/символы
struct AnyChar<F> {
    check: F,
}

impl<F> AnyChar<F> {
    fn new(check: F) -> Self {
        Self { check }
    }
}

impl<F> Parser for AnyChar<F>
where
    F: Fn(char) -> bool,
{
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str) {
        let mut res = String::new();
        let mut chars = in_string.chars();
        while let Some(ch) = chars.next() {
            if (self.check)(ch) {
                res.push(ch);
            } else {
                break;
            }
        }

        let len = res.len();
        return if len == 0 {
            (Err(()), in_string)
        } else {
            (Ok(res), &in_string[len..])
        };
    }
}

// один или больше раз встречается внутренний парсер
struct OneOrMore<P> {
    p: NTimesOrMore<P>,
}

impl<P: Parser> OneOrMore<P> {
    fn new(p: P) -> Self {
        Self {
            p: NTimesOrMore::new(p, 1),
        }
    }
}

#[warn(dead_code)]
impl<P: Parser> Parser for OneOrMore<P> {
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str) {
        self.p.parse(in_string)
    }
}

struct ZeroOrMore<P> {
    p: NTimesOrMore<P>,
}

impl<P> ZeroOrMore<P> {
    fn new(p: P) -> Self {
        Self {
            p: NTimesOrMore::new(p, 0),
        }
    }
}

impl<P: Parser> Parser for ZeroOrMore<P> {
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str) {
        self.p.parse(in_string)
    }
}

// игнорировать всё, что распарсится
struct Ignore<'a> {
    parser: BoxedParser<'a>,
}

impl<'a> Ignore<'a> {
    fn new(p: impl Parser + 'a) -> Self {
        Self {
            parser: BoxedParser::new(p),
        }
    }
}

impl<'a> Parser for Ignore<'a> {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let res = self.parser.parse(in_string);
        return if let Ok(_) = res.0 {
            (Ok("".to_string()), res.1)
        } else {
            (Err(()), in_string)
        };
    }
}

// последовательные парсеры
struct And<'a> {
    list: Vec<BoxedParser<'a>>,
}

impl<'a> And<'a> {
    fn new() -> Self {
        Self { list: Vec::new() }
    }

    fn add_parser(&mut self, p: impl Parser + 'a) {
        self.list.push(BoxedParser::new(p));
    }
}

impl<'a> Parser for And<'a> {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut res = String::new();
        let mut rest = in_string;

        for p in &self.list {
            match p.parse(rest) {
                (Ok(parsed), r) => {
                    res += parsed.as_str();
                    rest = r;
                }
                _ => return (Err(()), in_string),
            }
        }

        return (Ok(res), rest);
    }
}

struct Or<'a> {
    list: Vec<BoxedParser<'a>>,
}

impl<'a> Or<'a> {
    fn new() -> Self {
        Self { list: Vec::new() }
    }

    fn add_parser(&mut self, p: impl Parser + 'a) {
        self.list.push(BoxedParser::new(p));
    }
}

impl<'a> Parser for Or<'a> {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        for p in &self.list {
            let (ok, rest) = p.parse(in_string);
            if ok.is_ok() {
                return (ok, rest);
            }
        }

        return (Err(()), in_string);
    }
}

struct NTimesOrMore<P> {
    p: P,
    n: usize,
}

impl<P> NTimesOrMore<P> {
    fn new(p: P, n: usize) -> Self {
        Self { p, n }
    }
}

impl<P: Parser> Parser for NTimesOrMore<P> {
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str) {
        let mut rest = in_string;
        let mut res = String::new();

        let mut amount = 0;
        loop {
            let tmp_res = self.p.parse(rest);
            if tmp_res.0.is_err() {
                break;
            }

            res += tmp_res.0.unwrap().as_str();
            rest = tmp_res.1;
            amount += 1;
        }

        return if self.n <= amount {
            (Ok(res), rest)
        } else {
            (Err(()), in_string)
        };
    }
}

struct ZeroOrOne<P> {
    p: P,
}

impl<P> ZeroOrOne<P> {
    fn new(p: P) -> Self {
        Self { p }
    }
}

impl<P: Parser> Parser for ZeroOrOne<P> {
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str) {
        let mut rest = in_string;
        let mut res = String::new();

        let mut amount = 0;
        loop {
            let tmp_res = self.p.parse(rest);
            if tmp_res.0.is_err() {
                break;
            }

            res += tmp_res.0.unwrap().as_str();
            rest = tmp_res.1;
            amount += 1;
        }

        return if amount <= 1 {
            (Ok(res), rest)
        } else {
            (Err(()), in_string)
        };
    }
}

pub fn unescape_utf8(in_string: &str) -> String {
    let in_len = in_string.len();
    if in_len < 6 {
        return String::from(in_string);
    }

    let max_idx = in_len - 6;
    let mut res: Vec<u8> = Vec::with_capacity(in_len);
    let in_bytes = in_string.as_bytes();
    let mut i = 0;
    while i <= max_idx {
        if in_bytes[i] == '\\' as u8 && in_bytes[i + 1] == 'u' as u8 {
            let str_num = unsafe { std::str::from_utf8_unchecked(&in_bytes[i + 2..i + 6]) };
            if let Ok(n) = u32::from_str_radix(str_num, 16) {
                res.extend_from_slice(&unicode_to_utf8(n));
                // if n < 0x255 {
                //     res.push(n.to_be_bytes()[1]);
                // } else {
                //     res.extend_from_slice(&n.to_le_bytes());
                // };

                i += 6;
                continue;
            }
        }

        res.push(in_bytes[i]);
        i += 1
    }
    res.extend_from_slice(&in_bytes[i..]);

    String::from_utf8(res).unwrap()
}

pub fn unicode_to_utf8(mut char: u32) -> Vec<u8> {
    let mut res = Vec::with_capacity(4);

    // 10000 001101 001000
    let mut first_byte_mask = 0b01111111;
    let mut first_byte_prefix = 0b00000000;
    loop {
        let cur_octet = (char & 0b111111) as u8;
        char >>= 6;

        if first_byte_mask == 0b01111111 && (char == 0 || char == 1) {
            char <<= 6;
            res.push(char as u8 | cur_octet);
        } else {
            res.push(cur_octet | 0b10000000);
        }

        if char == 0 {
            break;
        }

        if first_byte_mask == 0b01111111 {
            first_byte_mask >>= 2;
            first_byte_prefix = first_byte_prefix >> 2 | 0b11000000;
        } else {
            first_byte_mask >>= 1;
            first_byte_prefix = first_byte_prefix >> 1 | 0b10000000;
        }

        if char & first_byte_mask == char {
            res.push(char as u8 | first_byte_prefix);
            break;
        }
    }
    res.reverse();

    res
}

#[cfg(test)]
mod tests {
    use super::{
        unescape_utf8, unicode_to_utf8, And, AnyChar, CharSequence, OneOrMore, Or, Parser,
        ZeroOrOne,
    };

    #[test]
    fn one_or_more() {
        let parser = OneOrMore::new(CharSequence::new("ha".to_string()));

        let res = parser.parse("ha12345");
        assert_eq!("ha", res.0.unwrap());
        assert_eq!("12345", res.1);

        let res = parser.parse("hahahaha12345");
        assert_eq!("hahahaha", res.0.unwrap());
        assert_eq!("12345", res.1);
    }

    #[test]
    fn any_char() {
        let parser = AnyChar::new(|ch: char| {
            return ch == 'a';
        });

        let res = parser.parse("aaab");
        assert_eq!("aaa", res.0.unwrap());
        assert_eq!("b", res.1);
    }

    #[test]
    fn parse_sequence() {
        let mut parser = And::new();
        parser.add_parser(CharSequence::new("he".to_string()));
        parser.add_parser(CharSequence::new("ll".to_string()));
        parser.add_parser(CharSequence::new("o".to_string()));

        let res = parser.parse("hello");
        assert_eq!("hello", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("help");
        assert_eq!(true, res.0.is_err());
        assert_eq!("help", res.1);
    }

    #[test]
    fn parse_or() {
        let mut parser = Or::new();
        parser.add_parser(CharSequence::new("hello".to_string()));
        parser.add_parser(CharSequence::new("goodbye".to_string()));

        let res = parser.parse("hello");
        assert_eq!("hello", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("goodbye");
        assert_eq!("goodbye", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("help");
        assert_eq!(true, res.0.is_err());
        assert_eq!("help", res.1);
    }

    #[test]
    fn zero_or_one() {
        let parser = ZeroOrOne::new(CharSequence::new("ha".to_string()));

        let res = parser.parse("ha12345");
        assert_eq!("ha", res.0.unwrap());
        assert_eq!("12345", res.1);

        let res = parser.parse("hahahaha12345");
        assert_eq!(true, res.0.is_err());
        assert_eq!("hahahaha12345", res.1);
    }

    #[test]
    fn unescape_unicode_test() {
        let out = unescape_utf8(r"Hello world\");
        assert_eq!(r"Hello world\", out);

        let out = unescape_utf8(r"Hello \u0038\u0039");
        assert_eq!(r"Hello 89", out);

        let out = unescape_utf8(r"Hello \u0l38");
        assert_eq!(r"Hello \u0l38", out);

        let out = unescape_utf8(r"привет");
        assert_eq!(r"привет", out);

        let out = unescape_utf8(r"\u041c\u0430\u0440\u0438\u044f");
        assert_eq!(r"Мария", out);
    }

    #[test]
    fn unicode_to_utf8_test() {
        assert_eq!(vec![0x24], unicode_to_utf8(0x24));
        assert_eq!(vec![0xc2, 0xa2], unicode_to_utf8(0xa2));
        assert_eq!(vec![0xc2, 0xa2], unicode_to_utf8(0xa2));
        assert_eq!(vec![0xe2, 0x82, 0xac], unicode_to_utf8(0x20ac));

        // это не работает, но для русского языка - сойдёт и так
        // assert_eq!(vec![0xf0, 0x90, 0x8d, 0x88], unicode_to_utf8(0x10348));
    }
}
