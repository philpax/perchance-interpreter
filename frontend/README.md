# Perchance Interpreter Frontend

A beautiful, interactive web frontend for the Perchance template language interpreter built with React, TypeScript, Vite, and WebAssembly.

## Features

- **Live Preview**: See your output update as you type (with debouncing)
- **Manual Execution**: Run templates on-demand with a specific seed
- **Multiple Samples**: Generate multiple random outputs from the same template
- **Error Handling**: Clear error messages with helpful formatting
- **Beautiful UI**: Modern, responsive design with Tailwind CSS
- **Fast**: Powered by WebAssembly for near-native performance

## Development

### Prerequisites

- Python 3.7+
- Node.js (v18 or later recommended)
- Rust toolchain with `wasm-pack` installed

### Quick Start (Recommended)

From the project root, use the Python build script:

```bash
# Build WASM and start development server
python build-frontend.py --dev

# Build for production
python build-frontend.py --build

# See all options
python build-frontend.py --help
```

### Manual Setup

1. Build the WASM module:
   ```bash
   cd ..
   wasm-pack build perchance-wasm --target web --out-dir ../frontend/src/wasm
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Start the development server:
   ```bash
   npm run dev
   ```

4. Open your browser to the URL shown (typically http://localhost:5173)

### Building for Production

```bash
npm run build
```

The built files will be in the `dist/` directory.

## Usage

1. **Editor Panel (Left)**:
   - Write or edit your Perchance template
   - Toggle auto-evaluation on/off
   - Set a seed for deterministic output
   - Click "Run" to manually evaluate
   - Specify number of samples and click "Generate Multiple"

2. **Preview Panel (Right)**:
   - See the output of your template
   - View error messages if the template is invalid
   - Browse multiple samples when generated

## Technology Stack

- **React**: UI framework
- **TypeScript**: Type-safe JavaScript
- **Vite**: Fast build tool and dev server
- **Tailwind CSS**: Utility-first CSS framework
- **WebAssembly**: High-performance Rust interpreter compiled to WASM
- **wasm-bindgen**: Rust-WASM-JavaScript interop
