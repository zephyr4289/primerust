//! Minimal strict JSON value model, parser, and emitter.
//!
//! The workspace builds hermetically (no new registry deps), so session
//! configs and logs use this subset instead of serde_json:
//! objects, arrays, strings (escapes \" \\ \/ \b \f \n \r \t \uXXXX BMP),
//! numbers (f64), true/false/null, insignificant whitespace.
//! Anything outside the subset is a parse error, never silent coercion.

#[derive(Clone, Debug, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            J::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            J::Num(n) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[J]> {
        match self {
            J::Arr(a) => Some(a),
            _ => None,
        }
    }
}

fn err(msg: &str, rest: &str) -> String {
    let head: String = rest.chars().take(24).collect();
    format!("{} near {:?}", msg, head)
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn lit(&mut self, s: &[u8]) -> bool {
        if self.b[self.i..].starts_with(s) {
            self.i += s.len();
            true
        } else {
            false
        }
    }

    fn string(&mut self) -> Result<String, String> {
        // Expects opening quote already consumed? No: consumes it here.
        if self.b.get(self.i) != Some(&b'"') {
            return Err(err("expected string", self.rest()));
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let c = *self.b.get(self.i).ok_or_else(|| err("unterminated string", ""))?;
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = *self.b.get(self.i).ok_or_else(|| err("bad escape", ""))?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            if self.i + 4 > self.b.len() {
                                return Err(err("bad \\u escape", ""));
                            }
                            let h = std::str::from_utf8(&self.b[self.i..self.i + 4])
                                .map_err(|_| err("bad \\u escape", ""))?;
                            let cp = u32::from_str_radix(h, 16)
                                .map_err(|_| err("bad \\u escape", ""))?;
                            self.i += 4;
                            out.push(char::from_u32(cp).ok_or_else(|| err("bad codepoint", ""))?);
                        }
                        _ => return Err(err("bad escape", "")),
                    }
                }
                0x20..=0x7e => out.push(c as char),
                _ => {
                    // Raw UTF-8 passthrough (multibyte).
                    let start = self.i - 1;
                    let s = std::str::from_utf8(&self.b[start..])
                        .map_err(|_| err("bad utf8", ""))?;
                    let ch = s.chars().next().ok_or_else(|| err("bad utf8", ""))?;
                    self.i = start + ch.len_utf8();
                    out.push(ch);
                }
            }
        }
    }

    fn rest(&self) -> &str {
        std::str::from_utf8(&self.b[self.i..]).unwrap_or("")
    }

    fn number(&mut self) -> Result<f64, String> {
        let start = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let mut any = false;
        while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
            self.i += 1;
            any = true;
        }
        if self.b.get(self.i) == Some(&b'.') {
            self.i += 1;
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
                any = true;
            }
        }
        if matches!(self.b.get(self.i), Some(b'e') | Some(b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            let mut exp = false;
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
                exp = true;
            }
            if !exp {
                return Err(err("bad number exponent", self.rest()));
            }
            any = true;
        }
        if !any {
            return Err(err("bad number", self.rest()));
        }
        self.rest_number(start)
    }

    fn rest_number(&self, start: usize) -> Result<f64, String> {
        std::str::from_utf8(&self.b[start..self.i])
            .map_err(|_| err("bad number", ""))?
            .parse::<f64>()
            .map_err(|_| err("bad number", ""))
    }

    fn value(&mut self) -> Result<J, String> {
        self.ws();
        let c = *self.b.get(self.i).ok_or_else(|| "empty json".to_string())?;
        match c {
            b'{' => {
                self.i += 1;
                let mut pairs = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(J::Obj(pairs));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') {
                        return Err(err("expected ':'", self.rest()));
                    }
                    self.i += 1;
                    let v = self.value()?;
                    pairs.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => {
                            self.i += 1;
                        }
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(J::Obj(pairs));
                        }
                        _ => return Err(err("expected ',' or '}'", self.rest())),
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut arr = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(J::Arr(arr));
                }
                loop {
                    let v = self.value()?;
                    arr.push(v);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => {
                            self.i += 1;
                        }
                        Some(b']') => {
                            self.i += 1;
                            return Ok(J::Arr(arr));
                        }
                        _ => return Err(err("expected ',' or ']'", self.rest())),
                    }
                }
            }
            b'"' => Ok(J::Str(self.string()?)),
            b't' => {
                if self.lit(b"true") {
                    Ok(J::Bool(true))
                } else {
                    Err(err("bad literal", self.rest()))
                }
            }
            b'f' => {
                if self.lit(b"false") {
                    Ok(J::Bool(false))
                } else {
                    Err(err("bad literal", self.rest()))
                }
            }
            b'n' => {
                if self.lit(b"null") {
                    Ok(J::Null)
                } else {
                    Err(err("bad literal", self.rest()))
                }
            }
            b'-' | b'0'..=b'9' => Ok(J::Num(self.number()?)),
            _ => Err(err("unexpected character", self.rest())),
        }
    }
}

