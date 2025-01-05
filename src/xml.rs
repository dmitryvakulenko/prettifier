use super::{
    And, AnyChar, AnyExcept, BoxedParser, CharSequence, Ignore, Or, Parser, ZeroOrMore,
};
use std::prelude::v1::*;

// имя элемента xml
struct ElementName {}

impl ElementName {
    fn new() -> Self {
        Self {}
    }
}

impl Parser for ElementName {
    fn parse<'a>(&self, in_string: &'a str) -> (Result<String, ()>, &'a str) {
        if in_string.is_empty() {
            return (Err(()), in_string);
        }

        let mut iter = in_string.chars();
        let first = iter.next().unwrap();
        if !first.is_alphabetic() {
            return (Err(()), in_string);
        }

        let mut res = first.to_string();
        while let Some(ch) = iter.next() {
            if ch.is_alphanumeric() || ch == ':' || ch == '_' || ch == '-' {
                res.push(ch);
            } else {
                break;
            }
        }

        let parsed_len = res.len();
        return (Ok(res), &in_string[parsed_len..]);
    }
}

// целый элемент xml, без детей и текста
struct ElementFull<'a, M> {
    level: usize,
    mapper: M,
    parser: And<'a>,
}

impl<'a, M> ElementFull<'a, M> {
    fn new(level: usize, mapper: M) -> Self {
        let mut parser = And::new();
        parser.add_parser(CharSequence::new("<".to_string()));
        parser.add_parser(ElementName::new());
        parser.add_parser(AttributeList::new());
        parser.add_parser(BoxedParser::new(CharSequence::new("/>".to_string())));

        Self {
            level,
            mapper,
            parser,
        }
    }
}

impl<'a, M> Parser for ElementFull<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let res = self.parser.parse(in_string);
        return if let Ok(r) = res.0 {
            let r1 = (self.mapper)(&r, self.level);
            (Ok(r1), res.1)
        } else {
            res
        };
    }
}

// начало элемента
struct ElementOpen<'a, M> {
    level: usize,
    mapper: M,
    parser: And<'a>,
}

impl<'a, M> ElementOpen<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
{
    fn new(level: usize, mapper: M) -> Self {
        let mut parser = And::new();
        parser.add_parser(CharSequence::new("<".to_string()));
        parser.add_parser(ElementName::new());
        parser.add_parser(AttributeList::new());
        parser.add_parser(CharSequence::new(">".to_string()));

        Self {
            level,
            mapper,
            parser,
        }
    }
}

impl<'a, M> Parser for ElementOpen<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let res = self.parser.parse(in_string);
        return if let Ok(r) = res.0 {
            let r1 = (self.mapper)(&r, self.level);
            (Ok(r1), res.1)
        } else {
            res
        };
    }
}

// завершение элемента
struct ElementClose<'a, M> {
    level: usize,
    mapper: M,
    parser: And<'a>,
}

impl<'a, M> ElementClose<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
{
    fn new(level: usize, mapper: M) -> Self {
        let mut parser = And::new();
        parser.add_parser(CharSequence::new("</".to_string()));
        parser.add_parser(ElementName::new());
        parser.add_parser(CharSequence::new(">".to_string()));

        Self {
            parser,
            level,
            mapper,
        }
    }
}

impl<'a, M> Parser for ElementClose<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let res = self.parser.parse(in_string);
        return if let Ok(r) = res.0 {
            let r1 = (self.mapper)(&r, self.level);
            (Ok(r1), res.1)
        } else {
            res
        };
    }
}

struct ElementWithText<'a, M> {
    parser: And<'a>,
    mapper: M,
    level: usize,
}

impl<'a, M> ElementWithText<'a, M> {
    fn new(level: usize, mapper: M) -> Self
    where
        M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
    {
        let mut parser = And::new();

        let no_map = |parsed: &str, _: usize| parsed.to_string();

        parser.add_parser(ElementOpen::new(level, no_map.clone()));
        parser.add_parser(ZeroOrMore::new(AnyExcept::new("<".to_string())));
        parser.add_parser(ElementClose::new(0, no_map.clone()));

        Self {
            parser,
            mapper,
            level,
        }
    }
}

