use clap::{Arg, ArgMatches, Command};
use colored::*;
use dirs_next::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as processCommand, Stdio};

const CONFIG_FILE_PATH: &str = "~/.config/bsh/commands.json";

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    categories: HashMap<String, HashMap<String, String>>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut clap_args = args.clone();

    // Check if the first argument is not a known subcommand and not a flag
    if args.len() > 1
        && ![
            "run",
            "r",
            "add",
            "a",
            "delete",
            "d",
            "help",
            "-V",
            "--version",
            "-h",
            "--help",
            "list",
            "l",
            "update",
            "u",
        ]
        .contains(&args[1].as_str())
    {
        // Prepend the 'run' command if it appears to be missing
        clap_args.insert(1, "run".to_string());
    }

    let matches = Command::new("bsh")
        .version("0.1.0")
        .author("Matisse Callewaert and Niels Savvides")
        .about("Organizes and provides quick access to frequently used shell commands")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("Adds a new command to a category or creates a new category if no command is given")
                .alias("a")
                .arg(Arg::new("CATEGORY")
                    .help("The category to add or to add the command to")
                    .required(true))
                .arg(Arg::new("ALIAS")
                    .help("The alias of the command to add")
                    .required(false))
                .arg(Arg::new("COMMAND")
                    .help("The command to add")
                    .required(false))
        )
        .subcommand(
            Command::new("run")
                .about("Runs a command from a specified category")
                .alias("r")
                .arg(Arg::new("CATEGORY")
                    .help("The category to run the command from")
                    .required(true))
                .arg(Arg::new("ALIAS")
                    .help("The alias of the command to run")
                    .required(true))
        )
        .subcommand(
            Command::new("delete")
                .about("Removes a command of a category or removes the category if no command is given")
                .alias("d")
                .arg(Arg::new("CATEGORY")
                    .help("The category to remove or remove the command from")
                    .required(true))
                .arg(Arg::new("ALIAS")
                    .help("The alias of the command to remove")
                    .required(false))
        )
        .subcommand(
            Command::new("update")
                .about("Updates a command of a category, if the category or command does not exist, it will be created")
                .alias("u")
                .arg(Arg::new("CATEGORY")
                    .help("The category to update the command from")
                    .required(true))
                .arg(Arg::new("ALIAS")
                    .help("The alias of the command to update")
                    .required(true))
                .arg(Arg::new("COMMAND")
                    .help("The command to add")
                    .required(true))
        )
        .subcommand(
            Command::new("list")
                .about("Lists all categories and there commands")
                .alias("l")
                .arg(Arg::new("category")
                    .help("Specify the category to list commands from")
                    .required(false))
        )
        .get_matches_from(clap_args);

    let pathbuf = check_for_config_file_or_create();
    let path = pathbuf.as_path();

    let data = fs::read_to_string(path).expect("Unable to read file");
    let mut config: Config = serde_json::from_str(&data).expect("Unable to parse JSON");

    match matches.subcommand() {
        Some(("add", sub_m)) => {
            let category = sub_m.get_one::<String>("CATEGORY").unwrap();
            let alias = sub_m.get_one::<String>("ALIAS");
            let command = sub_m.get_one::<String>("COMMAND");

            match (alias, command) {
                (Some(alias), Some(command)) => {
                    add_command(category, command, alias, &mut config, &path);
                }
                (None, None) => {
                    add_category_to_config(category, &mut config, &path);
                }
                _ => {
                    eprintln!("Error: When specifying an alias, a command must also be provided, and vice versa.");
                }
            }
        }
        Some(("run", sub_m)) => {
            let category = sub_m.get_one::<String>("CATEGORY").unwrap();
            let alias = sub_m.get_one::<String>("ALIAS").unwrap();
            run_command(category, alias, &config);
        }
        Some(("delete", sub_m)) => {
            let category = sub_m.get_one::<String>("CATEGORY").unwrap();
            let alias = sub_m.get_one::<String>("ALIAS");

            match alias {
                Some(alias) => {
                    remove_command_from_config(category, alias, &mut config, &path);
                }
                None => {
                    remove_category_from_config(category, &mut config, &path);
                }
            }
        }
        Some(("update", sub_m)) => {
            let category = sub_m.get_one::<String>("CATEGORY").unwrap();
            let alias = sub_m.get_one::<String>("ALIAS").unwrap();
            let command = sub_m.get_one::<String>("COMMAND").unwrap();

            update_command(category, command, alias, &mut config, &path);
        }
        Some(("list", sub_m)) => {
            handle_list_command(sub_m, &config);
        }
        _ => {}
    }
}

fn handle_list_command(matches: &ArgMatches, config: &Config) {
    if let Some(category) = matches.get_one::<String>("category") {
        list_all_commands_with_aliases_in_category(category, config);
    } else {
        list_all_commands_with_aliases(config);
    }
}

fn add_command(category: &str, command: &str, alias: &str, config: &mut Config, path: &Path) {
    if !check_if_category_exists(category, config) {
        println!("Adding Category '{}', because it does not exist", category);
        add_category_to_config(category, config, path);
    }
    if check_if_command_exists(category, alias, config) {
        println!("Command '{}' already exists in category '{}', if you want to update the command, use update", command, category);
    } else {
        println!("Adding command '{}' to category '{}'", command, category);
        add_command_to_config(category, command, alias, config, path);
    }
}

