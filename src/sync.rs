use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

const SCRIPT: &str = r#"
set -e
dir=$1; branch=$2
j() { jj --repository "$dir" "$@"; }
j st >/dev/null 2>&1 || exit 0
[ -n "$(j diff)" ] || exit 0
j git fetch
j rebase -d "$branch@origin"
j commit -m "$(date +%Y-%m-%d) - to describe"
j bookmark move "$branch" --to @-
j git push -b "$branch"
"#;

fn command(dir: &Path, branch: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", SCRIPT, "sh"]).arg(dir).arg(branch).stdin(Stdio::null());
    cmd
}

pub fn sync_blocking(dir: &Path, branch: &str) -> Result<(), String> {
    let out = command(dir, branch).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    Err(err.lines().last().unwrap_or("sync failed").to_string())
}

pub fn spawn_detached(dir: &Path, branch: &str) {
    let _ = command(dir, branch).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0).spawn();
}
