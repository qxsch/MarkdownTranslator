//! HTML fragment parser with source locations, compatible with parse5's `parseFragment(html,
//! { sourceCodeLocationInfo: true })` for the parts `html.rs` relies on: the tree shape (implied end tags,
//! void and raw-text elements, table fix-ups, foster parenting, active formatting reconstruction and the
//! adoption agency algorithm) and parse5's location rules. An element closed by its own end tag ends there;
//! one closed implicitly ends where the token that closed it starts; elements inserted by the parser
//! (`tbody`, `tr`, `colgroup`, an empty `p`, adoption agency clones) have no location, while reconstructed
//! formatting elements reuse the location of their original start tag.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HKind {
    Text,
    Comment,
    Element { tag: String, attrs: Vec<String>, foreign: bool },
}

#[derive(Clone, Debug)]
pub struct HNode {
    pub kind: HKind,
    /// `sourceCodeLocation` start/end; `None` for elements inserted by the parser.
    pub loc: Option<(usize, usize)>,
    pub start_tag: Option<(usize, usize)>,
    pub end_tag: Option<(usize, usize)>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

impl HNode {
    pub fn tag(&self) -> Option<&str> {
        match &self.kind {
            HKind::Element { tag, .. } => Some(tag),
            _ => None,
        }
    }

    pub fn attrs(&self) -> &[String] {
        match &self.kind {
            HKind::Element { attrs, .. } => attrs,
            _ => &[],
        }
    }
}

/// Parsed fragment; node 0 is the fragment root.
pub struct Fragment {
    pub nodes: Vec<HNode>,
}

impl Fragment {
    pub fn node(&self, id: usize) -> &HNode {
        &self.nodes[id]
    }

    pub fn root_children(&self) -> &[usize] {
        &self.nodes[0].children
    }
}

// ------------------------------------------------------------------ tokenizer

#[derive(Clone, Debug)]
enum Tok {
    Chars { start: usize, end: usize },
    Start { name: String, attrs: Vec<(String, String)>, self_closing: bool, start: usize, end: usize },
    End { name: String, start: usize, end: usize },
    Comment { start: usize, end: usize },
    Doctype,
    Eof { at: usize },
}

impl Tok {
    fn span(&self) -> (usize, usize) {
        match self {
            Tok::Chars { start, end } | Tok::Start { start, end, .. } | Tok::End { start, end, .. } | Tok::Comment { start, end } => (*start, *end),
            Tok::Doctype => (0, 0),
            Tok::Eof { at } => (*at, *at),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Raw {
    Text,
    PlainText,
}

struct Tokenizer<'a> {
    s: &'a [u8],
    pos: usize,
    raw: Option<(String, Raw)>,
    queued: Option<Tok>,
    in_foreign: bool,
}

fn is_ws(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | b'\x0C' | b'\r' | b' ')
}

impl<'a> Tokenizer<'a> {
    fn new(src: &'a str) -> Self {
        Tokenizer { s: src.as_bytes(), pos: 0, raw: None, queued: None, in_foreign: false }
    }

    fn starts_with_ci(&self, at: usize, lit: &str) -> bool {
        let l = lit.as_bytes();
        at + l.len() <= self.s.len() && self.s[at..at + l.len()].eq_ignore_ascii_case(l)
    }

    fn find(&self, from: usize, lit: &[u8]) -> Option<usize> {
        if from > self.s.len() {
            return None;
        }
        self.s[from..].windows(lit.len()).position(|w| w == lit).map(|i| i + from)
    }

    fn next(&mut self) -> Tok {
        if let Some(t) = self.queued.take() {
            return t;
        }
        let n = self.s.len();
        if self.pos >= n {
            return Tok::Eof { at: n };
        }
        if let Some((name, kind)) = self.raw.clone() {
            return self.raw_text(&name, kind);
        }
        let start = self.pos;
        let mut i = self.pos;
        while i < n {
            if self.s[i] != b'<' {
                i += 1;
                continue;
            }
            if let Some((tok, next)) = self.markup(i) {
                if i > start {
                    self.queued = tok;
                    self.pos = next;
                    return Tok::Chars { start, end: i };
                }
                self.pos = next;
                return match tok {
                    Some(t) => t,
                    None => self.next(),
                };
            }
            i += 1;
        }
        self.pos = n;
        Tok::Chars { start, end: n }
    }

    /// Markup starting at `<` at `i`: the token (None when it produces nothing) and the offset after it.
    /// Returns `None` when the `<` is text.
    fn markup(&mut self, i: usize) -> Option<(Option<Tok>, usize)> {
        let n = self.s.len();
        let c1 = self.s.get(i + 1).copied();
        match c1 {
            Some(b'!') => {
                if self.s[i..].starts_with(b"<!--") {
                    let body = i + 4;
                    let end = if self.s.get(body) == Some(&b'>') {
                        body + 1
                    } else if self.s[body.min(n)..].starts_with(b"->") {
                        body + 2
                    } else {
                        let a = self.find(body, b"-->").map(|p| p + 3);
                        let b = self.find(body, b"--!>").map(|p| p + 4);
                        match (a, b) {
                            (Some(a), Some(b)) => a.min(b),
                            (Some(a), None) => a,
                            (None, Some(b)) => b,
                            (None, None) => n,
                        }
                    };
                    return Some((Some(Tok::Comment { start: i, end }), end));
                }
                if self.starts_with_ci(i + 2, "doctype") {
                    let end = self.find(i, b">").map(|p| p + 1).unwrap_or(n);
                    return Some((Some(Tok::Doctype), end));
                }
                if self.in_foreign && self.s[i..].starts_with(b"<![CDATA[") {
                    let end = self.find(i + 9, b"]]>").map(|p| p + 3).unwrap_or(n);
                    return Some((Some(Tok::Chars { start: i + 9, end: end.saturating_sub(3).max(i + 9) }), end));
                }
                let end = self.find(i, b">").map(|p| p + 1).unwrap_or(n);
                Some((Some(Tok::Comment { start: i, end }), end))
            }
            Some(b'?') => {
                let end = self.find(i, b">").map(|p| p + 1).unwrap_or(n);
                Some((Some(Tok::Comment { start: i, end }), end))
            }
            Some(b'/') => match self.s.get(i + 2).copied() {
                Some(c) if c.is_ascii_alphabetic() => match self.tag(i, i + 2, true) {
                    Some((tok, end)) => Some((Some(tok), end)),
                    None => Some((None, n)),
                },
                Some(b'>') => Some((None, i + 3)),
                Some(_) => {
                    let end = self.find(i, b">").map(|p| p + 1).unwrap_or(n);
                    Some((Some(Tok::Comment { start: i, end }), end))
                }
                None => None,
            },
            Some(c) if c.is_ascii_alphabetic() => match self.tag(i, i + 1, false) {
                Some((tok, end)) => Some((Some(tok), end)),
                // EOF inside a tag: the tag is dropped.
                None => Some((None, n)),
            },
            _ => None,
        }
    }

