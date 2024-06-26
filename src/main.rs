use clap::{Arg, ArgMatches, Command};
use colored::*;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use dirs_next::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as processCommand, Stdio};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use tui::{backend::CrosstermBackend, Terminal};

const CONFIG_FILE_PATH: &str = "~/.config/bsh/commands.json";

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    categories: HashMap<String, HashMap<String, String>>,
}

#[derive(Default)]
struct AppState {
    categories: Vec<String>,
    commands: HashMap<String, Vec<(String, String)>>,
    selected_category: Option<usize>,
    selected_command: Option<usize>,
    selected_button: Option<usize>,
    mode: Mode,
    input_mode: InputMode,
    input: String,
}

#[derive(PartialEq)]
enum Mode {
    Category,
    Command,
    Buttons,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Category
    }
}

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Editing,
    Adding,
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::Normal
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        start_tui().unwrap();
        return;
    }

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
        .author("Matisse Callewaert")
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
                .about("Lists all categories and their commands")
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

fn start_tui() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::default();

    let pathbuf = check_for_config_file_or_create();
    let path = pathbuf.as_path();
    let data = fs::read_to_string(path).expect("Unable to read file");
    let mut config: Config = serde_json::from_str(&data).expect("Unable to parse JSON");

    app_state.categories = config.categories.keys().cloned().collect();
    for (category, commands) in &config.categories {
        let cmd_list: Vec<(String, String)> = commands
            .iter()
            .map(|(alias, cmd)| (alias.clone(), cmd.clone()))
            .collect();
        app_state.commands.insert(category.clone(), cmd_list);
    }

    let mut category_state = ListState::default();
    let mut command_state = ListState::default();
    category_state.select(Some(0));
    app_state.selected_category = Some(0);

    loop {
        terminal.draw(|f| {
            let size = f.size();

            // Split the layout into vertical chunks
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(6), // Fixed height for the logo chunk
                        Constraint::Min(0),    // Remaining space for categories and commands
                    ]
                    .as_ref(),
                )
                .split(size);

            // Split the logo and control instructions horizontally
            let logo_and_controls = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(50), // Space for logo
                        Constraint::Percentage(50), // Space for controls
                    ]
                    .as_ref(),
                )
                .split(vertical_chunks[0]);

            // Render the logo in the top left chunk
            let logo_paragraph = Paragraph::new(vec![
                Spans::from(Span::styled(
                    "██████╗░░██████╗██╗░░██╗",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "██╔══██╗██╔════╝██║░░██║",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "██████╦╝╚█████╗░███████║",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "██╔══██╗░╚═══██╗██╔══██║",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "██████╦╝██████╔╝██║░░██║",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "╚═════╝░╚═════╝░╚═╝░░╚═╝",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                )),
            ])
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(logo_paragraph, logo_and_controls[0]);

            // Render the controls in the top right chunk
            let controls_paragraph = Paragraph::new(vec![
                Spans::from(Span::styled(
                    "Controls:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "ESC - Quit",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "↓ - Down or add new when none below",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "d - Delete (Only on categories)",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Spans::from(Span::styled(
                    "Enter - Run command or activate buttons",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
            ])
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(controls_paragraph, logo_and_controls[1]);

            // Split the bottom chunk into horizontal chunks
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(20), // Space for categories
                        Constraint::Percentage(60), // Space for commands
                        Constraint::Percentage(10), // Space for update buttons
                        Constraint::Percentage(10), // Space for delete buttons
                    ]
                    .as_ref(),
                )
                .split(vertical_chunks[1]);

            // Render categories
            if app_state.categories.is_empty() {
                let no_categories_paragraph = Paragraph::new(Span::styled(
                    "No categories",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Yellow),
                        )
                        .title(Spans::from(Span::styled(
                            "Categories",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ))),
                );
                f.render_widget(no_categories_paragraph, horizontal_chunks[0]);
            } else {
                let category_list: Vec<ListItem> = app_state
                    .categories
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let style = if app_state.selected_category == Some(i) {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };
                        ListItem::new(Span::styled(c.to_string(), style))
                    })
                    .collect();

                let border_style_categories = if app_state.mode == Mode::Category {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                };

                let categories = List::new(category_list)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border_style_categories)
                            .title(Spans::from(Span::styled(
                                "Categories",
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            ))),
                    )
                    .highlight_style(
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Cyan),
                    )
                    .highlight_symbol("> ");
                f.render_stateful_widget(categories, horizontal_chunks[0], &mut category_state);
            }

            // Render commands for the selected category
            if let Some(selected_category) = app_state.selected_category {
                if selected_category < app_state.categories.len() {
                    if let Some(commands) = app_state
                        .commands
                        .get(&app_state.categories[selected_category])
                    {
                        let command_list: Vec<ListItem> = commands
                            .iter()
                            .enumerate()
                            .map(|(i, (alias, command))| {
                                let content = if app_state.selected_command == Some(i) {
                                    Spans::from(vec![
                                        Span::styled("> ", Style::default().fg(Color::Yellow)),
                                        Span::styled(
                                            alias.clone(),
                                            Style::default()
                                                .fg(Color::Green)
                                                .add_modifier(Modifier::BOLD),
                                        ),
                                        Span::styled(
                                            format!(": {}", command),
                                            Style::default().add_modifier(Modifier::BOLD),
                                        ),
                                    ])
                                } else {
                                    Spans::from(vec![
                                        Span::raw("  "),
                                        Span::styled(
                                            alias.clone(),
                                            Style::default().fg(Color::Green),
                                        ),
                                        Span::raw(format!(": {}", command)),
                                    ])
                                };
                                ListItem::new(content)
                            })
                            .collect();

                        let border_style_command = if app_state.mode == Mode::Command {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD)
                        };

                        let commands_list = List::new(command_list)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_style(border_style_command)
                                    .title(Spans::from(Span::styled(
                                        format!(
                                            "Commands in {}",
                                            &app_state.categories[selected_category]
                                        ),
                                        Style::default()
                                            .fg(Color::Red)
                                            .add_modifier(Modifier::BOLD),
                                    ))),
                            )
                            .highlight_style(Style::default());
                        f.render_stateful_widget(
                            commands_list,
                            horizontal_chunks[1],
                            &mut command_state,
                        );

                        // Define a fixed height for each button
                        let button_height = 1; // Adjust this value to match the height of the list item text

                        // Render update buttons for each command
                        let update_buttons: Vec<Paragraph> = commands
                            .iter()
                            .enumerate()
                            .map(|(i, _)| {
                                let style = if app_state.mode == Mode::Buttons
                                    && app_state.selected_button == Some(0)
                                    && app_state.selected_command == Some(i)
                                {
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default()
                                        .fg(Color::Blue)
                                        .add_modifier(Modifier::BOLD)
                                };
                                Paragraph::new(Span::styled("═══ U ═══", style))
                                    .block(Block::default().borders(Borders::NONE).style(style))
                            })
                            .collect();

                        // Render update buttons for each command
                        for (i, button) in update_buttons.into_iter().enumerate() {
                            let button_area = tui::layout::Rect {
                                x: horizontal_chunks[2].x,
                                y: horizontal_chunks[2].y
                                    + (i as u16 * button_height)
                                    + button_height,
                                width: horizontal_chunks[2].width,
                                height: button_height,
                            };
                            f.render_widget(button, button_area);
                        }

                        // Render delete buttons for each command
                        let delete_buttons: Vec<Paragraph> = commands
                            .iter()
                            .enumerate()
                            .map(|(i, _)| {
                                let style = if app_state.mode == Mode::Buttons
                                    && app_state.selected_button == Some(1)
                                    && app_state.selected_command == Some(i)
                                {
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                                };
                                Paragraph::new(Span::styled("═══ D ═══", style))
                                    .block(Block::default().borders(Borders::NONE).style(style))
                            })
                            .collect();

                        for (i, button) in delete_buttons.into_iter().enumerate() {
                            let button_area = tui::layout::Rect {
                                x: horizontal_chunks[3].x,
                                y: horizontal_chunks[3].y
                                    + (i as u16 * button_height)
                                    + button_height,
                                width: horizontal_chunks[3].width,
                                height: button_height,
                            };
                            f.render_widget(button, button_area);
                        }
                    }
                }
            }

            if app_state.input_mode == InputMode::Editing
                || app_state.input_mode == InputMode::Adding
            {
                let title = if app_state.input_mode == InputMode::Editing {
                    if app_state.mode == Mode::Category {
                        Spans::from(Span::styled(
                            "Update Category",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        if let Some(selected_command) = app_state.selected_command {
                            let category =
                                &app_state.categories[app_state.selected_category.unwrap()];
                            let alias =
                                &app_state.commands.get(category).unwrap()[selected_command].0;
                            Spans::from(Span::styled(
                                format!("Update alias {}", alias),
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            ))
                        } else {
                            Spans::from(Span::styled(
                                "Update Command",
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            ))
                        }
                    }
                } else {
                    if app_state.mode == Mode::Category {
                        Spans::from(Span::styled(
                            "Add Category",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        Spans::from(Span::styled(
                            "Add Command",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ))
                    }
                };

                let input_content =
                    if app_state.input.is_empty() && app_state.input_mode == InputMode::Adding {
                        if app_state.mode == Mode::Category {
                            Spans::from(vec![Span::styled(
                                "Category Name",
                                Style::default().fg(Color::DarkGray),
                            )])
                        } else {
                            Spans::from(vec![
                                Span::styled("alias ", Style::default().fg(Color::DarkGray)),
                                Span::styled("command", Style::default().fg(Color::DarkGray)),
                            ])
                        }
                    } else {
                        Spans::from(app_state.input.as_ref())
                    };

                let input_box = Paragraph::new(input_content).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                        .title(title),
                );

                let area = if app_state.mode == Mode::Category {
                    Rect::new(
                        horizontal_chunks[0].x,
                        horizontal_chunks[0].y,
                        horizontal_chunks[0].width,
                        horizontal_chunks[0].height,
                    )
                } else {
                    Rect::new(
                        horizontal_chunks[1].x,
                        horizontal_chunks[1].y,
                        horizontal_chunks[1].width,
                        horizontal_chunks[1].height,
                    )
                };

                f.render_widget(Clear, area);
                f.render_widget(input_box, area);
                f.set_cursor(area.x + app_state.input.len() as u16 + 1, area.y + 1);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match app_state.input_mode {
                InputMode::Normal => match app_state.mode {
                    Mode::Category => match key.code {
                        KeyCode::Esc => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            break;
                        }
                        KeyCode::Up => {
                            if let Some(selected) = category_state.selected() {
                                if selected > 0 {
                                    category_state.select(Some(selected - 1));
                                    app_state.selected_category = Some(selected - 1);
                                }
                            }
                        }
                        KeyCode::Down => {
                            if app_state.categories.is_empty() {
                                app_state.input_mode = InputMode::Adding;
                                app_state.input.clear();
                            } else if let Some(selected) = category_state.selected() {
                                if selected < app_state.categories.len() - 1 {
                                    category_state.select(Some(selected + 1));
                                    app_state.selected_category = Some(selected + 1);
                                } else {
                                    app_state.input_mode = InputMode::Adding;
                                    app_state.input.clear();
                                }
                            }
                        }
                        KeyCode::Enter | KeyCode::Right => {
                            if !app_state.categories.is_empty() {
                                app_state.mode = Mode::Command;
                                command_state.select(Some(0));
                                app_state.selected_command = Some(0);
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(selected) = category_state.selected() {
                                let category_to_delete = app_state.categories[selected].clone();

                                remove_category_from_config(&category_to_delete, &mut config, path);
                                app_state.categories.remove(selected);
                                app_state.commands.remove(&category_to_delete);

                                if selected >= app_state.categories.len() {
                                    let new_selection = app_state.categories.len().checked_sub(1);
                                    category_state.select(new_selection);
                                    app_state.selected_category = new_selection;
                                } else {
                                    category_state.select(Some(selected));
                                    app_state.selected_category = Some(selected);
                                }

                                update_config_file(&config, path);
                            }
                        }
                        _ => {}
                    },
                    Mode::Command => match key.code {
                        KeyCode::Esc => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            break;
                        }
                        KeyCode::Up => {
                            if let Some(selected) = command_state.selected() {
                                if selected > 0 {
                                    command_state.select(Some(selected - 1));
                                    app_state.selected_command = Some(selected - 1);
                                }
                            }
                        }
                        KeyCode::Down => {
                            if let Some(selected) = command_state.selected() {
                                if let Some(commands) = app_state.commands.get(
                                    &app_state.categories[app_state.selected_category.unwrap()],
                                ) {
                                    let commands_len = commands.len();
                                    if commands_len == 0 || selected >= commands_len - 1 {
                                        app_state.input_mode = InputMode::Adding;
                                        app_state.input.clear();
                                    } else {
                                        command_state.select(Some(selected + 1));
                                        app_state.selected_command = Some(selected + 1);
                                    }
                                }
                            }
                        }
                        KeyCode::Left => {
                            app_state.mode = Mode::Category;
                            app_state.selected_command = None;
                        }
                        KeyCode::Right => {
                            if let Some(selected_category) = app_state.selected_category {
                                if let Some(commands) = app_state
                                    .commands
                                    .get(&app_state.categories[selected_category])
                                {
                                    if !commands.is_empty() {
                                        app_state.mode = Mode::Buttons;
                                        app_state.selected_button = Some(0);
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;

                            if let Some(selected_category) = app_state.selected_category {
                                if let Some(commands) = app_state
                                    .commands
                                    .get(&app_state.categories[selected_category])
                                {
                                    if let Some(selected_command) = app_state.selected_command {
                                        run_command(
                                            &app_state.categories[selected_category],
                                            &commands[selected_command].0,
                                            &config,
                                        );
                                    }
                                }
                            }
                            break;
                        }
                        _ => {}
                    },
                    Mode::Buttons => match key.code {
                        KeyCode::Esc => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            break;
                        }
                        KeyCode::Left => {
                            if let Some(selected) = app_state.selected_button {
                                if selected > 0 {
                                    app_state.selected_button = Some(selected - 1);
                                } else {
                                    app_state.mode = Mode::Command;
                                }
                            }
                        }
                        KeyCode::Right => {
                            if let Some(selected) = app_state.selected_button {
                                if selected < 1 {
                                    app_state.selected_button = Some(selected + 1);
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(selected) = app_state.selected_button {
                                if selected == 0 {
                                    if let Some(selected_command) = app_state.selected_command {
                                        let category_index = app_state.selected_category.unwrap();
                                        let category = &app_state.categories[category_index];
                                        let command = &app_state.commands.get(category).unwrap()
                                            [selected_command]
                                            .1;

                                        app_state.input = command.clone();
                                        app_state.input_mode = InputMode::Editing;
                                    }
                                } else if selected == 1 {
                                    if let Some(selected_command) = app_state.selected_command {
                                        let category_index = app_state.selected_category.unwrap();
                                        let category = &app_state.categories[category_index];
                                        let alias = app_state.commands.get(category).unwrap()
                                            [selected_command]
                                            .0
                                            .clone();

                                        let commands =
                                            app_state.commands.get_mut(category).unwrap();
                                        commands.remove(selected_command);

                                        if let Some(category_commands) =
                                            config.categories.get_mut(category)
                                        {
                                            category_commands.remove(&alias);
                                        }

                                        update_config_file(&config, path);

                                        app_state.mode = Mode::Command;
                                        command_state.select(Some(0));
                                        app_state.selected_command = Some(0);
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                },
                InputMode::Editing => match key.code {
                    KeyCode::Enter => {
                        if let Some(selected_command) = app_state.selected_command {
                            let category =
                                &app_state.categories[app_state.selected_category.unwrap()];
                            let alias =
                                &app_state.commands.get(category).unwrap()[selected_command].0;
                            update_command(category, &app_state.input, alias, &mut config, path);
                            app_state.commands.get_mut(category).unwrap()[selected_command].1 =
                                app_state.input.clone();
                            update_config_file(&config, path);
                            app_state.input_mode = InputMode::Normal;
                            app_state.input.clear();
                        }
                    }
                    KeyCode::Esc => {
                        app_state.input_mode = InputMode::Normal;
                        app_state.input.clear();
                    }
                    KeyCode::Char(c) => {
                        app_state.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app_state.input.pop();
                    }
                    _ => {}
                },
                InputMode::Adding => match key.code {
                    KeyCode::Enter => {
                        if app_state.mode == Mode::Category && !app_state.input.is_empty() {
                            let category = app_state.input.clone();
                            if !app_state.categories.contains(&category) {
                                add_category_to_config(&category, &mut config, path);
                                app_state.categories.push(category.clone());
                                app_state.commands.insert(category, Vec::new());
                                update_config_file(&config, path);
                                app_state.input_mode = InputMode::Normal;
                                app_state.input.clear();
                            }
                        } else if app_state.mode == Mode::Command && !app_state.input.is_empty() {
                            let parts: Vec<&str> = app_state.input.splitn(2, ' ').collect();
                            if parts.len() == 2 {
                                let alias = parts[0].to_string();
                                let command = parts[1].to_string();

                                let category_index = app_state.selected_category.unwrap();
                                let category = &app_state.categories[category_index];

                                let alias_exists = config
                                    .categories
                                    .get(category)
                                    .map_or(false, |cmds| cmds.contains_key(&alias));
                                let category_exists = config.categories.contains_key(category);

                                if !alias_exists {
                                    if let Some(commands) = config.categories.get_mut(category) {
                                        commands.insert(alias.clone(), command.clone());
                                    } else {
                                        let mut new_commands = HashMap::new();
                                        if !category_exists {
                                            new_commands.insert(alias.clone(), command.clone());
                                            config
                                                .categories
                                                .insert(category.clone(), new_commands);
                                        }
                                    }

                                    app_state
                                        .commands
                                        .entry(category.clone())
                                        .or_insert_with(Vec::new)
                                        .push((alias.clone(), command.clone()));

                                    update_config_file(&config, path);
                                    app_state.mode = Mode::Command;
                                    app_state.input_mode = InputMode::Normal;
                                    app_state.input.clear();
                                }
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::Up => {
                        app_state.input_mode = InputMode::Normal;
                        app_state.input.clear();
                    }
                    KeyCode::Char(c) => {
                        app_state.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app_state.input.pop();
                    }
                    _ => {}
                },
            }
        }
    }

    Ok(())
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

    if command_to_run.trim().is_empty() {
        eprintln!("Command '{}' is empty", command_to_run);
        return;
    }

    let mut final_command = command_to_run.clone();
    while let Some(start) = final_command.find("<[") {
        if let Some(end) = final_command[start..].find("]>") {
            let placeholder = &final_command[start + 2..start + end];
            print!("Please enter a value for {}: ", placeholder);
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");
            let input = input.trim();
            final_command = final_command.replacen(&format!("<[{}]>", placeholder), input, 1);
        } else {
            eprintln!(
                "Mismatched placeholder brackets in command: {}",
                final_command
            );
            return;
        }
    }

    let output = processCommand::new("sh")
        .arg("-c")
        .arg(final_command)
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
