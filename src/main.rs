mod ast;
mod codegen;
mod lexer;
mod parser;

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{self, Command};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: toy-compiler <input.toy> [-o output]");
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = if args.len() >= 4 && args[2] == "-o" {
        PathBuf::from(&args[3])
    } else {
        // Default output name: input stem without extension
        let stem = std::path::Path::new(input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("a.out");
        PathBuf::from(stem)
    };

    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    // Lex
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            process::exit(1);
        }
    };

    // Parse
    let mut parser = parser::Parser::new(tokens);
    let stmts = match parser.parse_program() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    // Codegen
    let codegen = codegen::Codegen::new();
    let asm = match codegen.generate(&stmts) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Codegen error: {}", e);
            process::exit(1);
        }
    };

    // Write assembly to a temp file (use PID to avoid collisions)
    let tmp_dir = env::temp_dir();
    let pid = process::id();
    let asm_path = tmp_dir.join(format!("toy_output_{}.s", pid));
    let obj_path = tmp_dir.join(format!("toy_output_{}.o", pid));

    {
        let mut f = fs::File::create(&asm_path).expect("failed to create temp .s file");
        f.write_all(asm.as_bytes()).expect("failed to write assembly");
    }

    // Assemble
    let as_status = Command::new("as")
        .args(["-o", obj_path.to_str().unwrap(), asm_path.to_str().unwrap()])
        .status()
        .expect("failed to run assembler");

    if !as_status.success() {
        eprintln!("Assembly failed");
        process::exit(1);
    }

    // Link using cc (handles finding the right SDK and libraries)
    let cc_status = Command::new("cc")
        .args([
            "-o",
            output_path.to_str().unwrap(),
            obj_path.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run linker");

    if !cc_status.success() {
        eprintln!("Linking failed");
        process::exit(1);
    }

    // Clean up temp files
    let _ = fs::remove_file(&asm_path);
    let _ = fs::remove_file(&obj_path);
}
