//! Mott compiler driver.
//!
//! Usage: `mott <file.mott> [-o OUT] [--emit-c] [--keep-c]`
//!
//! Default pipeline: source → tokens → AST → C → clang → native binary.
//! `--emit-c` prints the generated C to stdout instead of invoking clang.
//! `--keep-c` leaves the intermediate C file next to the output binary.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use mott::codegen::{c_backend::CBackend, Backend};
use mott::lexer::Lexer;
use mott::parser::Parser;

struct Args {
    source: PathBuf,
    output: Option<PathBuf>,
    emit_c: bool,
    keep_c: bool,
}

fn parse_args() -> Args {
    let raw: Vec<String> = env::args().collect();
    let mut source: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut emit_c = false;
    let mut keep_c = false;

    let mut i = 1;
    while i < raw.len() {
        let a = &raw[i];
        match a.as_str() {
            "-o" => {
                i += 1;
                if i >= raw.len() {
                    die("missing path after -o");
                }
                output = Some(PathBuf::from(&raw[i]));
            }
            "--emit-c" => emit_c = true,
            "--keep-c" => keep_c = true,
            "-h" | "--help" => {
                print_usage();
                exit(0);
            }
            s if s.starts_with('-') => die(&format!("unknown flag `{}`", s)),
            s => {
                if source.is_some() {
                    die("only one input file is supported");
                }
                source = Some(PathBuf::from(s));
            }
        }
        i += 1;
    }

    let source = source.unwrap_or_else(|| {
        print_usage();
        exit(1);
    });

    Args {
        source,
        output,
        emit_c,
        keep_c,
    }
}

fn print_usage() {
    eprintln!("mott — Mott language compiler (v0.1)");
    eprintln!();
    eprintln!("usage: mott <file.mott> [-o OUT] [--emit-c] [--keep-c]");
    eprintln!();
    eprintln!("  -o OUT      output binary path (default: source stem in CWD)");
    eprintln!("  --emit-c    print generated C to stdout and exit");
    eprintln!("  --keep-c    keep the intermediate .c file next to the binary");
}

fn die(msg: &str) -> ! {
    eprintln!("mott: {}", msg);
    exit(1);
}

fn main() {
    let args = parse_args();

    let source = fs::read_to_string(&args.source).unwrap_or_else(|e| {
        die(&format!("cannot read {}: {}", args.source.display(), e));
    });

    let tokens = Lexer::new(&source).tokenize().unwrap_or_else(|e| {
        die(&format!("in {}: {}", args.source.display(), e));
    });

    let program = Parser::new(tokens).parse().unwrap_or_else(|e| {
        die(&format!("in {}: {}", args.source.display(), e));
    });

    let c_code = CBackend.emit(&program).unwrap_or_else(|e| {
        die(&format!("in {}: {}", args.source.display(), e));
    });

    if args.emit_c {
        print!("{}", c_code);
        return;
    }

    // Pick an output binary path.
    let stem = args
        .source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "a".into());
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(&stem));

    // Locate the runtime (header + .c). Baked-in CARGO_MANIFEST_DIR works
    // for dev; env override lets users relocate the installed runtime.
    let runtime = locate_runtime().unwrap_or_else(|| {
        die("cannot locate runtime directory — set MOTT_RUNTIME or install \
             alongside the compiler");
    });
    let rt_header_dir = runtime.clone();
    let rt_source = runtime.join("mott_rt.c");
    if !rt_source.is_file() {
        die(&format!(
            "runtime file missing: {}",
            rt_source.display()
        ));
    }

    // Write the generated C next to the output binary (or to a temp file).
    let c_path: PathBuf = if args.keep_c {
        let mut p = output.clone();
        p.set_extension("c");
        p
    } else {
        env::temp_dir().join(format!(
            "mott_{}_{}.c",
            stem,
            std::process::id()
        ))
    };
    fs::write(&c_path, &c_code).unwrap_or_else(|e| {
        die(&format!(
            "cannot write intermediate C to {}: {}",
            c_path.display(),
            e
        ));
    });

    // Invoke clang. -std=c11 because we use compound literals and _Bool.
    let status = Command::new("clang")
        .arg("-std=c11")
        .arg("-O2")
        .arg("-Wall")
        .arg("-Wno-unused-parameter")
        .arg("-o")
        .arg(&output)
        .arg(&c_path)
        .arg(&rt_source)
        .arg(format!("-I{}", rt_header_dir.display()))
        .status()
        .unwrap_or_else(|e| die(&format!("failed to spawn clang: {}", e)));

    if !status.success() {
        die("clang failed (see errors above)");
    }

    if !args.keep_c {
        let _ = fs::remove_file(&c_path);
    }
}

/// Resolve the runtime directory:
/// 1. `$MOTT_RUNTIME` if set
/// 2. `<CARGO_MANIFEST_DIR>/runtime` (development builds)
/// 3. `<cwd>/compiler/runtime` or `<cwd>/runtime` (fallback)
fn locate_runtime() -> Option<PathBuf> {
    if let Ok(p) = env::var("MOTT_RUNTIME") {
        let p = PathBuf::from(p);
        if p.is_dir() {
            return Some(p);
        }
    }
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dev = Path::new(manifest_dir).join("runtime");
    if dev.is_dir() {
        return Some(dev);
    }
    let cwd = env::current_dir().ok()?;
    for rel in ["compiler/runtime", "runtime"] {
        let c = cwd.join(rel);
        if c.is_dir() {
            return Some(c);
        }
    }
    None
}
