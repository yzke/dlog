# README.md

### `dlog`

A command-line tool for developers to easily log their work progress.

`dlog` is a simple yet powerful CLI utility written in Rust, designed for developers to keep a clean, searchable log of their daily tasks and progress directly from the command line.

#### Features

- **Fast Logging:** Record a quick log with a single command.
- **Interactive Mode:** Enter a multi-line interactive session for detailed logging.
- **Project-Aware:** Logs are associated with your current working directory, making it easy to retrieve relevant entries.
- **Search & Filter:** Retrieve recent logs and filter them by directory, tags, or number of entries.

#### Installation

Since `dlog` is a Rust project, you can install it using `cargo`:

1. Clone the repository to your local machine:
   `git clone [你的仓库链接]`
2. Navigate to the project directory:
   `cd dlog`
3. Build and install the binary:
   `cargo install --path .`

The `dlog` executable will be placed in your Cargo bin directory (`~/.cargo/bin/`), which should be in your system's PATH.

#### Usage

##### 1. Initialize the Database

First, you need to create the database file that will store all your logs.
`dlog init`

The database file will be created at `~/.config/dlog/dlog.db`.

##### 2. Log an Entry

You can log an entry in two ways:

- **Quick log:** For a short, one-line message.
  `dlog log -m "Implemented the user authentication feature."`

- **Interactive log:** For multi-line, detailed logs.
  `dlog log`
  _You will be prompted to enter your log message. Press `Ctrl + D` to finish and save._

##### 3. View Your Logs

Retrieve your logs from the command line.

- **View the latest log:**
  `dlog get`

- **View the latest 5 logs:**
  `dlog get -n 5`

- **View all logs from the current directory and its subdirectories:**
  `dlog get -r`

- **View logs with tags:**
  `dlog get -t`