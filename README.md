# AWS SSO Navigator

A CLI tool to interactively select and login to AWS SSO profiles with fuzzy selection.

## Installation

Make sure you have rust installed - [instructions](https://rust-lang.org/tools/install/)

```bash
cargo install --path .
```

## Getting Started

### Quick Start with Import

If you have an existing SSO session configured, you can skip the first step

1. **Configure your AWS SSO session** in `~/.aws/config`:

   ```ini
   [sso-session mysession]
   sso_start_url = https://example.awsapps.com/start
   sso_region = us-east-1
   ```

1. **Import profiles from your SSO session:**

   ```bash
   aws-sso-navigator import <your-sso-session-name>
   ```

   This will discover and add all accounts/roles you have access to.

1. **Start using profiles:**

   ```bash
   aws-sso-navigator
   ```

   Select and login to any imported profile.

### Manual Setup

If you prefer to manually configure profiles or don't have an SSO session yet:

1. **Add individual profiles** following the naming convention `client-account-role`:

   ```ini
   [profile myclient-dev-admin]
   sso_session = mysession
   sso_account_id = 123456789012
   sso_role_name = AdministratorAccess
   ```

1. **Start navigating:**

   ```bash
   aws-sso-navigator
   ```

## Usage

### Authentication (Default Command)

#### Interactive Mode

Navigate through clients, accounts, and roles step-by-step:

```bash
aws-sso-navigator
# or explicitly
aws-sso-navigator auth
```

#### Unified Mode

Select from all profiles in a single picker:

```bash
aws-sso-navigator auth --unified
```

#### Skip Selection Steps

Pre-select specific values to skip interactive steps:

```bash
# Skip client selection
aws-sso-navigator auth --client myclient

# Skip client and account selection
aws-sso-navigator auth --client myclient --account myaccount

# Skip all selections
aws-sso-navigator auth --client myclient --account myaccount --role myrole
```

#### Set as Default Profile

Set the selected profile as the default AWS profile:

```bash
aws-sso-navigator auth --set-default
```

#### List All Profiles

Show all available profiles without selection:

```bash
aws-sso-navigator auth --list
```

#### Recent Profiles First

Show recently used profiles at the top:

```bash
aws-sso-navigator auth --recent
```

#### Force Reauthentication

Force login even if session is still valid:

```bash
aws-sso-navigator auth --force-reauth
```

#### Open AWS Console

Open the AWS console in browser instead of CLI login:

```bash
aws-sso-navigator auth --console
```

### Import Profiles

Import all available profiles from an SSO session:

```bash
aws-sso-navigator import <sso-session-name>
```

This command will:

1. Login to the specified SSO session
2. Discover all accounts and roles you have access to
3. Add profiles to your AWS config file

### Global Options

#### Custom Config Path

Use a different AWS config file:

```bash
aws-sso-navigator --aws-config-path /path/to/config auth
aws-sso-navigator --aws-config-path /path/to/config import <session>
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

# Show auth command help
aws-sso-navigator auth --help

# Show import command help
aws-sso-navigator import --help

# Show version
aws-sso-navigator --version
```

## Configuration

Optional settings file at `~/.config/aws-sso-navigator/config.toml`:

```toml
# Default values to pre-select
default_client = "myclient"
default_account = "dev" 
default_role = "admin"

# Use unified mode by default
unified_mode = false

# Set selected profile as default
set_default = false

# List profiles without selection
list = false

# Show recent profiles first
recent = false

# Maximum number of recent profiles to keep
max_recent_profiles = 100

# Force reauthentication even if session is valid
force_reauth = false

# Check for existing valid sessions
check_session = true

# Custom AWS config path
# aws_config_path = "/path/to/custom/config"

# Custom browser for AWS SSO login
# browser = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
```

## Requirements

- AWS CLI installed and configured
- AWS SSO profiles configured in `~/.aws/config`
