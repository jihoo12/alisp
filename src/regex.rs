use crate::Evaluator;
use crate::expr::{Expr, expr_to_string};

// ---- Mini Regex Engine (zero dependencies) ----

#[derive(Debug, Clone)]
enum RNode {
    Literal(char),
    Dot,
    Class(Vec<ClassRange>),
    NegClass(Vec<ClassRange>),
    Star(Box<RNode>),
    Plus(Box<RNode>),
    Quest(Box<RNode>),
    Alt(Vec<RNode>),
    Seq(Vec<RNode>),
    Group(usize, Box<RNode>),
    AnchorStart,
    AnchorEnd,
    Ref(usize),
}

#[derive(Debug, Clone)]
struct ClassRange {
    start: char,
    end: char,
}

struct RProg {
    ast: Vec<RNode>,
}

impl RProg {
    fn compile(pattern: &str) -> Result<Self, String> {
        let chars: Vec<char> = pattern.chars().collect();
        let (nodes, _) = parse_sequence(&chars, 0)?;
        Ok(RProg { ast: nodes })
    }

    fn is_match(&self, input: &str) -> bool {
        let chars: Vec<char> = input.chars().collect();
        match_seq(&self.ast, &chars, 0).is_some()
    }

    fn find(&self, input: &str) -> Option<(usize, usize)> {
        let chars: Vec<char> = input.chars().collect();
        for i in 0..=chars.len() {
            if let Some(end) = match_seq(&self.ast, &chars, i) {
                return Some((i, end));
            }
        }
        None
    }

    fn find_all(&self, input: &str) -> Vec<(usize, usize)> {
        let chars: Vec<char> = input.chars().collect();
        let mut results = Vec::new();
        let mut pos = 0;
        while pos <= chars.len() {
            if let Some(end) = match_seq(&self.ast, &chars, pos) {
                results.push((pos, end));
                pos = if end == pos { pos + 1 } else { end };
            } else {
                pos += 1;
            }
        }
        results
    }

    fn replace_first(&self, input: &str, replacement: &str) -> String {
        if let Some((start, end)) = self.find(input) {
            let chars: Vec<char> = input.chars().collect();
            let before: String = chars[..start].iter().collect();
            let after: String = chars[end..].iter().collect();
            format!("{}{}{}", before, replacement, after)
        } else {
            input.to_string()
        }
    }

    fn replace_all_fn<F>(&self, input: &str, replacer: F) -> String
    where
        F: Fn(usize, usize) -> String,
    {
        let chars: Vec<char> = input.chars().collect();
        let matches = self.find_all(input);
        if matches.is_empty() {
            return input.to_string();
        }
        let mut result = String::new();
        let mut last_end = 0;
        for (start, end) in &matches {
            result.push_str(&chars[last_end..*start].iter().collect::<String>());
            result.push_str(&replacer(*start, *end));
            last_end = *end;
        }
        result.push_str(&chars[last_end..].iter().collect::<String>());
        result
    }

    fn split(&self, input: &str) -> Vec<String> {
        let chars: Vec<char> = input.chars().collect();
        let matches = self.find_all(input);
        if matches.is_empty() {
            return vec![input.to_string()];
        }
        let mut results = Vec::new();
        let mut last_end = 0;
        for (start, end) in &matches {
            results.push(chars[last_end..*start].iter().collect::<String>());
            last_end = *end;
        }
        results.push(chars[last_end..].iter().collect::<String>());
        results
    }
}

// ---- Parser ----

fn parse_sequence(chars: &[char], mut pos: usize) -> Result<(Vec<RNode>, usize), String> {
    let mut nodes = Vec::new();
    while pos < chars.len() && chars[pos] != ')' && chars[pos] != '|' {
        let (node, new_pos) = parse_atom(chars, pos)?;
        let (quantified, qpos) = parse_quantifier(chars, new_pos, node)?;
        nodes.push(quantified);
        pos = qpos;
    }
    Ok((nodes, pos))
}

fn parse_quantifier(chars: &[char], pos: usize, node: RNode) -> Result<(RNode, usize), String> {
    if pos >= chars.len() {
        return Ok((node, pos));
    }
    match chars[pos] {
        '*' => Ok((RNode::Star(Box::new(node)), pos + 1)),
        '+' => Ok((RNode::Plus(Box::new(node)), pos + 1)),
        '?' => Ok((RNode::Quest(Box::new(node)), pos + 1)),
        _ => Ok((node, pos)),
    }
}

