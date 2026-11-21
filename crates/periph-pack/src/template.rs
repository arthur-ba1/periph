//! Byte templates: `"06 A5 A4 {len} {addr:u16le} 00 00 {data}"`.
//! Hex pairs are literal bytes; `{name}` or `{name:width}` is a variable (u8, u16le, u16be);
//! `{data}` is a run of bytes whose length is the `len` variable.

use serde::{de, Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    U8,
    U16Le,
    U16Be,
}

impl Width {
    pub fn size(self) -> usize {
        match self {
            Width::U8 => 1,
            _ => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Lit(u8),
    Var(String, Width),
    Data,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template(pub Vec<Token>);

impl Template {
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut out = Vec::new();
        for tok in s.split_whitespace() {
            if let Some(inner) = tok.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
                let (name, width) = inner.split_once(':').unwrap_or((inner, "u8"));
                if name == "data" {
                    out.push(Token::Data);
                    continue;
                }
                let width = match width {
                    "u8" => Width::U8,
                    "u16le" => Width::U16Le,
                    "u16be" => Width::U16Be,
                    w => return Err(format!("unknown width \"{w}\" in {tok}")),
                };
                if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Err(format!("bad variable {tok}"));
                }
                out.push(Token::Var(name.into(), width));
            } else {
                out.push(Token::Lit(u8::from_str_radix(tok, 16).map_err(|_| format!("bad byte \"{tok}\""))?));
            }
        }
        if out.is_empty() {
            return Err("empty template".into());
        }
        Ok(Template(out))
    }

    pub fn vars(&self) -> impl Iterator<Item = &str> {
        self.0.iter().filter_map(|t| match t {
            Token::Var(n, _) => Some(n.as_str()),
            _ => None,
        })
    }

    pub fn has_data(&self) -> bool {
        self.0.contains(&Token::Data)
    }

    /// Size in bytes when it has no `{data}`.
    pub fn fixed_len(&self) -> usize {
        self.0
            .iter()
            .map(|t| match t {
                Token::Lit(_) => 1,
                Token::Var(_, w) => w.size(),
                Token::Data => 0,
            })
            .sum()
    }

    /// Fill the template. `var` returns each variable's value; it must fit the width.
    pub fn render(&self, var: impl Fn(&str) -> Option<u32>, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        for t in &self.0 {
            match t {
                Token::Lit(b) => out.push(*b),
                Token::Data => out.extend_from_slice(data),
                Token::Var(n, w) => {
                    let v = var(n).ok_or_else(|| format!("no value for {{{n}}}"))?;
                    match w {
                        Width::U8 => out.push(u8::try_from(v).map_err(|_| format!("{{{n}}} = {v} does not fit a byte"))?),
                        Width::U16Le | Width::U16Be => {
                            let v = u16::try_from(v).map_err(|_| format!("{{{n}}} = {v} does not fit 16 bits"))?;
                            out.extend_from_slice(&if *w == Width::U16Le { v.to_le_bytes() } else { v.to_be_bytes() });
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    /// Fill a `send` template: `{value}` becomes the encoded value bytes (its width comes from
    /// the setting type), `{index}` the item index.
    pub fn render_value(&self, index: Option<u32>, value: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        for t in &self.0 {
            match t {
                Token::Lit(b) => out.push(*b),
                Token::Var(n, _) if n == "value" => out.extend_from_slice(value),
                Token::Var(n, w) if n == "index" => {
                    let i = index.ok_or("{index} in a setting that is not an array")?;
                    match w {
                        Width::U8 => out.push(u8::try_from(i).map_err(|_| "index does not fit a byte")?),
                        Width::U16Le => out.extend_from_slice(&(i as u16).to_le_bytes()),
                        Width::U16Be => out.extend_from_slice(&(i as u16).to_be_bytes()),
                    }
                }
                Token::Var(n, _) => return Err(format!("unknown variable {{{n}}} (use value, index)")),
                Token::Data => return Err("{data} is only for the registers transport".into()),
            }
        }
        Ok(out)
    }

    /// Match `bytes` against the template: the variables, and the `{data}` run (whose length
    /// is the `len` variable). `None` if a literal differs or the bytes are too short.
    pub fn matches(&self, bytes: &[u8]) -> Option<(Vec<(String, u32)>, Vec<u8>)> {
        let mut vars = Vec::new();
        let mut data = Vec::new();
        let mut i = 0;
        for t in &self.0 {
            match t {
                Token::Lit(b) => {
                    if *bytes.get(i)? != *b {
                        return None;
                    }
                    i += 1;
                }
                Token::Var(n, w) => {
                    let v = match w {
                        Width::U8 => *bytes.get(i)? as u32,
                        Width::U16Le => u16::from_le_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]) as u32,
                        Width::U16Be => u16::from_be_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]) as u32,
                    };
                    i += w.size();
                    vars.push((n.clone(), v));
                }
                Token::Data => {
                    let len = vars.iter().find(|(n, _)| n == "len")?.1 as usize;
                    data = bytes.get(i..i + len)?.to_vec();
                    i += len;
                }
            }
        }
        Some((vars, data))
    }
}

impl<'de> Deserialize<'de> for Template {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Template::parse(&String::deserialize(d)?).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_and_match() {
        let t = Template::parse("06 A5 A5 {len} {addr:u16le} 00 00 {data}").unwrap();
        let vars = |n: &str| match n {
            "len" => Some(2),
            "addr" => Some(0x0880),
            _ => None,
        };
        let b = t.render(vars, &[1, 6]).unwrap();
        assert_eq!(b, [6, 0xA5, 0xA5, 2, 0x80, 0x08, 0, 0, 1, 6]);
        let (v, data) = t.matches(&b).unwrap();
        assert_eq!(v, [("len".to_string(), 2), ("addr".to_string(), 0x0880)]);
        assert_eq!(data, [1, 6]);
        assert!(t.matches(&[6, 0xA5, 0xA4]).is_none());
        assert!(Template::parse("06 {x:u32}").is_err());
        assert!(t.render(|_| Some(300), &[]).is_err());
    }
}
