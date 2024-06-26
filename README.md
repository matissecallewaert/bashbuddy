# BashBuddy

BashBuddy is a specialized CLI tool designed to organize your shell commands into easily manageable projects and aliases. It enhances productivity by streamlining your workflow through quick access to frequently used commands, organized into well-defined categories.

![BashBuddy](bashbuddy.png)

## Installation

### Development
First, clone the repository:

```bash
git clone git@github.com:matissecallewaert/bashbuddy.git
```

Then, navigate to the project directory and run the following command to install the package:

```bash
cargo build
```

Finally, you can test the installation by running the following command:

```bash
cargo run -- help
```

## Usage

### Using the TUI (Terminal User Interface)

To launch the TUI, run the following command:

```bash
bsh
```

![bsh tui](tui.png)

### Using the CLI (Command Line Interface)

Execute commands using:

```bash
bsh [CATEGORY] [COMMAND]
``` 

#### Available Commands

- **add or a**: Adds a new command to a category or creates a new category if no command is given.
- - **update or u**: Updates or adds a command to a category.
- **run or r**: Executes a command from a specified category.
- **delete or d**: Removes a command from a category or deletes the category entirely if no command is specified.

#### Global Options

- `-h, --help`: Displays help information.
- `-V, --version`: Displays the version information.

#### Detailed Command Usage

##### Adding Commands or Categories

```bash
bsh add <CATEGORY> [ALIAS] [COMMAND]
```

- `<CATEGORY>`: Specifies the category to which the command will be added, or a new category to be created.
- `[ALIAS]`: Optional. Specifies an alias for the command.
- `[COMMAND]`: Optional. Specifies the command to be added. If only the category is provided, a new category will be created without adding any commands.

**Example**:
```bash
bsh a utilities ping 'ping example.com'
```
This adds a new command 'ping example.com' with alias 'ping' to the 'utilities' category.

##### Running Commands

```bash
bsh r <CATEGORY> <ALIAS>
```

- `<CATEGORY>`: The category from which the command will be run.
- `<ALIAS>`: The alias of the command to be executed.

**Example**:
```bash
bsh run utilities ping
```
You can also run it like this:
```bash
bsh utilities ping
```
This executes the 'ping' command in the 'utilities' category.

##### Deleting Commands or Categories

```bash
bsh delete <CATEGORY> [ALIAS]
```

- `<CATEGORY>`: Specifies the category from which the command will be removed, or the category to be deleted if no alias is provided.
- `[ALIAS]`: Optional. Specifies the alias of the command to remove. If no alias is provided, the entire category will be deleted.

**Example**:
```bash
bsh delete utilities ping
```
This removes the 'ping' command from the 'utilities' category.

---

### Additional Information

- For a complete list of commands and their options, you can always run `bsh help` or `bsh help [COMMAND]` for details about a specific command.
- You can use placeholders in you commands which will be prompted when you want to run the command. You can add such a command as follows or use the TUI:
    ```bsh
    bsh a utilities rustiflow "sudo RUST_LOG=info rustiflow realtime wlo1 <[flow]> <[seconds]> print"
    ```


## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
