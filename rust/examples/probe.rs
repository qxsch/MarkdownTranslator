// Temporary parser probe; replaced by the real CLI.
use mdtranslate::markdown::ast::{Kind, Node};
use mdtranslate::markdown::parse::parse_markdown;
use mdtranslate::types::ParseOptions;

fn main() {
    let file = std::env::args().nth(1).expect("file");
    if file == "--yaml" {
        let path = std::env::args().nth(2).expect("file");
        let text = std::fs::read_to_string(path).unwrap();
        let bytes_at: Vec<usize> = text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect();
        for ev in saphyr_parser::Parser::new_from_str(&text) {
            match ev {
                Ok((saphyr_parser::Event::Scalar(v, style, _, _), span)) => {
                    println!("{:?} {}-{} {}", style, bytes_at[span.start.index()], bytes_at[span.end.index()], serde_json::to_string(&v).unwrap());
                }
                Ok(_) => {}
                Err(e) => {
                    println!("error {e}");
                    break;
                }
            }
        }
        return;
    }
    if file == "--html" {
        let path = std::env::args().nth(2).expect("file");
        let html = std::fs::read_to_string(path).unwrap();
        print_html(&html);
        return;
    }
    let math = std::env::args().any(|a| a == "--math");
    let mut src = std::fs::read_to_string(&file).unwrap();
    if src.starts_with('\u{FEFF}') {
        src = src['\u{FEFF}'.len_utf8()..].to_string();
    }
    let mdx = file.to_lowercase().ends_with(".mdx");
    let tree = parse_markdown(&src, ParseOptions { mdx, math_single_dollar: math }).unwrap();
    let mut out = Vec::new();
    visit(&tree, 0, &mut out);
    println!("{}", out.join("\n"));
}

fn print_html(html: &str) {
    use mdtranslate::markdown::html_parser::{parse_fragment, Fragment, HKind};
    let f = parse_fragment(html);
    fn visit(f: &Fragment, id: usize, depth: usize, out: &mut Vec<String>, html: &str) {
        let n = f.node(id);
        let loc = n.loc.map(|(s, e)| format!("{s}-{e}")).unwrap_or("?".into());
        let label = match &n.kind {
            HKind::Text => format!("#text {}", serde_json::to_string(&html[n.loc.unwrap().0..n.loc.unwrap().1]).unwrap()),
            HKind::Comment => "#comment".into(),
            HKind::Element { tag, .. } => {
                let st = n.start_tag.map(|(s, e)| format!(" st={s}-{e}")).unwrap_or_default();
                let et = n.end_tag.map(|(s, e)| format!(" et={s}-{e}")).unwrap_or_default();
                format!("{tag}{st}{et}")
            }
        };
        out.push(format!("{}{} {}", "  ".repeat(depth), label, loc));
        for c in &n.children {
            visit(f, *c, depth + 1, out, html);
        }
    }
    let mut out = Vec::new();
    for c in f.root_children() {
        visit(&f, *c, 0, &mut out, html);
    }
    println!("{}", out.join("\n"));
}

fn js(s: &str) -> String {
    serde_json::to_string(s).unwrap()
}

fn opt(s: &Option<String>) -> String {
    s.as_deref().map(js).unwrap_or("null".into())
}

fn visit(n: &Node, depth: usize, out: &mut Vec<String>) {
    let extra = match &n.kind {
        Kind::Text { value } | Kind::InlineCode { value } | Kind::Html { value } => format!(" {}", js(value)),
        Kind::Link { url, title } | Kind::Image { url, title, .. } | Kind::Definition { url, title, .. } => format!(" url={} title={}", js(url), opt(title)),
        Kind::Code { lang, meta, .. } => format!(" lang={} meta={}", opt(lang), opt(meta)),
        Kind::LinkReference { identifier, reference, .. } | Kind::ImageReference { identifier, reference, .. } => format!(" id={} ref={}", identifier, reference.as_str()),
        Kind::ContainerDirective { name, attributes } | Kind::LeafDirective { name, attributes } => {
            let obj: serde_json::Map<String, serde_json::Value> = attributes.iter().map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))).collect();
            format!(" name={} attrs={}", name, serde_json::Value::Object(obj))
        }
        Kind::Paragraph { directive_label: true } => " label".to_string(),
        _ => String::new(),
    };
    out.push(format!("{}{} {}-{}{}", "  ".repeat(depth), n.type_name(), n.start, n.end, extra));
    for c in &n.children {
        visit(c, depth + 1, out);
    }
}
