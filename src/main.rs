use caps::{CapSet, Capability};
use clap::Parser;
use nix::unistd::chroot;
use std::fs;
use std::fs::read_link;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, ExitStatus};
use anyhow::Context;
use which::which;

#[derive(Debug, Parser)]
struct Opts {
    root_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();

    println!("Chroot {:?}", opts.root_dir);

    // must be a dir
    if !opts.root_dir.is_dir() {
        anyhow::bail!("{} is not a directory", opts.root_dir.display());
    }

    // must be empty
    // if fs::read_dir(&opts.root_dir)?.count() > 0 {
    //     anyhow::bail!("{} is not empty", opts.root_dir.display());
    // }

    // todo;; must be writable by current user
    //

    // must have chroot cap
    caps::has_cap(None, CapSet::Effective, Capability::CAP_SYS_CHROOT)?;

    // create directory tree
    tree(&opts.root_dir)?;

    copy_exec(&which("bash")?, &opts.root_dir)?;
    copy_exec(&which("sh")?, &opts.root_dir)?;
    copy_exec(&which("ls")?, &opts.root_dir)?;
    copy_exec(&which("echo")?, &opts.root_dir)?;
    copy_exec(&which("tar")?, &opts.root_dir)?;
    copy_exec(&which("gzip")?, &opts.root_dir)?;
    copy_exec(&which("touch")?, &opts.root_dir)?;


    // chroot
    chroot(&opts.root_dir).expect("chroot failed");
    println!("chrooted into: {}", &opts.root_dir.display());
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

    println!("chroot shell: {shell}");
    let result = Command::new(shell).spawn()?.wait().context("waiting for shell failed")?;
    if !result.success() {
        Ok(())
    } else {
        anyhow::bail!("shell exited unsuccessfully");
    }
}

fn tree(root_dir: &Path) -> anyhow::Result<()> {
    add_dir(root_dir.join("bin"))?;
    add_dir(root_dir.join("lib64"))?;
    add_dir(root_dir.join("lib/x86_64-linux-gnu"))?;
    add_dir(root_dir.join("usr/bin"))?;
    add_dir(root_dir.join("usr/local/bin"))?;
    add_dir(root_dir.join("home"))?;
    add_dir(root_dir.join("opt"))?;
    add_dir(root_dir.join("tmp"))?;

    Ok(())
}

fn add_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    fs::create_dir_all(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn copy_exec(exec: &Path, root: &Path) -> anyhow::Result<()> {
    fs::copy(exec, root.join(exec.strip_prefix("/")?))?;
    for dep in get_exec_deps(exec)? {
        let dest = root.join(dep.strip_prefix("/")?);
        println!("copy {} to {}", dep.display(), dest.display());
        fs::copy(&dep, dest)?;
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
            follow(lhs.trim().into(), &mut result)?;
            continue;
        }
        if let Some((_, rhs)) = line.split_once("=>") {
            let (lhs, _) = rhs.rsplit_once("(").context("split deps failed")?;
            follow(lhs.trim().into(), &mut result)?;
        }
    }

    Ok(result)
}

fn follow(path: PathBuf, paths: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    paths.push(path.clone().into());
    if !path.is_symlink() {
        return Ok(())
    }
    let next = path.read_link()?;
    if next.is_relative() {
        let dir = path.parent().ok_or(anyhow::anyhow!("no parent"))?;
        follow(dir.join(next), paths)
    } else {
        follow(next, paths)
    }
}
