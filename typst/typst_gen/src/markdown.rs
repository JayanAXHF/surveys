use log::{debug, error, info, warn};
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::fmt::{Debug, Display};
use thiserror::Error;

pub type Result<T, E = MdParseError> = std::result::Result<T, E>;

#[derive(Error, Debug, Diagnostic)]
#[error("An error occurred:")]
#[diagnostic(
    code(oops::my::bad),
    url(docsrs),
    help("try doing it better next time?")
)]
pub struct MarkdownParser {
    #[source_code]
    src: NamedSource<String>,
    #[label("This bit here")]
    bad_bit: SourceSpan,
}

#[derive(Debug, Default)]
pub struct Context {
    prev_tk: Prev,
    top_level: bool,
}

impl Context {
    fn set_prev_tk(mut self, prev_tk: Prev) -> Self {
        self.prev_tk = prev_tk;
        self
    }

    fn set_top_level(mut self, top_level: bool) -> Self {
        self.top_level = top_level;
        self
    }
}

#[derive(Debug, Default, PartialEq)]
enum Prev {
    Whitespace,
    #[default]
    Newline,
    Escape,
    Any,
}

#[derive(Debug, Diagnostic, Error)]
#[error("welp")]
#[diagnostic()]
pub enum ParseError {
    #[error("An IO error occurred: {0}")]
    IoError(#[from] std::io::Error),
    #[error("An error occurred while parsing: {0}")]
    ParseError(#[from] MdParseError),
}

#[derive(Debug, Diagnostic, Error)]
#[error("welp")]
#[diagnostic(severity(Warning))]
pub enum MdParseError {
    #[error("There was a problem initializing the logger: {0}")]
    LoggerInitError(#[from] fern::InitError),
    #[error("There was an unmatched delimiter: ")]
    UnmatchedDelimiter,
}

trait Stream {
    fn next(&mut self) -> Option<char>;
    fn peek(&self) -> Option<char>;
    fn consume_until(&mut self, predicate: impl Fn(char) -> bool) -> Option<String>;
    fn consume_pattern<'a>(&mut self, pattern: &'a str) -> Option<&'a str>;
}

impl Stream for String {
    fn next(&mut self) -> Option<char> {
        let c = self.chars().next()?;
        *self = self[c.len_utf8()..].to_string();
        Some(c)
    }
    fn peek(&self) -> Option<char> {
        self.chars().next()
    }
    fn consume_until(&mut self, predicate: impl Fn(char) -> bool) -> Option<String> {
        let mut buf = String::new();
        while let Some(c) = self.peek() {
            if predicate(c) {
                break;
            }
            buf.push(c);
            self.next();
        }
        if buf.is_empty() { None } else { Some(buf) }
    }

    fn consume_pattern<'a>(&mut self, pattern: &'a str) -> Option<&'a str> {
        if self.starts_with(pattern) {
            let pat_len = pattern.len();
            *self = self[pat_len..].to_string();
            return Some(pattern);
        }
        None
    }
}

impl Stream for &str {
    fn next(&mut self) -> Option<char> {
        let mut iter = self.chars();
        let c = iter.next()?;
        let len = c.len_utf8();
        *self = &self[len..];
        debug!("next char: {:?}", c);
        Some(c)
    }

    fn peek(&self) -> Option<char> {
        self.chars().next()
    }

    fn consume_until(&mut self, predicate: impl Fn(char) -> bool) -> Option<String> {
        let mut buf = String::new();

        while let Some(c) = self.peek() {
            if predicate(c) {
                break;
            }
            buf.push(c);
            self.next();
        }

        if buf.is_empty() { None } else { Some(buf) }
    }

    fn consume_pattern<'a>(&mut self, pattern: &'a str) -> Option<&'a str> {
        if self.starts_with(pattern) {
            let pat_len = pattern.len();
            *self = &self[pat_len..];
            return Some(pattern);
        }
        None
    }
}

impl MarkdownParser {
    pub fn new(src: String) -> Self {
        let src = NamedSource::new("test", src);
        Self {
            src,
            bad_bit: SourceSpan::new(0.into(), 0),
        }
    }

