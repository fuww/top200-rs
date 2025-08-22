# GitHub Copilot Configuration

This repository is configured for optimal GitHub Copilot integration to enhance Rust development productivity.

## Setup

GitHub Copilot is configured with:

- **Enabled Languages**: Rust, TOML, SQL, Markdown, YAML, JSON
- **Excluded Files**: Defined in `.copilotignore` to exclude data files, build artifacts, and sensitive content
- **VS Code Integration**: Configured in `.vscode/settings.json` with Rust-specific optimizations

## Best Practices for Using Copilot with this Project

1. **Context-Aware Development**:
   - Copilot has been trained on the codebase structure and patterns
   - Use descriptive comments to guide Copilot suggestions
   - Leverage existing patterns in `src/` modules for consistency

2. **Rust-Specific Features**:
   - Copilot understands the async/await patterns used throughout the codebase
   - Suggestions include proper error handling with `anyhow::Result`
   - Database operations with SQLx are well-supported

3. **Financial Data Handling**:
   - Be explicit about data types when working with market cap calculations
   - Use comments to clarify currency conversion logic
   - Copilot can suggest appropriate validation for financial data

4. **CLI Command Development**:
   - Copilot understands the clap-based CLI structure
   - Use clear function names and comments for new subcommands
   - Follow existing patterns in `main.rs` for argument parsing

5. **Testing Suggestions**:
   - Copilot can generate test cases based on existing test patterns
   - Use descriptive test function names for better suggestions
   - Include edge cases for financial calculations

## Copilot Chat Usage

Use GitHub Copilot Chat for:
- Explaining complex financial calculation logic
- Generating SQL queries for market cap analysis
- Code reviews and optimization suggestions
- Documentation improvements

## Excluded from Copilot Context

The `.copilotignore` file excludes:
- Database files (`data.db`, `*.sqlite`)
- Build artifacts (`target/`, `debug/`)
- Environment files (`.env`, `.env.*`)
- Output data (`output/`, `*.csv`)
- License files and legal text
- System configuration files

This ensures Copilot focuses on code rather than data or configuration.