fn update_command(category: &str, command: &str, alias: &str, config: &mut Config, path: &Path) {
    if !check_if_category_exists(category, config) {
        println!("Adding Category '{}', because it does not exist", category);
        add_category_to_config(category, config, path);
    }
    if check_if_command_exists(category, alias, config) {
        update_command_in_config(category, command, alias, config, path);
    } else {
        println!("Adding command '{}' to category '{}'", command, category);
        add_command_to_config(category, command, alias, config, path);
    }
}

fn run_command(category: &str, alias: &str, config: &Config) {
    if !check_if_category_exists(category, config) {
        println!("Category '{}' does not exist", category);
    } else if !check_if_command_exists(category, alias, config) {
        println!(
            "Command '{}' does not exist in category '{}'",
            alias, category
        );
    } else {
        run_command_from_config(category, alias, config);
    }
}

fn check_for_config_file_or_create() -> PathBuf {
    let expanded_path = expand_home_dir(CONFIG_FILE_PATH).expect("Failed to expand home directory");

    if !config_file_exists(&expanded_path) {
        create_config_file(&expanded_path);
    }

    expanded_path
}

fn config_file_exists(path: &Path) -> bool {
    path.exists()
}

fn expand_home_dir(path: &str) -> Option<PathBuf> {
    if path.starts_with('~') {
        let home = home_dir()?;
        let remaining_path = path.strip_prefix("~").unwrap_or(path);
        let trimmed_path = remaining_path.trim_start_matches('/');
        Some(home.join(trimmed_path))
    } else {
        Some(PathBuf::from(path))
    }
}

fn create_config_file(file_path: &Path) {
    if let Some(parent) = file_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            panic!("Failed to create configuration directory: {}", e);
        }
    }

    fs::write(file_path, "{\"categories\":{}}").expect("Failed to create config file");
}

fn check_if_category_exists(category: &str, config: &Config) -> bool {
    config.categories.contains_key(category)
}

fn check_if_command_exists(category: &str, alias: &str, config: &Config) -> bool {
    config.categories.get(category).unwrap().contains_key(alias)
}

fn add_command_to_config(
    category: &str,
    command: &str,
    alias: &str,
    config: &mut Config,
    path: &Path,
) {
    if !config.categories.contains_key(category) {
        config
            .categories
            .insert(category.to_string(), HashMap::new());
    }
    config
        .categories
        .get_mut(category)
        .unwrap()
        .insert(alias.to_string(), command.to_string());
    update_config_file(config, path);
}

fn update_command_in_config(
    category: &str,
    command: &str,
    alias: &str,
    config: &mut Config,
    path: &Path,
) {
    config
        .categories
        .get_mut(category)
        .unwrap()
        .insert(alias.to_string(), command.to_string());
    update_config_file(config, path);
}

fn run_command_from_config(category: &str, alias: &str, config: &Config) {
    // Retrieve the command using safe navigation
    let command_to_run = match config.categories.get(category).and_then(|c| c.get(alias)) {
        Some(cmd) => cmd,
        None => {
            eprintln!(
                "Command for category '{}' and alias '{}' not found.",
                category, alias
            );
            return;
        }
    };

    // Ensure the command string is not empty
    if command_to_run.trim().is_empty() {
        eprintln!("Command '{}' is empty", command_to_run);
        return;
    }

    // Execute the command using a shell
    let output = processCommand::new("sh")
        .arg("-c")
        .arg(command_to_run)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output();

    if let Err(e) = output {
        eprintln!("Failed to execute command: {}", e);
    }
}

fn remove_command_from_config(category: &str, alias: &str, config: &mut Config, path: &Path) {
    if let Some(commands) = config.categories.get_mut(category) {
        if commands.remove(alias).is_some() {
            update_config_file(config, path);
        }
    }
}

fn add_category_to_config(category: &str, config: &mut Config, path: &Path) {
    if !config.categories.contains_key(category) {
        config
            .categories
            .insert(category.to_string(), HashMap::new());
        update_config_file(config, path);
    }
}

fn remove_category_from_config(category: &str, config: &mut Config, path: &Path) {
    if config.categories.remove(category).is_some() {
        update_config_file(config, path);
    }
}

fn list_all_commands_with_aliases(config: &Config) {
    if config.categories.is_empty() {
        println!("{}", "No categories available.".yellow().bold());
    } else {
        for (category, commands) in config.categories.iter() {
            println!(
                "{}{}{}",
                "Commands in category ".blue().bold(),
                "➜  ".yellow().bold(),
                category.red().bold()
            );
            if commands.is_empty() {
                println!("\t{}", "No commands available.".yellow());
            } else {
                for (alias, command) in commands.iter() {
                    println!(
                        "\t {} {}  {}",
                        alias.green().bold(),
                        "➜".yellow().bold(),
                        command
                    );
                }
            }
        }
    }
}

fn list_all_commands_with_aliases_in_category(category: &str, config: &Config) {
    if let Some(commands) = config.categories.get(category) {
        println!(
            "{}{}{}",
            "Commands in category ".blue().bold(),
            "➜  ".yellow().bold(),
            category.red().bold()
        );
        if commands.is_empty() {
            println!("\t{}", "No commands available in this category.".yellow());
        } else {
            for (alias, command) in commands.iter() {
                println!(
                    "\t {} {}  {}",
                    alias.green().bold(),
                    "➜".yellow().bold(),
                    command
                );
            }
        }
    } else {
        println!(
            "{} '{}' does not exist",
            "Category".blue().bold(),
            category.red().bold()
        );
    }
}

fn update_config_file(config: &Config, path: &Path) {
    let new_config_json = serde_json::to_string(config).expect("Failed to serialize config");
    fs::write(path, new_config_json).expect("Failed to write to config file");
}
