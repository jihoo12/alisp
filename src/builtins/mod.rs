mod shell;
mod file_io;
mod env;
mod string;
mod list;
mod arithmetic;
mod io;
mod http;
mod misc;

use crate::Evaluator;
use crate::expr::Expr;

impl Evaluator {
    pub(crate) fn call_builtin(&mut self, name: &str, args: &[Expr]) -> Result<Expr, String> {
        match name {
            // Shell
            "exec" | "shell" | "sh" => self.builtin_exec(args, false),
            "exec-result" => self.builtin_exec(args, true),
            // File I/O
            "read" | "read-file" => self.builtin_read(args),
            "read-lines" => self.builtin_read_lines(args),
            "write" | "write-file" => self.builtin_write(args),
            "append" | "append-file" => self.builtin_append(args),
            "exists" | "file-exists" => self.builtin_exists(args),
            "file?" => self.builtin_is_file(args),
            "dir?" => self.builtin_is_dir(args),
            "file-size" => self.builtin_file_size(args),
            "mtime" => self.builtin_mtime(args),
            "touch" => self.builtin_touch(args),
            "rm" | "delete" => self.builtin_rm(args),
            "mkdir" => self.builtin_mkdir(args),
            "cp" | "copy" => self.builtin_cp(args),
            "mv" | "move" => self.builtin_mv(args),
            "ls" | "list-dir" => self.builtin_ls(args),
            "glob" => self.builtin_glob(args),
            "cwd" | "pwd" => self.builtin_cwd(args),
            "cd" => self.builtin_cd(args),
            "basename" => self.builtin_basename(args),
            "dirname" => self.builtin_dirname(args),
            "ext" => self.builtin_ext(args),
            "join-path" => self.builtin_join_path(args),
            "realpath" => self.builtin_realpath(args),
            // Environment
            "getenv" | "env-get" => self.builtin_getenv(args),
            "setenv" | "env-set" => self.builtin_setenv(args),
            "env" => self.builtin_env(args),
            // String
            "str" => self.builtin_str(args),
            "split" => self.builtin_split(args),
            "join" => self.builtin_join(args),
            "trim" => self.builtin_trim(args),
            "contains" | "includes" => self.builtin_contains(args),
            "starts-with" => self.builtin_starts_with(args),
            "ends-with" => self.builtin_ends_with(args),
            "replace" => self.builtin_replace(args),
            "upper" => self.builtin_upper(args),
            "lower" => self.builtin_lower(args),
            "substr" => self.builtin_substr(args),
            "find" => self.builtin_find(args),
            "format" => self.builtin_format(args),
            // List
            "list" => self.builtin_list_fn(args),
            "car" | "head" | "first" => self.builtin_car(args),
            "cdr" | "tail" | "rest" => self.builtin_cdr(args),
            "cons" => self.builtin_cons(args),
            "len" | "length" | "size" => self.builtin_len(args),
            "push" => self.builtin_push(args),
            "nth" | "at" => self.builtin_nth(args),
            "map" => self.builtin_map(args),
            "filter" | "select" => self.builtin_filter(args),
            "reduce" | "fold" => self.builtin_reduce(args),
            "each" | "for-each" => self.builtin_each(args),
            "range" => self.builtin_range(args),
            "reverse" => self.builtin_reverse(args),
            "sort" => self.builtin_sort(args),
            "flatten" => self.builtin_flatten(args),
            "last" => self.builtin_last(args),
            "empty?" => self.builtin_empty(args),
            "any" => self.builtin_any(args),
            "all" => self.builtin_all(args),
            "zip" => self.builtin_zip(args),
            "assoc" => self.builtin_assoc(args),
            "dissoc" => self.builtin_dissoc(args),
            "keys" => self.builtin_keys(args),
            "values" => self.builtin_values(args),
            "merge" => self.builtin_merge(args),
            // Arithmetic
            "+" => self.builtin_add(args),
            "-" => self.builtin_sub(args),
            "*" => self.builtin_mul(args),
            "/" => self.builtin_div(args),
            "%" | "mod" => self.builtin_mod(args),
            "pow" => self.builtin_pow(args),
            "sqrt" => self.builtin_sqrt(args),
            "abs" => self.builtin_abs(args),
            "min" => self.builtin_min(args),
            "max" => self.builtin_max(args),
            "floor" => self.builtin_floor_fn(args),
            "ceil" => self.builtin_ceil_fn(args),
            "round" => self.builtin_round_fn(args),
            "rand" | "random" => self.builtin_rand(args),
            "inc" => self.builtin_inc(args),
            "dec" => self.builtin_dec(args),
            // Comparison
            "=" | "==" => self.builtin_eq(args),
            "!=" => self.builtin_ne(args),
            "<" => self.builtin_lt(args),
            ">" => self.builtin_gt(args),
            "<=" => self.builtin_lte(args),
            ">=" => self.builtin_gte(args),
            "not" => self.builtin_not(args),
            // Type
            "type" | "type-of" => self.builtin_type(args),
            "int" => self.builtin_int(args),
            "float" => self.builtin_float(args),
            "number?" => self.builtin_is_number(args),
            "string?" => self.builtin_is_string(args),
            "list?" => self.builtin_is_list(args),
            "nil?" => self.builtin_is_nil(args),
            "bool?" => self.builtin_is_bool(args),
            // IO
            "print" => self.builtin_print(args, false),
            "println" => self.builtin_print(args, true),
            "eprint" => self.builtin_eprint(args, false),
            "eprintln" => self.builtin_eprint(args, true),
            "input" => self.builtin_input(args),
            // HTTP
            "http-get" => self.builtin_http(args, "GET"),
            "http-post" => self.builtin_http_with_body(args, "POST"),
            "http-put" => self.builtin_http_with_body(args, "PUT"),
            "http-delete" => self.builtin_http(args, "DELETE"),
            "http" => self.builtin_http_request(args),
            // JSON (in json.rs)
            "json-parse" | "json" => self.builtin_json_parse(args),
            "json-stringify" | "json-str" => self.builtin_json_stringify(args),
            "json-get" | "jget" => self.builtin_json_get(args),
            "json-set" | "jset" => self.builtin_json_set(args),
            "json-keys" => self.builtin_json_keys(args),
            // Misc
            "sleep" => self.builtin_sleep(args),
            "time" => self.builtin_time(args),
            "timestamp" => self.builtin_timestamp(args),
            "exit" | "quit" => self.builtin_exit(args),
            // Regex (in regex.rs)
            "re-test" | "re-match?" => self.builtin_re_test(args),
            "re-match" => self.builtin_re_match(args),
            "re-find" => self.builtin_re_find(args),
            "re-find-all" => self.builtin_re_find_all(args),
            "re-replace" => self.builtin_re_replace(args),
            "re-replace-all" => self.builtin_re_replace_all(args),
            "re-split" => self.builtin_re_split(args),
            "re-scan" => self.builtin_re_scan(args),
            _ => Err(format!("Unknown function: {}", name)),
        }
    }
}
