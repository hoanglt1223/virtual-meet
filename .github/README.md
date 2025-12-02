# GitHub Actions CI/CD

This directory contains the simplified GitHub Actions workflow for VirtualMeet's automated build and deploy pipeline.

## 🚀 Simplified Build and Deploy Pipeline

### Build and Deploy Workflow (`build-deploy.yml`)
**Triggers**: Push to `main`/`develop`, Pull Requests

**Single Job**: All build and deploy functionality in one streamlined process

**Steps**:
1. **Setup**: Install pnpm, Node.js, Rust toolchain
2. **Dependencies**: Install and audit packages
3. **Quality Checks** (non-blocking):
   - TypeScript type checking
   - ESLint code quality verification
   - Rust formatting (cargo fmt)
   - Rust linting (cargo clippy)
   - Rust tests (cargo test)
4. **Build**: Frontend + Tauri application compilation
5. **Deploy**: Upload build artifacts (Windows executable and installer)

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