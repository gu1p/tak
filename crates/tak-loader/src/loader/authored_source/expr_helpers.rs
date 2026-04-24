use ruff_python_ast::Expr;

pub(super) fn namespace_method_name<'a>(expr: &'a Expr, namespace: &str) -> Option<&'a str> {
    let Expr::Attribute(attribute) = expr else {
        return None;
    };
    let Expr::Name(name) = attribute.value.as_ref() else {
        return None;
    };
    if name.id.as_str() != namespace {
        return None;
    }
    Some(attribute.attr.as_str())
}

pub(super) fn namespace_attribute_name<'a>(expr: &'a Expr, namespace: &str) -> Option<&'a str> {
    let Expr::Attribute(attribute) = expr else {
        return None;
    };
    let Expr::Name(name) = attribute.value.as_ref() else {
        return None;
    };
    if name.id.as_str() != namespace {
        return None;
    }
    Some(attribute.attr.as_str())
}

pub(super) fn is_tak_module(module_name: &str) -> bool {
    module_name == "tak" || module_name.starts_with("tak.")
}

pub(super) fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset];
    let line = prefix.chars().filter(|ch| *ch == '\n').count() + 1;
    let column = prefix.chars().rev().take_while(|ch| *ch != '\n').count() + 1;
    (line, column)
}