    /// Tag starting at `<` at `lt`, name at `name_start`. `None` on EOF in the tag.
    fn tag(&mut self, lt: usize, name_start: usize, end_tag: bool) -> Option<(Tok, usize)> {
        let s = self.s;
        let n = s.len();
        let mut i = name_start;
        while i < n && !is_ws(s[i]) && s[i] != b'/' && s[i] != b'>' {
            i += 1;
        }
        let name = String::from_utf8_lossy(&s[name_start..i]).to_ascii_lowercase();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        loop {
            while i < n && is_ws(s[i]) {
                i += 1;
            }
            if i >= n {
                return None;
            }
            match s[i] {
                b'>' => {
                    i += 1;
                    break;
                }
                b'/' => {
                    i += 1;
                    if i < n && s[i] == b'>' {
                        self_closing = true;
                        i += 1;
                        break;
                    }
                    continue;
                }
                _ => {}
            }
            // Attribute name (a leading `=` belongs to the name).
            let a = i;
            i += 1;
            while i < n && !is_ws(s[i]) && s[i] != b'/' && s[i] != b'>' && s[i] != b'=' {
                i += 1;
            }
            let aname = String::from_utf8_lossy(&s[a..i]).to_ascii_lowercase();
            while i < n && is_ws(s[i]) {
                i += 1;
            }
            let mut value = String::new();
            if i < n && s[i] == b'=' {
                i += 1;
                while i < n && is_ws(s[i]) {
                    i += 1;
                }
                if i >= n {
                    return None;
                }
                match s[i] {
                    q @ (b'"' | b'\'') => {
                        let v = i + 1;
                        let close = s[v..].iter().position(|c| *c == q).map(|p| p + v)?;
                        value = String::from_utf8_lossy(&s[v..close]).into_owned();
                        i = close + 1;
                    }
                    b'>' => {}
                    _ => {
                        let v = i;
                        while i < n && !is_ws(s[i]) && s[i] != b'>' {
                            i += 1;
                        }
                        value = String::from_utf8_lossy(&s[v..i]).into_owned();
                    }
                }
            }
            if !attrs.iter().any(|(k, _)| *k == aname) {
                attrs.push((aname, value));
            }
        }
        let tok = if end_tag {
            Tok::End { name, start: lt, end: i }
        } else {
            Tok::Start { name, attrs, self_closing, start: lt, end: i }
        };
        Some((tok, i))
    }

    fn raw_text(&mut self, name: &str, kind: Raw) -> Tok {
        let n = self.s.len();
        let start = self.pos;
        if kind == Raw::PlainText {
            self.pos = n;
            return Tok::Chars { start, end: n };
        }
        let mut i = start;
        while let Some(lt) = self.find(i, b"</") {
            let after = lt + 2 + name.len();
            if self.starts_with_ci(lt + 2, name) && (after >= n || is_ws(self.s[after]) || self.s[after] == b'/' || self.s[after] == b'>') {
                self.raw = None;
                if lt > start {
                    self.pos = lt;
                    return Tok::Chars { start, end: lt };
                }
                self.pos = lt;
                return self.next();
            }
            i = lt + 2;
        }
        self.pos = n;
        Tok::Chars { start, end: n }
    }
}

// ------------------------------------------------------------------ tree builder

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    InBody,
    InTable,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InTemplate,
    /// Inside a raw text element (script, style, title, textarea...).
    Text,
}

/// Entry of the list of active formatting elements.
#[derive(Clone, Debug)]
enum Afe {
    Marker,
    El { id: usize, name: String, attrs: Vec<(String, String)>, start: usize, end: usize },
}

const FORMATTING: &[&str] = &["a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong", "tt", "u"];

const SPECIAL: &[&str] = &[
    "address", "applet", "area", "article", "aside", "base", "basefont", "bgsound", "blockquote", "body", "br", "button", "caption", "center", "col",
    "colgroup", "dd", "details", "dir", "div", "dl", "dt", "embed", "fieldset", "figcaption", "figure", "footer", "form", "frame", "frameset", "h1", "h2",
    "h3", "h4", "h5", "h6", "head", "header", "hgroup", "hr", "html", "iframe", "img", "input", "keygen", "li", "link", "listing", "main", "marquee",
    "menu", "meta", "nav", "noembed", "noframes", "noscript", "object", "ol", "p", "param", "plaintext", "pre", "script", "search", "section", "select",
    "source", "style", "summary", "table", "tbody", "td", "template", "textarea", "tfoot", "th", "thead", "title", "tr", "track", "ul", "wbr", "xmp",
];
const CLOSES_P: &[&str] = &[
    "address", "article", "aside", "blockquote", "center", "details", "dialog", "dir", "div", "dl", "fieldset", "figcaption", "figure", "footer", "header",
    "hgroup", "main", "menu", "nav", "ol", "p", "search", "section", "summary", "ul",
];
const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];
const VOID: &[&str] = &["area", "br", "embed", "img", "keygen", "wbr", "input", "param", "source", "track"];
const HEAD_VOID: &[&str] = &["base", "basefont", "bgsound", "link", "meta"];
const IMPLIED_END: &[&str] = &["dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc"];
const BLOCK_END: &[&str] = &[
    "address", "article", "aside", "blockquote", "button", "center", "details", "dialog", "dir", "div", "dl", "fieldset", "figcaption", "figure", "footer",
    "header", "hgroup", "listing", "main", "menu", "nav", "ol", "pre", "search", "section", "summary", "ul",
];
const SCOPE: &[&str] = &["applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "foreignobject", "desc", "title", "mi", "mo", "mn", "ms", "mtext", "annotation-xml"];
const TABLE_SECTIONS: &[&str] = &["tbody", "tfoot", "thead"];
const BREAKOUT: &[&str] = &[
    "b", "big", "blockquote", "body", "br", "center", "code", "dd", "div", "dl", "dt", "em", "embed", "h1", "h2", "h3", "h4", "h5", "h6", "head", "hr", "i",
    "img", "li", "listing", "menu", "meta", "nobr", "ol", "p", "pre", "ruby", "s", "small", "span", "strong", "strike", "sub", "sup", "table", "tt", "u",
    "ul", "var",
];

enum Step {
    Done,
    Reprocess,
}

struct Builder<'a> {
    nodes: Vec<HNode>,
    stack: Vec<usize>,
    mode: Mode,
    template_modes: Vec<Mode>,
    foster: bool,
    skip_newline: bool,
    /// Process the current token with the insertion mode even though the current node is foreign.
    force_html: bool,
    /// Insertion mode to return to after a raw text element.
    original_mode: Mode,
    afe: Vec<Afe>,
    tokenizer: Tokenizer<'a>,
    src: &'a [u8],
}

/// Location info of the token being processed, for end locations of popped elements.
#[derive(Clone, Copy)]
struct Cur<'t> {
    start: usize,
    end: usize,
    end_tag: Option<&'t str>,
}

pub fn parse_fragment(html: &str) -> Fragment {
    let root = HNode { kind: HKind::Element { tag: "html".into(), attrs: Vec::new(), foreign: false }, loc: None, start_tag: None, end_tag: None, children: Vec::new(), parent: None };
    let mut b = Builder {
        nodes: vec![root],
        stack: vec![0],
        mode: Mode::InTemplate,
        template_modes: vec![Mode::InTemplate],
        foster: false,
        skip_newline: false,
        force_html: false,
        original_mode: Mode::InBody,
        afe: Vec::new(),
        tokenizer: Tokenizer::new(html),
        src: html.as_bytes(),
    };
    loop {
        b.tokenizer.in_foreign = b.stack.len() > 1 && b.is_foreign(b.current());
        let tok = b.tokenizer.next();
        let eof = matches!(tok, Tok::Eof { .. });
        // A newline right after <pre>, <listing> or <textarea> is dropped only by the very next token.
        let skipping = b.skip_newline;
        b.dispatch(tok);
        if skipping {
            b.skip_newline = false;
        }
        if eof {
            break;
        }
    }
    Fragment { nodes: b.nodes }
}

