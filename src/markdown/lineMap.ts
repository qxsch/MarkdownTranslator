/**
 * Maps offsets inside a node's `value` (container prefixes and fence indentation removed)
 * back to offsets in the full source. Returns null when a line cannot be matched.
 */
export function buildLineMap(src: string, firstSourceLineStart: number, value: string): ((v: number) => number) | null {
  const deltas: { vStart: number; delta: number }[] = [];
  let srcPos = firstSourceLineStart;
  let vPos = 0;
  const lines = value.split('\n');
  for (let j = 0; j < lines.length; j++) {
    const vLine = lines[j].replace(/\r$/, '');
    let srcEnd = src.indexOf('\n', srcPos);
    if (srcEnd === -1) srcEnd = src.length;
    const srcLine = src.slice(srcPos, srcEnd).replace(/\r$/, '');
    if (!srcLine.endsWith(vLine)) return null;
    deltas.push({ vStart: vPos, delta: srcPos + srcLine.length - vLine.length - vPos });
    vPos += lines[j].length + 1;
    srcPos = srcEnd + 1;
  }
  return (v: number) => {
    let d = deltas[0].delta;
    for (const x of deltas) {
      if (x.vStart <= v) d = x.delta;
      else break;
    }
    return v + d;
  };
}

export function lineStartOf(src: string, offset: number): number {
  return src.lastIndexOf('\n', offset - 1) + 1;
}

/** Longest line (in characters, without line terminators) covering [from, to). */
export function maxLineWidth(src: string, from: number, to: number): number {
  const start = lineStartOf(src, from);
  let end = src.indexOf('\n', to);
  if (end === -1) end = src.length;
  return Math.max(...src.slice(start, end).split('\n').map((l) => l.replace(/\r$/, '').length));
}
