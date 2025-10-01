# AWS SSO Navigator

A CLI tool to interactively select and login to AWS SSO profiles with fuzzy selection.

## Installation

```bash
cargo build --release
```

## Usage

### Interactive Mode (Default)

Navigate through clients, accounts, and roles step-by-step:

```bash
aws-sso-navigator
```

### Unified Mode

Select from all profiles in a single picker:

```bash
aws-sso-navigator --unified
```

### Skip Selection Steps

Pre-select specific values to skip interactive steps:

```bash
# Skip client selection
aws-sso-navigator --client myclient

# Skip client and account selection
aws-sso-navigator --client myclient --account myaccount

# Skip all selections
aws-sso-navigator --client myclient --account myaccount --role myrole
```

### Custom Config Path

Use a different AWS config file:

```bash
aws-sso-navigator --aws-config-path /path/to/config
```

## Profile Format

Profiles must follow the naming convention: `client-account-role`

Example AWS config:

```ini
[profile myclient-dev-admin]
sso_start_url = https://example.awsapps.com/start
sso_region = us-east-1
sso_account_id = 123456789012
sso_role_name = AdministratorAccess

[profile myclient-prod-readonly]
sso_start_url = https://example.awsapps.com/start
sso_region = us-east-1
sso_account_id = 987654321098
sso_role_name = ReadOnlyAccess
```

## Help

```bash
# Show help
aws-sso-navigator --help

# Show version
aws-sso-navigator --version
```

## Requirements

- AWS CLI installed and configured
- AWS SSO profiles configured in `~/.aws/config`
