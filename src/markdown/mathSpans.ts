/**
 * Single-dollar inline math, detected conservatively so prices such as "$5 and $10" stay prose.
 * Pandoc's rules: the opening `$` is followed by a non-space, the closing `$` follows a non-space and is not followed
 * by a digit. In addition the content must look like a formula (LaTeX command, operator, sub/superscript or a variable).
 */

const isEscaped = (s: string, i: number) => {
  let n = 0;
  while (i - 1 - n >= 0 && s[i - 1 - n] === '\\') n++;
  return n % 2 === 1;
};

export function looksLikeMath(content: string): boolean {
  const body = content.replace(/\\(?:text|textrm|mathrm|operatorname|mbox)\{[^{}]*\}/g, ' ');
  // Three or more plain words (not LaTeX commands) mean prose between two dollar amounts.
  if ((body.match(/(?<![\\A-Za-z])[A-Za-z]{2,}/g)?.length ?? 0) >= 3) return false;
  const t = content.trim();
  return (
    /\\[A-Za-z]+|[\^_=<>{}|]/.test(t) ||
    /^[A-Za-z](?:_?\d+)?'*$/.test(t) ||
    /^[A-Za-z0-9.]+\s*[-+*/\u00d7\u00b7]\s*[A-Za-z0-9.(]/.test(t)
  );
}

/** Offsets of `$...$` spans (dollars included) in `text` that are inline math. */
export function findDollarMath(text: string): { start: number; end: number }[] {
  const out: { start: number; end: number }[] = [];
  for (let i = 0; i < text.length; i++) {
    if (text[i] !== '$' || text[i + 1] === '$' || text[i - 1] === '$' || isEscaped(text, i)) continue;
    if (i + 1 >= text.length || /\s/.test(text[i + 1])) continue;
    let j = i + 1;
    while (j < text.length && !(text[j] === '$' && !isEscaped(text, j)) && !(text[j] === '\n' && text[j + 1] === '\n')) j++;
    if (text[j] !== '$' || text[j + 1] === '$') continue;
    if (/\s/.test(text[j - 1]) || /\d/.test(text[j + 1] ?? '') || !looksLikeMath(text.slice(i + 1, j))) continue;
    out.push({ start: i, end: j + 1 });
    i = j;
  }
  return out;
}