fn parse_atom(chars: &[char], pos: usize) -> Result<(RNode, usize), String> {
    if pos >= chars.len() {
        return Err("unexpected end of regex".into());
    }
    match chars[pos] {
        '(' => {
            let inner_pos = pos + 1;
            let group_idx = 0;
            let (nodes, end) = parse_alt(chars, inner_pos)?;
            if end >= chars.len() || chars[end] != ')' {
                return Err("unmatched '(' in regex".into());
            }
            Ok((RNode::Group(group_idx, Box::new(RNode::Seq(nodes))), end + 1))
        }
        '|' => Ok((RNode::Alt(Vec::new()), pos)),
        '[' => {
            let (class, end) = parse_char_class(chars, pos)?;
            Ok((class, end))
        }
        '\\' => {
            if pos + 1 >= chars.len() {
                return Err("unexpected end after '\\' in regex".into());
            }
            let c = chars[pos + 1];
            match c {
                'd' => Ok((RNode::Class(vec![
                    ClassRange { start: '0', end: '9' },
                ]), pos + 2)),
                'D' => Ok((RNode::NegClass(vec![
                    ClassRange { start: '0', end: '9' },
                ]), pos + 2)),
                'w' => Ok((RNode::Class(vec![
                    ClassRange { start: 'a', end: 'z' },
                    ClassRange { start: 'A', end: 'Z' },
                    ClassRange { start: '0', end: '9' },
                    ClassRange { start: '_', end: '_' },
                ]), pos + 2)),
                'W' => Ok((RNode::NegClass(vec![
                    ClassRange { start: 'a', end: 'z' },
                    ClassRange { start: 'A', end: 'Z' },
                    ClassRange { start: '0', end: '9' },
                    ClassRange { start: '_', end: '_' },
                ]), pos + 2)),
                's' => Ok((RNode::Class(vec![
                    ClassRange { start: ' ', end: ' ' },
                    ClassRange { start: '\t', end: '\t' },
                    ClassRange { start: '\n', end: '\n' },
                    ClassRange { start: '\r', end: '\r' },
                ]), pos + 2)),
                'S' => Ok((RNode::NegClass(vec![
                    ClassRange { start: ' ', end: ' ' },
                    ClassRange { start: '\t', end: '\t' },
                    ClassRange { start: '\n', end: '\n' },
                    ClassRange { start: '\r', end: '\r' },
                ]), pos + 2)),
                'n' => Ok((RNode::Literal('\n'), pos + 2)),
                't' => Ok((RNode::Literal('\t'), pos + 2)),
                'r' => Ok((RNode::Literal('\r'), pos + 2)),
                _ => Ok((RNode::Literal(c), pos + 2)),
            }
        }
        '^' => Ok((RNode::AnchorStart, pos + 1)),
        '$' => Ok((RNode::AnchorEnd, pos + 1)),
        '.' => Ok((RNode::Dot, pos + 1)),
        _ => Ok((RNode::Literal(chars[pos]), pos + 1)),
    }
}

fn parse_alt(chars: &[char], pos: usize) -> Result<(Vec<RNode>, usize), String> {
    let (first, mut pos) = parse_sequence(chars, pos)?;
    let mut alternatives = vec![RNode::Seq(first)];
    while pos < chars.len() && chars[pos] == '|' {
        pos += 1;
        let (next, new_pos) = parse_sequence(chars, pos)?;
        alternatives.push(RNode::Seq(next));
        pos = new_pos;
    }
    if alternatives.len() == 1 {
        Ok((alternatives.remove(0).into_seq().unwrap_or_else(|| vec![]), pos))
    } else {
        Ok((vec![RNode::Alt(alternatives)], pos))
    }
}

fn parse_char_class(chars: &[char], pos: usize) -> Result<(RNode, usize), String> {
    let mut i = pos + 1;
    let negated = if i < chars.len() && chars[i] == '^' {
        i += 1;
        true
    } else {
        false
    };
    let mut ranges = Vec::new();
    if i < chars.len() && chars[i] == ']' {
        ranges.push(ClassRange { start: ']', end: ']' });
        i += 1;
    }
    while i < chars.len() && chars[i] != ']' {
        let start = chars[i];
        i += 1;
        if i < chars.len() && chars[i] == '-' && i + 1 < chars.len() && chars[i + 1] != ']' {
            i += 1;
            let end = chars[i];
            i += 1;
            ranges.push(ClassRange { start, end });
        } else {
            ranges.push(ClassRange { start, end: start });
        }
    }
    if i >= chars.len() {
        return Err("unmatched '[' in regex".into());
    }
    i += 1;
    if negated {
        Ok((RNode::NegClass(ranges), i))
    } else {
        Ok((RNode::Class(ranges), i))
    }
}

