#!/usr/bin/env python3
"""
Cross-platform build script for the Perchance Interpreter frontend.
Builds the WASM module and sets up the frontend for development or production.
"""

import argparse
import os
import platform
import subprocess
import sys
from pathlib import Path


def run_command(cmd, cwd=None, description=None):
    """Run a command and handle errors."""
    if description:
        print(f"\n{description}...")

    # On Windows, use shell=True to resolve .cmd and .bat files
    use_shell = platform.system() == 'Windows'

    # For display purposes, join the command
    if use_shell and isinstance(cmd, list):
        cmd_str = ' '.join(cmd)
        print(f"$ {cmd_str}")
    else:
        print(f"$ {' '.join(cmd)}")

    try:
        result = subprocess.run(
            cmd,
            cwd=cwd,
            check=True,
            capture_output=False,
            text=True,
            shell=use_shell
        )
        return result
    except subprocess.CalledProcessError as e:
        print(f"Error: Command failed with exit code {e.returncode}", file=sys.stderr)
        sys.exit(1)
    except FileNotFoundError:
        cmd_name = cmd[0] if isinstance(cmd, list) else cmd.split()[0]
        print(f"Error: Command not found: {cmd_name}", file=sys.stderr)
        print(f"Make sure {cmd_name} is installed and in your PATH", file=sys.stderr)
        sys.exit(1)


def check_prerequisites():
    """Check that required tools are installed."""
    tools = {
        'wasm-pack': 'wasm-pack --version',
        'npm': 'npm --version',
    }

    # On Windows, use shell=True to resolve .cmd and .bat files
    use_shell = platform.system() == 'Windows'

    missing = []
    for tool, check_cmd in tools.items():
        try:
            subprocess.run(
                check_cmd.split(),
                capture_output=True,
                check=True,
                shell=use_shell
            )
        except (subprocess.CalledProcessError, FileNotFoundError):
            missing.append(tool)

    if missing:
        print("Error: Missing required tools:", file=sys.stderr)
        for tool in missing:
            print(f"  - {tool}", file=sys.stderr)
        print("\nPlease install the missing tools and try again.", file=sys.stderr)
        sys.exit(1)


def build_wasm(project_root):
    """Build the WASM module."""
    print("=" * 60)
    print("Building WASM Module")
    print("=" * 60)

    wasm_dir = project_root / "perchance-wasm"
    output_dir = project_root / "frontend" / "src" / "wasm"

    run_command(
        [
            "wasm-pack", "build",
            str(wasm_dir),
            "--target", "web",
            "--out-dir", str(output_dir)
        ],
        cwd=project_root,
        description="Building WASM module with wasm-pack"
    )

    print("✓ WASM module built successfully")


def install_dependencies(frontend_dir):
    """Install frontend dependencies."""
    print("\n" + "=" * 60)
    print("Installing Frontend Dependencies")
    print("=" * 60)

    run_command(
        ["npm", "install"],
        cwd=frontend_dir,
        description="Installing npm packages"
    )

    print("✓ Dependencies installed successfully")


def build_frontend(frontend_dir):
    """Build the frontend for production."""
    print("\n" + "=" * 60)
    print("Building Frontend")
    print("=" * 60)

    run_command(
        ["npm", "run", "build"],
        cwd=frontend_dir,
        description="Building production bundle"
    )

    print("✓ Frontend built successfully")


def dev_server(frontend_dir):
    """Start the development server."""
    print("\n" + "=" * 60)
    print("Starting Development Server")
    print("=" * 60)

    print("\nPress Ctrl+C to stop the server\n")

    # On Windows, use shell=True to resolve .cmd and .bat files
    use_shell = platform.system() == 'Windows'

    try:
        subprocess.run(
            ["npm", "run", "dev"],
            cwd=frontend_dir,
            check=True,
            shell=use_shell
        )
    except KeyboardInterrupt:
        print("\n\nDevelopment server stopped.")


def main():
    parser = argparse.ArgumentParser(
        description="Build the Perchance Interpreter frontend",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                    # Build WASM and install dependencies
  %(prog)s --build            # Full production build
  %(prog)s --dev              # Build and start dev server
  %(prog)s --skip-wasm        # Skip WASM build (use existing)
        """
    )

    parser.add_argument(
        "--build",
        action="store_true",
        help="Build frontend for production"
    )

    parser.add_argument(
        "--dev",
        action="store_true",
        help="Start development server after building"
    )

    parser.add_argument(
        "--skip-wasm",
        action="store_true",
        help="Skip WASM build (use existing WASM module)"
    )

    parser.add_argument(
        "--skip-install",
        action="store_true",
        help="Skip npm install (use existing node_modules)"
    )

    args = parser.parse_args()

    # Determine project root (script location)
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir
    frontend_dir = project_root / "frontend"

    # Check prerequisites
    check_prerequisites()

    print("\n" + "=" * 60)
    print("Perchance Interpreter Frontend Build")
    print("=" * 60)
    print(f"Project root: {project_root}")
    print(f"Frontend dir: {frontend_dir}")

    # Build WASM
    if not args.skip_wasm:
        build_wasm(project_root)
    else:
        print("\nSkipping WASM build (--skip-wasm)")

    # Install dependencies
    if not args.skip_install:
        install_dependencies(frontend_dir)
    else:
        print("\nSkipping dependency installation (--skip-install)")

    # Build or dev
    if args.build:
        build_frontend(frontend_dir)
        print("\n" + "=" * 60)
        print("Build Complete!")
        print("=" * 60)
        print(f"\nProduction build available in: {frontend_dir / 'dist'}")
        print("\nTo preview the production build:")
        print(f"  cd {frontend_dir}")
        print("  npm run preview")
    elif args.dev:
        dev_server(frontend_dir)
    else:
        print("\n" + "=" * 60)
        print("Setup Complete!")
        print("=" * 60)
        print("\nNext steps:")
        print(f"  cd {frontend_dir}")
        print("  npm run dev        # Start development server")
        print("  npm run build      # Build for production")
        print("\nOr use this script:")
        print("  python build-frontend.py --dev     # Start dev server")
        print("  python build-frontend.py --build   # Production build")


if __name__ == "__main__":
    main()
