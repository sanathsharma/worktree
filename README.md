# Worktree

A CLI tool for managing and navigating git worktrees across multiple directories with fuzzy finding capabilities.

## Overview

Worktree helps you quickly navigate between git worktrees across multiple root directories. It provides a clean interface using fzf for selecting and switching to different worktrees, with preview capabilities showing directory contents and git information.

## Features

- Scans multiple root directories for git repositories
- Lists all worktrees from discovered repositories
- Interactive fuzzy finding with fzf
- Clean display showing directory/worktree names with branch/commit info
- Rich preview showing full path, branch, commit hash, and directory listing
- Configurable via JSON config file or command line arguments

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd worktree

# Build the project
cargo build --release

# Optional: Move the binary to your PATH
cp target/release/worktree ~/.local/bin/
```

## Usage

### Basic Usage

```bash
# Use with default config
worktree

# Specify directories via command line
worktree -d "~/projects,~/work,~/personal"

# Use custom config file
worktree -c ~/.config/worktree/custom-config.json
```

### Shell Integration

Since the tool outputs the selected path, you need to use shell-specific methods to change directories:

#### Bash/Zsh
```bash
# Create an alias or function
alias wt='cd "$(worktree)"'

# Or as a function for more control
wt() {
    local result=$(worktree "$@")
    if [[ -n "$result" ]]; then
        cd "$result"
    fi
}
```

#### Fish Shell
```fish
# Create a function in ~/.config/fish/functions/wt.fish
function wt
    set result (worktree $argv)
    if test -n "$result"
        cd $result
    end
end

# Or as an alias
alias wt 'cd (worktree)'
```

#### Nushell
```nushell
# Create an alias
alias wt = cd (worktree)

# Or as a custom command
def wt [] {
    let result = (worktree)
    if ($result | is-not-empty) {
        cd $result
    }
}
```

## Configuration

### Config File

Create a config file at `~/.config/worktree/config.json`:

```json
{
  "directories": [
    "~/projects",
    "~/work",
    "~/personal",
    "/path/to/other/repos_parent"
  ],
  "sort": "tmux"
}
```

The tool will automatically expand `~` to your home directory.

### Default Config Location

By default, the tool looks for config at `~/.config/worktree/config.json`. If the file doesn't exist, it will show an info message and use an empty directory list.

### Command Line Options

```bash
worktree --help
```

- `-d, --directories <DIRS>`: Comma-separated list of directories to scan
- `-c, --config <PATH>`: Path to config file (default: `~/.config/worktree/config.json`)
- `--sort <CRITERIA>`: Sort worktrees by criteria (currently supports: `tmux`). Can also be set in config file.

## Sorting Options

### Tmux Sorting

When using `--sort=tmux`, worktrees are sorted based on tmux session activity with a specific priority order:

1. **Previous session** - The tmux session you were in before the current one (most recent non-current session)
2. **Current session** - Your currently active tmux session
3. **Other recent sessions** - Remaining tmux sessions sorted by most recently used
4. **Worktrees without tmux sessions** - Sorted alphabetically at the end

This ordering makes it easy to quickly swap between your current and previous tmux sessions.

```bash
# Sort worktrees by tmux session activity
worktree --sort=tmux
```

## How It Works

1. **Load Configuration**: Reads from config file or command line arguments
2. **Scan Directories**: Looks for git repositories in each specified root directory
3. **Collect Worktrees**: Runs `git worktree list --porcelain` on each discovered repository
4. **Format Display**: Shows directory name with branch and short commit hash
5. **Interactive Selection**: Uses fzf for selection with rich preview
6. **Output Path**: Prints the selected worktree path for shell integration

## fzf Preview

The tool provides rich preview information when selecting worktrees:

- **Path**: Full path to the worktree
- **Branch**: Git branch name
- **Commit**: Full commit hash
- **Directory Listing**: Contents of the worktree directory

## Requirements

- Rust 1.70 or later
- Git (for worktree commands)
- fzf (for interactive selection)

## Example Workflow

1. Set up your config file with root directories
2. Create a shell alias/function for easy access
3. Use `wt` (or your chosen alias) to quickly navigate between worktrees
4. Enjoy fast navigation across all your projects!

```bash
# Setup
echo '{"directories": ["~/projects", "~/work"]}' > ~/.config/worktree/config.json

# Usage (after setting up shell integration)
wt                    # Opens fzf with all worktrees
# Select a worktree and you're there!
```

## License

[Copyright (c) 2025 Sanath Sharma](LICENSE)

