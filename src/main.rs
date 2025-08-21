mod args;
mod config;
mod git;

use std::process::{Command, Stdio};
use std::io::Write;
use tokio::task::JoinSet;

use crate::git::{is_git_repo, Worktree};

async fn get_directories() -> Vec<String> {
	let config = config::get_config().await;
	let matches = args::get_matches();
	
	if let Some(dirs) = matches.get_one::<String>("directories") {
		dirs.split(',').map(|s| s.trim().to_string()).collect()
	} else {
		config.directories
	}
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
			format!("{}\t{}\t{}\t{}\t({}) [{}]", dir_name, wt.path, branch_clean, wt.commit, branch_clean, commit_short)
		})
		.collect()
}

async fn run_fzf_selection(formatted_lines: Vec<String>) -> Result<Option<String>, Box<dyn std::error::Error>> {
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
	let all_worktrees = collect_all_worktrees(directories).await;
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