impl<'a, M> Parser for ElementWithText<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let res = self.parser.parse(in_string);
        return if let Ok(r) = res.0 {
            let r1 = (self.mapper)(&r, self.level);
            (Ok(r1), res.1)
        } else {
            res
        };
    }
}

struct ElementsSet<'a> {
    parser: And<'a>,
}

impl<'a> ElementsSet<'a> {
    fn new<M>(level: usize, mapper: M) -> Self
    where
        M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
    {
        let mut parser = And::new();
        let mut or = Or::new();
        or.add_parser(ElementWithText::new(level, mapper.clone()));
        or.add_parser(ElementFull::new(level, mapper.clone()));
        or.add_parser(ElementAny::new(level, mapper.clone()));
        parser.add_parser(or);

        let is_space = |ch: char| ch.is_whitespace();
        parser.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(is_space))));

        Self { parser }
    }
}

impl<'a> Parser for ElementsSet<'a> {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        self.parser.parse(in_string)
    }
}

struct ElementAny<M> {
    level: usize,
    mapper: M,
}

impl<'a, M> ElementAny<M> {
    fn new(level: usize, mapper: M) -> Self {
        Self { level, mapper }
    }
}

impl<'a, M> Parser for ElementAny<M>
where
    M: for<'c> Fn(&'c str, usize) -> String + 'a + Copy,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let mut parser = And::new();

        let is_space = |ch: char| ch.is_ascii_whitespace();

        parser.add_parser(ElementOpen::new(self.level, self.mapper.clone()));
        parser.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(is_space))));
        parser.add_parser(ZeroOrMore::new(ElementsSet::new(
            self.level + 1,
            self.mapper.clone(),
        )));
        parser.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(is_space))));
        parser.add_parser(ElementClose::new(self.level, self.mapper.clone()));

        parser.parse(in_string)
    }
}

struct ElementXml<'a, M> {
    parser: And<'a>,
    mapper: M,
}

impl<'a, M> ElementXml<'a, M> {
    fn new(mapper: M) -> Self {
        let mut parser = And::new();
        parser.add_parser(CharSequence::new("<?".to_string()));
        parser.add_parser(ElementName::new());

        let zero_or_more = ZeroOrMore::new(AnyChar::new(|ch| {
            return ch != '?' && ch != '/' && ch != '>';
        }));
        parser.add_parser(zero_or_more);

        parser.add_parser(CharSequence::new("?>".to_string()));

        Self { parser, mapper }
    }
}

impl<'a, M> Parser for ElementXml<'a, M>
where
    M: for<'c> Fn(&'c str, usize) -> String + 'a,
{
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let res = self.parser.parse(in_string);
        return if let Ok(r) = res.0 {
            let r1 = (self.mapper)(&r, 0);
            (Ok(r1), res.1)
        } else {
            res
        };
    }
}

struct Attribute<'a> {
    parser: And<'a>,
}

impl<'a> Attribute<'a> {
    fn new() -> Self {
        let mut parser = And::new();

        parser.add_parser(ElementName::new());
        parser.add_parser(CharSequence::new("=\"".to_string()));

        let zero_or_more = ZeroOrMore::new(AnyChar::new(|ch| ch != '"'));
        parser.add_parser(zero_or_more);

        parser.add_parser(CharSequence::new("\"".to_string()));

        Self { parser }
    }
}

impl<'a> Parser for Attribute<'a> {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        let res = self.parser.parse(in_string);
        return if let Ok(r) = res.0 {
            (Ok(" ".to_string() + &r), res.1)
        } else {
            res
        };
    }
}

struct AttributeList<'a> {
    parser: ZeroOrMore<And<'a>>,
}

