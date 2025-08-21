use tokio::process::Command;

pub struct Worktree {
  pub path: String,
  pub branch: String,
  pub commit: String,
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum GitError {
  #[error("Failed to execute {0}")]
  FailedToExecuteCmd(String),
}

type Result<T> = std::result::Result<T, GitError>;

pub async fn is_git_repo(dir: &str) -> bool {
  let output = Command::new("git")
    .args(&["rev-parse", "--git-dir"])
    .current_dir(dir)
    .output()
    .await;

  let output = match output {
    Ok(output) => output,
    Err(_) => return false,
  };

  output.status.success()
}

pub async fn get_worktrees(dir: &str) -> Result<Vec<Worktree>> {
  let output = Command::new("git")
    .args(&["worktree", "list", "--porcelain"])
    .current_dir(dir)
    .output()
    .await
    .map_err(|_| GitError::FailedToExecuteCmd(String::from("git worktree list --porcelain")))?;

  if !output.status.success() {
    return Err(GitError::FailedToExecuteCmd(String::from(
      "git worktree list --porcelain",
    )));
  }

  parse_worktrees_output(&String::from_utf8_lossy(&output.stdout))
}

pub fn parse_worktrees_output(output: &str) -> Result<Vec<Worktree>> {
  let mut worktrees: Vec<Worktree> = Vec::new();
  let lines = String::from(output)
    .lines()
    .map(|line| line.to_string())
    .collect::<Vec<String>>();

  let mut path = String::new();
  let mut branch = String::new();
  let mut commit = String::new();

  for (index, line) in lines.iter().enumerate() {
    // 1. if line starts with worktree, get the path after first space
    if line.starts_with("worktree") {
      path = line.split_whitespace().nth(1).unwrap().to_string();
    }
    // 2. if line starts with branch, get the branch after first space
    if line.starts_with("branch") {
      branch = line.split_whitespace().nth(1).unwrap().to_string();
    }
    // 3. if line starts with HEAD, get the commit after first space
    if line.starts_with("HEAD") {
      commit = line.split_whitespace().nth(1).unwrap().to_string();
    }
    // 4. if line starts with bare, skip the worktree
    if line.starts_with("bare") {
      path.clear();
      continue;
    }

    let is_last = lines.len() - 1 == index;
    // 5. if empty line, commit or skip current iteration
    if line.is_empty() || is_last {
      if !path.is_empty() {
        worktrees.push(Worktree {
          path: path.clone(),
          branch: branch.clone(),
          commit: commit.clone(),
        });
      }
      path.clear();
      branch.clear();
      commit.clear();
    }
  }

  Ok(worktrees)
}
