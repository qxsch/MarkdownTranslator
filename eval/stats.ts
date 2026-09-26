/** Statistics used by the evaluation report. Pure functions, covered by test/eval-stats.test.ts. */

/** Two-sided exact sign test (binomial, p = 0.5). */
export function binomialP(k: number, n: number): number {
  if (n === 0) return 1;
  const lo = Math.min(k, n - k);
  let logC = 0;
  let p = 0;
  for (let i = 0; i <= lo; i++) {
    if (i > 0) logC += Math.log(n - i + 1) - Math.log(i);
    p += Math.exp(logC - n * Math.LN2);
  }
  return Math.min(1, 2 * p);
}

/** Wilson score interval for a proportion k / n. */
export function wilson(k: number, n: number, z = 1.96): [number, number] {
  if (n === 0) return [0, 1];
  const p = k / n;
  const d = 1 + (z * z) / n;
  const c = (p + (z * z) / (2 * n)) / d;
  const h = (z * Math.sqrt((p * (1 - p)) / n + (z * z) / (4 * n * n))) / d;
  return [Math.max(0, c - h), Math.min(1, c + h)];
}

/** Holm-Bonferroni adjusted p-values (same order as the input). */
export function holm(ps: number[]): number[] {
  const order = ps.map((p, i) => [p, i] as const).sort((a, b) => a[0] - b[0]);
  const out = new Array<number>(ps.length);
  let running = 0;
  order.forEach(([p, i], rank) => {
    running = Math.max(running, Math.min(1, p * (ps.length - rank)));
    out[i] = running;
  });
  return out;
}

export function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function quantile(sorted: number[], q: number): number {
  if (!sorted.length) return NaN;
  const pos = (sorted.length - 1) * q;
  const lo = Math.floor(pos);
  const hi = Math.ceil(pos);
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo);
}

/**
 * Paired bootstrap of the MQM reduction per 100 source words, (sum(off) - sum(on)) / words * 100.
 * Positive values mean the feature lowers the error score. Items are resampled with replacement.
 */
export function bootstrapReduction(items: { words: number; on: number; off: number }[], iterations = 2000, seed = 7): { estimate: number; lo: number; hi: number } {
  const stat = (xs: typeof items) => {
    const w = xs.reduce((n, x) => n + x.words, 0);
    return w ? ((xs.reduce((n, x) => n + x.off, 0) - xs.reduce((n, x) => n + x.on, 0)) / w) * 100 : 0;
  };
  if (!items.length) return { estimate: NaN, lo: NaN, hi: NaN };
  const rnd = mulberry32(seed);
  const samples: number[] = [];
  for (let b = 0; b < iterations; b++) {
    const draw = Array.from({ length: items.length }, () => items[Math.floor(rnd() * items.length)]);
    samples.push(stat(draw));
  }
  samples.sort((a, b) => a - b);
  return { estimate: stat(items), lo: quantile(samples, 0.025), hi: quantile(samples, 0.975) };
}

/** Cohen's kappa for two raters over the same items. */
export function cohenKappa(pairs: [string, string][]): number {
  if (!pairs.length) return NaN;
  const cats = [...new Set(pairs.flat())];
  const n = pairs.length;
  const po = pairs.filter(([a, b]) => a === b).length / n;
  const pe = cats.reduce((s, c) => s + (pairs.filter(([a]) => a === c).length / n) * (pairs.filter(([, b]) => b === c).length / n), 0);
  return pe === 1 ? 1 : (po - pe) / (1 - pe);
}

/** Fleiss' kappa for items that were each rated by the same number of raters. */
export function fleissKappa(ratings: string[][]): number {
  const items = ratings.filter((r) => r.length >= 2);
  if (!items.length) return NaN;
  const m = items[0].length;
  const usable = items.filter((r) => r.length === m);
  const cats = [...new Set(usable.flat())];
  const n = usable.length;
  const pj = cats.map((c) => usable.reduce((s, r) => s + r.filter((x) => x === c).length, 0) / (n * m));
  const pi = usable.map((r) => (cats.reduce((s, c) => s + r.filter((x) => x === c).length ** 2, 0) - m) / (m * (m - 1)));
  const pBar = pi.reduce((a, b) => a + b, 0) / n;
  const peBar = pj.reduce((s, p) => s + p * p, 0);
  return peBar === 1 ? 1 : (pBar - peBar) / (1 - peBar);
}

/** Majority vote over raters; returns "tie" without a strict majority between the two sides. */
export function majority(votes: string[]): 'on' | 'off' | 'tie' {
  const on = votes.filter((v) => v === 'on').length;
  const off = votes.filter((v) => v === 'off').length;
  return on > off ? 'on' : off > on ? 'off' : 'tie';
}
