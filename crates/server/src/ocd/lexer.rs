//! Splits source into indented lines of whitespace-separated tokens. A quoted string or a
//! `{json}`/`[json]` value is one token even when it contains spaces; `//` after whitespace
//! starts a comment. Columns count characters, not bytes, from 1.
use super::error::{OcdError, Result};

/// One token and where it starts.
#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn error(&self, message: impl Into<String>) -> OcdError {
        OcdError::new(self.line, self.col, message)
    }
    /// The part of this token from character `offset`, keeping its true column. It scans from
    /// the start: callers cutting a token into many parts use [`Token::part`] instead.
    pub fn slice(&self, offset: usize, len: Option<usize>) -> Token {
        let chars = self.text.chars().skip(offset);
        let text = match len {
            Some(len) => chars.take(len).collect(),
            None => chars.collect(),
        };
        self.part(text, offset)
    }
    /// A token for `text` found at character `offset` of this one.
    pub fn part(&self, text: String, offset: usize) -> Token {
        Token {
            text,
            line: self.line,
            col: self.col + offset,
        }
    }
}

/// A non-empty line: its nesting depth (2 spaces per level) and tokens.
#[derive(Debug)]
pub struct Line {
    pub depth: usize,
    pub tokens: Vec<Token>,
}

pub fn lines(source: &str) -> Result<Vec<Line>> {
    let mut out = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let chars: Vec<char> = raw.chars().collect();
        let indent = chars.iter().take_while(|c| **c == ' ').count();
        let tokens = tokens(&chars, indent, line)?;
        if tokens.is_empty() {
            continue;
        }
        if chars[indent].is_whitespace() {
            let message = match chars[indent] {
                '\t' => "Indent with spaces, not tabs".to_owned(),
                other => format!("Indent with ASCII spaces, not U+{:04X}", other as u32),
            };
            return Err(OcdError::new(line, indent + 1, message));
        }
        if indent % 2 != 0 {
            return Err(OcdError::new(
                line,
                indent + 1,
                format!("Indent is {indent} spaces; use 2 spaces per level"),
            ));
        }
        out.push(Line {
            depth: indent / 2,
            tokens,
        });
    }
    Ok(out)
}

fn tokens(chars: &[char], start: usize, line: usize) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    let mut i = start;
    loop {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || (chars[i] == '/' && chars.get(i + 1) == Some(&'/')) {
            return Ok(out);
        }
        let begin = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i = match chars[i] {
                '"' => skip_string(chars, i, line)?,
                '{' | '[' => skip_json(chars, i, line)?,
                _ => i + 1,
            };
        }
        out.push(Token {
            text: chars[begin..i].iter().collect(),
            line,
            col: begin + 1,
        });
    }
}

/// Index after the closing quote of the string opened at `open`.
fn skip_string(chars: &[char], open: usize, line: usize) -> Result<usize> {
    let mut i = open + 1;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '"' => return Ok(i + 1),
            _ => i += 1,
        }
    }
    Err(OcdError::new(
        line,
        open + 1,
        "Unterminated string; close it with \" on the same line",
    ))
}

/// Index after the bracket closing the JSON value opened at `open`.
fn skip_json(chars: &[char], open: usize, line: usize) -> Result<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                i = skip_string(chars, i, line)?;
                continue;
            }
            '{' | '[' => depth += 1,
            '}' | ']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err(OcdError::new(
        line,
        open + 1,
        format!(
            "Unclosed {}; a JSON value must end on its line",
            chars[open]
        ),
    ))
}
