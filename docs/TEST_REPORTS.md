# Test Reports Documentation

This document describes the comprehensive test reporting system for the STG project.

## Overview

The test reporting system provides multiple ways to generate and view test results:

1. **Text-based reports** - Simple, readable summaries
2. **HTML reports** - Beautiful, interactive visual reports with charts
3. **Combined reports** - Both text and HTML reports in one command

## Available Scripts

### 1. Complete Test Report Generator (Recommended)

Generates both text and HTML reports in one command:

```bash
./scripts/generate-complete-test-report.sh
```

**Options:**
- `--output DIR` - Specify output directory (default: `./test-reports`)
- `--html-only` - Generate only HTML report
- `--text-only` - Generate only text reports
- `--help` - Show usage information

**Examples:**
```bash
# Generate all reports
./scripts/generate-complete-test-report.sh

# Generate only HTML report
./scripts/generate-complete-test-report.sh --html-only

# Generate reports in custom directory
./scripts/generate-complete-test-report.sh --output ./reports
```

### 2. Text Report Generator

Generates traditional text-based test reports:

```bash
./scripts/generate-test-report.sh
```

**Options:**
- `--type TYPE` - Report type: `unit`, `coverage`, `integration`, `all` (default: `all`)
- `--output DIR` - Output directory (default: `./test-reports`)

### 3. HTML Report Generator

Generates beautiful HTML reports with charts and visualizations:

```bash
./scripts/generate-simple-html-report.sh
```

**Options:**
- `--output DIR` - Output directory (default: `./test-reports`)

## Generated Reports

### Text Reports

- `combined-summary.txt` - Overview of all test results
- `unit-test-summary.txt` - Unit test results and statistics
- `integration-test-summary.txt` - Integration test results
- `coverage-summary.txt` - Code coverage statistics

### HTML Reports

- `test-report.html` - Main interactive HTML report with:
  - 📊 Test results overview with charts
  - 📈 Code coverage visualization
  - 🔍 Detailed test breakdowns
  - 📄 Links to all detailed reports

### Coverage Reports

- `coverage-html/index.html` - Detailed HTML coverage report (if available)

## HTML Report Features

The HTML report includes:

### 📊 Visual Elements
- **Interactive Charts** - Doughnut chart for test results, bar chart for coverage
- **Color-coded Statistics** - Green for passed, red for failed, yellow for warnings
- **Responsive Design** - Works on desktop and mobile devices
- **Modern UI** - Glassmorphism design with gradients and animations

### 📈 Data Visualization
- **Test Results Overview** - Shows passed, failed, and ignored tests
- **Code Coverage** - Visual coverage bar with color coding
- **Test Breakdown** - Separate sections for unit and integration tests
- **Real-time Statistics** - Live calculation of totals and percentages

### 🔗 Navigation
- **Tabbed Interface** - Easy switching between different test types
- **Direct Links** - Quick access to detailed text reports
- **External Links** - Links to HTML coverage reports

## Prerequisites

### Required Tools

The scripts require the following tools to be installed:

```bash
# Install cargo-nextest for test running
cargo install cargo-nextest

# Install cargo-tarpaulin for coverage analysis
cargo install cargo-tarpaulin

# Install jq for JSON processing (Ubuntu/Debian)
sudo apt-get install jq

# Install jq for JSON processing (macOS)
brew install jq
```

### Project Structure

The scripts expect to be run from the project root directory with:
- `Cargo.toml` file present
- `backend/` directory containing the Rust code
- `scripts/` directory containing the report generators

## Usage Examples

### Quick Start

Generate all reports with one command:

```bash
./scripts/generate-complete-test-report.sh
```

### View HTML Report

After generation, open the HTML report in your browser:

```bash
# On Linux
xdg-open ./test-reports/test-report.html

# On macOS
open ./test-reports/test-report.html

# On Windows
start ./test-reports/test-report.html
```

### Custom Output Directory

Generate reports in a custom location:

```bash
./scripts/generate-complete-test-report.sh --output ./my-reports
```

### HTML Only

Generate only the visual HTML report:

```bash
./scripts/generate-complete-test-report.sh --html-only
```

## Report Structure

### Text Reports Format

```
Comprehensive Test Report Summary
=================================
Generated: [timestamp]

Unit Tests:
-----------
[unit test summary]

Integration Tests:
------------------
[integration test summary]

Code Coverage:
--------------
[coverage summary]

Report Files:
-------------
[list of generated files]
```

### HTML Report Sections

1. **Header** - Project title and generation timestamp
2. **Statistics Grid** - Key metrics in colored cards
3. **Test Results Overview** - Interactive doughnut chart
4. **Code Coverage** - Visual coverage bar and percentage
5. **Test Breakdown** - Detailed unit and integration test stats
6. **Report Links** - Quick access to all detailed reports

## Troubleshooting

### Common Issues

**Script not found:**
```bash
# Make scripts executable
chmod +x scripts/*.sh
```

**Missing tools:**
```bash
# Install required tools
cargo install cargo-nextest cargo-tarpaulin
sudo apt-get install jq  # Ubuntu/Debian
```

**No test data:**
```bash
# Run tests first to generate data
cargo test
./scripts/generate-test-report.sh
```

**HTML report not loading:**
- Ensure you're opening the file in a web browser
- Check that the file path is correct
- Verify the HTML file was generated successfully

### Debug Mode

For troubleshooting, you can run scripts with verbose output:

```bash
# Enable debug output
set -x
./scripts/generate-complete-test-report.sh
set +x
```

## Customization

### Modifying HTML Report

The HTML report can be customized by editing `scripts/generate-simple-html-report.sh`:

- **Colors** - Modify CSS variables in the style section
- **Charts** - Adjust Chart.js configuration
- **Layout** - Modify HTML structure and CSS grid
- **Data** - Change how statistics are extracted and displayed

### Adding New Report Types

To add new report types:

1. Create a new script in the `scripts/` directory
2. Follow the naming convention: `generate-[type]-report.sh`
3. Update the complete report generator to include the new type
4. Add documentation to this file

## Best Practices

### When to Use Each Report Type

- **Text Reports** - For CI/CD pipelines, logging, and quick command-line review
- **HTML Reports** - For team presentations, stakeholder reviews, and detailed analysis
- **Combined Reports** - For comprehensive project health assessment

### Integration with CI/CD

Add to your CI/CD pipeline:

```yaml
# Example GitHub Actions step
- name: Generate Test Reports
  run: |
    ./scripts/generate-complete-test-report.sh
    # Upload reports as artifacts
    # Or publish to a web server
```

### Regular Reporting

Set up automated reporting:

```bash
# Add to crontab for daily reports
0 9 * * * cd /path/to/project && ./scripts/generate-complete-test-report.sh
```

## Support

For issues or questions about the test reporting system:

1. Check this documentation
2. Review the script help messages: `./scripts/generate-complete-test-report.sh --help`
3. Check the generated reports for error messages
4. Verify all prerequisites are installed

---

*Generated by STG Test Report Generator*