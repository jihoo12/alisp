use std::fs;
use std::env;
use crate::Evaluator;
use crate::expr::{Expr, expr_to_string};

fn cp_recursive(src: &str, dst: &str) -> Result<(), std::io::Error> {
    let src_path = std::path::Path::new(src);
    let dst_path = std::path::Path::new(dst);
    if !dst_path.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src_path)? {
        let entry = entry?;
        let src_name = entry.file_name();
        let src_child = entry.path();
        let dst_child = dst_path.join(&src_name);
        if src_child.is_dir() {
            cp_recursive(
                &src_child.to_string_lossy(),
                &dst_child.to_string_lossy(),
            )?;
        } else {
            fs::copy(&src_child, &dst_child)?;
        }
    }
    Ok(())
}

fn glob_simple(pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    if pattern.contains("**") {
        glob_recursive(pattern, &mut results);
    } else if let Some(pos) = pattern.rfind('/') {
        let dir = &pattern[..pos];
        let file_pattern = &pattern[pos + 1..];
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if glob_match(file_pattern, &name) {
                    results.push(format!("{}/{}", dir, name));
                }
            }
        }
    } else if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if glob_match(pattern, &name) {
                results.push(name);
            }
        }
    }
    results.sort();
    results
}

fn glob_recursive(pattern: &str, results: &mut Vec<String>) {
    if let Some(pos) = pattern.find("**") {
        let before = &pattern[..pos];
        let after = &pattern[pos + 2..];
        let after_trimmed = after.strip_prefix('/').unwrap_or(after);
        let start_dir = if before.is_empty() {
            ".".to_string()
        } else {
            before.strip_suffix('/').unwrap_or(before).to_string()
        };
        let mut stack = vec![start_dir.clone()];
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    if path.is_dir() {
                        stack.push(path.to_string_lossy().to_string());
                    }
                    if after_trimmed.is_empty() {
                        let full = if dir == "." {
                            name.clone()
                        } else {
                            format!("{}/{}", dir, name)
                        };
                        results.push(full);
                    } else if glob_match(after_trimmed, &name) {
                        let full = if dir == "." {
                            name.clone()
                        } else {
                            format!("{}/{}", dir, name)
                        };
                        if path.is_file() {
                            results.push(full);
                        }
                    }
                }
            }
        }
    } else {
        results.extend(glob_simple(pattern));
    }
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_match_inner(&p, &n)
}

fn glob_match_inner(p: &[char], n: &[char]) -> bool {
    if p.is_empty() { return n.is_empty(); }
    if p[0] == '*' {
        for i in 0..=n.len() {
            if glob_match_inner(&p[1..], &n[i..]) { return true; }
        }
        false
    } else if p[0] == '?' {
        if !n.is_empty() {
            glob_match_inner(&p[1..], &n[1..])
        } else {
            false
        }
    } else if !n.is_empty() && p[0] == n[0] {
        glob_match_inner(&p[1..], &n[1..])
    } else {
        false
    }
}

