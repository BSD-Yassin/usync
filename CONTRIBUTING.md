# Contributing to usync

Thank you for your interest in contributing to usync! This document provides guidelines and information for contributors.

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on constructive feedback

## How to Contribute

### Reporting Bugs

1. Check if the issue already exists
2. Create a new issue with:
   - Clear description of the problem
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, Rust version, etc.)

### Suggesting Features

1. Open an issue to discuss the feature
2. Explain the use case and benefits
3. Wait for feedback before implementing

### Submitting Changes

**Important**: Direct pushes to `main` are **not allowed**, even for maintainers. All changes must go through Pull Requests from forks.

1. **Fork the repository** (required for all contributors, including maintainers)
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/usync.git`
3. Add upstream: `git remote add upstream https://github.com/BSD-Yassin/usync.git`
4. Create a feature branch: `git checkout -b feature/amazing-feature`
5. Make your changes
6. Add tests for new functionality
7. Ensure all tests pass: `cargo test`
8. Commit your changes: `git commit -m 'Add amazing feature'`
9. Push to your fork: `git push origin feature/amazing-feature`
10. **Open a Pull Request** from your fork to the main repository

### Pull Request Requirements

- All PRs require at least 1 approval before merging
- All CI checks must pass (linting, tests)
- PRs must be up to date with `main` branch
- Conversation must be resolved before merging

### Code Style

- Follow Rust conventions and use `rustfmt`
- Run `cargo clippy` and fix warnings
- Add comments for complex logic
- Write tests for new features

### Commit Messages

- Use clear, descriptive messages
- Reference issue numbers when applicable
- Keep commits focused and atomic

## Development Setup

### Using Nix (Recommended)

usync includes Nix development environment files for easy setup:

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/usync.git
cd usync

# Enter Nix development environment (traditional)
nix-shell

# Or using flakes
nix develop

# All dependencies are automatically provided
# Build the project
cargo build

# Run tests
cargo test
./tests/test_runner.sh
```

The Nix files (`shell.nix` and `flake.nix`) are tracked in git to ensure a consistent development environment. They provide:
- Rust toolchain (latest stable)
- Cargo
- All build dependencies (pkg-config, openssl, etc.)
- Development tools
- Platform-specific dependencies (handled automatically)

### Manual Setup

If you prefer not to use Nix:

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/usync.git
cd usync

# Install Rust (if not already installed)
# See https://rustup.rs/

# Build the project
cargo build

# Run tests
cargo test
./tests/test_runner.sh
```

## AI-Assisted Development

This project acknowledges the use of AI-assisted development tools. Important points:

- **AI is a tool, not a replacement**: All code is reviewed by human developers
- **Review process**: Every change, including AI-generated code, goes through code review
- **Quality standards**: AI-generated code must meet the same quality standards as human-written code
- **Transparency**: We're open about using AI tools while maintaining high code quality through human oversight

If you use AI tools when contributing, please:
- Review and understand all generated code
- Test thoroughly
- Ensure it follows project conventions
- Be ready to explain and justify the changes

## Testing

- Write tests for new features
- Ensure existing tests still pass
- Add integration tests for new protocols or major features
- Test on multiple platforms when possible

## Documentation

- Update README.md for user-facing changes
- Add code comments for complex logic
- Update this file if contributing guidelines change

## Questions?

Feel free to open an issue for questions or discussions. We're here to help!

Thank you for contributing to usync! 🎉

