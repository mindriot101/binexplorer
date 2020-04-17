use anyhow::Result;
use pest::Parser;
use pest_derive::Parser;

use crate::{MultipleParseChar, ParseChar};

#[derive(Parser)]
#[grammar = "parser.pest"]
struct BinParser;

pub(crate) fn parse_input<S: AsRef<str>>(input: S) -> Result<Vec<MultipleParseChar>> {
    let mut out = Vec::new();
    let pairs = BinParser::parse(Rule::line, input.as_ref())?;

    for pair in pairs {
        let rules: Vec<_> = pair.into_inner().collect();
        match rules[0].as_rule() {
            Rule::digits => {
                assert_eq!(rules.len(), 2);
                let span = rules[0].as_span();
                // Unwrap is safe as the type is parsed as digits
                let count: usize = span.as_str().parse().unwrap();

                match rules[1].as_rule() {
                    Rule::I8 => out.push(MultipleParseChar::many(ParseChar::I8, count)),
                    Rule::U8 => out.push(MultipleParseChar::many(ParseChar::U8, count)),
                    Rule::digit | Rule::digits | Rule::multiple | Rule::WHITESPACE | Rule::line => {
                        unreachable!()
                    }
                }
            }
            Rule::I8 => out.push(MultipleParseChar::single(ParseChar::I8)),
            Rule::U8 => out.push(MultipleParseChar::single(ParseChar::U8)),

            // Other rules cannot happen
            Rule::digit | Rule::multiple | Rule::WHITESPACE | Rule::line => unreachable!(),
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_parsing() {
        let text = "b";
        let result = parse_input(text).unwrap();
        assert_eq!(result, vec![MultipleParseChar::single(ParseChar::I8)]);
    }

    #[test]
    fn test_multiple_parsing() {
        let text = "Bb";
        let result = parse_input(text).unwrap();
        assert_eq!(
            result,
            vec![
                MultipleParseChar::single(ParseChar::U8),
                MultipleParseChar::single(ParseChar::I8)
            ]
        );
    }

    #[test]
    fn test_repeated_parsing() {
        let text = "20b";
        let result = parse_input(text).unwrap();
        assert_eq!(result, vec![MultipleParseChar::many(ParseChar::I8, 20),]);
    }

    #[test]
    fn test_parsing_with_spaces() {
        let text = "20b 5B";
        let result = parse_input(text).unwrap();
        assert_eq!(
            result,
            vec![
                MultipleParseChar::many(ParseChar::I8, 20),
                MultipleParseChar::many(ParseChar::U8, 5)
            ]
        );
    }

    #[test]
    fn test_long_parsing() {
        let text = "20b5Bb B";
        let result = parse_input(text).unwrap();
        assert_eq!(
            result,
            vec![
                MultipleParseChar::many(ParseChar::I8, 20),
                MultipleParseChar::many(ParseChar::U8, 5),
                MultipleParseChar::single(ParseChar::I8),
                MultipleParseChar::single(ParseChar::U8),
            ]
        );
    }
}
