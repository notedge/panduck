use super::*;
use crate::{ExtensionHandler, ExtensionRegistrar, PanduckError, Result};
use notedown_ast::ASTKind;
use serde_json::{Map, Value};
use std::{collections::BTreeSet, iter::FromIterator};

pub fn register_jupyter(r: &mut ExtensionRegistrar) {
    let ext = vec!["ipynb"];
    let new = ExtensionHandler {
        name: String::from("jupyter"),
        try_extension: BTreeSet::from_iter(ext.into_iter().map(String::from)),
        parser: parse_jupyter,
    };
    r.insert(new)
}

pub fn parse_jupyter(text: &str) -> Result<ASTNode> {
    let v: Value = serde_json::from_str(text)?;
    Ok(jupyter_from_json(&v)?)
}

pub fn jupyter_from_json(root: &Value) -> Result<ASTNode> {
    match root {
        Value::Object(o) => Ok(jupyter_root(o)),
        _ => Err(PanduckError::parse_error("Not a valid jupyter json")),
    }
}

fn jupyter_root(dict: &Map<String, Value>) -> ASTNode {
    if let Some(cells) = dict.get("cells") {
        if let Value::Array(v) = cells {
            return jupyter_cells(v);
        }
    }
    ASTKind::statements(vec![], None)
}

fn jupyter_cells(cells: &Vec<Value>) -> ASTNode {
    let mut out = vec![];
    for cell in cells {
        if let Value::Object(o) = cell {
            out.extend(jupyter_cell(o))
        }
    }
    ASTKind::statements(out, None)
}

fn jupyter_cell(dict: &Map<String, Value>) -> Vec<ASTNode> {
    let mut cell_type = "markdown";
    if let Some(key) = dict.get("cell_type") {
        if let Value::String(s) = key {
            cell_type = s
        }
    };
    match cell_type {
        "markdown" => jupyter_markdown(dict.get("source").unwrap()),
        _ => vec![],
    }
}

fn jupyter_markdown(dict: &Value) -> Vec<ASTNode> {
    let lines: Vec<String> = match dict {
        Value::Array(v) => v.into_iter().map(|v| jupyter_string(v)).collect(),
        _ => vec![],
    };
    if let Ok(o) = parse_common_markdown(&lines.join("\n")) {
        match o.kind {
            ASTKind::Statements(v) => {
                return v;
            }
            _ => {}
        }
    }
    vec![]
}

fn jupyter_string(o: &Value) -> String {
    match o {
        Value::String(v) => v.to_owned(),
        _ => String::new(),
    }
}
