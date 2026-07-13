use alisp::{count_parens, expr_to_string, Evaluator, VERSION};
use std::io::{self, Write, BufRead};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut eval = Evaluator::new();

    if args.is_empty() {
        // REPL mode
        eprintln!("alisp v{} - A tiny Lisp for AI", VERSION);
        eprintln!("Type (exit) or Ctrl+D to quit.\n");
        let stdin = io::stdin();
        loop {
            eprint!("alisp> ");
            io::stderr().flush().ok();

            let mut input = String::new();
            match stdin.lock().read_line(&mut input) {
                Ok(0) => {
                    eprintln!();
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Read error: {}", e);
                    break;
                }
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            // Handle multiline input
            let mut full_input = input.to_string();
            while count_parens(&full_input) > 0 {
                eprint!("  ... ");
                io::stderr().flush().ok();
                let mut line = String::new();
                match stdin.lock().read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
                full_input.push('\n');
                full_input.push_str(line.trim_end());
            }

            match eval.eval_str(&full_input) {
                Ok(Some(result)) => {
                    let s = expr_to_string(&result);
                    if s != "nil" {
                        println!("{}", s);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    } else if args[0] == "-e" {
        // Execute code string
        let code = args[1..].join(" ");
        if let Err(e) = eval.eval_str(&code) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        // Execute file
        let path = &args[0];
        if let Err(e) = eval.eval_file(path) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