impl RNode {
    fn into_seq(self) -> Option<Vec<RNode>> {
        if let RNode::Seq(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

// ---- Matcher ----

fn match_seq(nodes: &[RNode], input: &[char], pos: usize) -> Option<usize> {
    match_nodes(nodes, 0, input, pos)
}

fn match_nodes(nodes: &[RNode], idx: usize, input: &[char], pos: usize) -> Option<usize> {
    if idx >= nodes.len() {
        return Some(pos);
    }
    match &nodes[idx] {
        RNode::Seq(sub) => {
            match_nodes(sub, 0, input, pos).and_then(|p| match_nodes(nodes, idx + 1, input, p))
        }
        RNode::Alt(alts) => {
            for alt in alts {
                match match_node(alt, input, pos) {
                    Some(p) => {
                        if let Some(final_p) = match_nodes(nodes, idx + 1, input, p) {
                            return Some(final_p);
                        }
                    }
                    None => continue,
                }
            }
            None
        }
        RNode::Star(node) => match_star(node, nodes, idx + 1, input, pos),
        RNode::Plus(node) => {
            match_node(node, input, pos).and_then(|p| match_star(node, nodes, idx + 1, input, p))
        }
        RNode::Quest(node) => {
            if let Some(p) = match_node(node, input, pos) {
                if let Some(fp) = match_nodes(nodes, idx + 1, input, p) {
                    return Some(fp);
                }
            }
            match_nodes(nodes, idx + 1, input, pos)
        }
        RNode::Group(_gid, inner) => {
            match_node(inner, input, pos).and_then(|p| match_nodes(nodes, idx + 1, input, p))
        }
        RNode::AnchorStart => {
            if pos == 0 {
                match_nodes(nodes, idx + 1, input, pos)
            } else {
                None
            }
        }
        RNode::AnchorEnd => {
            if pos >= input.len() {
                match_nodes(nodes, idx + 1, input, pos)
            } else {
                None
            }
        }
        _ => {
            match_single(&nodes[idx], input, pos).and_then(|p| {
                match_nodes(nodes, idx + 1, input, p)
            })
        }
    }
}

fn match_star(node: &RNode, rest: &[RNode], rest_idx: usize, input: &[char], pos: usize) -> Option<usize> {
    let max = input.len();
    let mut p = pos;
    let mut stack = Vec::new();
    loop {
        stack.push(p);
        if p > max {
            break;
        }
        if let Some(new_p) = match_node(node, input, p) {
            p = new_p;
        } else {
            break;
        }
        if p > max {
            break;
        }
    }
    while let Some(back_p) = stack.pop() {
        if let Some(final_p) = match_nodes(rest, rest_idx, input, back_p) {
            return Some(final_p);
        }
    }
    None
}

fn match_single(node: &RNode, input: &[char], pos: usize) -> Option<usize> {
    if pos >= input.len() {
        return None;
    }
    match node {
        RNode::Literal(c) => {
            if input[pos] == *c {
                Some(pos + 1)
            } else {
                None
            }
        }
        RNode::Dot => Some(pos + 1),
        RNode::Class(ranges) => {
            let c = input[pos];
            for r in ranges {
                if c >= r.start && c <= r.end {
                    return Some(pos + 1);
                }
            }
            None
        }
        RNode::NegClass(ranges) => {
            let c = input[pos];
            for r in ranges {
                if c >= r.start && c <= r.end {
                    return None;
                }
            }
            Some(pos + 1)
        }
        _ => None,
    }
}

fn match_node(node: &RNode, input: &[char], pos: usize) -> Option<usize> {
    match node {
        RNode::Literal(c) => {
            if pos < input.len() && input[pos] == *c {
                Some(pos + 1)
            } else {
                None
            }
        }
        RNode::Dot => {
            if pos < input.len() {
                Some(pos + 1)
            } else {
                None
            }
        }
        RNode::Class(ranges) => {
            if pos >= input.len() {
                return None;
            }
            let c = input[pos];
            for r in ranges {
                if c >= r.start && c <= r.end {
                    return Some(pos + 1);
                }
            }
            None
        }
        RNode::NegClass(ranges) => {
            if pos >= input.len() {
                return None;
            }
            let c = input[pos];
            for r in ranges {
                if c >= r.start && c <= r.end {
                    return None;
                }
            }
            Some(pos + 1)
        }
        RNode::Alt(alts) => {
            for alt in alts {
                if let Some(p) = match_node(alt, input, pos) {
                    return Some(p);
                }
            }
            None
        }
        RNode::Seq(nodes) => match_seq(nodes, input, pos),
        RNode::Group(_gid, inner) => match_node(inner, input, pos),
        RNode::AnchorStart => {
            if pos == 0 { Some(0) } else { None }
        }
        RNode::AnchorEnd => {
            if pos >= input.len() { Some(pos) } else { None }
        }
        RNode::Star(_) | RNode::Plus(_) | RNode::Quest(_) => {
            match_seq(&[node.clone()], input, pos)
        }
        _ => None,
    }
}

// ---- Public API wrappers ----

fn regex_compile(pattern: &str) -> Result<RProg, String> {
    RProg::compile(pattern)
}

fn regex_match(prog: &RProg, input: &str) -> bool {
    prog.is_match(input)
}

fn regex_find(prog: &RProg, input: &str) -> Option<(usize, usize)> {
    prog.find(input)
}

fn regex_find_all(prog: &RProg, input: &str) -> Vec<(usize, usize)> {
    prog.find_all(input)
}

// ---- Evaluator builtins ----

impl Evaluator {
    pub(crate) fn builtin_re_test(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(re-test pattern string)".into());
        }
        let pattern = expr_to_string(&args[0]);
        let s = expr_to_string(&args[1]);
        let prog = regex_compile(&pattern).map_err(|e| format!("re-test: {}", e))?;
        Ok(Expr::Bool(regex_match(&prog, &s)))
    }

    pub(crate) fn builtin_re_find(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(re-find pattern string)".into());
        }
        let pattern = expr_to_string(&args[0]);
        let s = expr_to_string(&args[1]);
        let prog = regex_compile(&pattern).map_err(|e| format!("re-find: {}", e))?;
        match regex_find(&prog, &s) {
            Some((start, end)) => {
                let chars: Vec<char> = s.chars().collect();
                let matched: String = chars[start..end].iter().collect();
                Ok(Expr::Str(matched))
            }
            None => Ok(Expr::Nil),
        }
    }

    pub(crate) fn builtin_re_find_all(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(re-find-all pattern string)".into());
        }
        let pattern = expr_to_string(&args[0]);
        let s = expr_to_string(&args[1]);
        let prog = regex_compile(&pattern).map_err(|e| format!("re-find-all: {}", e))?;
        let matches = regex_find_all(&prog, &s);
        let chars: Vec<char> = s.chars().collect();
        let results: Vec<Expr> = matches
            .into_iter()
            .map(|(start, end)| {
                let matched: String = chars[start..end].iter().collect();
                Expr::Str(matched)
            })
            .collect();
        Ok(Expr::List(results))
    }