    pub fn parse(&self) -> Result<TokenTree, MdParseError> {
        let ctx = Context::default().set_top_level(true);
        self.parse_with_ctx(self.src.inner(), ctx)
    }
    pub fn parse_with_ctx(&self, mut src: &str, ctx: Context) -> Result<TokenTree, MdParseError> {
        info!("parsing src {}", src);
        use Prev::*;
        let mut tokens = Vec::new();
        let Context {
            mut prev_tk,
            top_level,
        } = ctx;
        let mut text_buf = String::new();

        while !src.is_empty() {
            let next_prev = match src.peek() {
                Some(c) => match c {
                    '\n' => Newline,
                    c if c.is_ascii_whitespace() => Whitespace,
                    '\\' => Escape,
                    _ => Any,
                },
                None => Any,
            };
            let Some(next_tk) = src.peek() else {
                break;
            };
            let next_tk = next_tk.to_string();
            let prefix = Delimiter::match_prefix(src);
            if prefix.is_some() {
                debug!("prefix: {:?}", prefix);
                tokens.push(Token::Text(std::mem::take(&mut text_buf)));
            }
            match prefix {
                Some(tk) if tk.is_simple_pattern() => {
                    // warn!("pattern matched: {:?}", tk);
                    let token = self.parse_simple_pattern(&mut src, &tk.to_string())?;
                    tokens.push(token);
                }
                Some(tk) if tk.is_top_lvl_pat() && top_level && prev_tk == Prev::Newline => {
                    info!("passing src: {:?}", src);
                    let token = self.parse_top_lvl(&mut src, tk)?;
                    tokens.push(token);
                    src.next();
                    continue;
                }
                None => {
                    text_buf.push_str(&next_tk);
                }
                _ => {}
            };
            src.next();
            prev_tk = next_prev;
        }
        tokens.push(Token::Text(std::mem::take(&mut text_buf)));

        Ok(TokenTree(tokens))
    }

    fn parse_simple_pattern(&self, src: &mut &str, pattern: &str) -> Result<Token, MdParseError> {
        info!(target: "parse_simple_pattern()",  "pattern: {pattern:?}");
        info!(target: "parse_simple_pattern()", "src: {src:?}");
        let ctx = Context::default()
            .set_prev_tk(Prev::Any)
            .set_top_level(false);
        match Delimiter::match_prefix(src) {
            Some(d) if d.is_strong() => {
                src.consume_pattern(&d.to_string());
                let Some(inner) = src.consume_until(|c| c == d.to_string().chars().next().unwrap())
                else {
                    return Ok(Token::Bold(TokenTree(vec![])));
                };
                let inner = inner.trim().to_string();
                let inner_tt = self.parse_with_ctx(&inner, ctx)?;
                src.consume_pattern(&d.to_string());
                Ok(Token::Bold(inner_tt))
            }
            Some(d) if d.is_emph() => {
                info!(target: "simple match emph", "matched italic text: {:?}", src);
                src.consume_pattern(&d.to_string());
                let Some(inner) = src.consume_until(|c| c == d.to_string().chars().next().unwrap())
                else {
                    return Ok(Token::Italic(TokenTree(vec![])));
                };
                let inner = inner.trim().to_string();
                let inner_tt = self.parse_with_ctx(&inner, ctx)?;
                src.consume_pattern(&d.to_string());
                info!("new src : {:?}", src);
                Ok(Token::Italic(inner_tt))
            }
            Some(d) if d.is_inline_code() => {
                src.consume_pattern(&d.to_string());
                let inner = src.consume_until(|c| c == d.to_string().chars().next().unwrap());
                src.consume_pattern(&d.to_string());
                Ok(Token::InlineCode(inner.unwrap_or_default()))
            }
            Some(d) if d.is_strike() => {
                src.consume_pattern(&d.to_string());
                let Some(inner) = src.consume_until(|c| c == d.to_string().chars().next().unwrap())
                else {
                    return Ok(Token::Bold(TokenTree(vec![])));
                };
                let inner = inner.trim().to_string();
                let inner_tt = self.parse_with_ctx(&inner, ctx)?;
                src.consume_pattern(&d.to_string());
                Ok(Token::Strike(inner_tt))
            }
            //NOTE: This is unreachable as the other delimiters are
            // not simple patterns
            _ => {
                warn!("pattern not matched: {:?}", pattern);
                unreachable!()
            }
        }
    }

