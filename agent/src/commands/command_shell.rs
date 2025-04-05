use std::io::{self, Write}; 
use std::process::Command; 
use std::path::Path;
use std::fs;
use std::time::SystemTime;

#[derive(Debug)]
enum OpCode {
    Exec = 0x01,
    Cd = 0x02,
    Exit = 0x03,
    Help = 0x04,
    Tree = 0x05,
}

struct CommandLogger {
    entries: Vec<(SystemTime, String)>,
}

impl CommandLogger {
    fn new() -> Self {
        CommandLogger {
            entries: Vec::new(),
        }
    }

    fn log(&mut self, command: &str) {
        self.entries.push((SystemTime::now(), command.to_string()));
        println!("[DEBUG] Logged command: {}", command);
    }
}

#[derive(Debug)]
enum ShellCommand {
    Execute(String, Vec<String>),
    ChangeDir(String),
    Exit,
    Help,
    TreeView(String),
}

impl ShellCommand {
    fn from_opcode(code: u8, args: &[u8]) -> Option<Self> {
        match code {
            0x01 => Some(ShellCommand::Execute(
                String::from_utf8_lossy(&args[0..args.iter().position(|&x| x == 0).unwrap_or(args.len())]).to_string(),
                Vec::new()
            )),
            0x02 => Some(ShellCommand::ChangeDir(
                String::from_utf8_lossy(&args[0..args.iter().position(|&x| x == 0).unwrap_or(args.len())]).to_string()
            )),
            0x03 => Some(ShellCommand::Exit),
            0x04 => Some(ShellCommand::Help),
            0x05 => Some(ShellCommand::TreeView(".".to_string())),
            _ => None,
        }
    }
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
/// * `logger` - The CommandLogger for logging commands
/// # Effects
/// * May change current directory (cd)
/// * May spawn new processes (execute)
/// * May print to stdout (help, tree)
fn execute_shell_command(cmd: ShellCommand, logger: &mut CommandLogger) {
    logger.log(&format!("{:?}", cmd));
    
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
    let mut logger = CommandLogger::new();
    
    println!("Enhanced Command Shell (type 'help' for commands)");
    
    loop {
        print!("{} > ", std::env::current_dir().unwrap().display());
        io::stdout().flush().unwrap();
        input.clear();
        
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        // Try parsing as binary first
        if let Some(bytes) = input.trim().as_bytes().first() {
            if let Some(cmd) = ShellCommand::from_opcode(*bytes, &input.as_bytes()[1..]) {
                match cmd {
                    ShellCommand::Exit => break,
                    cmd => execute_shell_command(cmd, &mut logger),
                }
                continue;
            }
        }

        // Fall back to text parsing
        match parse_command(input.trim()) {
            Some(ShellCommand::Exit) => break,
            Some(cmd) => execute_shell_command(cmd, &mut logger),
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
