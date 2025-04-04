use std::io::{self, Write};
use std::process::Command;
use std::path::Path;
use std::fs;

#[derive(Debug)]
enum ShellCommand {
    Execute(String, Vec<String>),
    ChangeDir(String),
    Exit,
    Help,
    TreeView(String),
}

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

fn parse_command(input: &str) -> Option<ShellCommand> {
    let mut parts = input.split_whitespace();
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

fn execute_shell_command(cmd: ShellCommand) {
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

fn main() {
    println!("Starting command shell test...");
    run_shell();
}
