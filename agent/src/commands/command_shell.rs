// Import required modules for IO, process handling, and filesystem operations
use std::io::{self, Write}; 
use std::process::Command; 
use std::path::Path;
use std::fs;

/// Represents different commands that can be executed in the shell
/// Each variant corresponds to a specific type of operation
#[derive(Debug)]
enum ShellCommand {
    Execute(String, Vec<String>),
    ChangeDir(String),
    Exit,
    Help,
    TreeView(String),
}

/// Recursively displays a directory tree structure
/// # Arguments
/// * `path` - The starting path to display
/// * `prefix` - The prefix to use for the current line (for formatting)
/// * `is_last` - Whether this is the last item in the current directory
/// # Returns
/// * `io::Result<()>` - Success or error status
fn display_tree(path: &Path, prefix: &str, is_last: bool) -> io::Result<()> {
    let display = path.file_name()
        .unwrap_or_else(|| path.as_os_str())
        .to_string_lossy();
    
    println!("{}{}{}", prefix, 
        if is_last { "└── " } else { "├── " }, 
        display);

    if path.is_dir() {
        let entries = fs::read_dir(path)?
            .collect::<Result<Vec<_>, io::Error>>()?;
        
        for (i, entry) in entries.iter().enumerate() {
            let new_prefix = format!("{}{}",
                prefix,
                if is_last { "    " } else { "│   " }
            );
            display_tree(&entry.path(), &new_prefix, i == entries.len() - 1)?;
        }
    }
    Ok(())
}

/// Parses user input into a shell command
/// # Arguments
/// * `input` - The raw string input from the user
/// # Returns
/// * `Option<ShellCommand>` - None if input is invalid, Some(command) if valid
/// # Examples
/// ```
/// let cmd = parse_command("cd /home");
/// assert!(matches!(cmd, Some(ShellCommand::ChangeDir(_))));
/// ```
fn parse_command(input: &str) -> Option<ShellCommand> {
    let mut parts = input.split_whitespace(); // Split the input into parts
    match parts.next()? {
        "cd" => Some(ShellCommand::ChangeDir(parts.collect::<Vec<_>>().join(" "))),
        "exit" => Some(ShellCommand::Exit),
        "help" => Some(ShellCommand::Help),
        "tree" => Some(ShellCommand::TreeView(
            parts.next().unwrap_or(".").to_string()
        )),
        cmd => Some(ShellCommand::Execute(
            cmd.to_string(),
            parts.map(String::from).collect(),
        )),
    }
}

/// Executes a parsed shell command
/// # Arguments
/// * `cmd` - The ShellCommand to execute
/// # Effects
/// * May change current directory (cd)
/// * May spawn new processes (execute)
/// * May print to stdout (help, tree)
fn execute_shell_command(cmd: ShellCommand) {
    // using cmd to match the command
    match cmd {
        ShellCommand::Execute(program, args) => {
            match Command::new(&program).args(&args).spawn() {
                Ok(mut child) => { let _ = child.wait(); },
                Err(e) => eprintln!("Error: {}", e),
            }
        },
        ShellCommand::ChangeDir(dir) => {
            if let Err(e) = std::env::set_current_dir(&dir) {
                eprintln!("cd: {}", e);
            }
        },
        ShellCommand::TreeView(path) => {
            println!(".");
            if let Err(e) = display_tree(Path::new(&path), "", true) {
                eprintln!("Error displaying tree: {}", e);
            }
        },
        ShellCommand::Help => {
            println!("Available commands:");
            println!("  cd <dir>  - Change directory");
            println!("  help      - Show this help");
            println!("  exit      - Exit the shell");
            println!("  <command> - Execute command");
            println!("  tree [dir] - Display directory tree");
        },
        ShellCommand::Exit => {},
    }
}

/// Main shell loop that handles user interaction
/// # Effects
/// * Continuously reads from stdin
/// * Prints prompt to stdout
/// * Executes commands
/// * Maintains shell state until exit
pub fn run_shell() {
    let mut input = String::new();
    
    println!("Simple Command Shell (type 'help' for commands)");
    
    loop {
        print!("{} > ", std::env::current_dir().unwrap().display());
        io::stdout().flush().unwrap();
        input.clear();
        
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        match parse_command(input.trim()) {
            Some(ShellCommand::Exit) => break,
            Some(cmd) => execute_shell_command(cmd),
            None => if !input.trim().is_empty() {
                eprintln!("Invalid command");
            },
        }
    }
}

/// Unit tests for shell functionality
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_command() {
        if let Some(ShellCommand::Execute(cmd, args)) = parse_command("ls -la") {
            assert_eq!(cmd, "ls");
            assert_eq!(args, vec!["-la"]);
        } else {
            panic!("Failed to parse command");
        }
    }
}

/// Entry point for testing the shell directly
/// # Effects
/// * Starts the command shell
/// * Handles user input until exit
fn main() {
    println!("Starting command shell test...");
    run_shell();
}
