import { useMemo } from 'react';

interface AnsiToken {
  text: string;
  bold?: boolean;
  fgColor?: string;
  bgColor?: string;
}

// ANSI color codes to CSS colors
const ANSI_COLORS: Record<number, string> = {
  30: '#000000', // Black
  31: '#ef4444', // Red
  32: '#22c55e', // Green
  33: '#eab308', // Yellow
  34: '#3b82f6', // Blue
  35: '#a855f7', // Magenta
  36: '#06b6d4', // Cyan
  37: '#f3f4f6', // White
  90: '#6b7280', // Bright Black (Gray)
  91: '#f87171', // Bright Red
  92: '#86efac', // Bright Green
  93: '#fde047', // Bright Yellow
  94: '#93c5fd', // Bright Blue
  95: '#d8b4fe', // Bright Magenta
  96: '#67e8f9', // Bright Cyan
  97: '#ffffff', // Bright White
};

const ANSI_BG_COLORS: Record<number, string> = {
  40: '#000000',
  41: '#7f1d1d',
  42: '#14532d',
  43: '#713f12',
  44: '#1e3a8a',
  45: '#581c87',
  46: '#164e63',
  47: '#1f2937',
  100: '#374151',
  101: '#991b1b',
  102: '#166534',
  103: '#854d0e',
  104: '#1e40af',
  105: '#6b21a8',
  106: '#155e75',
  107: '#4b5563',
};

function parseAnsi(text: string): AnsiToken[] {
  const tokens: AnsiToken[] = [];
  // Match ANSI escape sequences
  const ansiRegex = /\x1b\[([0-9;]+)m/g;

  let currentBold = false;
  let currentFgColor: string | undefined;
  let currentBgColor: string | undefined;

  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = ansiRegex.exec(text)) !== null) {
    // Add text before this escape code
    if (match.index > lastIndex) {
      const textContent = text.substring(lastIndex, match.index);
      if (textContent) {
        tokens.push({
          text: textContent,
          bold: currentBold,
          fgColor: currentFgColor,
          bgColor: currentBgColor,
        });
      }
    }

    // Parse the escape code
    const codes = match[1].split(';').map(Number);
    for (const code of codes) {
      if (code === 0) {
        // Reset all
        currentBold = false;
        currentFgColor = undefined;
        currentBgColor = undefined;
      } else if (code === 1) {
        currentBold = true;
      } else if (code === 22) {
        currentBold = false;
      } else if (code >= 30 && code <= 37) {
        currentFgColor = ANSI_COLORS[code];
      } else if (code >= 90 && code <= 97) {
        currentFgColor = ANSI_COLORS[code];
      } else if (code >= 40 && code <= 47) {
        currentBgColor = ANSI_BG_COLORS[code];
      } else if (code >= 100 && code <= 107) {
        currentBgColor = ANSI_BG_COLORS[code];
      } else if (code === 38) {
        // Extended foreground color (not fully implemented)
        currentFgColor = '#ffffff';
      } else if (code === 48) {
        // Extended background color (not fully implemented)
        currentBgColor = '#000000';
      }
    }

    lastIndex = match.index + match[0].length;
  }

  // Add remaining text
  if (lastIndex < text.length) {
    const textContent = text.substring(lastIndex);
    if (textContent) {
      tokens.push({
        text: textContent,
        bold: currentBold,
        fgColor: currentFgColor,
        bgColor: currentBgColor,
      });
    }
  }

  return tokens;
}

interface AnsiRendererProps {
  text: string;
  className?: string;
}

export default function AnsiRenderer({ text, className = '' }: AnsiRendererProps) {
  const tokens = useMemo(() => parseAnsi(text), [text]);

  return (
    <pre className={className}>
      {tokens.map((token, index) => {
        const style: React.CSSProperties = {};
        if (token.fgColor) style.color = token.fgColor;
        if (token.bgColor) style.backgroundColor = token.bgColor;
        if (token.bold) style.fontWeight = 'bold';

        return (
          <span key={index} style={style}>
            {token.text}
          </span>
        );
      })}
    </pre>
  );
}
