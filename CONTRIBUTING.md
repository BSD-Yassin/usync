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

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

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

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/usync.git
cd usync

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