    pub(crate) fn builtin_re_match(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(re-match pattern string)".into());
        }
        let pattern = expr_to_string(&args[0]);
        let s = expr_to_string(&args[1]);
        let prog = regex_compile(&pattern).map_err(|e| format!("re-match: {}", e))?;
        let chars: Vec<char> = s.chars().collect();
        match regex_find(&prog, &s) {
            Some((start, end)) if start == 0 && end == chars.len() => {
                Ok(Expr::List(vec![Expr::Str(s), Expr::Num(0.0), Expr::Num(chars.len() as f64)]))
            }
            _ => Ok(Expr::Nil),
        }
    }

    pub(crate) fn builtin_re_replace(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(re-replace string pattern replacement)".into());
        }
        let s = expr_to_string(&args[0]);
        let pattern = expr_to_string(&args[1]);
        let replacement = expr_to_string(&args[2]);
        let prog = regex_compile(&pattern).map_err(|e| format!("re-replace: {}", e))?;
        Ok(Expr::Str(prog.replace_first(&s, &replacement)))
    }

    pub(crate) fn builtin_re_replace_all(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 3 {
            return Err("(re-replace-all string pattern replacement)".into());
        }
        let s = expr_to_string(&args[0]);
        let pattern = expr_to_string(&args[1]);
        let replacement = expr_to_string(&args[2]);
        let prog = regex_compile(&pattern).map_err(|e| format!("re-replace-all: {}", e))?;
        let result = prog.replace_all_fn(&s, |_start, _end| replacement.clone());
        Ok(Expr::Str(result))
    }

    pub(crate) fn builtin_re_split(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(re-split pattern string)".into());
        }
        let pattern = expr_to_string(&args[0]);
        let s = expr_to_string(&args[1]);
        let prog = regex_compile(&pattern).map_err(|e| format!("re-split: {}", e))?;
        let parts = prog.split(&s);
        Ok(Expr::List(parts.into_iter().map(Expr::Str).collect()))
    }

    pub(crate) fn builtin_re_scan(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(re-scan pattern string)".into());
        }
        let pattern = expr_to_string(&args[0]);
        let s = expr_to_string(&args[1]);
        let prog = regex_compile(&pattern).map_err(|e| format!("re-scan: {}", e))?;
        let matches = regex_find_all(&prog, &s);
        let chars: Vec<char> = s.chars().collect();
        let results: Vec<Expr> = matches
            .into_iter()
            .map(|(start, end)| {
                let matched: String = chars[start..end].iter().collect();
                Expr::List(vec![
                    Expr::Str(matched),
                    Expr::Num(start as f64),
                    Expr::Num(end as f64),
                ])
            })
            .collect();
        Ok(Expr::List(results))
    }
}
