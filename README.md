# rs-cleaner

Small Rust CLI for finding leftover development directories in project folders.

It looks for projects with `package.json` or `Cargo.toml` and reports removable directories such as:

- `node_modules`
- `target`

## Usage

```bash
rs-cleaner [PATH] [OPTIONS]
```

Examples:

```bash
rs-cleaner
rs-cleaner ~/Code
rs-cleaner ~/Code --depth 3
rs-cleaner ~/Code --older-than 30
rs-cleaner ~/Code --preview
```

## Options

- `-d, --depth <LEVEL>` Maximum directory depth to search. Default: `2`
- `-o, --older-than <DAYS>` Only include projects older than the given number of days
- `-p, --preview` Preview mode
- `-y, --yes` Auto-accept prompts
- `-v, --verbose` Verbose output
