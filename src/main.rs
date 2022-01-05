use std::env::{args, current_dir};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use anyhow::{bail, Context, Result};
use clap::{App, Arg};

fn main() {
    if let Err(e) = actual_main() {
        eprintln!("Error: {}", e);
        for c in e.chain().skip(1) {
            eprintln!("    {}", c);
        }
        exit(1);
    }
}

fn actual_main() -> Result<()> {
    let mut args: Vec<String> = args().collect();
    if args.len() >= 2 && &args[1] == "recursive" {
        args.remove(1);
    }

    let matches = App::new("cargo recursive")
        .bin_name("cargo recursive")
        .arg(
            Arg::with_name("depth")
                .long("depth")
                .default_value("64")
                .help("Max depth to search into"),
        )
        .arg(
            Arg::with_name("path")
                .short("p")
                .long("path")
                .help("Target directory"),
        )
        .arg(
            Arg::with_name("dry-run")
                .short("d")
                .long("dry-run")
                .help("Only display matched directories, don't actually run the commands"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Verbose output"),
        )
        .arg(
            Arg::with_name("suppress-output")
                .short("s")
                .long("suppress-output")
                .help("Don't print the output of the executed commands"),
        )
        .arg(
            Arg::with_name("exit-on-error")
                .short("e")
                .long("exit")
                .help("Stop if any executed command returns with a nonzero exit code"),
        )
        .arg(
            Arg::with_name("external")
                .short("x")
                .long("external")
                .help("Run any command instead of a cargo command"),
        )
        .arg(
            Arg::with_name("command")
                .multiple(true)
                .help("The command to run"),
        )
        .get_matches_from(&args);

    let depth: usize = matches
        .value_of("depth")
        .expect("'depth' missing")
        .parse()
        .with_context(|| "depth must be an integer")?;

    let path = if let Some(path) = matches.value_of("path") {
        PathBuf::from(path)
    } else {
        current_dir().context("getting current_dir")?
    };

    let dry_run: bool = matches.is_present("dry-run");
    let verbose: bool = matches.is_present("verbose");
    let output: bool = !matches.is_present("suppress-output");
    let exit_on_error: bool = matches.is_present("exit-on-error");
    let external: bool = matches.is_present("external");
    let args = matches
        .values_of("command")
        .map(|vals| vals.collect::<Vec<_>>())
        .expect("Argument command invalid or missing");

    let cmd = CommandInfo {
        external,
        args,
        output,
        exit_on_error,
    };

    process_dir(Path::new(&path), depth, verbose, dry_run, &cmd)?;

    Ok(())
}

fn process_dir(
    path: &Path,
    depth: usize,
    verbose: bool,
    dry_run: bool,
    cmd: &CommandInfo,
) -> Result<()> {
    if depth == 0 {
        return Ok(());
    }

    if path.join("Cargo.toml").exists() {
        if verbose {
            eprintln!("Running in {:?}", path);
        }

        if !dry_run {
            cmd.run(path)
                .with_context(|| format!("running in directory {:?}", path))?;
        }
    }

    for e in path
        .read_dir()
        .with_context(|| format!("reading directory {:?}", path.canonicalize()))?
    {
        let e = e?;
        if e.file_type()?.is_dir() {
            if let Err(e) = process_dir(&e.path(), depth - 1, verbose, dry_run, cmd) {
                if cmd.exit_on_error {
                    return Err(e);
                }
                eprintln!("Warn: {}", e);
                for c in e.chain().skip(1) {
                    eprintln!("    {}", c);
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct CommandInfo<'a> {
    /// Use external binary (i.e. from PATH or absolute path)
    /// instead of implicitly using `cargo` as the binary
    external: bool,
    /// Arguments, see above for the first item
    args: Vec<&'a str>,
    /// Display output of the command after execution
    output: bool,
    /// Exit on error
    exit_on_error: bool,
}
impl<'a> CommandInfo<'a> {
    fn run(&self, path: &Path) -> Result<()> {
        let mut args = self.args.clone();
        if args.is_empty() {
            bail!("Argument list empty");
        }
        let mut cmd = if self.external {
            let cmd_str = args.remove(0);
            Command::new(cmd_str)
        } else {
            Command::new("cargo")
        };

        let output = cmd.args(&args).current_dir(path).output()?;
        if self.output {
            io::stdout().write_all(&output.stdout).unwrap();
            io::stderr().write_all(&output.stderr).unwrap();
        }

        if self.exit_on_error && !output.status.success() {
            if let Some(code) = output.status.code() {
                bail!("Command returned a nonzero code {}", code);
            } else {
                bail!("Command returned an error");
            }
        }
        Ok(())
    }
}
