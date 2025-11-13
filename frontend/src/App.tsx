import { useState, useEffect, useCallback, useRef } from 'react';
import init, { evaluate_multiple, get_available_generators } from './wasm/perchance_wasm';
import AutocompleteDropdown from './AutocompleteDropdown';

const DEFAULT_TEMPLATE = `animal
\tdog
\tcat^2
\tbird

color
\tred
\tblue
\tgreen

output
\tI saw a {beautiful|pretty|cute} [color] [animal]!`;

function App() {
  const [wasmReady, setWasmReady] = useState(false);
  const [template, setTemplate] = useState(DEFAULT_TEMPLATE);
  const [error, setError] = useState<string | null>(null);
  const [autoEval, setAutoEval] = useState(true);
  const [seed, setSeed] = useState<string>('42');
  const [sampleCount, setSampleCount] = useState<number>(5);
  const [samples, setSamples] = useState<string[]>([]);

  // Autocomplete state
  const [availableGenerators, setAvailableGenerators] = useState<string[]>([]);
  const [showAutocomplete, setShowAutocomplete] = useState(false);
  const [autocompletePosition, setAutocompletePosition] = useState({ top: 0, left: 0 });
  const [autocompleteSearch, setAutocompleteSearch] = useState('');
  const [autocompleteSelectedIndex, setAutocompleteSelectedIndex] = useState(0);
  const [importStartPos, setImportStartPos] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Initialize WASM
  useEffect(() => {
    init().then(() => {
      setWasmReady(true);
      // Load available generators
      try {
        const generators = get_available_generators();
        setAvailableGenerators(generators);
      } catch (e) {
        console.error('Failed to load generators:', e);
      }
    });
  }, []);

  // Randomize seed
  const randomizeSeed = () => {
    setSeed(Math.floor(Math.random() * 1000000).toString());
  };

  // Generate multiple samples
  const generateSamples = useCallback(async (code: string, count: number, evalSeed: string) => {
    if (!wasmReady) return;

    try {
      const seedValue = BigInt(parseInt(evalSeed) || 42);
      const results = await evaluate_multiple(code, count, seedValue);
      setSamples(results as string[]);
      setError(null);
    } catch (e) {
      setError(String(e));
      setSamples([]);
    }
  }, [wasmReady]);

  // Auto-evaluate on template/seed/count change
  useEffect(() => {
    if (autoEval && wasmReady) {
      const timer = setTimeout(() => {
        generateSamples(template, sampleCount, seed);
      }, 300); // Debounce
      return () => clearTimeout(timer);
    }
  }, [template, sampleCount, seed, autoEval, wasmReady, generateSamples]);

  // Handle slider change (1-10)
  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSampleCount(parseInt(e.target.value));
  };

  // Handle text input change (unbounded)
  const handleTextChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseInt(e.target.value);
    if (!isNaN(value) && value > 0) {
      setSampleCount(value);
    }
  };

  // Handle template change with autocomplete detection
  const handleTemplateChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    const cursorPos = e.target.selectionStart;

    setTemplate(newValue);

    // Check if we should show autocomplete
    const beforeCursor = newValue.substring(0, cursorPos);
    const importMatch = beforeCursor.match(/\{import:([^}]*)$/);

    if (importMatch && textareaRef.current) {
      // Found {import: pattern
      const searchTerm = importMatch[1];
      setAutocompleteSearch(searchTerm);
      setImportStartPos(cursorPos - searchTerm.length);
      setAutocompleteSelectedIndex(0);

      // Calculate position for dropdown
      const textarea = textareaRef.current;
      const textBeforeCursor = beforeCursor;
      const lines = textBeforeCursor.split('\n');
      const currentLineIndex = lines.length - 1;
      const currentLineText = lines[currentLineIndex];

      // Rough calculation of position
      const lineHeight = 20; // approximate
      const charWidth = 8; // approximate for monospace
      const top = textarea.offsetTop + (currentLineIndex + 1) * lineHeight + 40;
      const left = textarea.offsetLeft + currentLineText.length * charWidth + 20;

      setAutocompletePosition({ top, left });
      setShowAutocomplete(true);
    } else {
      setShowAutocomplete(false);
    }
  };

  // Filter generators based on search
  const filteredGenerators = availableGenerators.filter((gen) =>
    gen.toLowerCase().includes(autocompleteSearch.toLowerCase())
  );

  // Handle autocomplete selection
  const handleAutocompleteSelect = (generatorName: string) => {
    if (!textareaRef.current) return;

    const cursorPos = textareaRef.current.selectionStart;
    const before = template.substring(0, importStartPos);
    const after = template.substring(cursorPos);

    const newTemplate = `${before}${generatorName}}${after}`;
    setTemplate(newTemplate);

    // Move cursor after the inserted text
    const newCursorPos = importStartPos + generatorName.length + 1;
    setTimeout(() => {
      if (textareaRef.current) {
        textareaRef.current.selectionStart = newCursorPos;
        textareaRef.current.selectionEnd = newCursorPos;
        textareaRef.current.focus();
      }
    }, 0);

    setShowAutocomplete(false);
  };

  // Handle autocomplete navigation
  const handleAutocompleteNavigate = (direction: 'up' | 'down') => {
    setAutocompleteSelectedIndex((prev) => {
      if (direction === 'down') {
        return Math.min(prev + 1, filteredGenerators.length - 1);
      } else {
        return Math.max(prev - 1, 0);
      }
    });
  };

  // Close autocomplete
  const handleAutocompleteClose = () => {
    setShowAutocomplete(false);
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 text-white">
      <div className="container mx-auto px-4 py-8">
        {/* Header */}
        <div className="mb-8 text-center">
          <h1 className="text-5xl font-bold bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent mb-2">
            Perchance Interpreter
          </h1>
          <p className="text-gray-400 text-lg">
            A deterministic random text generator •{' '}
            <a
              href="https://perchance.org/tutorial"
              target="_blank"
              rel="noopener noreferrer"
              className="text-purple-400 hover:text-purple-300 transition-colors underline"
            >
              Tutorial
            </a>
          </p>
        </div>

        {!wasmReady && (
          <div className="text-center py-12">
            <div className="inline-block animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-purple-500"></div>
            <p className="mt-4 text-gray-400">Loading interpreter...</p>
          </div>
        )}

        {wasmReady && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Editor Panel */}
            <div className="bg-slate-800/50 backdrop-blur rounded-lg shadow-2xl border border-slate-700/50 overflow-hidden">
              <div className="bg-gradient-to-r from-purple-600 to-pink-600 px-6 py-3">
                <h2 className="text-xl font-semibold">Template Editor</h2>
              </div>
              <div className="p-6 relative">
                <textarea
                  ref={textareaRef}
                  value={template}
                  onChange={handleTemplateChange}
                  className="w-full h-[500px] bg-slate-900 text-gray-100 font-mono text-sm p-4 rounded-lg border border-slate-600 focus:border-purple-500 focus:ring-2 focus:ring-purple-500/20 focus:outline-none transition-all resize-none"
                  placeholder="Enter your Perchance template here..."
                  spellCheck={false}
                />

                {/* Autocomplete Dropdown */}
                {showAutocomplete && (
                  <AutocompleteDropdown
                    items={filteredGenerators}
                    position={autocompletePosition}
                    selectedIndex={autocompleteSelectedIndex}
                    onSelect={handleAutocompleteSelect}
                    onClose={handleAutocompleteClose}
                    onNavigate={handleAutocompleteNavigate}
                  />
                )}

                {/* Controls */}
                <div className="mt-4 space-y-4">
                  <div className="flex items-center gap-4 flex-wrap">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={autoEval}
                        onChange={(e) => setAutoEval(e.target.checked)}
                        className="w-4 h-4 rounded border-slate-600 bg-slate-700 text-purple-600 focus:ring-purple-500 focus:ring-offset-slate-900"
                      />
                      <span className="text-sm text-gray-300">Auto-evaluate</span>
                    </label>

                    <div className="flex items-center gap-2">
                      <label className="text-sm text-gray-300">Seed:</label>
                      <input
                        type="number"
                        value={seed}
                        onChange={(e) => setSeed(e.target.value)}
                        className="w-24 px-3 py-1 bg-slate-700 border border-slate-600 rounded text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500/20 focus:outline-none"
                      />
                      <button
                        onClick={randomizeSeed}
                        className="px-3 py-1 bg-slate-700 hover:bg-slate-600 border border-slate-600 rounded text-sm transition-colors"
                        title="Randomize seed"
                      >
                        🎲
                      </button>
                    </div>
                  </div>

                  <div className="space-y-2">
                    <div className="flex items-center gap-2">
                      <label className="text-sm text-gray-300">Samples: {sampleCount}</label>
                    </div>
                    <div className="flex items-center gap-4">
                      <input
                        type="range"
                        min="1"
                        max="10"
                        value={Math.min(sampleCount, 10)}
                        onChange={handleSliderChange}
                        className="flex-1 h-2 bg-slate-700 rounded-lg appearance-none cursor-pointer accent-purple-600"
                      />
                      <input
                        type="number"
                        value={sampleCount}
                        onChange={handleTextChange}
                        min="1"
                        className="w-20 px-3 py-1 bg-slate-700 border border-slate-600 rounded text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500/20 focus:outline-none"
                      />
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Preview Panel */}
            <div className="bg-slate-800/50 backdrop-blur rounded-lg shadow-2xl border border-slate-700/50 overflow-hidden">
              <div className="bg-gradient-to-r from-blue-600 to-cyan-600 px-6 py-3">
                <h2 className="text-xl font-semibold">Output Samples</h2>
              </div>
              <div className="p-6">
                {error ? (
                  <div className="bg-red-900/30 border border-red-500 rounded-lg p-4">
                    <div className="flex items-start gap-3">
                      <svg className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
                      </svg>
                      <div className="flex-1">
                        <h3 className="text-red-400 font-semibold mb-1">Error</h3>
                        <pre className="text-red-300 text-sm whitespace-pre-wrap font-mono">{error}</pre>
                      </div>
                    </div>
                  </div>
                ) : samples.length > 0 ? (
                  <div className="space-y-3">
                    <div className="space-y-2 max-h-[580px] overflow-y-auto">
                      {samples.map((sample, i) => (
                        <div
                          key={i}
                          className="bg-slate-900/70 rounded-lg p-4 border border-slate-700 hover:border-slate-600 transition-colors"
                        >
                          <div className="flex items-start gap-3">
                            <span className="text-xs text-purple-400 font-semibold bg-purple-900/30 px-2 py-1 rounded">
                              #{i + 1}
                            </span>
                            <p className="text-gray-100 flex-1">{sample}</p>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ) : (
                  <div className="flex flex-col items-center justify-center h-full min-h-[500px] text-gray-500">
                    <svg className="w-16 h-16 mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                    <p className="text-lg">Output will appear here</p>
                    <p className="text-sm mt-2">Edit the template to generate samples</p>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Footer */}
        <div className="mt-12 text-center text-gray-500 text-sm">
          <p>
            Built with React, TypeScript, and WebAssembly •{' '}
            <a
              href="https://github.com/philpax/perchance-interpreter"
              target="_blank"
              rel="noopener noreferrer"
              className="text-purple-400 hover:text-purple-300 transition-colors"
            >
              View Source
            </a>
          </p>
        </div>
      </div>
    </div>
  );
}

export default App;
