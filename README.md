# f — File Manager (GUI)

`f` is a lightweight dual-pane file manager GUI built with Rust and `iced`, inspired by Total Commander.

## Features ✨

- Dual-pane directory browsing (left / right)
- File filtering/search (input contains match)
- Basic file operations: copy, delete, refresh
- Planned: rename, move, preview, keyboard shortcuts

## Installation

For development and testing:

```bash
cargo run --release
```

Binary releases and `cargo install` may be provided in the future.

## Example

Run the program to open a window with two panels representing directories. Click items to select them and use the buttons below to copy, delete or refresh.

## For Contributors

Contributions welcome — open issues or PRs and use `feature/*` branches. Please follow `rustfmt` and `clippy` conventions.

## License

This project is licensed under GPLv3 (see `LICENSE`).

---

Further documentation and roadmap will be published on crates.io and docs.rs.