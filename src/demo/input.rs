use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use serde_json::Value as JsonValue;
use serde_yaml_bw;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEncoding {
    Json,
    Yaml,
}

impl InputEncoding {
    pub fn label(self) -> &'static str {
        match self {
            InputEncoding::Json => "json",
            InputEncoding::Yaml => "yaml",
        }
    }
}

#[derive(Debug)]
pub enum InputSource {
    Inline(InputEncoding),
    File {
        path: PathBuf,
        encoding: InputEncoding,
    },
}

#[derive(Debug)]
pub struct ParsedInput {
    pub value: JsonValue,
    pub source: InputSource,
}

pub fn parse_input(value: &str) -> Result<ParsedInput> {
    if let Some(rest) = value.strip_prefix('@') {
        let path = PathBuf::from(rest);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("unable to read input file {}", path.display()))?;
        let (value, encoding) = parse_text(&contents)
            .with_context(|| format!("unable to parse input file {}", path.display()))?;
        Ok(ParsedInput {
            value,
            source: InputSource::File { path, encoding },
        })
    } else {
        let (value, encoding) = parse_text(value)?;
        Ok(ParsedInput {
            value,
            source: InputSource::Inline(encoding),
        })
    }
}

fn parse_text(text: &str) -> Result<(JsonValue, InputEncoding)> {
    match serde_json::from_str(text) {
        Ok(value) => Ok((value, InputEncoding::Json)),
        Err(json_err) => match serde_yaml_bw::from_str(text) {
            Ok(value) => Ok((value, InputEncoding::Yaml)),
            Err(yaml_err) => Err(anyhow!(
                "input parse error: json={json_err}; yaml={yaml_err}"
            )),
        },
    }
}
