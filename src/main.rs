use clap::{Arg, ArgMatches, Command};
use std::fs;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::process::{Command as processCommand, Stdio};

const CONFIG_FILE_PATH: &str = "commands.json";

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    categories: HashMap<String, HashMap<String, String>>,
}

fn main() {
    check_for_config_file_or_create();
    let data = fs::read_to_string(CONFIG_FILE_PATH).expect("Unable to read file");
    let mut config: Config = serde_json::from_str(&data).expect("Unable to parse JSON");

    // Manually parse the raw arguments
    let args: Vec<String> = std::env::args().collect();
    let mut clap_args = args.clone();

    // Check if the first argument is not a known subcommand and not a flag
    if args.len() > 1 && !["run", "r", "add", "a", "delete", "d", "help", "-V", "--version", "-h", "--help", "list", "l", "update", "u"].contains(&args[1].as_str()) {
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

    match matches.subcommand() {
        Some(("add", sub_m)) => {
            let category = sub_m.get_one::<String>("CATEGORY").unwrap();
            let alias = sub_m.get_one::<String>("ALIAS");
            let command = sub_m.get_one::<String>("COMMAND");

            match (alias, command) {
                (Some(alias), Some(command)) => {
                    add_command(category, command, alias, &mut config);
                },
                (None, None) => {
                    add_category_to_config(category, &mut config);
                },
                _ => {
                    eprintln!("Error: When specifying an alias, a command must also be provided, and vice versa.");
                }
            }
        },
        Some(("run", sub_m)) => {
            let category = sub_m.get_one::<String>("CATEGORY").unwrap();
            let alias = sub_m.get_one::<String>("ALIAS").unwrap();
            run_command(category, alias, &config);
        },
        Some(("delete", sub_m)) => {
            let category = sub_m.get_one::<String>("CATEGORY").unwrap();
            let alias = sub_m.get_one::<String>("ALIAS");

            match alias {
                Some(alias) => {
                    remove_command_from_config(category, alias, &mut config);
                },
                None => {
                    remove_category_from_config(category, &mut config);
                }
            }
        },
        Some(("update", sub_m)) => {
            let category = sub_m.get_one::<String>("CATEGORY").unwrap();
            let alias = sub_m.get_one::<String>("ALIAS").unwrap();
            let command = sub_m.get_one::<String>("COMMAND").unwrap();

            update_command(category, command, alias, &mut config);
        },
        Some(("list", sub_m)) => {
            handle_list_command(sub_m, &config);
        },
        _ => {},
    }
}

fn handle_list_command(matches: &ArgMatches, config: &Config) {
    if let Some(category) = matches.get_one::<String>("category") {
        list_all_commands_with_aliases_in_category(category, config);
    } else {
        list_all_commands_with_aliases(config);
    }
}


fn add_command(category: &str, command: &str, alias: &str, config: &mut Config) {
    check_for_config_file_or_create();

    if !check_if_category_exists(category, config) {
        println!("Adding Category '{}', because it does not exist", category);
        add_category_to_config(category, config);
    } if check_if_command_exists(category, alias, config) {
        println!("Command '{}' already exists in category '{}', if you want to update the command, add -u", command, category);
    } else {
        println!("Adding command '{}' to category '{}'", command, category);
        add_command_to_config(category, command, alias, config);
    }
}

fn update_command(category: &str, command: &str, alias: &str, config: &mut Config) {
    check_for_config_file_or_create();

    if !check_if_category_exists(category, config) {
        println!("Adding Category '{}', because it does not exist", category);
        add_category_to_config(category, config);
    } if check_if_command_exists(category, alias, config) {
        update_command_in_config(category, command, alias, config);
    } else {
        println!("Adding command '{}' to category '{}'", command, category);
        add_command_to_config(category, command, alias, config);
    }
}

fn run_command(category: &str, alias: &str, config: &Config) {
    check_for_config_file_or_create();

    if !check_if_category_exists(category, config) {
        println!("Category '{}' does not exist", category);
    } else if !check_if_command_exists(category, alias, config) {
        println!("Command '{}' does not exist in category '{}'", alias, category);
    } else {
        run_command_from_config(category, alias, config);
    }
}

fn check_for_config_file_or_create() {
    if !config_file_exists(&CONFIG_FILE_PATH) {
        create_config_file(&CONFIG_FILE_PATH);
    }
}

fn config_file_exists(file_path: &str) -> bool {
    fs::metadata(file_path).is_ok()
}

fn create_config_file(file_path: &str) {
    fs::write(file_path, "{\"categories\":{}}").expect("Failed to create config file");
}

fn check_if_category_exists(category: &str, config: &Config) -> bool {
    config.categories.contains_key(category)
}

fn check_if_command_exists(category: &str, alias: &str, config: &Config) -> bool {
    config.categories.get(category).unwrap().contains_key(alias)
}

fn add_command_to_config(category: &str, command: &str, alias: &str, config: &mut Config) {
    if !config.categories.contains_key(category) {
        config.categories.insert(category.to_string(), HashMap::new());
    }
    config.categories.get_mut(category).unwrap().insert(alias.to_string(), command.to_string());
    update_config_file(config);
}

fn update_command_in_config(category: &str, command: &str, alias: &str, config: &mut Config) {
    config.categories.get_mut(category).unwrap().insert(alias.to_string(), command.to_string());
    update_config_file(config);
}

fn run_command_from_config(category: &str, alias: &str, config: &Config) {
    let command_to_run = config.categories.get(category).unwrap().get(alias).unwrap();

    let parts: Vec<&str> = command_to_run.split_whitespace().collect();
    let (command, args) = parts.split_first().expect("No command provided");

    let child = processCommand::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    match child {
        Ok(mut child) => {
            let result = child.wait().expect("Failed to wait on child");
            if !result.success() {
                eprintln!("Command failed with status: {}", result);
            }
        },
        Err(e) => eprintln!("Failed to start command: {}", e),
    }
}

fn remove_command_from_config(category: &str, alias: &str, config: &mut Config) {
    if let Some(commands) = config.categories.get_mut(category) {
        if commands.remove(alias).is_some() {
            update_config_file(config);
        }
    }
}


fn add_category_to_config(category: &str, config: &mut Config) {
    if !config.categories.contains_key(category) {
        config.categories.insert(category.to_string(), HashMap::new());
        update_config_file(config);
    }
}


fn remove_category_from_config(category: &str, config: &mut Config) {
    if config.categories.remove(category).is_some() {
        update_config_file(config);
    }
}

fn list_all_commands_with_aliases(config: &Config) {
    for (category, commands) in config.categories.iter() {
        println!("Commands in category '{}':", category);
        for (command, alias) in commands.iter() {
            println!("\t - {}: {} ({})", category, command, alias);
        }
    }
}

fn list_all_commands_with_aliases_in_category(category: &str, config: &Config) {
    if let Some(commands) = config.categories.get(category) {
        println!("Commands in category '{}':", category);
        for (command, alias) in commands.iter() {
            println!("\t - {}: {} ({})", category, command, alias);
        }
    } else {
        println!("Category '{}' does not exist", category);
    }
}

fn update_config_file(config: &Config) {
    let new_config_json = serde_json::to_string(config).expect("Failed to serialize config");
    fs::write(CONFIG_FILE_PATH, new_config_json).expect("Failed to write to config file");
}

