use std::prelude::v1::*;

use super::{
    unescape_utf8, And, AnyChar, AnyExcept, BoxedParser, CharSequence, Ignore, Or, Parser,
    ZeroOrMore, ZeroOrOne,
};

pub fn format(body: &str, ident: usize) -> Result<String, String> {
    let mapper = move |parsed: &str, level: usize| " ".repeat(ident * level) + parsed + "\n";
    let parser = Value::new(&mapper, 0);
    let res = parser.parse(body);

    if let Ok(parsed) = res.0 {
        Ok(parsed)
    } else {
        Ok(body.to_string())
    }
}

// ключ в объекте
struct Key<'a> {
    p: BoxedParser<'a>,
}

impl<'a> Key<'a> {
    fn new() -> Self {
        let start = CharSequence::new(String::from("\""));
        let end = CharSequence::new(String::from("\""));
        let content = |ch: char| ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '@';

        let mut obj = And::new();
        obj.add_parser(start);
        obj.add_parser(AnyChar::new(content));
        obj.add_parser(end);

        Self {
            p: BoxedParser::new(obj),
        }
    }
}

impl<'a> Parser for Key<'a> {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        self.p.parse(in_string)
    }
}

// значение в массиве или объекте
struct SpecialValue {}

impl SpecialValue {
    fn new() -> Self {
        Self {}
    }
}

impl Parser for SpecialValue {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut p = Or::new();
        p.add_parser(CharSequence::new(String::from("true")));
        p.add_parser(CharSequence::new(String::from("TRUE")));
        p.add_parser(CharSequence::new(String::from("false")));
        p.add_parser(CharSequence::new(String::from("FALSE")));
        p.add_parser(CharSequence::new(String::from("null")));
        p.add_parser(CharSequence::new(String::from("NULL")));

        p.parse(in_string)
    }
}

// строка, нужно только для исправления кодировки utf8
struct StringValue {}

impl StringValue {
    fn new() -> Self {
        Self {}
    }
}

impl Parser for StringValue {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let start = CharSequence::new(String::from("\""));
        let end = CharSequence::new(String::from("\""));
        let mut p = And::new();
        p.add_parser(start);
        p.add_parser(ZeroOrOne::new(AnyExcept::new(String::from("\""))));
        p.add_parser(end);

        let res = p.parse(in_string);
        if let Ok(r) = res.0 {
            (Ok(unescape_utf8(&r)), res.1)
        } else {
            res
        }
    }
}

// значение в массиве или объекте
struct Value<'a, M> {
    mapper: &'a M,
    level: usize,
}

impl<'a, M> Value<'a, M> {
    fn new(mapper: &'a M, level: usize) -> Self {
        Self { mapper, level }
    }
}

impl<'a, M> Parser for Value<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + Clone,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut p = Or::new();
        p.add_parser(StringValue::new());
        p.add_parser(AnyChar::new(|ch: char| ch.is_ascii_digit() || ch == '.'));
        p.add_parser(Object::new(self.mapper, self.level));
        p.add_parser(Array::new(self.mapper, self.level));
        p.add_parser(SpecialValue::new());

        p.parse(in_string)
    }
}

// поле со скалярным значением, не массив и не объект
struct KeyAndValue<'a, M> {
    mapper: &'a M,
    level: usize,
}

impl<'a, M> KeyAndValue<'a, M> {
    fn new(mapper: &'a M, level: usize) -> Self {
        Self { level, mapper }
    }
}

impl<'a, M> Parser for KeyAndValue<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + Clone,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut p = And::new();
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(Key::new());
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(CharSequence::new(String::from(":")));
        p.add_parser(ZeroOrMore::new(AnyChar::new(|ch: char| ch.is_whitespace())));
        p.add_parser(Value::new(self.mapper, self.level));
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(ZeroOrOne::new(CharSequence::new(String::from(","))));

        let res = p.parse(in_string);
        if let Ok(r) = res.0 {
            let r1 = (self.mapper)(&r, self.level);
            (Ok(r1), res.1)
        } else {
            res
        }
    }
}

struct Object<'a, M> {
    mapper: &'a M,
    level: usize,
}

impl<'a, M> Object<'a, M> {
    fn new(mapper: &'a M, level: usize) -> Self {
        Self { mapper, level }
    }
}

