use anyhow::{Result, bail};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

const INCLUDE_MARKER: &str = "tak-docs:include";

pub(super) struct MarkedFence {
    pub(super) title: String,
    pub(super) language: String,
    pub(super) code: String,
}

#[derive(Default)]
struct FenceState {
    pending_title: Option<String>,
    active_title: Option<String>,
    active_language: String,
    active_code: String,
    examples: Vec<MarkedFence>,
}

pub(super) fn extract_marked_fences(markdown: &str) -> Result<Vec<MarkedFence>> {
    let mut state = FenceState::default();
    for event in Parser::new(markdown) {
        state.handle(event)?;
    }
    Ok(state.examples)
}

impl FenceState {
    fn handle(&mut self, event: Event<'_>) -> Result<()> {
        if self.active_title.is_some() {
            return self.handle_active(event);
        }

        match event {
            Event::Html(html) => {
                self.pending_title = marker_title(&html);
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) => {
                if let Some(title) = self.pending_title.take() {
                    validate_rust_doctest_language(&language)?;
                    self.active_title = Some(title);
                    self.active_language = language.to_string();
                }
            }
            Event::Start(Tag::HtmlBlock) => {}
            Event::SoftBreak
            | Event::HardBreak
            | Event::End(TagEnd::HtmlBlock)
            | Event::End(TagEnd::Paragraph) => {}
            Event::Text(text) if text.trim().is_empty() => {}
            _ => {
                self.pending_title = None;
            }
        }
        Ok(())
    }

    fn handle_active(&mut self, event: Event<'_>) -> Result<()> {
        match event {
            Event::Text(text) => self.active_code.push_str(&text),
            Event::End(TagEnd::CodeBlock) => self.finish_active(),
            _ => {}
        }
        Ok(())
    }

    fn finish_active(&mut self) {
        let title = self.active_title.take().expect("active doctest title");
        self.examples.push(MarkedFence {
            title,
            language: self.active_language.trim().to_owned(),
            code: self.active_code.trim_end().to_owned(),
        });
        self.active_language.clear();
        self.active_code.clear();
    }
}

fn marker_title(html: &str) -> Option<String> {
    let marker = html
        .trim()
        .strip_prefix("<!--")?
        .strip_suffix("-->")?
        .trim();
    if !marker.starts_with(INCLUDE_MARKER) {
        return None;
    }
    quoted_attr(marker, "title")
}

fn quoted_attr(marker: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = marker.find(&needle)? + needle.len();
    let rest = marker.get(start..)?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn validate_rust_doctest_language(language: &str) -> Result<()> {
    let token = language
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .next()
        .unwrap_or("");
    if matches!(token, "" | "rust" | "no_run" | "compile_fail") {
        Ok(())
    } else {
        bail!("marked docs dump example uses non-Rust code fence `{language}`")
    }
}
