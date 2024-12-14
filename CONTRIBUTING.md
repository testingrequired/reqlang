# Contributing

Thank you for your interest in contributing to this project! We welcome contributions of all kinds, from bug reports and feature requests to code contributions and documentation improvements. This document outlines how to get started.

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
- [VS Code](https://code.visualstudio.com/)

#### Tests

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