    fn parse_top_lvl(&self, src: &mut &str, prefix: Delimiter) -> Result<Token, MdParseError> {
        info!("parsing top level pattern: {}", prefix);
        match prefix {
            Delimiter::Heading => {
                let Some(lvl) = src.consume_until(|c| c != '#').map(|s| s.len()) else {
                    return Ok(Token::Heading {
                        level: 1,
                        text: TokenTree(vec![]),
                    });
                };

                let Some(inner) = src.consume_until(|c| c == '\n') else {
                    return Ok(Token::Heading {
                        level: lvl as u8,
                        text: TokenTree(vec![]),
                    });
                };
                let inner = inner.trim();
                info!("inner: {}", inner);
                let inner_tt =
                    self.parse_with_ctx(inner, Context::default().set_prev_tk(Prev::Whitespace))?;
                Ok(Token::Heading {
                    level: lvl as u8,
                    text: inner_tt,
                })
            }
            Delimiter::CodeBlock => {
                info!("src: {}", src);
                *src = &src[3..];
                let lang = src.consume_until(|c| c == '\n');
                let cb_end = src.find("```").unwrap_or(src.len() - 1);
                let inner = src[..cb_end].trim();
                Ok(Token::CodeBlock {
                    code: inner.to_string(),
                    language: lang,
                })
            }
            _ => {
                unreachable!()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenTree(Vec<Token>);

#[derive(Debug, Clone)]
pub enum Token {
    Italic(TokenTree),
    Strike(TokenTree),
    InlineCode(String),
    Text(String),
    Bold(TokenTree),
    CodeBlock {
        code: String,
        language: Option<String>,
    },
    Heading {
        level: u8,
        text: TokenTree,
    },
    Link {
        text: Box<Token>,
        url: Box<Token>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Delimiter {
    BoldA,
    BoldB,
    ItalicA,
    ItalicB,
    Break,
    Heading,
    LinkStart,
    LinkEnd,
    Strike,
    UrlBlkStart,
    UrlBlkEnd,
    CodeBlock,
    InlineCodeBlock,
}

impl Display for Delimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Delimiter::BoldA => "**",
            Delimiter::BoldB => "__",
            Delimiter::ItalicA => "*",
            Delimiter::ItalicB => "_",
            Delimiter::Break => "---",
            Delimiter::Heading => "#",
            Delimiter::LinkStart => "[",
            Delimiter::LinkEnd => "]",
            Delimiter::Strike => "~~",
            Delimiter::UrlBlkStart => "(",
            Delimiter::UrlBlkEnd => ")",
            Delimiter::CodeBlock => "```",
            Delimiter::InlineCodeBlock => "`",
        };
        write!(f, "{}", s)
    }
}

impl Delimiter {
    pub(super) fn matches_str(s: &str) -> Option<Delimiter> {
        let s = s.trim();
        match s {
            "**" => Some(Delimiter::BoldA),
            "__" => Some(Delimiter::BoldB),
            "*" => Some(Delimiter::ItalicA),
            "_" => Some(Delimiter::ItalicB),
            "---" => Some(Delimiter::Break),
            "#" => Some(Delimiter::Heading),
            "[" => Some(Delimiter::LinkStart),
            "]" => Some(Delimiter::LinkEnd),
            "~" => Some(Delimiter::Strike),
            "(" => Some(Delimiter::UrlBlkStart),
            ")" => Some(Delimiter::UrlBlkEnd),
            "```" => Some(Delimiter::CodeBlock),
            "`" => Some(Delimiter::InlineCodeBlock),
            _ => None,
        }
    }
    #[track_caller]
    pub(super) fn match_prefix(s: &str) -> Option<Delimiter> {
        let caller_location = std::panic::Location::caller();
        match s {
            _ if s.starts_with("*") => {
                info!("prefix src: {s:?}\ncalled at: {caller_location}");
                if s.starts_with("**") {
                    Some(Delimiter::BoldA)
                } else {
                    Some(Delimiter::ItalicA)
                }
            }
            _ if s.starts_with("_") => {
                if s.starts_with("__") {
                    Some(Delimiter::BoldB)
                } else {
                    Some(Delimiter::ItalicB)
                }
            }
            _ if s.starts_with("---") => Some(Delimiter::Break),
            _ if s.starts_with("#") => Some(Delimiter::Heading),
            _ if s.starts_with("[") => Some(Delimiter::LinkStart),
            _ if s.starts_with("]") => Some(Delimiter::LinkEnd),
            _ if s.starts_with("~") => Some(Delimiter::Strike),
            _ if s.starts_with("(") => Some(Delimiter::UrlBlkStart),
            _ if s.starts_with(")") => Some(Delimiter::UrlBlkEnd),
            _ if s.starts_with("`") => {
                if s.starts_with("``") {
                    Some(Delimiter::CodeBlock)
                } else {
                    Some(Delimiter::InlineCodeBlock)
                }
            }
            _ => None,
        }
    }
    pub(super) fn is_simple_pattern(&self) -> bool {
        matches!(
            self,
            Delimiter::BoldA
                | Delimiter::BoldB
                | Delimiter::ItalicA
                | Delimiter::ItalicB
                | Delimiter::Strike
                | Delimiter::InlineCodeBlock
        )
    }
    pub(super) fn is_top_lvl_pat(&self) -> bool {
        matches!(
            self,
            Delimiter::CodeBlock | Delimiter::Heading | Delimiter::Break
        )
    }
    pub(super) fn is_strong(&self) -> bool {
        matches!(self, Delimiter::BoldA | Delimiter::BoldB)
    }
    pub(super) fn is_emph(&self) -> bool {
        matches!(self, Delimiter::ItalicA | Delimiter::ItalicB)
    }
    pub(super) fn is_inline_code(&self) -> bool {
        matches!(self, Delimiter::InlineCodeBlock)
    }
    pub(super) fn is_strike(&self) -> bool {
        matches!(self, Delimiter::Strike)
    }
}

// #[derive(Debug, Default)]
// pub struct LineStream<'a> {
//     lines: Vec<Line<'a>>,
//     current: usize,
// }
//
// impl<'a> LineStream<'a> {
//     pub fn new(src: &'a str) -> Self {
//         let lines = src
//             .lines()
//             .map(|l| Line {
//                 line: l,
//                 current: 0,
//             })
//             .collect::<Vec<_>>();
//         Self { lines, current: 0 }
//     }
// }
//
// impl<'a> LineStream<'a> {
//     pub fn peek(&self) -> Option<&Line<'a>> {
//         self.lines.get(self.current)
//     }
//     pub fn peek_nth(&self, n: usize) -> Option<&Line<'a>> {
//         self.lines.get(self.current + n)
//     }
//     pub fn next(&mut self) -> Option<Line<'a>> {
//         self.current += 1;
//         let (tok, rest) = self.lines.split_first()?;
//         let tok = tok.clone();
//         self.lines = rest.to_vec();
//         Some(tok)
//     }
// }
//
// #[derive(Debug, Clone)]
// pub struct Line<'a> {
//     line: &'a str,
//     current: usize,
// }
//
// impl<'a> Line<'a> {
//     pub fn peek(&self) -> Option<char> {
//         self.line.chars().nth(self.current)
//     }
//     pub fn peek_nth(&self, n: usize) -> Option<char> {
//         self.line.chars().nth(self.current + n)
//     }
//     pub fn next(&mut self) -> Option<char> {
//         self.current += 1;
//         self.line.chars().nth(self.current - 1)
//     }
// }

// impl<'a> Iterator for LineStream<'a> {
//     type Item = Line<'a>;
//     fn next(&mut self) -> Option<Self::Item> {
//         self.next()
//     }
// }

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use color_eyre::owo_colors::OwoColorize;
    use fern::colors::ColoredLevelConfig;

    use super::*;

    fn setup_logger() -> Result<(), fern::InitError> {
        let colors = ColoredLevelConfig::new()
            // use builder methods
            .debug(fern::colors::Color::Magenta)
            .info(fern::colors::Color::Green);
        fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{}{} {} {}{} {}",
                    "[".dimmed(),
                    humantime::format_rfc3339_seconds(SystemTime::now()).dimmed(),
                    colors.color(record.level()),
                    record.target().dimmed(),
                    ']'.dimmed(),
                    message
                ))
            })
            .level(log::LevelFilter::Debug)
            .chain(std::io::stdout())
            .chain(fern::log_file("output.log")?)
            .apply()?;
        Ok(())
    }

    #[test]
    fn test_parse() -> Result<(), MdParseError> {
        setup_logger()?;
        let input = r#"
### Heading 3
*Italic Text*
```rust
fn main() {
    println!("Hello, world!");
}
```
"#;
        let tokens = MarkdownParser::new(input.to_string()).parse()?;
        info!("tokens: {:?}", tokens);
        Ok(())
    }

    #[test]
    fn test_parse_emph_strong() -> Result<(), MdParseError> {
        setup_logger()?;
        let input = r#"*Italic Text*
**Bold Text**
"#;
        let tokens = MarkdownParser::new(input.to_string()).parse()?;
        info!("tokens: {:#?}", tokens);
        Ok(())
    }

    #[test]
    fn test_file() -> Result<(), MdParseError> {
        setup_logger()?;
        let input = include_str!(
            "../../../surveys/2024-annual-survey/report/2025-02-13-2024-State-Of-Rust-Survey-results.md"
        );
        let tokens = MarkdownParser::new(input.to_string()).parse()?;
        info!("tokens: {:?}", tokens);
        Ok(())
    }

    #[test]
    fn test_match_prefix() -> Result<(), MdParseError> {
        let input = "**test**";
        let newline = "\n";
        let newline_matches = Delimiter::match_prefix(newline);
        let matches = Delimiter::match_prefix(input);
        assert_eq!(matches, Some(Delimiter::BoldA));
        assert_eq!(newline_matches, None);
        Ok(())
    }
}
