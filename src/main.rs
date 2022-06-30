use std::error::Error;
use std::io::{stdin, stdout, Write};
use std::io::{BufRead, Read};
use std::ops::ControlFlow;
use std::{collections::HashMap, env, fs, process};

use compiler::common::Ident;
use compiler::compilation::{compile_block, compile_expr, compile_stmt};
use compiler::lexing::Lexer;
use compiler::parsing::Parser;
use compiler::syntax_tree::{Assign, Expr, ExprStmt, Stmt, VarDecl};
use compiler::{codegen, Diagnostic};

fn main() {
    let args = env::args();
    let mut mode = Mode::Run;
    let mut path = None;
    for arg in args.skip(1) {
        match &*arg {
            "parse" => mode = Mode::Parse,
            "bytecode" => mode = Mode::Bytecode,
            "build" => mode = Mode::Build,
            "run" => mode = Mode::Run,
            name if path.is_none() => path = Some(name.to_string()),
            arg => {
                eprintln!(
                    "Unknown command {}, only one source file can be specified",
                    arg
                );
                process::exit(1);
            }
        }
    }

    if let Some(path) = path {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("{}", err);
                process::exit(1);
            }
        };
        if let Err(err) = run(file, mode) {
            eprintln!("{}", err);
            process::exit(1);
        }
    } else {
        let input = stdin();
        if let Err(err) = run(input, mode) {
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Parse,
    Build,
    Bytecode,
    Run,
}

pub fn repl(mode: Mode) -> Result<(), Box<dyn Error>> {
    let mut line = String::new();
    let mut scope = HashMap::new();
    loop {
        line.clear();
        print!("\x1b[34m>\x1b[0m ");
        stdout().flush()?;
        stdin().lock().read_line(&mut line)?;
        if line.is_empty() {
            println!();
            break Ok(());
        }

        match run_stmt(&line, mode, &mut scope)? {
            ControlFlow::Continue(_) => continue,
            ControlFlow::Break(_) => break Ok(()),
        }
    }
}

pub fn run_stmt(
    src: &str,
    mode: Mode,
    scope: &mut HashMap<String, i64>,
) -> Result<ControlFlow<()>, Box<dyn Error>> {
    let mut lexer_diagnostics = Vec::new();
    let mut lexer = Lexer::new(src.chars().peekable(), &mut lexer_diagnostics).peekable();

    let mut parsing_diagnostics = Vec::new();
    let stmt = Parser::new(&mut lexer, &mut parsing_diagnostics).stmt();
    if let Some(span) = lexer.peek().map(|t| t.span) {
        if parsing_diagnostics.is_empty() {
            lexer_diagnostics.push(Diagnostic::warning(span, "Unexpected token, expected EOF"))
        }
    }

    lexer_diagnostics.append(&mut parsing_diagnostics);
    let diagnostics = lexer_diagnostics;

    if !diagnostics.is_empty() {
        for d in diagnostics {
            eprintln!("{}", d.display(src));
        }
    }

    let stmt = if let Some(stmt) = stmt {
        stmt
    } else {
        return Ok(ControlFlow::Continue(()));
    };

    if mode == Mode::Parse {
        println!("{:#?}", stmt);
        return Ok(ControlFlow::Continue(()));
    }

    let (func, print_result) = match &stmt {
        Stmt::Expr(ExprStmt { expr, .. }) => {
            (compile_expr(expr, scope), /*semicolon.is_none()*/ true)
        }
        stmt => (compile_stmt(stmt, scope), false),
    };

    if mode == Mode::Bytecode {
        for (i, block) in func.blocks.iter().enumerate() {
            println!("block{}:", i);
            for instr in &block.instrs {
                println!("    {:?}", instr);
            }
        }
        return Ok(ControlFlow::Continue(()));
    }

    if mode == Mode::Build {
        codegen::output_to_file(&func, "output.o");
        return Ok(ControlFlow::Break(()));
    }

    if mode == Mode::Run {
        let res = codegen::run_jit(&func);
        if let Stmt::Assign(Assign {
            assignee: Expr::Ident(Ident { name, .. }),
            ..
        })
        | Stmt::VarDecl(VarDecl {
            ident: Ident { name, .. },
            ..
        }) = stmt
        {
            scope.insert(name, res);
        }
        if print_result {
            println!("{}", res);
        }
        return Ok(ControlFlow::Continue(()));
    }

    unimplemented!("Unimplemented mode {:?}", mode)
}

pub fn run(mut input: impl Read, mode: Mode) -> Result<(), Box<dyn Error>> {
    let mut src = String::new();
    input.read_to_string(&mut src)?;
    let src = src;

    let mut lexer_diagnostics = Vec::new();
    let mut lexer = Lexer::new(src.chars().peekable(), &mut lexer_diagnostics).peekable();

    let mut parsing_diagnostics = Vec::new();
    let block = Parser::new(&mut lexer, &mut parsing_diagnostics).block();
    if let Some(span) = lexer.peek().map(|t| t.span) {
        if parsing_diagnostics.is_empty() {
            lexer_diagnostics.push(Diagnostic::warning(span, "Unexpected token, expected EOF"))
        }
    }

    lexer_diagnostics.append(&mut parsing_diagnostics);
    let diagnostics = lexer_diagnostics;

    if !diagnostics.is_empty() {
        for d in diagnostics {
            eprintln!("{}", d.display(&src));
        }
    }

    let block = if let Some(block) = block {
        block
    } else {
        return Ok(());
    };

    if mode == Mode::Parse {
        println!("{:#?}", block);
        return Ok(());
    }

    let func = compile_block(&block);

    if mode == Mode::Bytecode {
        for (i, block) in func.blocks.iter().enumerate() {
            println!("block{}:", i);
            for instr in &block.instrs {
                println!("    {:?}", instr);
            }
        }
        return Ok(());
    }

    if mode == Mode::Build {
        codegen::output_to_file(&func, "output.o");
        return Ok(());
    }

    if mode == Mode::Run {
        codegen::run_jit(&func);
        return Ok(());
    }

    unimplemented!("Unimplemented mode {:?}", mode)
}
