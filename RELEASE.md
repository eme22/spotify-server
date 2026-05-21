# Release Automation

This project uses GitHub Actions for automated releases and continuous integration.

## Release Process

### Automatic Releases

Releases are automatically created when you push a version tag:

```bash
# Create and push a new version tag
git tag v1.0.0
git push origin v1.0.0
```

This will automatically:
1. Create a GitHub release
2. Build binaries for Windows, macOS, and Linux
3. Upload the binaries to the release
4. Optionally publish to crates.io (if configured)

### Manual Release

You can also trigger a release manually:
1. Go to the Actions tab in your GitHub repository
2. Select the "Release" workflow
3. Click "Run workflow"
4. Enter the tag name (e.g., `v1.0.0`)

## Supported Platforms

The automated release builds for:
- **Windows** (x64): `spotify-server-windows-x64.exe`
- **macOS** (Intel): `spotify-server-macos-x64`
- **macOS** (Apple Silicon): `spotify-server-macos-arm64`
- **Linux** (x64): `spotify-server-linux-x64`

## Configuration

### Required Secrets

For full automation, configure these GitHub secrets:

#### Optional: Crates.io Publishing
- `CRATES_IO_TOKEN`: Your crates.io API token for publishing
### Setting up Secrets

1. Go to your repository settings
2. Navigate to "Secrets and variables" → "Actions"
3. Add the secrets listed above

## Continuous Integration

The CI workflow runs on every push and pull request:
- **Tests**: Runs `cargo test`
- **Formatting**: Checks with `cargo fmt`
- **Linting**: Runs `cargo clippy`
- **Build**: Tests compilation on Windows, macOS, and Linux

## Dependency Updates

Dependencies are automatically checked for updates every Monday:
- Creates a pull request with updated dependencies
- Ensures tests still pass
- Requires manual review and merge

## Version Bumping

To create a new release:

1. Update the version in `Cargo.toml`:
   ```toml
   [package]
   version = "1.0.1"  # Increment as needed
   ```

2. Commit the change:
   ```bash
   git add Cargo.toml
   git commit -m "bump version to 1.0.1"
   ```

3. Create and push the tag:
   ```bash
   git tag v1.0.1
   git push origin main
   git push origin v1.0.1
   ```

The release will be automatically created with binaries.

## Troubleshooting

### Build Failures
- Check the Actions tab for detailed error logs
- Ensure all dependencies are correctly specified in `Cargo.toml`
- Verify platform-specific code is properly conditional

### Release Issues
- Ensure the tag follows the pattern `v*.*.*` (e.g., `v1.0.0`)
- Verify you have the necessary permissions to create releases
