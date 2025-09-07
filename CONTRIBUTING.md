# Contributing to ArthaChain

**ArthaChain - Developed by DIIGOO Tech Private Limited**  
*Decentralised Indian Innovation for Global Open Opportunities (DIIGOO)*

*Meaning. Power. Code.*

---

Thank you for your interest in contributing to ArthaChain! This document provides guidelines for contributing to the project.

## Development Process

### Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a new branch for your feature or bugfix
4. Make your changes
5. Test your changes thoroughly
6. Submit a pull request

### Development Environment Setup

1. Install Rust toolchain (latest stable version)
2. Clone the repository: `git clone https://github.com/arthachain/arthachain.git`
3. Navigate to the project directory: `cd arthachain`
4. Build the project: `cargo build`
5. Run tests: `cargo test`

## Code Standards

### Rust Guidelines

- Follow Rust naming conventions and idioms
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for common issues
- Write comprehensive documentation for public APIs
- Include unit tests for new functionality

### Commit Messages

Commit messages should be clear and descriptive:

```
[area] Brief description of changes

Detailed explanation of what was changed and why.
Reference any related issues.
```

Examples:
- `[consensus] Fix block validation logic`
- `[api] Add new endpoint for transaction history`
- `[docs] Update installation instructions`

### Testing

- Write unit tests for all new functionality
- Ensure all existing tests continue to pass
- Add integration tests for complex features
- Test edge cases and error conditions

### Documentation

- Update relevant documentation for API changes
- Include code examples where appropriate
- Keep README files up to date
- Document any breaking changes

## Pull Request Process

1. Ensure your branch is up to date with the main branch
2. Run all tests and ensure they pass
3. Update documentation if necessary
4. Submit a pull request with a clear description
5. Respond to review feedback promptly
6. Keep the PR focused on a single feature or bugfix

### Pull Request Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
Describe the tests you ran to verify your changes

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Tests added/updated
- [ ] Documentation updated
```

## Issue Reporting

When reporting issues, please include:

- Clear description of the problem
- Steps to reproduce
- Expected behavior
- Actual behavior
- System information (OS, Rust version, etc.)
- Relevant logs or error messages

## Security

For security-related issues, please contact security@arthachain.in instead of creating a public issue.

## Code Review Process

- All code changes require review
- Reviewers will check for correctness, style, and completeness
- Address all review feedback before merging
- Maintainers have final authority on all changes

## Community Guidelines

- Be respectful and constructive in all interactions
- Help others learn and grow
- Follow the Code of Conduct
- Ask questions if you're unsure about anything

## Getting Help

- Check existing documentation
- Search existing issues and discussions
- Join our Discord community
- Ask questions in GitHub discussions

Thank you for contributing to ArthaChain!
