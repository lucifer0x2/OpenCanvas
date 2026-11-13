//! Syntax of one line: a node (`kind[#id][.style] parts… ["text"]`) or a directive
//! (`@name positional… parts…`). Meaning (IDs, shorthands, defaults) is applied in `expand`.
use super::error::{OcdError, Result};
use super::lexer::{Line, Token};

/// Node kinds as written in source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Page,
    Frame,
    Group,
    Rect,
    Text,
    Image,
    Comp,
    Use,
}

const KINDS: [(&str, Kind); 9] = [
    ("page", Kind::Page),
    ("frame", Kind::Frame),
    ("group", Kind::Group),
    ("rect", Kind::Rect),
    ("rectangle", Kind::Rect),
    ("text", Kind::Text),
    ("image", Kind::Image),
    ("comp", Kind::Comp),
    ("use", Kind::Use),
];

/// What follows the head, in source order (later parts win).
#[derive(Debug, Clone)]
pub enum Part {
    /// `.styleId`
    Style(Token),
    /// `400x44`: the width and height tokens.
    Size(Token, Token),
    /// `row`, `col`, `free`, `wrap`
    Flag(Token),
    /// `key=value`
    Attr(Token, Token),
    /// A trailing quoted string.
    Text(Token),
}

#[derive(Debug)]
pub struct NodeStmt {
    pub head: Token,
    pub kind: Kind,
    pub id: Option<Token>,
    /// `use <componentId>`
    pub component: Option<Token>,
    pub parts: Vec<Part>,
}

#[derive(Debug)]
pub struct DirectiveStmt {
    pub name: Token,
    pub positional: Vec<Token>,
    pub parts: Vec<Part>,
}

#[derive(Debug)]
pub enum Stmt {
    Node(NodeStmt),
    Directive(DirectiveStmt),
}

/// Positional arguments each directive takes (min, max) before its parts, and its usage.
const DIRECTIVES: [(&str, usize, usize, &str); 7] = [
    ("token", 2, 2, "@token id value, e.g. @token ink #111827"),
    ("style", 1, 1, "@style id key=value…"),
    ("set", 1, 1, "@set id key=value…"),
    ("del", 1, 1, "@del id"),
    ("move", 2, 3, "@move id parentId [index]"),
    ("note", 1, 1, "@note nodeId \"text\""),
    ("in", 1, 1, "@in parentId"),
];

pub fn statement(line: Line) -> Result<Stmt> {
    let mut tokens = line.tokens.into_iter();
    let head = tokens.next().expect("lexer drops empty lines");
    if let Some(name) = head.text.strip_prefix('@') {
        let Some(&(_, min, max, usage)) = DIRECTIVES.iter().find(|d| d.0 == name) else {
            return Err(head.error(format!(
                "Unknown directive `{}`; use @token, @style, @set, @del, @move, @note or @in",
                head.text
            )));
        };
        let rest: Vec<Token> = tokens.collect();
        // A quoted string is a part (note text), except as a token's value: `@token ink "#111"`.
        let positional = |(i, t): &(usize, &Token)| match t.text.starts_with('"') {
            true => name == "token" && *i == 1,
            false => !t.text.contains('='),
        };
        let count = rest
            .iter()
            .enumerate()
            .take(max)
            .take_while(positional)
            .count();
        if count < min {
            let at = rest.get(count).unwrap_or(&head);
            return Err(at.error(format!("@{name} needs {min} argument(s): {usage}")));
        }
        let positional = rest[..count].to_vec();
        let parts = parts(rest[count..].to_vec())?;
        return Ok(Stmt::Directive(DirectiveStmt {
            name: head.slice(1, None),
            positional,
            parts,
        }));
    }
    let (kind, id, style) = split_head(&head)?;
    let component = if kind == Kind::Use {
        let bare = |t: &Token| !t.text.contains('=') && !t.text.starts_with(['"', '.']);
        let token = tokens.next().filter(bare);
        Some(token.ok_or_else(|| head.error("use needs a component ID: use#id <componentId>"))?)
    } else {
        None
    };
    let mut parts = parts(tokens.collect())?;
    if let Some(style) = style {
        parts.insert(0, Part::Style(style));
    }
    Ok(Stmt::Node(NodeStmt {
        head,
        kind,
        id,
        component,
        parts,
    }))
}

/// `kind[#id][.style]`
fn split_head(head: &Token) -> Result<(Kind, Option<Token>, Option<Token>)> {
    let text = &head.text;
    let kind_len = text.chars().take_while(|c| c.is_ascii_alphabetic()).count();
    let word = &text[..kind_len];
    let kind = KINDS
        .iter()
        .find(|(name, _)| *name == word)
        .map(|(_, k)| *k);
    let kind = kind.ok_or_else(|| {
        head.error(format!(
            "Unknown node kind `{word}`; use page, frame, group, rect, text, image, comp or use"
        ))
    })?;
    let rest = &text[kind_len..];
    let rest_len = rest.chars().count();
    let style_at = rest.chars().position(|c| c == '.').unwrap_or(rest_len);
    let id = match rest.strip_prefix('#') {
        Some(_) => Some(head.slice(kind_len + 1, Some(style_at - 1))),
        None if style_at == 0 => None,
        None => return Err(head.error(format!("Expected #id or .style after `{word}`"))),
    };
    let style = (style_at < rest_len).then(|| head.slice(kind_len + style_at, None));
    Ok((kind, id, style))
}

fn parts(tokens: Vec<Token>) -> Result<Vec<Part>> {
    let last = tokens.len().saturating_sub(1);
    let mut out = Vec::new();
    for (index, token) in tokens.into_iter().enumerate() {
        let text = token.text.as_str();
        // A complete `.5x10` is a size; any other `.x` is a style with an ordinary ID (`.123`).
        let dotted = text.starts_with('.');
        let dotted_size = if dotted { size(&token) } else { None };
        let part = if let Some(size) = dotted_size {
            size
        } else if dotted {
            Part::Style(token)
        } else if text.starts_with('"') {
            if index != last {
                return Err(token.error("A quoted text must be the last item on its line"));
            }
            Part::Text(token)
        } else if let Some(eq) = text.find('=') {
            let eq = text[..eq].chars().count();
            if eq == 0 {
                return Err(token.error("Missing key before ="));
            }
            Part::Attr(token.slice(0, Some(eq)), token.slice(eq + 1, None))
        } else if matches!(text, "row" | "col" | "free" | "wrap") {
            Part::Flag(token)
        } else if let Some(size) = size(&token) {
            size
        } else {
            return Err(unexpected(&token));
        };
        out.push(part);
    }
    Ok(out)
}

/// `WxH` with plain numbers on both sides (`.5x10` included).
fn size(token: &Token) -> Option<Part> {
    let (w, h) = token.text.split_once('x')?;
    let numeric = |s: &str| !s.is_empty() && s.parse::<f64>().is_ok_and(f64::is_finite);
    let first = w
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_digit() || *b == b'.');
    (numeric(w) && numeric(h) && first).then(|| {
        let w_len = w.chars().count();
        Part::Size(token.slice(0, Some(w_len)), token.slice(w_len + 1, None))
    })
}

fn unexpected(token: &Token) -> OcdError {
    let flag = match token.text.as_str() {
        "column" | "vertical" | "stack" => Some("col"),
        "horizontal" => Some("row"),
        "absolute" => Some("free"),
        _ => None,
    };
    token
        .error(format!(
            "Unexpected `{}`; expected key=value, a flag (row, col, free, wrap), WxH, .style or a final \"text\"",
            token.text
        ))
        .suggest(flag)
}
