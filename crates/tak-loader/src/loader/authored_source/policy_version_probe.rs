use std::collections::HashSet;

use anyhow::{Result, anyhow};
use ruff_python_ast::{
    Expr, Identifier, Stmt,
    visitor::source_order::{self, SourceOrderVisitor},
};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

use super::super::PRELUDE;

const PROBE_NAME: &str = "__tak_internal_policy_version_probe";

pub(crate) struct PreparedPolicyVersionProbe {
    pub(crate) initializer_source: String,
    pub(crate) activation_source: String,
    pub(crate) runtime_source: String,
}

pub(crate) fn prepare(source: &str) -> Result<PreparedPolicyVersionProbe> {
    let syntax = parse_module(source)
        .map_err(|err| anyhow!("failed to parse lowered TASKS.py source: {err}"))?
        .into_syntax();
    let probe_name = available_probe_name(&syntax.body);
    let runtime_source = instrument_final_expression(source, &syntax.body, &probe_name);
    Ok(PreparedPolicyVersionProbe {
        initializer_source: initializer_source(&probe_name),
        activation_source: activation_source(&probe_name),
        runtime_source,
    })
}

fn available_probe_name(body: &[Stmt]) -> String {
    let mut identifiers = ParsedIdentifiers::default();
    identifiers.visit_body(body);
    let mut name = PROBE_NAME.to_owned();
    let mut suffix = 0_u64;
    while identifiers.names.contains(name.as_str()) || PRELUDE.contains(&name) {
        suffix += 1;
        name = format!("{PROBE_NAME}_{suffix}");
    }
    name
}

#[derive(Default)]
struct ParsedIdentifiers<'a> {
    names: HashSet<&'a str>,
}

impl<'a> SourceOrderVisitor<'a> for ParsedIdentifiers<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            self.names.insert(name.id.as_str());
        }
        source_order::walk_expr(self, expr);
    }

    fn visit_identifier(&mut self, identifier: &'a Identifier) {
        self.names.insert(identifier.id.as_str());
    }
}

fn instrument_final_expression(source: &str, body: &[Stmt], probe_name: &str) -> String {
    let mut instrumented = source.to_owned();
    let Some(Stmt::Expr(statement)) = body.last() else {
        if !instrumented.ends_with('\n') {
            instrumented.push('\n');
        }
        instrumented.push_str("[0, 0]\n");
        return instrumented;
    };
    let range = statement.range();
    let expression = &source[range.start().to_usize()..range.end().to_usize()];
    instrumented.replace_range(
        range.start().to_usize()..range.end().to_usize(),
        &format!("{probe_name}(({expression}))"),
    );
    instrumented
}

fn initializer_source(name: &str) -> String {
    format!(
        r#"def {name}(_module_spec, _isinstance=isinstance, _dict=dict, _bool=bool, _int=int, _str=str, _keys=('spec_version', 'project_id', 'tasks', 'limiters', 'queues', 'exclude', 'includes', 'defaults')):
  _registered = []
  def tracked(*args, **kwargs):
    value = _module_spec(*args, **kwargs)
    _registered.append(value)
    return value
  def probe(value):
    registered_identity = False
    for candidate in _registered:
      if value is candidate:
        registered_identity = True
        break
    if not registered_identity:
      if not _isinstance(value, _dict):
        return [0, 0]
      for key in _keys:
        if key not in value:
          return [0, 0]
    version = value.get('spec_version', 1)
    if _isinstance(version, _bool) or not _isinstance(version, _int):
      return [2, 0]
    if version < 0 or version > 4294967295:
      return [2, 0]
    return [1, _int(_str(version))]
  return [tracked, probe]
"#
    )
}

fn activation_source(name: &str) -> String {
    format!("module_spec, {name} = {name}(module_spec)\n")
}