impl<'a> AttributeList<'a> {
    fn new() -> Self {
        let is_space = |ch: char| ch.is_whitespace();

        let mut parser = And::new();
        parser.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(is_space.clone()))));
        parser.add_parser(Attribute::new());
        parser.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(is_space.clone()))));

        Self {
            parser: ZeroOrMore::new(parser),
        }
    }
}

impl<'a> Parser for AttributeList<'a> {
    fn parse<'b>(&self, in_string: &'b str) -> (Result<String, ()>, &'b str) {
        self.parser.parse(in_string)
    }
}

pub fn format(body: &str, ident: usize) -> Result<String, String> {
    let mapper = move |parsed: &str, level: usize| " ".repeat(ident * level) + parsed + "\n";
    let mut parser = And::new();

    let is_space = |ch: char| ch.is_whitespace();
    parser.add_parser(ElementXml::new(mapper));
    parser.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(is_space))));
    parser.add_parser(ElementAny::new(0, mapper));
    parser.add_parser(Ignore::new(ZeroOrMore::new(AnyChar::new(is_space))));

    let res = parser.parse(body);
    return if let Ok(parsed) = res.0 {
        Ok(parsed)
    } else {
        Ok(body.to_string())
    };
}

#[cfg(test)]
mod tests {
    use super::{
        Attribute, AttributeList, ElementAny, ElementClose, ElementFull, ElementName, ElementOpen,
        ElementWithText, ElementXml, Parser, format
    };