/// Parse a full JSON document (subset). Trailing garbage is an error.
pub fn parse(text: &str) -> Result<J, String> {
    let mut p = P { b: text.as_bytes(), i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(err("trailing characters", p.rest()));
    }
    Ok(v)
}

fn esc_into(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Compact JSON emission (no insignificant whitespace).
pub fn emit(v: &J) -> String {
    let mut out = String::new();
    emit_into(&mut out, v);
    out
}

fn emit_into(out: &mut String, v: &J) {
    match v {
        J::Null => out.push_str("null"),
        J::Bool(true) => out.push_str("true"),
        J::Bool(false) => out.push_str("false"),
        J::Num(n) => {
            // Exact integers only within f64's integer range. π anchors above
            // 2^53 (e.g. π(10^19)) MUST be carried as strings, never numbers —
            // the fixture extractor enforces this; the π check is a substring
            // match, never a numeric conversion.
            if n.fract() == 0.0 && n.abs() < 9007199254740992.0 {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&format!("{}", n));
            }
        }
        J::Str(s) => esc_into(out, s),
        J::Arr(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                emit_into(out, x);
            }
            out.push(']');
        }
        J::Obj(pairs) => {
            out.push('{');
            for (i, (k, x)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                esc_into(out, k);
                out.push(':');
                emit_into(out, x);
            }
            out.push('}');
        }
    }
}

/// Pretty JSON emission (2-space indent) for human-read artifacts.
pub fn emit_pretty(v: &J) -> String {
    let mut out = String::new();
    pretty_into(&mut out, v, 0);
    out.push('\n');
    out
}

fn pretty_into(out: &mut String, v: &J, depth: usize) {
    let pad = "  ".repeat(depth);
    let pad1 = "  ".repeat(depth + 1);
    match v {
        J::Arr(a) if a.is_empty() => out.push_str("[]"),
        J::Obj(p) if p.is_empty() => out.push_str("{}"),
        J::Arr(a) => {
            out.push_str("[\n");
            for (i, x) in a.iter().enumerate() {
                out.push_str(&pad1);
                pretty_into(out, x, depth + 1);
                if i + 1 < a.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&pad);
            out.push(']');
        }
        J::Obj(pairs) => {
            out.push_str("{\n");
            for (i, (k, x)) in pairs.iter().enumerate() {
                out.push_str(&pad1);
                esc_into(out, k);
                out.push_str(": ");
                pretty_into(out, x, depth + 1);
                if i + 1 < pairs.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&pad);
            out.push('}');
        }
        _ => emit_into(out, v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_mixed() {
        let doc = r#"{"a":[1,-2.5,1e3,true,false,null],"b":{"c":"x\"y\\z\n"}}"#;
        let v = parse(doc).expect("parse");
        assert_eq!(v.get("a").unwrap().as_arr().unwrap().len(), 6);
        assert_eq!(
            v.get("b").unwrap().get("c").unwrap().as_str().unwrap(),
            "x\"y\\z\n"
        );
        let back = parse(&emit(&v)).expect("reparse");
        assert_eq!(v, back);
    }

    #[test]
    fn rejects_trailing_and_bare() {
        assert!(parse("{} garbage").is_err());
        assert!(parse("{a:1}").is_err());
        assert!(parse("[1,]").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn int_emit_exact() {
        assert_eq!(emit(&J::Num(279238341033925.0)), "279238341033925");
    }
}
