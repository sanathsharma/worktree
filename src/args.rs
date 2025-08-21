use clap::{Arg, ArgMatches, Command};
use std::env;

pub fn get_matches() -> ArgMatches {
  Command::new("worktree")
    .version(env!("CARGO_PKG_VERSION"))
    .about("Manage multiple git worktrees")
    .arg(
      Arg::new("directories")
        .short('d')
        .long("directories")
        .help("Comma-separated list of directories to list worktrees from")
        .value_name("DIRS"),
    )
    .arg(
      Arg::new("config")
        .short('c')
        .long("config")
        .help("Path to the config file")
        .default_value("~/.config/worktree/config.json")
        .value_name("PATH"),
    )
    .get_matches()
}