    #[test]
    fn parse_element_name() {
        let parser = ElementName::new();

        let res = parser.parse("hello");
        assert_eq!("hello", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("1hello");
        assert_eq!(Err(()), res.0);
        assert_eq!("1hello", res.1);

        let res = parser.parse("hello_world");
        assert_eq!("hello_world", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("std:hello_world");
        assert_eq!("std:hello_world", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("hello attr=");
        assert_eq!("hello", res.0.unwrap());
        assert_eq!(" attr=", res.1);
    }

    #[test]
    fn parse_attribute() {
        let parser = Attribute::new();

        let res = parser.parse("hello=\"1\"");
        assert_eq!(" hello=\"1\"", res.0.unwrap());
        assert_eq!("", res.1);
    }

    #[test]
    fn parse_attribute_list() {
        let parser = AttributeList::new();

        let res = parser.parse(" hello=\"1\" test=\"aaabbb\"");
        assert_eq!(" hello=\"1\" test=\"aaabbb\"", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("\n hello=\"1\"\n\n\n test=\"aaabbb\"");
        assert_eq!(" hello=\"1\" test=\"aaabbb\"", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("\n hello=\"1\"\n\n\n test=\"aaabbb\" \t\t\n");
        assert_eq!(" hello=\"1\" test=\"aaabbb\"", res.0.unwrap());
        assert_eq!("", res.1);
    }

    #[test]
    fn parse_full_element() {
        let parser = ElementFull::new(0, |parsed: &str, _: usize| parsed.to_string());
        let res = parser.parse("<body>");
        assert_eq!(true, res.0.is_err());
        assert_eq!("<body>", res.1);

        let res = parser.parse("<body/>");
        assert_eq!("<body/>", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("<body aaa=\"bbb\"/>");
        assert_eq!("<body aaa=\"bbb\"/>", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("<body aaa=\"bbb\"");
        assert_eq!(true, res.0.is_err());
        assert_eq!("<body aaa=\"bbb\"", res.1);
    }

    #[test]
    fn parse_element_open() {
        let parser = ElementOpen::new(0, |parsed: &str, _: usize| parsed.to_string());
        let res = parser.parse("<body>");
        assert_eq!("<body>", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("<body aaa=\"bbb\">");
        assert_eq!("<body aaa=\"bbb\">", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("<body aaa=\"bbb\"/>");
        assert_eq!(true, res.0.is_err());
        assert_eq!("<body aaa=\"bbb\"/>", res.1);
    }

    #[test]
    fn parse_element_close() {
        let parser = ElementClose::new(0, |parsed: &str, _: usize| parsed.to_string());
        let res = parser.parse("</body>");
        assert_eq!("</body>", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("</body aaa=\"bbb\">");
        assert_eq!(true, res.0.is_err());
        assert_eq!("</body aaa=\"bbb\">", res.1);
    }

    #[test]
    fn parse_element_with_text() {
        let parser = ElementWithText::new(0, |parsed: &str, level: usize| {
            " ".repeat(level * 4) + parsed + "\n"
        });
        let res = parser.parse("<body></body>");
        assert_eq!("<body></body>\n", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("<body>aaabbb</body>");
        assert_eq!("<body>aaabbb</body>\n", res.0.unwrap());
        assert_eq!("", res.1);
    }

    #[test]
    fn parse_element_with_children() {
        let parser = ElementAny::new(0, |parsed: &str, level: usize| {
            " ".repeat(level * 4) + parsed + "\n"
        });

        let res = parser.parse("<body><inner></inner></body>");
        assert_eq!("<body>\n    <inner></inner>\n</body>\n", res.0.unwrap());
        assert_eq!("", res.1);

        let res = parser.parse("<body><inner>a</inner><inner>b</inner></body>");
        assert_eq!(
            "<body>\n    <inner>a</inner>\n    <inner>b</inner>\n</body>\n",
            res.0.unwrap()
        );
        assert_eq!("", res.1);

        let res = parser.parse("<body><inner>a</inner><inner>b</inner>");
        assert_eq!(true, res.0.is_err());
        assert_eq!("<body><inner>a</inner><inner>b</inner>", res.1);

        let res = parser.parse("<body><inner>a</inner><inner>b</inner></body>");
        assert_eq!(
            "<body>\n    <inner>a</inner>\n    <inner>b</inner>\n</body>\n",
            res.0.unwrap()
        );
    }

    #[test]
    fn space_ignoring() {
        let parser = ElementAny::new(0, |parsed: &str, level: usize| {
            " ".repeat(level * 4) + parsed + "\n"
        });
        let input = "<body>    <node>test</node>          </body>\n";
        let expect = "<body>\n    <node>test</node>\n</body>\n";
        let res = parser.parse(input);
        assert_eq!(expect, res.0.unwrap());
    }

    #[test]
    fn xml_prefix_element() {
        let input = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";
        let expect = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

        let parser = ElementXml::new(|parsed: &str, _: usize| parsed.to_string() + "\n");
        let res = parser.parse(input);
        assert_eq!(expect, res.0.unwrap());
    }

    #[test]
    fn full_xml() {
        let input = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><body><node>test</node><node>test</node></body>";
        let expect = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<body>\n    <node>test</node>\n    <node>test</node>\n</body>\n";

        let res = format(input, 4);
        assert_eq!(expect, res.unwrap());
    }

    #[test]
    fn soap_xml() {
        let input = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><SOAP-ENV:Envelope xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:awsse=\"http://xml.amadeus.com/2010/06/Session_v3\" xmlns:wsa=\"http://www.w3.org/2005/08/addressing\"></SOAP-ENV:Envelope>";
        let expect = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<SOAP-ENV:Envelope xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:awsse=\"http://xml.amadeus.com/2010/06/Session_v3\" xmlns:wsa=\"http://www.w3.org/2005/08/addressing\">\n</SOAP-ENV:Envelope>\n";

        let res = format(input, 4);
        assert_eq!(expect, res.unwrap());
    }

    #[test]
    fn real_soap_xml() {
        let content = include_str!("testdata/request.xml");
        let res = format(&content, 4);
        assert_eq!(true, res.is_ok());

        let content = include_str!("testdata/response.xml");
        let res = format(&content, 4);
        assert_eq!(true, res.is_ok());
    }

    #[test]
    fn complex_soap_xml() {
        let content = include_str!("testdata/sirena_request.xml");
        let res = format(&content, 4);
        assert_eq!(true, res.is_ok());

        let content = include_str!("testdata/sirena_response.xml");
        assert_eq!(res.unwrap(), content);
    }
}
