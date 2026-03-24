mod config;

use anyhow::Context;
use caps::{CapSet, Capability};
use clap::Parser;
use nix::unistd::chroot;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which_all;

#[derive(Debug, Parser)]
struct Opts {
    root_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let cfg = config::load("chroot.toml")?;

    // must be a dir
    if !opts.root_dir.is_dir() {
        anyhow::bail!("{} is not a directory", opts.root_dir.display());
    }

    // todo;; must be writable by current user
    //

    // must have chroot cap
    caps::has_cap(None, CapSet::Effective, Capability::CAP_SYS_CHROOT)?;

    // create directory tree
    for dir in cfg.tree {
        add_dir(opts.root_dir.join(dir))?;
    }

    for exec in cfg.exec {
        copy_exec(which_all(exec)?, &opts.root_dir)?;
    }

    // chroot
    chroot(&opts.root_dir).expect("chroot failed");
    std::env::set_current_dir("/").expect("set pwd failed");

    // drop caps
    for cset in [CapSet::Effective, CapSet::Permitted, CapSet::Inheritable] {
        caps::clear(None, cset).expect("caps cleared failed");
    }

    let shells = ["/usr/bin/bash", "/usr/bin/sh"];
    let shell = shells
        .iter()
        .find(|s| std::path::Path::new(s).exists())
        .context("no usable shell found inside chroot")?;

    println!("chroot with {shell} @ {}", &opts.root_dir.display());
    let result = Command::new(shell).env("PATH", "/bin:/usr/bin").spawn()?.wait().context("waiting for shell failed")?;
    println!("exited chroot: {}", result.code().unwrap_or(-999));

    Ok(())
}

fn add_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    fs::create_dir_all(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn copy_exec<P: Iterator<Item = PathBuf>>(execs: P, chroot: &Path) -> anyhow::Result<()> {
    let all = execs.collect::<Vec<_>>();
    for exec in all.iter() {
        fs::copy(exec, chroot.join(exec.strip_prefix("/")?))?;
    }
    if let Some(exec) = all.first() {
        for dep in get_exec_deps(exec)? {
            let dest = chroot.join(dep.strip_prefix("/")?);
            //println!("copy {} to {}", dep.display(), dest.display());
            fs::copy(&dep, dest)?;
        }
    }
    Ok(())
}

fn get_exec_deps(exec: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let output = Command::new("ldd").arg(exec).output()?.stdout;
    let output = String::from_utf8(output)?;

    let mut result = Vec::new();
    for line in output.lines().map(|l| l.trim()).filter(|l| l.starts_with("/") || l.contains("=>")) {
        if line.starts_with("/") {
            let (lhs, _) = line.rsplit_once("(").context("split deps failed")?;
            result.push(lhs.trim().into());
            continue;
        }
        if let Some((_, rhs)) = line.split_once("=>") {
            let (lhs, _) = rhs.rsplit_once("(").context("split deps failed")?;
            result.push(lhs.trim().into());
        }
    }

    Ok(result)
}
