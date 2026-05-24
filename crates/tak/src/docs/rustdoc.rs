use anyhow::{Result, anyhow, bail};
use syn::{Attribute, Expr, ExprLit, ImplItem, Item, Lit, Meta};

use super::model::EmbeddedRustSource;
use super::text::normalize_doc_text;

#[path = "rustdoc_fence.rs"]
mod rustdoc_fence;

use self::rustdoc_fence::extract_marked_fences;

#[derive(Debug)]
pub(super) struct VerifiedRustExample {
    pub(super) title: String,
    pub(super) crate_name: &'static str,
    pub(super) path: &'static str,
    pub(super) language: String,
    pub(super) code: String,
}

pub(super) fn extract_rust_module_docs(source: &str) -> Result<String> {
    Ok(normalize_doc_text(&extract_rust_module_markdown(source)?))
}

pub(super) fn extract_rust_module_markdown(source: &str) -> Result<String> {
    let syntax =
        syn::parse_file(source).map_err(|err| anyhow!("failed to parse Rust docs: {err}"))?;
    let docs = docs_from_attrs(&syntax.attrs).join("\n");
    if docs.trim().is_empty() {
        bail!("source does not contain module docs");
    }
    Ok(docs)
}

pub(super) fn collect_verified_rust_examples(
    sources: &[EmbeddedRustSource],
) -> Result<Vec<VerifiedRustExample>> {
    let mut examples = Vec::new();
    for source in sources {
        let extracted = extract_source_examples(source)?;
        if !extracted.is_empty() && !source.doctest_enabled {
            bail!(
                "`{}` marks doctest examples for docs dump but disables Cargo doctests",
                source.path
            );
        }
        examples.extend(extracted);
    }
    Ok(examples)
}

fn extract_source_examples(source: &EmbeddedRustSource) -> Result<Vec<VerifiedRustExample>> {
    let syntax = syn::parse_file(source.body)
        .map_err(|err| anyhow!("failed to parse Rust docs in `{}`: {err}", source.path))?;
    let mut docs = Vec::new();
    for item in &syntax.items {
        collect_item_docs(item, &mut docs);
    }

    let mut examples = Vec::new();
    for markdown in docs {
        for fence in extract_marked_fences(&markdown)? {
            examples.push(VerifiedRustExample {
                title: fence.title,
                crate_name: source.crate_name,
                path: source.path,
                language: fence.language,
                code: fence.code,
            });
        }
    }
    Ok(examples)
}

fn collect_item_docs(item: &Item, docs: &mut Vec<String>) {
    if let Some(markdown) = item_doc_markdown(item) {
        docs.push(markdown);
    }
    match item {
        Item::Impl(item_impl) => collect_impl_docs(&item_impl.items, docs),
        Item::Mod(module) => {
            if let Some((_, items)) = &module.content {
                for item in items {
                    collect_item_docs(item, docs);
                }
            }
        }
        _ => {}
    }
}

fn item_doc_markdown(item: &Item) -> Option<String> {
    let attrs = match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        _ => return None,
    };
    non_empty_docs(attrs)
}

fn collect_impl_docs(items: &[ImplItem], docs: &mut Vec<String>) {
    for item in items {
        let attrs = match item {
            ImplItem::Const(item) => &item.attrs,
            ImplItem::Fn(item) => &item.attrs,
            ImplItem::Type(item) => &item.attrs,
            _ => continue,
        };
        if let Some(markdown) = non_empty_docs(attrs) {
            docs.push(markdown);
        }
    }
}

fn non_empty_docs(attrs: &[Attribute]) -> Option<String> {
    let docs = docs_from_attrs(attrs);
    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

fn docs_from_attrs(attrs: &[Attribute]) -> Vec<String> {
    attrs.iter().filter_map(doc_attr_text).collect()
}

fn doc_attr_text(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("doc") {
        return None;
    }
    let Meta::NameValue(meta) = &attr.meta else {
        return None;
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Str(lit), ..
    }) = &meta.value
    else {
        return None;
    };
    let value = lit.value();
    Some(value.strip_prefix(' ').unwrap_or(&value).to_owned())
}
