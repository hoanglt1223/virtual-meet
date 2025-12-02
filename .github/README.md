# GitHub Actions CI/CD

This directory contains the GitHub Actions workflows for VirtualMeet's automated CI/CD pipeline.

## 🚀 Workflows

### 1. CI Pipeline (`ci.yml`)
**Triggers**: Push to `main`/`develop`, Pull Requests

**Jobs**:
- **Rust Checks**: Code formatting, clippy linting, tests
- **Frontend Checks**: ESLint, TypeScript verification, build
- **Tauri Build**: Windows builds with artifact upload (main/develop only)
- **Security Audit**: Vulnerability scanning
- **Dependency Check**: Outdated package monitoring

### 2. Release Pipeline (`release.yml`)
**Triggers**: Git tags `v*`

**Jobs**:
- **Release Creation**: Automatic GitHub release
- **Windows Builds**: Installer and portable versions
- **Artifact Upload**: Release assets with checksums
- **Release Notes**: Automatic documentation updates

### 3. Maintenance (`maintenance.yml`)
**Triggers**: Weekly (Mondays 9AM UTC), Manual dispatch

**Jobs**:
- **Dependency Updates**: Automated PRs for outdated packages
- **Security Audit**: Weekly vulnerability reports
- **Artifact Cleanup**: Remove old artifacts

## 🔧 Setup Required

Add these secrets to your GitHub repository:

```yaml
TAURI_PRIVATE_KEY: "your_tauri_private_key"
TAURI_KEY_PASSWORD: "your_tauri_key_password"
```

## 📋 Usage

### Development
- Create PRs for automatic testing
- Merge to `main`/`develop` for additional checks
- Review workflow results in GitHub Actions tab

### Releases
1. Update version numbers
2. Create annotated tag: `git tag v1.0.0`
3. Push tag: `git push origin v1.0.0`
4. Automated release will be created

### Maintenance
- Weekly dependency updates via PRs
- Manual trigger available for dependency checks
- Automatic security vulnerability reports

## 📊 Monitoring

- **CI Status**: Automated success/failure notifications
- **Build Artifacts**: Available for 30 days
- **Security Reports**: Weekly GitHub issues
- **Dependency Health**: Continuous monitoring

## 🚨 Troubleshooting

### Common Issues
1. **Build Failures**: Check dependency versions and Rust toolchain
2. **Code Signing**: Ensure Tauri secrets are properly configured
3. **Cache Issues**: Clear caches in GitHub Actions settings
4. **Permission Errors**: Verify GitHub token permissions

### Debug Steps
1. Review workflow logs in Actions tab
2. Check individual job outputs
3. Verify dependency versions
4. Validate environment configurations

## 📚 Documentation

For detailed implementation information, see [CI/CD Pipeline Implementation](../todo.md#-cicd-pipeline-implementation) in the main TODO.md file.