impl<'a, M> Parser for Object<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + Clone,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut p = And::new();
        p.add_parser(CharSequence::new(String::from("{")));
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(ObjectContent::new(self.mapper, self.level + 1));
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(CharSequence::new(String::from("}")));

        let res = p.parse(in_string);
        return if let Ok(r) = res.0 {
            let res_len = r.len();
            let r1 = if res_len > 2 {
                let last_str = &(self.mapper)("}", self.level);
                String::from(&r[0..res_len - 1]) + &last_str[0..last_str.len() - 1]
            } else {
                r
            };
            (Ok(r1), res.1)
        } else {
            res
        };
    }
}

struct ObjectContent<'a, M> {
    mapper: &'a M,
    level: usize,
}

// список полей
impl<'a, M> ObjectContent<'a, M> {
    fn new(mapper: &'a M, level: usize) -> Self {
        Self { mapper, level }
    }
}

impl<'a, M> Parser for ObjectContent<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + Clone,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut p = And::new();
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(ZeroOrMore::new(KeyAndValue::new(self.mapper, self.level)));
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));

        let res = p.parse(in_string);
        return if let Ok(r) = res.0 {
            let res_len = r.len();
            let r1 = if res_len != 0 {
                String::from("\n") + &r
            } else {
                r
            };
            (Ok(r1), res.1)
        } else {
            res
        };
    }
}

struct ValueAndComma<'a, M> {
    mapper: &'a M,
    level: usize,
}

// список полей
impl<'a, M> ValueAndComma<'a, M> {
    fn new(mapper: &'a M, level: usize) -> Self {
        Self { mapper, level }
    }
}

impl<'a, M> Parser for ValueAndComma<'a, M>
where
    M: (for<'c> Fn(&'c str, usize) -> String) + Clone,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut p = And::new();
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(Value::new(self.mapper, self.level));
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(ZeroOrOne::new(CharSequence::new(String::from(","))));

        let res = p.parse(in_string);
        if let Ok(r) = res.0 {
            (Ok((self.mapper)(&r, self.level)), res.1)
        } else {
            res
        }
    }
}

struct ArrayContent<'a, M> {
    mapper: &'a M,
    level: usize,
}

// список полей
impl<'a, M> ArrayContent<'a, M> {
    fn new(mapper: &'a M, level: usize) -> Self {
        Self { mapper, level }
    }
}

impl<'a, M> Parser for ArrayContent<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + Clone,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut p = And::new();
        p.add_parser(ZeroOrMore::new(ValueAndComma::new(self.mapper, self.level)));

        let res = p.parse(in_string);
        if let Ok(r) = res.0 {
            let res_len = r.len();
            let r1 = if res_len != 0 {
                String::from("\n") + &r
            } else {
                r
            };
            (Ok(r1), res.1)
        } else {
            res
        }
    }
}

struct Array<'a, M> {
    mapper: &'a M,
    level: usize,
}

impl<'a, M> Array<'a, M> {
    fn new(mapper: &'a M, level: usize) -> Self {
        Self { mapper, level }
    }
}

