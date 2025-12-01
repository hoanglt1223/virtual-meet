# Package Management with pnpm

This project uses [pnpm](https://pnpm.io/) as the package manager for better performance and disk space efficiency.

## Why pnpm?

- **Fast**: pnpm is up to 2x faster than npm
- **Efficient**: Uses a content-addressable filesystem to store all packages
- **Strict**: Helps avoid phantom dependencies by default
- **Monorepo-friendly**: Excellent support for monorepos

## Getting Started

### Prerequisites

- Node.js 18.0.0 or higher
- pnpm 9.0.0 or higher

### Installation

```bash
# Install pnpm globally
npm install -g pnpm@latest

# Or using corepack (Node.js 16.10+)
corepack enable
corepack prepare pnpm@latest --activate
```

### Common Commands

```bash
# Install dependencies
pnpm install

# Start development server
pnpm dev

# Build for production
pnpm build

# Build Tauri app
pnpm tauri:build

# Run Tauri in development mode
pnpm tauri:dev

# Type checking
pnpm type-check

# Update dependencies
pnpm up

# Check for outdated packages
pnpm outdated

# Security audit
pnpm audit

# Clean install (remove node_modules and reinstall)
pnpm reinstall

# Clean build artifacts
pnpm clean
```

### Development Scripts

The project includes several useful npm scripts:

- `pnpm dev` - Start Vite development server
- `pnpm build` - TypeScript compilation + Vite build
- `pnpm preview` - Preview production build
- `pnpm tauri:dev` - Run Tauri app in development mode
- `pnpm tauri:build` - Build Tauri app for production
- `pnpm type-check` - Run TypeScript type checking without emitting files
- `pnpm clean` - Remove dist directory
- `pnpm reinstall` - Clean reinstall of all dependencies
- `pnpm up` - Update all dependencies to their latest versions
- `pnpm outdated` - Show outdated dependencies
- `pnpm audit` - Check for security vulnerabilities

## pnpm Workspace

This is a single-package project, but it's configured to be pnpm-workspace compatible for potential future expansion.

## Configuration

The project uses a `.npmrc` file for pnpm-specific configuration:

- `shamefully-hoist=true` - Ensures compatibility with tools expecting traditional node_modules structure
- `prefer-frozen-lockfile=true` - Ensures reproducible builds
- `strict-peer-dependencies=true` - Catches version conflicts early
- `auto-install-peers=true` - Automatically installs peer dependencies

## Troubleshooting

### Common Issues

1. **Permission errors**: Run with elevated permissions or fix npm permissions
2. **Cache issues**: Clear pnpm cache with `pnpm store prune`
3. **Lock file conflicts**: Delete `pnpm-lock.yaml` and run `pnpm install`

### Cache Management

```bash
# View cache location
pnpm store path

# Prune old packages from cache
pnpm store prune

# List all packages in cache
pnpm store ls
```

## Migration from npm

If you're migrating from npm:

1. Delete `node_modules` and `package-lock.json`
2. Run `pnpm install`
3. Update any CI/CD scripts to use `pnpm` instead of `npm`

## Resources

- [pnpm Documentation](https://pnpm.io/)
- [pnpm vs npm](https://pnpm.io/Comparison-with-npm)
- [pnpm Frequently Asked Questions](https://pnpm.io/FAQ)