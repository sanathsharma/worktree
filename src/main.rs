mod args;
mod config;
mod git;

use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use tokio::task::JoinSet;

use crate::git::{Worktree, is_git_repo};

async fn get_directories() -> Vec<String> {
  let config = config::get_config().await;
  let matches = args::get_matches();

  if let Some(dirs) = matches.get_one::<String>("directories") {
    dirs.split(',').map(|s| s.trim().to_string()).collect()
  } else {
    config.directories
  }
}

async fn get_sort_option() -> Option<String> {
  let config = config::get_config().await;
  let matches = args::get_matches();

  matches
    .get_one::<String>("sort")
    .cloned()
    .or_else(|| config.sort)
}

async fn collect_all_worktrees(directories: Vec<String>) -> Vec<Worktree> {
  let mut set = JoinSet::new();

  for dir in directories {
    let expanded_dir = shellexpand::tilde(&dir).to_string();
    set.spawn(collect_worktrees_from_directory(expanded_dir));
  }

  let mut all_worktrees = Vec::new();
  while let Some(result) = set.join_next().await {
    if let Ok(worktrees) = result {
      all_worktrees.extend(worktrees);
    }
  }

  all_worktrees
}

async fn collect_worktrees_from_directory(root_dir: String) -> Vec<Worktree> {
  let Ok(entries) = std::fs::read_dir(&root_dir) else {
    return Vec::new();
  };

  let git_repos: Vec<_> = entries
    .filter_map(|entry| entry.ok())
    .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
    .map(|entry| entry.path().to_string_lossy().to_string())
    .collect();

  let mut set = JoinSet::new();

  for repo_path in git_repos {
    set.spawn(async move {
      if is_git_repo(&repo_path).await {
        git::get_worktrees(&repo_path).await.unwrap_or_default()
      } else {
        Vec::new()
      }
    });
  }

  let mut all_worktrees = Vec::new();
  while let Some(result) = set.join_next().await {
    if let Ok(worktrees) = result {
      all_worktrees.extend(worktrees);
    }
  }

  all_worktrees
}

fn format_worktrees(worktrees: &[Worktree]) -> Vec<String> {
  worktrees
    .iter()
    .map(|wt| {
      let commit_short = if wt.commit.len() >= 8 {
        &wt.commit[..8]
      } else {
        &wt.commit
      };
      let branch_clean = wt.branch.strip_prefix("refs/heads/").unwrap_or(&wt.branch);
      let dir_name = std::path::Path::new(&wt.path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&wt.path);
      format!(
        "{}\t{}\t{}\t{}\t({}) [{}]",
        dir_name, wt.path, branch_clean, wt.commit, branch_clean, commit_short
      )
    })
    .collect()
}

fn get_tmux_sessions() -> HashMap<String, u64> {
  let output = match Command::new("tmux")
    .args(&[
      "list-sessions",
      "-F",
      "#{session_name}:#{session_last_attached}",
    ])
    .output()
  {
    Ok(output) if output.status.success() => output,
    _ => return HashMap::new(),
  };

  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut sessions = HashMap::new();

  for line in stdout.lines() {
    if let Some((name, timestamp)) = line.split_once(':') {
      if let Ok(ts) = timestamp.parse::<u64>() {
        sessions.insert(name.to_string(), ts);
      }
    }
  }

  sessions
}

fn get_current_tmux_session() -> Option<String> {
  let output = Command::new("tmux")
    .args(&["display-message", "-p", "#S"])
    .output()
    .ok()?;

  if output.status.success() {
    let session_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(session_name)
  } else {
    None
  }
}

fn sort_worktrees_by_tmux(worktrees: &mut [Worktree], tmux_sessions: &HashMap<String, u64>) {
  let current_session = get_current_tmux_session();

  // Find the previous session (most recent session that is not the current session)
  let previous_session = if let Some(ref current) = current_session {
    tmux_sessions
      .iter()
      .filter(|(name, _)| *name != current)
      .max_by_key(|(_, time)| *time)
      .map(|(name, _)| name.clone())
  } else {
    None
  };

  worktrees.sort_by(|a, b| {
    let dir_name_a = std::path::Path::new(&a.path)
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or(&a.path);
    let dir_name_b = std::path::Path::new(&b.path)
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or(&b.path);

    let time_a = tmux_sessions.get(dir_name_a).copied().unwrap_or(0);
    let time_b = tmux_sessions.get(dir_name_b).copied().unwrap_or(0);

    let is_previous_a = previous_session.as_deref() == Some(dir_name_a);
    let is_previous_b = previous_session.as_deref() == Some(dir_name_b);
    let is_current_a = current_session.as_deref() == Some(dir_name_a);
    let is_current_b = current_session.as_deref() == Some(dir_name_b);

    // Priority order:
    // 1. Previous session (most recent non-current session)
    // 2. Current session
    // 3. Other sessions with tmux activity (by recency)
    // 4. Sessions without tmux activity (alphabetical)

    match (is_previous_a, is_previous_b, is_current_a, is_current_b) {
      (true, _, _, _) => std::cmp::Ordering::Less, // Previous session first
      (_, true, _, _) => std::cmp::Ordering::Greater, // Others after previous
      (_, _, true, _) => std::cmp::Ordering::Less, // Current session before others (except previous)
      (_, _, _, true) => std::cmp::Ordering::Greater, // Others after current
      (_, _, _, _) => {
        // Neither previous nor current
        match (time_a > 0, time_b > 0) {
          (true, true) => {
            // Both have activity - sort by time (most recent first)
            time_b.cmp(&time_a).then(dir_name_a.cmp(dir_name_b))
          }
          (true, false) => std::cmp::Ordering::Less, // Has activity before no activity
          (false, true) => std::cmp::Ordering::Greater, // No activity after has activity
          (false, false) => dir_name_a.cmp(dir_name_b), // Neither has activity - alphabetical
        }
      }
    }
  });
}

async fn run_fzf_selection(
  formatted_lines: Vec<String>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
  let mut fzf = Command::new("fzf")
		.arg("--height=20")
		.arg("--reverse")
		.arg("--delimiter=\t")
		.arg("--with-nth=1,5")
		.arg("--preview=echo 'Path: {2}' && echo 'Branch: {3}' && echo 'Commit: {4}' && echo '' && ls -la {2}")
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()?;

  if let Some(mut stdin) = fzf.stdin.take() {
    for line in formatted_lines {
      writeln!(stdin, "{}", line)?;
    }
  }

  let output = fzf.wait_with_output()?;

  if output.status.success() {
    let selected = String::from_utf8(output.stdout)?;
    let selected = selected.trim();

    if !selected.is_empty() {
      return Ok(Some(selected.to_string()));
    }
  }

  Ok(None)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let directories = get_directories().await;
  let mut all_worktrees = collect_all_worktrees(directories).await;

  // Handle sorting
  if let Some(sort_option) = get_sort_option().await {
    if sort_option == "tmux" {
      let tmux_sessions = get_tmux_sessions();
      sort_worktrees_by_tmux(&mut all_worktrees, &tmux_sessions);
    }
  }

  let formatted_lines = format_worktrees(&all_worktrees);

  if formatted_lines.is_empty() {
    eprintln!("No worktrees found");
    return Ok(());
  }

  if let Some(selected) = run_fzf_selection(formatted_lines).await? {
    let path = selected.split('\t').nth(1).unwrap();
    println!("{path}");
  }

  std::process::exit(0);
}
