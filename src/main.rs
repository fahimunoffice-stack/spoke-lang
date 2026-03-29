mod lexer;
mod ast;
mod parser;
mod error;
mod rules;
mod codegen;

use clap::{Parser as ClapParser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use parser::Parser;
use rules::RuleEngine;
use codegen::WebTarget;
use error::{print_error, print_success, print_warning};

#[derive(ClapParser)]
#[command(name = "spokec", version = "0.1.0", about = "The Spoke language compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile a .spoke file into a real app
    Build {
        file: PathBuf,
        #[arg(long, value_delimiter = ' ', num_args = 1..)]
        target: Vec<String>,
    },
    /// Check for errors without generating code
    Check { file: PathBuf },
    /// Debug internals: spokec debug ast|tokens|plan app.spoke
    Debug { what: String, file: PathBuf },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { file, target } => cmd_build(file, target),
        Command::Check { file }         => cmd_check(file),
        Command::Debug { what, file }   => cmd_debug(what, file),
    }
}

// ─── Build ────────────────────────────────────────────────────────────────────

fn cmd_build(file: PathBuf, targets: Vec<String>) {
    let source = unwrap_or_exit(read_file(&file));

    println!("{}", format!("spokec {}", env!("CARGO_PKG_VERSION")).dimmed());
    println!("Compiling {} ...\n", file.display().to_string().cyan());

    let tokens   = unwrap_or_exit(lexer::tokenize(&source));
    print_success(&format!("Lexed:    {} tokens", tokens.len()));

    let ast      = unwrap_or_exit(Parser::new(tokens).parse());
    print_success(&format!("Parsed:   {} declarations", ast.declarations.len()));

    let resolved = unwrap_or_exit(RuleEngine::resolve(&ast));
    print_success(&format!(
        "Resolved: {} entities, {} actions, {} pages",
        resolved.entities.len(), resolved.actions.len(), resolved.pages.len()
    ));

    let targets = if targets.is_empty() { vec!["web".to_string()] } else { targets };
    println!();

    for target in &targets {
        let out_dir = PathBuf::from(format!("out/{}", target));
        std::fs::create_dir_all(&out_dir).ok();

        match target.as_str() {
            "web" => {
                println!("{}", "[ web target ]".cyan().bold());
                let files = WebTarget::generate(&resolved, &out_dir);
                for f in &files {
                    print_success(&format!("  {}", f.path));
                }
                println!();
                println!("{}", "Done! Run your app:".green().bold());
                println!("  cd out/web");
                println!("  cp .env.example .env   # fill in DB url + secrets");
                println!("  npm install");
                println!("  npm run db:push");
                println!("  npm run dev");
            }
            "mobile"  => print_warning("Flutter codegen coming soon"),
            "desktop" => print_warning("Tauri codegen coming soon"),
            "server"  => print_warning("Go codegen coming soon"),
            unknown   => eprintln!("{} Unknown target '{}'", "error:".red().bold(), unknown),
        }
    }
}

// ─── Check ────────────────────────────────────────────────────────────────────

fn cmd_check(file: PathBuf) {
    let source = unwrap_or_exit(read_file(&file));
    let tokens = unwrap_or_exit(lexer::tokenize(&source));
    match Parser::new(tokens).parse() {
        Ok(ast) => print_success(&format!(
            "{} — no errors ({} declarations)",
            file.display(), ast.declarations.len()
        )),
        Err(e) => { print_error(&e); std::process::exit(1); }
    }
}

// ─── Debug ────────────────────────────────────────────────────────────────────

fn cmd_debug(what: String, file: PathBuf) {
    let source = unwrap_or_exit(read_file(&file));

    match what.as_str() {
        "tokens" => {
            let tokens = unwrap_or_exit(lexer::tokenize(&source));
            for t in tokens {
                println!("  {}:{}\t{:?}", t.line, t.col, t.token);
            }
        }
        "ast" => {
            let tokens = unwrap_or_exit(lexer::tokenize(&source));
            let ast    = unwrap_or_exit(Parser::new(tokens).parse());
            println!("{}", serde_json::to_string_pretty(&ast).unwrap());
        }
        "plan" => {
            let tokens   = unwrap_or_exit(lexer::tokenize(&source));
            let ast      = unwrap_or_exit(Parser::new(tokens).parse());
            let resolved = unwrap_or_exit(RuleEngine::resolve(&ast));

            println!("\n{}", "=== ENTITIES ===".cyan().bold());
            for e in &resolved.entities {
                println!("\n{} (table: {}s)", e.name.green().bold(), e.table);
                for f in &e.fields {
                    println!("  {:<20} {:?}{}",
                        f.name,
                        f.field_type,
                        if f.required { "" } else { "  (optional)" }
                    );
                }
            }

            println!("\n{}", "=== AUTH ===".cyan().bold());
            match &resolved.auth {
                Some(a) => {
                    println!("  email+password : {}", a.email_password);
                    println!("  magic link     : {}", a.magic_link);
                    println!("  oauth          : {:?}", a.oauth_providers);
                    println!("  session        : {} days", a.session_days);
                    println!("  after login    : {}", a.login_redirect);
                }
                None => println!("  none"),
            }

            println!("\n{}", "=== ACTIONS ===".cyan().bold());
            for a in &resolved.actions {
                println!("  {:?}  {}  ({:?})", a.kind, a.entity, a.ownership);
            }

            println!("\n{}", "=== PAGES ===".cyan().bold());
            for p in &resolved.pages {
                println!("  {}  ->  {}", p.name, p.route);
            }

            println!("\n{}", "=== TRIGGERS ===".cyan().bold());
            for t in &resolved.triggers {
                println!("  on {}:", t.event);
                for act in &t.actions {
                    println!("    {}", act);
                }
            }
        }
        other => {
            eprintln!("{} Unknown '{}'. Use: ast, tokens, plan", "error:".red().bold(), other);
            std::process::exit(1);
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn read_file(path: &PathBuf) -> Result<String, error::SpokeError> {
    std::fs::read_to_string(path).map_err(|e| error::SpokeError::FileRead {
        path: path.display().to_string(),
        reason: e.to_string(),
    })
}

fn unwrap_or_exit<T>(r: Result<T, error::SpokeError>) -> T {
    match r {
        Ok(v)  => v,
        Err(e) => { print_error(&e); std::process::exit(1); }
    }
}
