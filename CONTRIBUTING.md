# Contributing

Thank you for your interest in contributing to this project! We welcome contributions of all kinds, from bug reports and feature requests to code contributions and documentation improvements. This document outlines how to get started.

## Navigating the Project

- [**`cli`**](./cli/): A CLI for interacting with request files from the terminal.
- [**`examples`**](./examples/): Valid and invalid request files. Designed to illustrate the format's features. Also used when running the [integration tests](./integration-tests/).
- [**`integration-tests`**](./integration-tests/): Test suite that parses the [example](./examples/) request files.
- [**`reqlang`**](./reqlang/): Rust library for handling request files (parsing, templating, executing, diagnostics, etc...)
- [**`reqlang-client`**](./reqlang-client/): A simple desktop GUI applicationfor testing and demonstration purposes.
- [**`reqlang-lsp`**](./reqlang-lsp/): The reqlang language server. Used by the VS Code [extension](./vsc/).
- [**`vsc`**](./vsc/): The VS Code extension for working with request files. It also acts as a client for executing requests.

## Ways to Contribute

### Reporting Bugs

If you encounter a bug, please open a new issue on GitHub. Be sure to include:

- A clear and concise description of the bug.
- Steps to reproduce the bug.
- The expected behavior.
- The actual behavior.
- Your operating system and version.
- Any relevant screenshots or logs.

```markdown
## Summary

The "Run" button in request files is unresponsive

## Steps to Reproduce

1. Install reqlang VS Code extension
2. Open a request file in VS Code
3. Click on the "Run" button

## Expected Behavior

The "Run" button should execute the request and display the results.

## Actual Behavior

Nothing happens when the "Run" button is clicked. No error messages or visual feedback is given.

## Reqlang Version

0.1.0

## Operating System and Version

macOS 12.4

## Other Version

VS Code: 1.62.3
```

### Contributing Code

If you'd like to contribute code, please follow these steps:

1. **Fork** the repository.
2. **Create a new branch** for your changes.
3. **Make your changes** and commit them with clear and concise commit messages.
4. **Push** your branch to your forked repository.
5. **Open a pull request** on GitHub. Be sure to include a clear and concise description of your changes.

All contributions are appreciated but not all will be accepted. We will review your contributions carefully and make sure they align with the project's goals.

#### Preqrequisites

- [NodeJS](https://nodejs.org/en/download/package-manager)/[nvm](https://github.com/nvm-sh/nvm)
- [Rust](https://rustup.rs/)
- [Just](https://just.systems/)
- [watch-exec](https://github.com/watchexec/watchexec)
- [VS Code](https://code.visualstudio.com/)

#### Build

Builds everything.

```shell
just build
```

##### Output Directories

- `target` The compiled binaries for the project
- `types/dist` Generated typescript types from the [types](./types/) crate
- `vsc/out` The packaged VS Code extension

#### Verify

Builds the project, runs all the tests, and checks for linting issues.

```shell
just verify
```

#### Install

Builds the project, installs the binaries and the VS Code extension.

```shell
just install
```

### Improving Documentation

If you find any errors or omissions in the documentation, please open a new issue or submit a pull request with the necessary corrections.