impl<'a> Builder<'a> {
    fn tag_of(&self, id: usize) -> &str {
        self.nodes[id].tag().unwrap_or("")
    }

    fn current(&self) -> usize {
        *self.stack.last().unwrap()
    }

    fn current_tag(&self) -> &str {
        self.tag_of(self.current())
    }

    fn is_foreign(&self, id: usize) -> bool {
        matches!(self.nodes[id].kind, HKind::Element { foreign: true, .. })
    }

    fn dispatch(&mut self, tok: Tok) {
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 64 {
                return;
            }
            let foreign = !self.force_html && self.stack.len() > 1 && self.is_foreign(self.current()) && !matches!(tok, Tok::Eof { .. });
            self.force_html = false;
            let step = if foreign {
                self.foreign(&tok)
            } else {
                match self.mode {
                    Mode::InBody => self.in_body(&tok),
                    Mode::InTable => self.in_table(&tok),
                    Mode::InCaption => self.in_caption(&tok),
                    Mode::InColumnGroup => self.in_column_group(&tok),
                    Mode::InTableBody => self.in_table_body(&tok),
                    Mode::InRow => self.in_row(&tok),
                    Mode::InCell => self.in_cell(&tok),
                    Mode::InSelect => self.in_select(&tok),
                    Mode::InTemplate => self.in_template(&tok),
                    Mode::Text => self.in_text(&tok),
                }
            };
            if let Step::Done = step {
                return;
            }
        }
    }

    // ---------------------------------------------------------- tree operations

    /// Parent and index for inserting a node (foster parenting when enabled and the target is a table part).
    fn insertion_point(&self) -> (usize, usize) {
        let target = self.current();
        if self.foster && matches!(self.tag_of(target), "table" | "tbody" | "tfoot" | "thead" | "tr") {
            if let Some(pos) = self.stack.iter().rposition(|id| self.tag_of(*id) == "table") {
                let table = self.stack[pos];
                if let Some(parent) = self.nodes[table].parent {
                    let idx = self.nodes[parent].children.iter().position(|c| *c == table).unwrap_or(0);
                    return (parent, idx);
                }
                let prev = self.stack[pos.saturating_sub(1)];
                return (prev, self.nodes[prev].children.len());
            }
            return (0, self.nodes[0].children.len());
        }
        (target, self.nodes[target].children.len())
    }

    fn attach(&mut self, node: HNode) -> usize {
        let (parent, idx) = self.insertion_point();
        let id = self.nodes.len();
        let mut node = node;
        node.parent = Some(parent);
        self.nodes.push(node);
        self.nodes[parent].children.insert(idx, id);
        id
    }

    fn insert_text(&mut self, start: usize, end: usize) {
        let (mut start, end) = (start, end);
        if self.skip_newline {
            self.skip_newline = false;
            if self.src.get(start) == Some(&b'\r') && self.src.get(start + 1) == Some(&b'\n') {
                start += 2;
            } else if matches!(self.src.get(start), Some(b'\n') | Some(b'\r')) {
                start += 1;
            }
        }
        if start >= end {
            return;
        }
        let (parent, idx) = self.insertion_point();
        if idx > 0 {
            let prev = self.nodes[parent].children[idx - 1];
            if self.nodes[prev].kind == HKind::Text {
                if let Some(loc) = &mut self.nodes[prev].loc {
                    loc.1 = end;
                }
                return;
            }
        }
        let id = self.nodes.len();
        self.nodes.push(HNode { kind: HKind::Text, loc: Some((start, end)), start_tag: None, end_tag: None, children: Vec::new(), parent: Some(parent) });
        self.nodes[parent].children.insert(idx, id);
    }

    fn insert_comment(&mut self, start: usize, end: usize) {
        let saved = self.foster;
        self.foster = false;
        self.attach(HNode { kind: HKind::Comment, loc: Some((start, end)), start_tag: None, end_tag: None, children: Vec::new(), parent: None });
        self.foster = saved;
    }

    fn element(tag: &str, attrs: &[(String, String)], start: usize, end: usize, foreign: bool) -> HNode {
        HNode {
            kind: HKind::Element { tag: tag.to_string(), attrs: attrs.iter().map(|(k, _)| k.clone()).collect(), foreign },
            loc: Some((start, end)),
            start_tag: Some((start, end)),
            end_tag: None,
            children: Vec::new(),
            parent: None,
        }
    }

    fn insert(&mut self, tag: &str, attrs: &[(String, String)], start: usize, end: usize) -> usize {
        let id = self.attach(Self::element(tag, attrs, start, end, false));
        self.stack.push(id);
        id
    }

    fn insert_void(&mut self, tag: &str, attrs: &[(String, String)], start: usize, end: usize) {
        self.attach(Self::element(tag, attrs, start, end, false));
    }

    fn insert_fake(&mut self, tag: &str) {
        let id = self.attach(HNode {
            kind: HKind::Element { tag: tag.to_string(), attrs: Vec::new(), foreign: false },
            loc: None,
            start_tag: None,
            end_tag: None,
            children: Vec::new(),
            parent: None,
        });
        self.stack.push(id);
    }

    fn pop(&mut self, cur: Cur) {
        if self.stack.len() <= 1 {
            return;
        }
        let id = self.stack.pop().unwrap();
        let tag = self.tag_of(id).to_string();
        let n = &mut self.nodes[id];
        if let Some(loc) = &mut n.loc {
            if cur.end_tag == Some(tag.as_str()) {
                n.end_tag = Some((cur.start, cur.end));
                loc.1 = cur.end;
            } else {
                loc.1 = cur.start;
            }
        }
    }

    fn pop_until(&mut self, names: &[&str], cur: Cur) {
        while self.stack.len() > 1 {
            let done = names.contains(&self.current_tag());
            self.pop(cur);
            if done {
                break;
            }
        }
    }

    fn in_scope_with(&self, names: &[&str], extra: &[&str]) -> bool {
        for id in self.stack.iter().rev() {
            let t = self.tag_of(*id);
            if names.contains(&t) {
                return true;
            }
            if SCOPE.contains(&t) || extra.contains(&t) || *id == 0 {
                return false;
            }
        }
        false
    }

    fn in_scope(&self, names: &[&str]) -> bool {
        self.in_scope_with(names, &[])
    }

    fn in_button_scope(&self, name: &str) -> bool {
        self.in_scope_with(&[name], &["button"])
    }

    fn in_table_scope(&self, names: &[&str]) -> bool {
        for id in self.stack.iter().rev() {
            let t = self.tag_of(*id);
            if names.contains(&t) {
                return true;
            }
            if matches!(t, "html" | "table" | "template") || *id == 0 {
                return false;
            }
        }
        false
    }

    fn generate_implied_end(&mut self, except: Option<&str>, cur: Cur) {
        while self.stack.len() > 1 && IMPLIED_END.contains(&self.current_tag()) && Some(self.current_tag()) != except {
            self.pop(cur);
        }
    }

    fn close_p(&mut self, cur: Cur) {
        if self.in_button_scope("p") {
            self.generate_implied_end(Some("p"), cur);
            self.pop_until(&["p"], cur);
        }
    }

    fn clear_to(&mut self, names: &[&str], cur: Cur) {
        while self.stack.len() > 1 && !names.contains(&self.current_tag()) && self.current_tag() != "template" {
            self.pop(cur);
        }
    }

    fn reset_mode(&mut self) {
        for (i, id) in self.stack.iter().enumerate().rev() {
            if *id == 0 {
                break;
            }
            let last = i == 0;
            match self.tag_of(*id) {
                "select" => {
                    self.mode = Mode::InSelect;
                    return;
                }
                "td" | "th" if !last => {
                    self.mode = Mode::InCell;
                    return;
                }
                "tr" => {
                    self.mode = Mode::InRow;
                    return;
                }
                "tbody" | "thead" | "tfoot" => {
                    self.mode = Mode::InTableBody;
                    return;
                }
                "caption" => {
                    self.mode = Mode::InCaption;
                    return;
                }
                "colgroup" => {
                    self.mode = Mode::InColumnGroup;
                    return;
                }
                "table" => {
                    self.mode = Mode::InTable;
                    return;
                }
                "template" => {
                    self.mode = *self.template_modes.last().unwrap_or(&Mode::InBody);
                    return;
                }
                _ => {}
            }
        }
        // Fragment case: the context element is a template.
        self.mode = *self.template_modes.last().unwrap_or(&Mode::InBody);
    }

    fn eof(&mut self, at: usize) -> Step {
        let cur = Cur { start: at, end: at, end_tag: None };
        while self.stack.len() > 1 {
            self.pop(cur);
        }
        Step::Done
    }

    // ---------------------------------------------------------- raw text and formatting elements

    fn start_raw(&mut self, name: &str) {
        self.tokenizer.raw = Some((name.to_string(), Raw::Text));
        self.original_mode = self.mode;
        self.mode = Mode::Text;
    }

    fn in_text(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Chars { start, end } => {
                self.insert_text(*start, *end);
                Step::Done
            }
            Tok::Eof { at } => {
                self.pop(Cur { start: *at, end: *at, end_tag: None });
                self.mode = self.original_mode;
                Step::Reprocess
            }
            Tok::End { name, start, end } => {
                self.pop(Cur { start: *start, end: *end, end_tag: Some(name.as_str()) });
                self.mode = self.original_mode;
                Step::Done
            }
            _ => Step::Done,
        }
    }

    fn push_marker(&mut self) {
        self.afe.push(Afe::Marker);
    }

    fn clear_afe_to_marker(&mut self) {
        while let Some(e) = self.afe.pop() {
            if matches!(e, Afe::Marker) {
                break;
            }
        }
    }

    fn push_formatting(&mut self, id: usize, name: &str, attrs: &[(String, String)], start: usize, end: usize) {
        self.afe.push(Afe::El { id, name: name.to_string(), attrs: attrs.to_vec(), start, end });
    }

    /// Reopens formatting elements that were closed implicitly; the new elements keep the location of the
    /// original start tag, as in parse5.
    fn reconstruct(&mut self) {
        let Some(last) = self.afe.last() else {
            return;
        };
        match last {
            Afe::Marker => return,
            Afe::El { id, .. } if self.stack.contains(id) => return,
            _ => {}
        }
        let mut i = self.afe.len() - 1;
        while i > 0 {
            match &self.afe[i - 1] {
                Afe::Marker => break,
                Afe::El { id, .. } if self.stack.contains(id) => break,
                _ => i -= 1,
            }
        }
        for k in i..self.afe.len() {
            if let Afe::El { name, attrs, start, end, .. } = self.afe[k].clone() {
                let id = self.insert(&name, &attrs, start, end);
                if let Afe::El { id: slot, .. } = &mut self.afe[k] {
                    *slot = id;
                }
            }
        }
    }

    /// Last formatting element named `name` after the last marker.
    fn afe_find(&self, name: &str) -> Option<usize> {
        for (i, e) in self.afe.iter().enumerate().rev() {
            match e {
                Afe::Marker => return None,
                Afe::El { name: n, .. } if n == name => return Some(i),
                _ => {}
            }
        }
        None
    }

    /// The adoption agency algorithm for misnested formatting end tags.
    fn adoption_agency(&mut self, name: &str, cur: Cur) {
        let current = self.current();
        if self.tag_of(current) == name && !self.afe.iter().any(|e| matches!(e, Afe::El { id, .. } if *id == current)) {
            self.pop(cur);
            return;
        }
        for _ in 0..8 {
            let Some(fi) = self.afe_find(name) else {
                self.any_other_end(name, cur);
                return;
            };
            let Afe::El { id: fe, .. } = self.afe[fi] else {
                return;
            };
            let Some(fe_pos) = self.stack.iter().position(|x| *x == fe) else {
                self.afe.remove(fi);
                return;
            };
            if !self.in_scope(&[name]) {
                return;
            }
            let furthest = self.stack[fe_pos + 1..].iter().position(|id| SPECIAL.contains(&self.tag_of(*id))).map(|p| fe_pos + 1 + p);
            let Some(fb_pos) = furthest else {
                while self.stack.len() > fe_pos {
                    self.pop(cur);
                }
                self.afe.remove(fi);
                return;
            };
            let furthest_block = self.stack[fb_pos];
            let common_ancestor = self.stack[fe_pos - 1];
            let mut bookmark = fi;
            let mut node_pos = fb_pos;
            let mut last_node = furthest_block;
            let mut inner = 0;
            loop {
                inner += 1;
                node_pos -= 1;
                let node = self.stack[node_pos];
                if node == fe {
                    break;
                }
                let mut in_afe = self.afe.iter().position(|e| matches!(e, Afe::El { id, .. } if *id == node));
                if inner > 3 {
                    if let Some(i) = in_afe {
                        self.afe.remove(i);
                        if i < bookmark {
                            bookmark -= 1;
                        }
                        in_afe = None;
                    }
                }
                let Some(ai) = in_afe else {
                    self.remove_from_stack(node_pos, cur);
                    continue;
                };
                let clone = self.clone_element(node);
                if let Afe::El { id, .. } = &mut self.afe[ai] {
                    *id = clone;
                }
                self.stack[node_pos] = clone;
                if last_node == furthest_block {
                    bookmark = ai + 1;
                }
                self.detach(last_node);
                self.append_child(clone, last_node);
                last_node = clone;
            }
            self.detach(last_node);
            if matches!(self.tag_of(common_ancestor), "table" | "tbody" | "tfoot" | "thead" | "tr") {
                let saved = (self.foster, self.stack.clone());
                self.foster = true;
                self.stack.push(common_ancestor);
                let (parent, idx) = self.insertion_point();
                self.stack = saved.1;
                self.foster = saved.0;
                self.insert_child(parent, idx, last_node);
            } else {
                self.append_child(common_ancestor, last_node);
            }
            let clone = self.clone_element(fe);
            let kids = std::mem::take(&mut self.nodes[furthest_block].children);
            for k in &kids {
                self.nodes[*k].parent = Some(clone);
            }
            self.nodes[clone].children = kids;
            self.append_child(furthest_block, clone);
            let entry = self.afe.remove(fi);
            if bookmark > fi {
                bookmark -= 1;
            }
            if let Afe::El { name, attrs, start, end, .. } = entry {
                self.afe.insert(bookmark.min(self.afe.len()), Afe::El { id: clone, name, attrs, start, end });
            }
            // Removing the formatting element from the stack pops it (parse5 sets its end location here).
            let fe_now = self.stack.iter().position(|x| *x == fe).unwrap();
            self.remove_from_stack(fe_now, cur);
            let fb_now = self.stack.iter().position(|x| *x == furthest_block).unwrap();
            self.stack.insert(fb_now + 1, clone);
        }
    }

    /// Removes an element from the middle of the stack, recording its end location like a pop.
    fn remove_from_stack(&mut self, pos: usize, cur: Cur) {
        let id = self.stack.remove(pos);
        let tag = self.tag_of(id).to_string();
        let n = &mut self.nodes[id];
        if let Some(loc) = &mut n.loc {
            if cur.end_tag == Some(tag.as_str()) {
                n.end_tag = Some((cur.start, cur.end));
                loc.1 = cur.end;
            } else {
                loc.1 = cur.start;
            }
        }
    }

    /// A copy of an element made by the adoption agency algorithm (no source location in parse5).
    fn clone_element(&mut self, id: usize) -> usize {
        let kind = self.nodes[id].kind.clone();
        let new = self.nodes.len();
        self.nodes.push(HNode { kind, loc: None, start_tag: None, end_tag: None, children: Vec::new(), parent: None });
        new
    }

    fn detach(&mut self, id: usize) {
        if let Some(p) = self.nodes[id].parent.take() {
            self.nodes[p].children.retain(|c| *c != id);
        }
    }

    fn append_child(&mut self, parent: usize, id: usize) {
        let idx = self.nodes[parent].children.len();
        self.insert_child(parent, idx, id);
    }

    fn insert_child(&mut self, parent: usize, idx: usize, id: usize) {
        self.nodes[id].parent = Some(parent);
        self.nodes[parent].children.insert(idx, id);
    }

    // ---------------------------------------------------------- insertion modes

    fn in_head_start(&mut self, name: &str, attrs: &[(String, String)], start: usize, end: usize) {
        match name {
            _ if HEAD_VOID.contains(&name) => self.insert_void(name, attrs, start, end),
            "template" => {
                self.insert(name, attrs, start, end);
                self.push_marker();
                self.mode = Mode::InTemplate;
                self.template_modes.push(Mode::InTemplate);
            }
            _ => {
                // title (RCDATA), style, script, noframes (raw text)
                self.insert(name, attrs, start, end);
                self.start_raw(name);
            }
        }
    }

    fn end_template(&mut self, cur: Cur) {
        if !self.stack.iter().any(|id| *id != 0 && self.tag_of(*id) == "template") {
            return;
        }
        while self.stack.len() > 1 && IMPLIED_END.contains(&self.current_tag()) {
            self.pop(cur);
        }
        self.pop_until(&["template"], cur);
        self.clear_afe_to_marker();
        self.template_modes.pop();
        self.reset_mode();
    }

    fn any_other_end(&mut self, name: &str, cur: Cur) {
        for i in (1..self.stack.len()).rev() {
            let id = self.stack[i];
            let t = self.tag_of(id).to_string();
            if t == name {
                self.generate_implied_end(Some(name), cur);
                while self.stack.len() > i {
                    self.pop(cur);
                }
                return;
            }
            if SPECIAL.contains(&t.as_str()) {
                return;
            }
        }
    }

    fn in_body(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Chars { start, end } => {
                self.reconstruct();
                self.insert_text(*start, *end);
                Step::Done
            }
            Tok::Comment { start, end } => {
                self.insert_comment(*start, *end);
                Step::Done
            }
            Tok::Doctype => Step::Done,
            Tok::Eof { at } => {
                if self.template_modes.len() > 1 {
                    return self.in_template(tok);
                }
                self.eof(*at)
            }
            Tok::Start { name, attrs, self_closing, start, end } => {
                let (start, end) = (*start, *end);
                let cur = Cur { start, end, end_tag: None };
                let name = name.as_str();
                match name {
                    "html" | "body" | "frameset" | "head" | "caption" | "col" | "colgroup" | "frame" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" => {}
                    "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style" | "template" | "title" => {
                        self.in_head_start(name, attrs, start, end)
                    }
                    _ if CLOSES_P.contains(&name) => {
                        self.close_p(cur);
                        self.insert(name, attrs, start, end);
                    }
                    _ if HEADINGS.contains(&name) => {
                        self.close_p(cur);
                        if HEADINGS.contains(&self.current_tag()) {
                            self.pop(cur);
                        }
                        self.insert(name, attrs, start, end);
                    }
                    "pre" | "listing" => {
                        self.close_p(cur);
                        self.insert(name, attrs, start, end);
                        self.skip_newline = true;
                    }
                    "form" => {
                        self.close_p(cur);
                        self.insert(name, attrs, start, end);
                    }
                    "li" | "dd" | "dt" => {
                        let targets: &[&str] = if name == "li" { &["li"] } else { &["dd", "dt"] };
                        for i in (1..self.stack.len()).rev() {
                            let t = self.tag_of(self.stack[i]).to_string();
                            if targets.contains(&t.as_str()) {
                                self.generate_implied_end(Some(&t), cur);
                                self.pop_until(&[t.as_str()], cur);
                                break;
                            }
                            if SPECIAL.contains(&t.as_str()) && !matches!(t.as_str(), "address" | "div" | "p") {
                                break;
                            }
                        }
                        self.close_p(cur);
                        self.insert(name, attrs, start, end);
                    }
                    "plaintext" => {
                        self.close_p(cur);
                        self.insert(name, attrs, start, end);
                        self.tokenizer.raw = Some((name.to_string(), Raw::PlainText));
                    }
                    "button" => {
                        if self.in_scope(&["button"]) {
                            self.generate_implied_end(None, cur);
                            self.pop_until(&["button"], cur);
                        }
                        self.reconstruct();
                        self.insert(name, attrs, start, end);
                    }
                    "a" => {
                        if let Some(i) = self.afe_find("a") {
                            let Afe::El { id, .. } = self.afe[i] else { unreachable!() };
                            self.adoption_agency("a", cur);
                            self.afe.retain(|e| !matches!(e, Afe::El { id: x, .. } if *x == id));
                            if let Some(p) = self.stack.iter().position(|x| *x == id) {
                                self.stack.remove(p);
                            }
                        }
                        self.reconstruct();
                        let id = self.insert(name, attrs, start, end);
                        self.push_formatting(id, name, attrs, start, end);
                    }
                    "nobr" => {
                        self.reconstruct();
                        if self.in_scope(&["nobr"]) {
                            self.adoption_agency("nobr", cur);
                            self.reconstruct();
                        }
                        let id = self.insert(name, attrs, start, end);
                        self.push_formatting(id, name, attrs, start, end);
                    }
                    _ if FORMATTING.contains(&name) => {
                        self.reconstruct();
                        let id = self.insert(name, attrs, start, end);
                        self.push_formatting(id, name, attrs, start, end);
                    }
                    "applet" | "marquee" | "object" => {
                        self.reconstruct();
                        self.insert(name, attrs, start, end);
                        self.push_marker();
                    }
                    "table" => {
                        self.close_p(cur);
                        self.insert(name, attrs, start, end);
                        self.mode = Mode::InTable;
                    }
                    "param" | "source" | "track" => self.insert_void(name, attrs, start, end),
                    _ if VOID.contains(&name) => {
                        self.reconstruct();
                        self.insert_void(name, attrs, start, end);
                    }
                    "image" => {
                        self.reconstruct();
                        self.insert_void("img", attrs, start, end);
                    }
                    "hr" => {
                        self.close_p(cur);
                        self.insert_void(name, attrs, start, end);
                    }
                    "textarea" => {
                        self.insert(name, attrs, start, end);
                        self.start_raw(name);
                        self.skip_newline = true;
                    }
                    "xmp" => {
                        self.close_p(cur);
                        self.reconstruct();
                        self.insert(name, attrs, start, end);
                        self.start_raw(name);
                    }
                    "iframe" | "noembed" | "noscript" => {
                        self.insert(name, attrs, start, end);
                        self.start_raw(name);
                    }
                    "select" => {
                        self.reconstruct();
                        self.insert(name, attrs, start, end);
                        self.mode = Mode::InSelect;
                    }
                    "optgroup" | "option" => {
                        if self.current_tag() == "option" {
                            self.pop(cur);
                        }
                        self.reconstruct();
                        self.insert(name, attrs, start, end);
                    }
                    "rb" | "rtc" | "rp" | "rt" => {
                        if self.in_scope(&["ruby"]) {
                            self.generate_implied_end(if matches!(name, "rp" | "rt") { Some("rtc") } else { None }, cur);
                        }
                        self.insert(name, attrs, start, end);
                    }
                    "math" | "svg" => {
                        self.reconstruct();
                        let id = self.attach(Self::element(name, attrs, start, end, true));
                        if !*self_closing {
                            self.stack.push(id);
                        }
                    }
                    _ => {
                        self.reconstruct();
                        self.insert(name, attrs, start, end);
                    }
                }
                Step::Done
            }
            Tok::End { name, start, end } => {
                let cur = Cur { start: *start, end: *end, end_tag: Some(name.as_str()) };
                let name = name.as_str();
                match name {
                    "template" => self.end_template(cur),
                    "body" | "html" => {}
                    _ if BLOCK_END.contains(&name) => {
                        if self.in_scope(&[name]) {
                            self.generate_implied_end(None, cur);
                            self.pop_until(&[name], cur);
                        }
                    }
                    "form" => {
                        if self.in_scope(&["form"]) {
                            self.generate_implied_end(None, cur);
                            self.pop_until(&["form"], cur);
                        }
                    }
                    "p" => {
                        if !self.in_button_scope("p") {
                            self.insert_fake("p");
                        }
                        self.generate_implied_end(Some("p"), cur);
                        self.pop_until(&["p"], cur);
                    }
                    "li" => {
                        if self.in_scope_with(&["li"], &["ol", "ul"]) {
                            self.generate_implied_end(Some("li"), cur);
                            self.pop_until(&["li"], cur);
                        }
                    }
                    "dd" | "dt" => {
                        if self.in_scope(&[name]) {
                            self.generate_implied_end(Some(name), cur);
                            self.pop_until(&[name], cur);
                        }
                    }
                    _ if HEADINGS.contains(&name) => {
                        if self.in_scope(HEADINGS) {
                            self.generate_implied_end(None, cur);
                            self.pop_until(HEADINGS, cur);
                        }
                    }
                    "applet" | "marquee" | "object" => {
                        if self.in_scope(&[name]) {
                            self.generate_implied_end(None, cur);
                            self.pop_until(&[name], cur);
                            self.clear_afe_to_marker();
                        }
                    }
                    "br" => {
                        self.reconstruct();
                        self.insert_void("br", &[], *start, *end);
                    }
                    _ if FORMATTING.contains(&name) => self.adoption_agency(name, cur),
                    _ => self.any_other_end(name, cur),
                }
                Step::Done
            }
        }
    }

    fn foreign(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Chars { start, end } => {
                self.insert_text(*start, *end);
                Step::Done
            }
            Tok::Comment { start, end } => {
                self.insert_comment(*start, *end);
                Step::Done
            }
            Tok::Doctype | Tok::Eof { .. } => Step::Done,
            Tok::Start { name, attrs, self_closing, start, end } => {
                let breakout = BREAKOUT.contains(&name.as_str()) || (name == "font" && attrs.iter().any(|(k, _)| matches!(k.as_str(), "color" | "face" | "size")));
                if breakout {
                    let cur = Cur { start: *start, end: *end, end_tag: None };
                    while self.stack.len() > 1 && self.is_foreign(self.current()) {
                        self.pop(cur);
                    }
                    return Step::Reprocess;
                }
                let id = self.attach(Self::element(name, attrs, *start, *end, true));
                if !*self_closing {
                    self.stack.push(id);
                }
                Step::Done
            }
            Tok::End { name, start, end } => {
                let cur = Cur { start: *start, end: *end, end_tag: Some(name.as_str()) };
                let mut i = self.stack.len() - 1;
                while i >= 1 {
                    let id = self.stack[i];
                    if self.tag_of(id).eq_ignore_ascii_case(name) {
                        while self.stack.len() > i {
                            self.pop(cur);
                        }
                        return Step::Done;
                    }
                    if i == 1 || !self.is_foreign(self.stack[i - 1]) {
                        break;
                    }
                    i -= 1;
                }
                // Back in HTML content: the current insertion mode handles the end tag.
                self.force_html = true;
                Step::Reprocess
            }
        }
    }

    fn in_template(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Chars { .. } | Tok::Comment { .. } | Tok::Doctype => self.in_body(tok),
            Tok::Start { name, attrs, start, end, .. } => {
                let next = match name.as_str() {
                    "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style" | "template" | "title" => {
                        self.in_head_start(name, attrs, *start, *end);
                        return Step::Done;
                    }
                    "caption" | "colgroup" | "tbody" | "tfoot" | "thead" => Mode::InTable,
                    "col" => Mode::InColumnGroup,
                    "tr" => Mode::InTableBody,
                    "td" | "th" => Mode::InRow,
                    _ => Mode::InBody,
                };
                self.template_modes.pop();
                self.template_modes.push(next);
                self.mode = next;
                Step::Reprocess
            }
            Tok::End { name, start, end } => {
                if name == "template" {
                    self.end_template(Cur { start: *start, end: *end, end_tag: Some("template") });
                }
                Step::Done
            }
            Tok::Eof { at } => {
                if !self.stack.iter().any(|id| *id != 0 && self.tag_of(*id) == "template") {
                    return self.eof(*at);
                }
                let cur = Cur { start: *at, end: *at, end_tag: None };
                self.pop_until(&["template"], cur);
                self.template_modes.pop();
                self.reset_mode();
                Step::Reprocess
            }
        }
    }

    fn table_anything_else(&mut self, tok: &Tok) -> Step {
        self.foster = true;
        let step = self.in_body(tok);
        self.foster = false;
        step
    }

    fn in_table(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Chars { start, end } => {
                if matches!(self.current_tag(), "table" | "tbody" | "tfoot" | "thead" | "tr") {
                    let ws = self.src[*start..*end].iter().all(|b| is_ws(*b));
                    if ws {
                        self.insert_text(*start, *end);
                        return Step::Done;
                    }
                }
                self.table_anything_else(tok)
            }
            Tok::Comment { start, end } => {
                self.insert_comment(*start, *end);
                Step::Done
            }
            Tok::Doctype => Step::Done,
            Tok::Eof { .. } => self.in_body(tok),
            Tok::Start { name, attrs, start, end, .. } => {
                let cur = Cur { start: *start, end: *end, end_tag: None };
                match name.as_str() {
                    "caption" => {
                        self.clear_to(&["table", "html"], cur);
                        self.insert(name, attrs, *start, *end);
                        self.push_marker();
                        self.mode = Mode::InCaption;
                        Step::Done
                    }
                    "colgroup" => {
                        self.clear_to(&["table", "html"], cur);
                        self.insert(name, attrs, *start, *end);
                        self.mode = Mode::InColumnGroup;
                        Step::Done
                    }
                    "col" => {
                        self.clear_to(&["table", "html"], cur);
                        self.insert_fake("colgroup");
                        self.mode = Mode::InColumnGroup;
                        Step::Reprocess
                    }
                    "tbody" | "tfoot" | "thead" => {
                        self.clear_to(&["table", "html"], cur);
                        self.insert(name, attrs, *start, *end);
                        self.mode = Mode::InTableBody;
                        Step::Done
                    }
                    "td" | "th" | "tr" => {
                        self.clear_to(&["table", "html"], cur);
                        self.insert_fake("tbody");
                        self.mode = Mode::InTableBody;
                        Step::Reprocess
                    }
                    "table" => {
                        if !self.in_table_scope(&["table"]) {
                            return Step::Done;
                        }
                        self.pop_until(&["table"], cur);
                        self.reset_mode();
                        Step::Reprocess
                    }
                    "style" | "script" | "template" => {
                        self.in_head_start(name, attrs, *start, *end);
                        Step::Done
                    }
                    _ => self.table_anything_else(tok),
                }
            }
            Tok::End { name, start, end } => {
                let cur = Cur { start: *start, end: *end, end_tag: Some(name.as_str()) };
                match name.as_str() {
                    "table" => {
                        if self.in_table_scope(&["table"]) {
                            self.pop_until(&["table"], cur);
                            self.reset_mode();
                        }
                        Step::Done
                    }
                    "body" | "caption" | "col" | "colgroup" | "html" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" => Step::Done,
                    "template" => {
                        self.end_template(cur);
                        Step::Done
                    }
                    _ => self.table_anything_else(tok),
                }
            }
        }
    }

    fn in_caption(&mut self, tok: &Tok) -> Step {
        let closes = match tok {
            Tok::Start { name, .. } => matches!(name.as_str(), "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr"),
            Tok::End { name, .. } => matches!(name.as_str(), "caption" | "table"),
            _ => false,
        };
        if closes {
            let (start, end) = tok.span();
            let end_tag = if let Tok::End { name, .. } = tok { Some(name.as_str()) } else { None };
            let cur = Cur { start, end, end_tag };
            if !self.in_table_scope(&["caption"]) {
                return Step::Done;
            }
            self.generate_implied_end(None, cur);
            self.pop_until(&["caption"], cur);
            self.clear_afe_to_marker();
            self.mode = Mode::InTable;
            return if matches!(tok, Tok::End { name, .. } if name == "caption") { Step::Done } else { Step::Reprocess };
        }
        if let Tok::End { name, .. } = tok {
            if matches!(name.as_str(), "body" | "col" | "colgroup" | "html" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr") {
                return Step::Done;
            }
        }
        self.in_body(tok)
    }

    fn in_column_group(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Chars { start, end } if self.src[*start..*end].iter().all(|b| is_ws(*b)) => {
                self.insert_text(*start, *end);
                Step::Done
            }
            Tok::Comment { start, end } => {
                self.insert_comment(*start, *end);
                Step::Done
            }
            Tok::Start { name, attrs, start, end, .. } if name == "col" => {
                self.insert_void(name, attrs, *start, *end);
                Step::Done
            }
            Tok::Start { name, attrs, start, end, .. } if name == "template" => {
                self.in_head_start(name, attrs, *start, *end);
                Step::Done
            }
            Tok::End { name, start, end } if name == "colgroup" => {
                if self.current_tag() == "colgroup" {
                    self.pop(Cur { start: *start, end: *end, end_tag: Some("colgroup") });
                    self.mode = Mode::InTable;
                }
                Step::Done
            }
            Tok::End { name, .. } if name == "col" => Step::Done,
            Tok::End { name, start, end } if name == "template" => {
                self.end_template(Cur { start: *start, end: *end, end_tag: Some("template") });
                Step::Done
            }
            Tok::Eof { .. } => self.in_body(tok),
            _ => {
                if self.current_tag() != "colgroup" {
                    return Step::Done;
                }
                let (start, _) = tok.span();
                self.pop(Cur { start, end: start, end_tag: None });
                self.mode = Mode::InTable;
                Step::Reprocess
            }
        }
    }

    fn in_table_body(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Start { name, attrs, start, end, .. } => {
                let cur = Cur { start: *start, end: *end, end_tag: None };
                match name.as_str() {
                    "tr" => {
                        self.clear_to(&["tbody", "tfoot", "thead", "html"], cur);
                        self.insert(name, attrs, *start, *end);
                        self.mode = Mode::InRow;
                        Step::Done
                    }
                    "th" | "td" => {
                        self.clear_to(&["tbody", "tfoot", "thead", "html"], cur);
                        self.insert_fake("tr");
                        self.mode = Mode::InRow;
                        Step::Reprocess
                    }
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" => {
                        if !self.in_table_scope(TABLE_SECTIONS) {
                            return Step::Done;
                        }
                        self.clear_to(&["tbody", "tfoot", "thead", "html"], cur);
                        self.pop(cur);
                        self.mode = Mode::InTable;
                        Step::Reprocess
                    }
                    _ => self.in_table(tok),
                }
            }
            Tok::End { name, start, end } => {
                let cur = Cur { start: *start, end: *end, end_tag: Some(name.as_str()) };
                match name.as_str() {
                    "tbody" | "tfoot" | "thead" => {
                        if self.in_table_scope(&[name.as_str()]) {
                            self.clear_to(&["tbody", "tfoot", "thead", "html"], cur);
                            self.pop(cur);
                            self.mode = Mode::InTable;
                        }
                        Step::Done
                    }
                    "table" => {
                        if !self.in_table_scope(TABLE_SECTIONS) {
                            return Step::Done;
                        }
                        self.clear_to(&["tbody", "tfoot", "thead", "html"], cur);
                        self.pop(Cur { end_tag: None, ..cur });
                        self.mode = Mode::InTable;
                        Step::Reprocess
                    }
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" | "tr" => Step::Done,
                    _ => self.in_table(tok),
                }
            }
            _ => self.in_table(tok),
        }
    }

    fn in_row(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Start { name, attrs, start, end, .. } => {
                let cur = Cur { start: *start, end: *end, end_tag: None };
                match name.as_str() {
                    "th" | "td" => {
                        self.clear_to(&["tr", "html"], cur);
                        self.insert(name, attrs, *start, *end);
                        self.push_marker();
                        self.mode = Mode::InCell;
                        Step::Done
                    }
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr" => {
                        if !self.in_table_scope(&["tr"]) {
                            return Step::Done;
                        }
                        self.clear_to(&["tr", "html"], cur);
                        self.pop(cur);
                        self.mode = Mode::InTableBody;
                        Step::Reprocess
                    }
                    _ => self.in_table(tok),
                }
            }
            Tok::End { name, start, end } => {
                let cur = Cur { start: *start, end: *end, end_tag: Some(name.as_str()) };
                match name.as_str() {
                    "tr" => {
                        if self.in_table_scope(&["tr"]) {
                            self.clear_to(&["tr", "html"], cur);
                            self.pop(cur);
                            self.mode = Mode::InTableBody;
                        }
                        Step::Done
                    }
                    "table" => {
                        if !self.in_table_scope(&["tr"]) {
                            return Step::Done;
                        }
                        self.clear_to(&["tr", "html"], cur);
                        self.pop(Cur { end_tag: None, ..cur });
                        self.mode = Mode::InTableBody;
                        Step::Reprocess
                    }
                    "tbody" | "tfoot" | "thead" => {
                        if !self.in_table_scope(&[name.as_str()]) || !self.in_table_scope(&["tr"]) {
                            return Step::Done;
                        }
                        self.clear_to(&["tr", "html"], cur);
                        self.pop(Cur { end_tag: None, ..cur });
                        self.mode = Mode::InTableBody;
                        Step::Reprocess
                    }
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" => Step::Done,
                    _ => self.in_table(tok),
                }
            }
            _ => self.in_table(tok),
        }
    }

    fn close_cell(&mut self, cur: Cur) {
        self.generate_implied_end(None, cur);
        self.pop_until(&["td", "th"], cur);
        self.clear_afe_to_marker();
        self.mode = Mode::InRow;
    }

    fn in_cell(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::End { name, start, end } if matches!(name.as_str(), "td" | "th") => {
                let cur = Cur { start: *start, end: *end, end_tag: Some(name.as_str()) };
                if self.in_table_scope(&[name.as_str()]) {
                    self.close_cell(cur);
                }
                Step::Done
            }
            Tok::Start { name, start, end, .. } if matches!(name.as_str(), "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr") => {
                if !self.in_table_scope(&["td", "th"]) {
                    return Step::Done;
                }
                self.close_cell(Cur { start: *start, end: *end, end_tag: None });
                Step::Reprocess
            }
            Tok::End { name, .. } if matches!(name.as_str(), "body" | "caption" | "col" | "colgroup" | "html") => Step::Done,
            Tok::End { name, start, end } if matches!(name.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr") => {
                if !self.in_table_scope(&[name.as_str()]) {
                    return Step::Done;
                }
                self.close_cell(Cur { start: *start, end: *end, end_tag: None });
                Step::Reprocess
            }
            _ => self.in_body(tok),
        }
    }

    fn in_select(&mut self, tok: &Tok) -> Step {
        match tok {
            Tok::Chars { start, end } => {
                self.insert_text(*start, *end);
                Step::Done
            }
            Tok::Comment { start, end } => {
                self.insert_comment(*start, *end);
                Step::Done
            }
            Tok::Doctype => Step::Done,
            Tok::Eof { .. } => self.in_body(tok),
            Tok::Start { name, attrs, start, end, .. } => {
                let cur = Cur { start: *start, end: *end, end_tag: None };
                match name.as_str() {
                    "option" => {
                        if self.current_tag() == "option" {
                            self.pop(cur);
                        }
                        self.insert(name, attrs, *start, *end);
                    }
                    "optgroup" => {
                        if self.current_tag() == "option" {
                            self.pop(cur);
                        }
                        if self.current_tag() == "optgroup" {
                            self.pop(cur);
                        }
                        self.insert(name, attrs, *start, *end);
                    }
                    "hr" => {
                        if self.current_tag() == "option" {
                            self.pop(cur);
                        }
                        if self.current_tag() == "optgroup" {
                            self.pop(cur);
                        }
                        self.insert_void(name, attrs, *start, *end);
                    }
                    "select" | "input" | "keygen" | "textarea" => {
                        if self.stack.iter().any(|id| self.tag_of(*id) == "select") {
                            self.pop_until(&["select"], cur);
                            self.reset_mode();
                            if name != "select" {
                                return Step::Reprocess;
                            }
                        }
                    }
                    "script" | "template" => self.in_head_start(name, attrs, *start, *end),
                    _ => {}
                }
                Step::Done
            }
            Tok::End { name, start, end } => {
                let cur = Cur { start: *start, end: *end, end_tag: Some(name.as_str()) };
                match name.as_str() {
                    "optgroup" => {
                        let n = self.stack.len();
                        if self.current_tag() == "option" && n >= 2 && self.tag_of(self.stack[n - 2]) == "optgroup" {
                            self.pop(Cur { end_tag: None, ..cur });
                        }
                        if self.current_tag() == "optgroup" {
                            self.pop(cur);
                        }
                    }
                    "option" => {
                        if self.current_tag() == "option" {
                            self.pop(cur);
                        }
                    }
                    "select" => {
                        if self.stack.iter().any(|id| self.tag_of(*id) == "select") {
                            self.pop_until(&["select"], cur);
                            self.reset_mode();
                        }
                    }
                    "template" => self.end_template(cur),
                    _ => {}
                }
                Step::Done
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump(html: &str) -> String {
        let f = parse_fragment(html);
        let mut out = Vec::new();
        fn visit(f: &Fragment, id: usize, depth: usize, out: &mut Vec<String>, html: &str) {
            let n = f.node(id);
            let loc = n.loc.map(|(s, e)| format!("{s}-{e}")).unwrap_or("?".into());
            let label = match &n.kind {
                HKind::Text => format!("#text {:?}", &html[n.loc.unwrap().0..n.loc.unwrap().1]),
                HKind::Comment => "#comment".into(),
                HKind::Element { tag, .. } => format!("{tag}{}", if n.end_tag.is_some() { "" } else { " (no end tag)" }),
            };
            out.push(format!("{}{} {}", "  ".repeat(depth), label, loc));
            for c in &n.children {
                visit(f, *c, depth + 1, out, html);
            }
        }
        for c in f.root_children() {
            visit(&f, *c, 0, &mut out, html);
        }
        out.join("\n")
    }

    #[test]
    fn implied_paragraph_end() {
        let d = dump("<p>one<div>two</div>");
        assert_eq!(d, "p (no end tag) 0-6\n  #text \"one\" 3-6\ndiv 6-20\n  #text \"two\" 11-14");
    }

    #[test]
    fn void_and_inline() {
        let d = dump("<div align=\"center\">\n  <img src=\"a\" alt=\"Logo\">\n  <p>Text <em>x</em>.</p>\n</div>");
        assert!(d.contains("img (no end tag) 23-47"), "{d}");
        assert!(d.contains("em 58-68"), "{d}");
        assert!(d.contains("div 0-80"), "{d}");
    }

    #[test]
    fn table_inserts_tbody() {
        let d = dump("<table><tr><td>A</td></tr></table>");
        assert!(d.contains("tbody (no end tag) ?"), "{d}");
        assert!(d.contains("td 11-21"), "{d}");
    }

    #[test]
    fn list_items_close() {
        let d = dump("<ul><li>a<li>b</ul>");
        assert_eq!(d, "ul 0-19\n  li (no end tag) 4-9\n    #text \"a\" 8-9\n  li (no end tag) 9-14\n    #text \"b\" 13-14");
    }

    #[test]
    fn raw_text_and_comments() {
        let d = dump("<script>if (a < b) {}</script><!-- c --><style>p{}</style>");
        assert!(d.contains("script 0-30"), "{d}");
        assert!(d.contains("#comment 30-40"), "{d}");
    }

    #[test]
    fn stray_end_tags_are_ignored_in_template_context() {
        assert_eq!(dump("text</p>"), "#text \"text\" 0-4");
    }

    #[test]
    fn misnested_formatting_is_reconstructed() {
        let d = dump("<b>bold<i>both</b>italic</i>");
        assert!(d.ends_with("i 7-28\n  #text \"italic\" 18-24"), "{d}");
    }
}
