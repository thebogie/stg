# Version System Documentation

This document explains how the version system works in the STG frontend application.

## Overview

The version system provides multiple ways to display version information throughout the application:

1. **Navigation Bar**: Shows version in the top-right corner (desktop) and mobile menu
2. **Footer**: Shows full version information in the footer component
3. **API Integration**: Can fetch version info from backend if needed
4. **Build Information**: Captures git commit and build date

## Components

### Version Display Components

- **`VersionDisplay`**: Simple version text display
- **`VersionTooltip`**: Interactive tooltip with detailed version info
- **`Footer`**: Footer component with version information

### Version Module

The `frontend/src/version.rs` module provides:

```rust
// Get current version (e.g., "0.1.0")
Version::current()

// Get application name (e.g., "frontend")
Version::name()

// Get full version string (e.g., "frontend v0.1.0")
Version::full()

// Get short version string (e.g., "v0.1.0")
Version::short()

// Get build information (e.g., "frontend v0.1.0 (build: 2025-08-06 14:27:26 UTC, commit: 9fcdbd2)")
Version::build_info()
```

## Usage

### In Navigation

The version is automatically displayed in the navigation bar:
- **Desktop**: Shows as a tooltip in the top-right corner
- **Mobile**: Shows in the mobile menu at the bottom

### In Components

```rust
use crate::components::version_display::VersionDisplay;

// Simple version display
<VersionDisplay />

// Full version with name
<VersionDisplay show_full={true} />

// Build information
<VersionDisplay show_build_info={true} />

// With custom styling
<VersionDisplay class={classes!("text-sm", "text-gray-500")} />
```

### Version Tooltip

```rust
use crate::components::version_display::VersionTooltip;

// Interactive tooltip with detailed info
<VersionTooltip />
```

## Build Information

The build system automatically captures:

- **Git Commit**: Short commit hash (e.g., "9fcdbd2")
- **Build Date**: UTC timestamp of when the build was created
- **Version**: From `Cargo.toml` workspace version

### Build Scripts

- **Development**: `./frontend/run_frontend.sh` includes build info
- **Production**: `./frontend/run_prod_frontend.sh` includes build info
- **Manual**: `./scripts/build-info.sh` can be run independently

### Environment Variables

The build process sets these environment variables:

```bash
export GIT_COMMIT="9fcdbd2"
export BUILD_DATE="2025-08-06 14:27:26 UTC"
```

## Updating Versions

### Frontend Version

Update the version in the workspace `Cargo.toml`:

```toml
[workspace.package]
version = "0.1.0"  # Change this to update version
```

### Package.json Versions

You can also update the version in `package.json` files for consistency:

```json
{
  "version": "0.1.0"
}
```

## API Integration

The version system includes API integration for fetching version info from the backend:

```rust
use crate::api::version::get_version_info;

let version_info = get_version_info().await?;
println!("Backend version: {}", version_info.version);
```

## Testing

Run version tests:

```bash
cd frontend
cargo test version
```

## Customization

### Styling

Version components use Tailwind CSS classes and can be customized:

```rust
<VersionDisplay class={classes!("text-lg", "font-bold", "text-blue-600")} />
```

### Content

Modify the version display content in `frontend/src/version.rs`:

```rust
impl Version {
    pub fn custom_display() -> String {
        format!("STG v{}", Self::current())
    }
}
```

### Build Information

Customize build information in `scripts/build-info.sh`:

```bash
# Add custom build info
export CUSTOM_INFO="production"
export DEPLOYMENT_ENV="staging"
```

## Troubleshooting

### Version Not Showing

1. Check that the version module is imported in `lib.rs`
2. Verify the navigation component includes `VersionTooltip`
3. Ensure build scripts are executable: `chmod +x scripts/build-info.sh`

### Build Info Missing

1. Run `./scripts/build-info.sh` to verify it works
2. Check that git repository is accessible
3. Ensure build scripts source the build info script

### Styling Issues

1. Check Tailwind CSS classes are correct
2. Verify responsive design classes (`lg:block`, `md:hidden`)
3. Test on different screen sizes

## Future Enhancements

Potential improvements to the version system:

1. **Semantic Versioning**: Parse and display major/minor/patch versions
2. **Release Notes**: Link version to release notes or changelog
3. **Update Notifications**: Alert users when new versions are available
4. **Environment Indicators**: Show dev/staging/production indicators
5. **Performance Metrics**: Include build performance information 