//! Build tasks for beryl-router eBPF programs.

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::{
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Debug, Parser)]
enum Args {
    /// Build the eBPF program
    BuildEbpf(BuildEbpf),
    /// Build the userspace binary
    Build(Build),
}

#[derive(Debug, Parser)]
struct BuildEbpf {
    /// Build in release mode
    #[arg(long)]
    release: bool,
}

#[derive(Debug, Parser)]
struct Build {
    /// Build in release mode
    #[arg(long)]
    release: bool,

    /// Target triple for cross-compilation
    #[arg(long, default_value = "aarch64-unknown-linux-musl")]
    target: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args {
        Args::BuildEbpf(opts) => build_ebpf(opts),
        Args::Build(opts) => build(opts),
    }
}

fn build_ebpf(opts: BuildEbpf) -> Result<()> {
    let dir = workspace_root().join("beryl-router-ebpf");

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .args([
            "+nightly",
            "build",
            "--target=bpfel-unknown-none",
            "-Z",
            "build-std=core",
        ]);

    if opts.release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("Failed to run cargo")?;
    if !status.success() {
        bail!("eBPF build failed");
    }

    Ok(())
}

fn build(opts: Build) -> Result<()> {
    // First build the eBPF program
    build_ebpf(BuildEbpf {
        release: opts.release,
    })?;

    // Then build the userspace binary
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .args(["build", "--package", "beryl-routerd"])
        .arg(format!("--target={}", opts.target));

    if opts.release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("Failed to run cargo")?;
    if !status.success() {
        bail!("Userspace build failed");
    }

    println!("\nBuild complete!");
    println!(
        "Binary: target/{}/{}/beryl-routerd",
        opts.target,
        if opts.release { "release" } else { "debug" }
    );

    Ok(())
}

fn workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .to_path_buf()
}
