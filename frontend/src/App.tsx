import { useState, useEffect, useCallback } from 'react';
import init, { evaluate_perchance, evaluate_multiple } from './wasm/perchance_wasm';

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
  const [output, setOutput] = useState<string>('');
  const [error, setError] = useState<string | null>(null);
  const [autoEval, setAutoEval] = useState(true);
  const [seed, setSeed] = useState<string>('42');
  const [sampleCount, setSampleCount] = useState<string>('5');
  const [samples, setSamples] = useState<string[]>([]);
  const [showSamples, setShowSamples] = useState(false);

  // Initialize WASM
  useEffect(() => {
    init().then(() => {
      setWasmReady(true);
    });
  }, []);

  // Evaluate template
  const evaluate = useCallback(async (code: string, evalSeed?: number) => {
    if (!wasmReady) return;

    try {
      const seedValue = BigInt(evalSeed ?? (parseInt(seed) || 42));
      const result = await evaluate_perchance(code, seedValue);
      setOutput(result);
      setError(null);
    } catch (e) {
      setError(String(e));
      setOutput('');
    }
  }, [wasmReady, seed]);

  // Auto-evaluate on template change
  useEffect(() => {
    if (autoEval && wasmReady) {
      const timer = setTimeout(() => {
        evaluate(template);
      }, 300); // Debounce
      return () => clearTimeout(timer);
    }
  }, [template, autoEval, wasmReady, evaluate]);

  // Generate multiple samples
  const generateSamples = async () => {
    if (!wasmReady) return;

    try {
      const count = parseInt(sampleCount) || 5;
      const results = await evaluate_multiple(template, count, undefined);
      setSamples(results as string[]);
      setShowSamples(true);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900 text-white">
      <div className="container mx-auto px-4 py-8">
        {/* Header */}
        <div className="mb-8 text-center">
          <h1 className="text-5xl font-bold bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent mb-2">
            Perchance Interpreter
          </h1>
          <p className="text-gray-400 text-lg">
            A deterministic random text generator with live preview
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
              <div className="p-6">
                <textarea
                  value={template}
                  onChange={(e) => setTemplate(e.target.value)}
                  className="w-full h-[500px] bg-slate-900 text-gray-100 font-mono text-sm p-4 rounded-lg border border-slate-600 focus:border-purple-500 focus:ring-2 focus:ring-purple-500/20 focus:outline-none transition-all resize-none"
                  placeholder="Enter your Perchance template here..."
                  spellCheck={false}
                />

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
                    </div>

                    <button
                      onClick={() => evaluate(template)}
                      className="px-4 py-2 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 rounded-lg font-medium text-sm transition-all shadow-lg hover:shadow-purple-500/50"
                    >
                      Run
                    </button>
                  </div>

                  <div className="flex items-center gap-4 flex-wrap">
                    <div className="flex items-center gap-2">
                      <label className="text-sm text-gray-300">Samples:</label>
                      <input
                        type="number"
                        value={sampleCount}
                        onChange={(e) => setSampleCount(e.target.value)}
                        min="1"
                        max="50"
                        className="w-20 px-3 py-1 bg-slate-700 border border-slate-600 rounded text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500/20 focus:outline-none"
                      />
                    </div>

                    <button
                      onClick={generateSamples}
                      className="px-4 py-2 bg-gradient-to-r from-blue-600 to-cyan-600 hover:from-blue-700 hover:to-cyan-700 rounded-lg font-medium text-sm transition-all shadow-lg hover:shadow-blue-500/50"
                    >
                      Generate Multiple
                    </button>
                  </div>
                </div>
              </div>
            </div>

            {/* Preview Panel */}
            <div className="bg-slate-800/50 backdrop-blur rounded-lg shadow-2xl border border-slate-700/50 overflow-hidden">
              <div className="bg-gradient-to-r from-blue-600 to-cyan-600 px-6 py-3">
                <h2 className="text-xl font-semibold">
                  {showSamples ? 'Multiple Samples' : 'Output Preview'}
                </h2>
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
                ) : showSamples ? (
                  <div className="space-y-3">
                    <div className="flex justify-between items-center mb-4">
                      <p className="text-gray-400 text-sm">
                        Generated {samples.length} samples
                      </p>
                      <button
                        onClick={() => setShowSamples(false)}
                        className="text-sm text-gray-400 hover:text-white transition-colors"
                      >
                        ← Back to single output
                      </button>
                    </div>
                    <div className="space-y-2 max-h-[500px] overflow-y-auto">
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
                  <div className="bg-slate-900/70 rounded-lg p-6 min-h-[500px] border border-slate-700">
                    {output ? (
                      <div>
                        <div className="flex items-center gap-2 mb-3 pb-3 border-b border-slate-700">
                          <svg className="w-5 h-5 text-green-400" fill="currentColor" viewBox="0 0 20 20">
                            <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                          </svg>
                          <span className="text-green-400 font-semibold text-sm">Success</span>
                        </div>
                        <p className="text-gray-100 text-lg leading-relaxed">{output}</p>
                      </div>
                    ) : (
                      <div className="flex flex-col items-center justify-center h-full text-gray-500">
                        <svg className="w-16 h-16 mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                        </svg>
                        <p className="text-lg">Output will appear here</p>
                        <p className="text-sm mt-2">Edit the template or click Run to generate</p>
                      </div>
                    )}
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