impl<'a, M> Parser for Array<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + Clone,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut p = And::new();
        p.add_parser(CharSequence::new(String::from("[")));
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));
        p.add_parser(ArrayContent::new(self.mapper, self.level + 1));
        p.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(|ch: char| {
            ch.is_whitespace()
        }))));

        p.add_parser(CharSequence::new(String::from("]")));

        let res = p.parse(in_string);
        return if let Ok(r) = res.0 {
            let res_len = r.len();
            let r1 = if res_len > 2 {
                let last_str = &(self.mapper)("]", self.level);
                String::from(&r[0..res_len - 1]) + &last_str[0..last_str.len() - 1]
            } else {
                r
            };
            (Ok(r1), res.1)
        } else {
            res
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ArrayContent, Key, KeyAndValue, ObjectContent, Value, ValueAndComma, Parser
    };

    #[test]
    fn key() {
        let p = Key::new();

        let input = "\"aaa\": 234234";
        let res = p.parse(&input);
        assert_eq!("\"aaa\"", res.0.unwrap());
        assert_eq!(": 234234", res.1);

        let input = "\"aaa+32\": 234234";
        let res = p.parse(&input);
        assert_eq!(false, res.0.is_ok());
        assert_eq!("\"aaa+32\": 234234", res.1);
    }

    #[test]
    fn value_and_comma() {
        let mapper = move |parsed: &str, level: usize| " ".repeat(4 * level) + parsed + "\n";
        let p = ValueAndComma::new(&mapper, 1);

        let input = "\"aaaa\",";
        let res = p.parse(&input);
        assert_eq!("    \"aaaa\",\n", res.0.unwrap());
        assert_eq!("", res.1);

        let input = "        \t\t\"aaaa\"   ,";
        let res = p.parse(&input);
        assert_eq!("    \"aaaa\",\n", res.0.unwrap());
        assert_eq!("", res.1);
    }

    #[test]
    fn value() {
        let mapper = move |parsed: &str, level: usize| " ".repeat(4 * level) + parsed + "\n";
        let p = Value::new(&mapper, 0);

        let input = "\"aklsdkj33+++390  sldk sdf sdf ''\"";
        let res = p.parse(&input);
        assert_eq!(input, res.0.unwrap());
        assert_eq!("", res.1);

        let input = "12345";
        let res = p.parse(&input);
        assert_eq!(input, res.0.unwrap());
        assert_eq!("", res.1);

        let input = "234  ";
        let res = p.parse(&input);
        assert_eq!("234", res.0.unwrap());
        assert_eq!("  ", res.1);

        let input = "{}";
        let res = p.parse(&input);
        assert_eq!("{}", res.0.unwrap());
        assert_eq!("", res.1);

        let input = "[]";
        let res = p.parse(&input);
        assert_eq!("[]", res.0.unwrap());
        assert_eq!("", res.1);

        let input = "{\r\n\"aaa\": \"bbb\"\r\n}";
        let res = p.parse(&input);
        assert_eq!("{\n    \"aaa\": \"bbb\"\n}", res.0.unwrap());
        assert_eq!("", res.1);

        let input = "[1, 2, 3, {\"aaa\": 1}]";
        let res = p.parse(&input);
        assert_eq!(
            "[\n    1,\n    2,\n    3,\n    {\n        \"aaa\": 1\n    }\n]",
            res.0.unwrap()
        );
        assert_eq!("", res.1);

        let input = "{\"a\": 1, \"b\": 2, \"c\": 3, \"d\": {\"aaa\": 1}}";
        let res = p.parse(&input);
        assert_eq!(
            "{\n    \"a\": 1,\n    \"b\": 2,\n    \"c\": 3,\n    \"d\": {\n        \"aaa\": 1\n    }\n}",
            res.0.unwrap()
        );
        assert_eq!("", res.1);

        let input = "{\"total\":1,\"errors\":null}";
        let res = p.parse(&input);
        assert_eq!(
            "{\n    \"total\":1,\n    \"errors\":null\n}",
            res.0.unwrap()
        );
        assert_eq!("", res.1);
    }

    #[test]
    fn full_field_with_value() {
        let ident = 4;
        let mapper = move |parsed: &str, level: usize| " ".repeat(ident * level) + parsed + "\n";
        let p = KeyAndValue::new(&mapper, 1);

        let input = "\"key\"   : \"value\"";
        let res = p.parse(&input);
        assert_eq!("    \"key\": \"value\"\n", res.0.unwrap());
        assert_eq!("", res.1);

        let input = "   \"key\"   : \"value\"    ,";
        let res = p.parse(&input);
        assert_eq!("    \"key\": \"value\",\n", res.0.unwrap());
        assert_eq!("", res.1);

        let input = "\"key\" : { \"key2\": 1234 }";
        let res = p.parse(&input);
        assert_eq!(
            "    \"key\": {\n        \"key2\": 1234\n    }\n",
            res.0.unwrap()
        );
        assert_eq!("", res.1);

        let input = "\n          \"key\" : { \"key2\": 1234 }";
        let res = p.parse(&input);
        assert_eq!(
            "    \"key\": {\n        \"key2\": 1234\n    }\n",
            res.0.unwrap()
        );
        assert_eq!("", res.1);
    }

    #[test]
    fn object_content() {
        let ident = 4;
        let mapper = move |parsed: &str, level: usize| " ".repeat(ident * level) + parsed + "\n";
        let p = ObjectContent::new(&mapper, 1);

        let input = " \"key\" : \"value\" ";
        let res = p.parse(&input);
        assert_eq!("\n    \"key\": \"value\"\n", res.0.unwrap());
        assert_eq!("", res.1);

        let input = " \"key\" : \"value\" ,    \"key2\" : \"value2\"";
        let res = p.parse(&input);
        assert_eq!(
            "\n    \"key\": \"value\",\n    \"key2\": \"value2\"\n",
            res.0.unwrap()
        );
        assert_eq!("", res.1);
    }

    #[test]
    fn array_content() {
        let ident = 4;
        let mapper = move |parsed: &str, level: usize| " ".repeat(ident * level) + parsed + "\n";
        let p = ArrayContent::new(&mapper, 1);

        let input = "1, 2, 3";
        let res = p.parse(&input);
        assert_eq!("\n    1,\n    2,\n    3\n", res.0.unwrap());
        assert_eq!("", res.1);
    }
}