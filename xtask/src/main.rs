use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Automation tasks for lit-bit")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run CI checks for a specific target
    Ci {
        /// Target triple to build for
        #[arg(long, default_value = "x86_64-unknown-linux-gnu")]
        target: String,
    },
    /// Run all tests
    Test,
    /// Run benchmarks in smoke mode
    Bench {
        /// Run in smoke mode (quick)
        #[arg(long)]
        smoke: bool,
    },
    /// Check all targets
    CheckAll,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ci { target } => run_ci(&target),
        Commands::Test => run_tests(),
        Commands::Bench { smoke } => run_benchmarks(smoke),
        Commands::CheckAll => check_all_targets(),
    }
}

fn run_ci(target: &str) -> Result<()> {
    println!("Running CI for target: {}", target);

    match target {
        "thumbv7m-none-eabi" | "riscv32imac-unknown-none-elf" => {
            // Embedded targets - only check core with no-std
            run_command(&[
                "cargo",
                "check",
                "--target",
                target,
                "-p",
                "lit-bit-core",
                "--no-default-features",
            ])?;
            println!("✓ Embedded target {} builds successfully", target);
        }
        _ => {
            // Host targets - run full CI
            run_command(&["cargo", "check", "--workspace"])?;
            run_command(&["cargo", "test", "-p", "lit-bit-tests"])?;
            run_command(&["cargo", "check", "-p", "lit-bit-bench"])?;
            println!("✓ Host target {} passes all checks", target);
        }
    }

    Ok(())
}

fn run_tests() -> Result<()> {
    println!("Running all tests...");
    run_command(&["cargo", "test", "-p", "lit-bit-tests"])?;
    println!("✓ All tests passed");
    Ok(())
}

fn run_benchmarks(smoke: bool) -> Result<()> {
    if smoke {
        println!("Running benchmarks in smoke mode...");
        run_command(&["cargo", "check", "-p", "lit-bit-bench"])?;
        println!("✓ Benchmarks compile successfully");
    } else {
        println!("Running full benchmarks...");
        run_command(&["cargo", "bench", "-p", "lit-bit-bench"])?;
        println!("✓ Benchmarks completed");
    }
    Ok(())
}

fn check_all_targets() -> Result<()> {
    let targets = [
        "x86_64-unknown-linux-gnu",
        "thumbv7m-none-eabi",
        "riscv32imac-unknown-none-elf",
    ];

    for target in &targets {
        println!("Checking target: {}", target);
        run_ci(target)?;
    }

    println!("✓ All targets check successfully");
    Ok(())
}

fn run_command(args: &[&str]) -> Result<()> {
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);

    let output = cmd.output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Command failed: {}\nstdout: {}\nstderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