impl Evaluator {
    pub(crate) fn builtin_read(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("read requires a filename".into());
        }
        let path = expr_to_string(&args[0]);
        let content = fs::read_to_string(&path).map_err(|e| format!("read '{}': {}", path, e))?;
        Ok(Expr::Str(content))
    }

    pub(crate) fn builtin_read_lines(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("read-lines requires a filename".into());
        }
        let path = expr_to_string(&args[0]);
        let content = fs::read_to_string(&path).map_err(|e| format!("read-lines '{}': {}", path, e))?;
        let lines: Vec<Expr> = content.lines().map(|l| Expr::Str(l.to_string())).collect();
        Ok(Expr::List(lines))
    }

    pub(crate) fn builtin_write(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(write path content)".into());
        }
        let path = expr_to_string(&args[0]);
        let content = expr_to_string(&args[1]);
        fs::write(&path, &content).map_err(|e| format!("write '{}': {}", path, e))?;
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_append(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(append path content)".into());
        }
        let path = expr_to_string(&args[0]);
        let content = expr_to_string(&args[1]);
        use std::fs::OpenOptions;
        use std::io::Write;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("append '{}': {}", path, e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("append '{}': {}", path, e))?;
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_exists(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("exists requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        Ok(Expr::Bool(std::path::Path::new(&path).exists()))
    }

    pub(crate) fn builtin_is_file(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("file? requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        Ok(Expr::Bool(std::path::Path::new(&path).is_file()))
    }

    pub(crate) fn builtin_is_dir(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("dir? requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        Ok(Expr::Bool(std::path::Path::new(&path).is_dir()))
    }

    pub(crate) fn builtin_file_size(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("file-size requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        let meta = fs::metadata(&path).map_err(|e| format!("file-size '{}': {}", path, e))?;
        Ok(Expr::Num(meta.len() as f64))
    }

    pub(crate) fn builtin_mtime(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("mtime requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        let meta = fs::metadata(&path).map_err(|e| format!("mtime '{}': {}", path, e))?;
        let modified = meta.modified().map_err(|e| format!("mtime '{}': {}", path, e))?;
        let duration = modified
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("mtime '{}': {}", path, e))?;
        Ok(Expr::Num(duration.as_secs() as f64))
    }

    pub(crate) fn builtin_touch(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("touch requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        let p = std::path::Path::new(&path);
        if p.exists() {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(|e| format!("touch '{}': {}", path, e))?;
            file.set_len(0).map_err(|e| format!("touch '{}': {}", path, e))?;
        } else {
            std::fs::File::create(&path).map_err(|e| format!("touch '{}': {}", path, e))?;
        }
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_rm(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("rm requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        if std::path::Path::new(&path).is_dir() {
            fs::remove_dir_all(&path).map_err(|e| format!("rm '{}': {}", path, e))?;
        } else {
            fs::remove_file(&path).map_err(|e| format!("rm '{}': {}", path, e))?;
        }
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_mkdir(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("mkdir requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        fs::create_dir_all(&path).map_err(|e| format!("mkdir '{}': {}", path, e))?;
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_cp(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(cp src dst)".into());
        }
        let src = expr_to_string(&args[0]);
        let dst = expr_to_string(&args[1]);
        let src_path = std::path::Path::new(&src);
        if src_path.is_dir() {
            cp_recursive(&src, &dst).map_err(|e| format!("cp '{}' '{}': {}", src, dst, e))?;
        } else {
            fs::copy(&src, &dst).map_err(|e| format!("cp '{}' '{}': {}", src, dst, e))?;
        }
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_mv(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.len() < 2 {
            return Err("(mv src dst)".into());
        }
        let src = expr_to_string(&args[0]);
        let dst = expr_to_string(&args[1]);
        fs::rename(&src, &dst).map_err(|e| format!("mv '{}' '{}': {}", src, dst, e))?;
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_ls(&mut self, args: &[Expr]) -> Result<Expr, String> {
        let path = if args.is_empty() {
            ".".to_string()
        } else {
            expr_to_string(&args[0])
        };
        let entries = fs::read_dir(&path).map_err(|e| format!("ls '{}': {}", path, e))?;
        let mut names: Vec<Expr> = entries
            .filter_map(|e| e.ok())
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                Expr::Str(name)
            })
            .collect();
        names.sort_by(|a, b| expr_to_string(a).cmp(&expr_to_string(b)));
        Ok(Expr::List(names))
    }

    pub(crate) fn builtin_glob(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("glob requires a pattern".into());
        }
        let pattern = expr_to_string(&args[0]);
        let entries = glob_simple(&pattern);
        Ok(Expr::List(
            entries.into_iter().map(Expr::Str).collect(),
        ))
    }

    pub(crate) fn builtin_cwd(&mut self, _args: &[Expr]) -> Result<Expr, String> {
        let cwd = env::current_dir().map_err(|e| format!("cwd: {}", e))?;
        Ok(Expr::Str(cwd.to_string_lossy().to_string()))
    }

    pub(crate) fn builtin_cd(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("cd requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        env::set_current_dir(&path).map_err(|e| format!("cd '{}': {}", path, e))?;
        Ok(Expr::Bool(true))
    }

    pub(crate) fn builtin_basename(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("basename requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        let p = std::path::Path::new(&path);
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let result = if args.len() > 1 {
            let ext = expr_to_string(&args[1]);
            name.strip_suffix(&ext).map(|s| s.to_string()).unwrap_or(name)
        } else {
            name
        };
        Ok(Expr::Str(result))
    }

    pub(crate) fn builtin_dirname(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("dirname requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        let p = std::path::Path::new(&path);
        let dir = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string());
        Ok(Expr::Str(dir))
    }

    pub(crate) fn builtin_ext(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("ext requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        let p = std::path::Path::new(&path);
        let ext = p.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        Ok(Expr::Str(ext))
    }

    pub(crate) fn builtin_join_path(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("join-path requires at least one argument".into());
        }
        let mut result = std::path::PathBuf::new();
        for arg in args {
            result.push(expr_to_string(arg));
        }
        Ok(Expr::Str(result.to_string_lossy().to_string()))
    }

    pub(crate) fn builtin_realpath(&mut self, args: &[Expr]) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("realpath requires a path".into());
        }
        let path = expr_to_string(&args[0]);
        let p = fs::canonicalize(&path).map_err(|e| format!("realpath '{}': {}", path, e))?;
        Ok(Expr::Str(p.to_string_lossy().to_string()))
    }
}
