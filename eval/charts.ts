/** Minimal dependency-free SVG charts for the evaluation report. */

export const COLORS = { on: '#2563eb', off: '#f97316', win: '#16a34a', loss: '#dc2626', tie: '#9ca3af', grid: '#e5e7eb', text: '#111827', muted: '#6b7280' };
const PALETTE = ['#2563eb', '#f97316', '#16a34a', '#dc2626', '#9333ea', '#0891b2', '#ca8a04', '#db2777', '#4b5563', '#65a30d', '#7c3aed', '#0d9488', '#b45309', '#be123c'];
const FONT = 'font-family="Segoe UI, Helvetica, Arial, sans-serif"';

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const fmt = (v: number) => (Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 10 ? v.toFixed(1) : v.toFixed(2)).replace(/\.0+$|(\.\d*[1-9])0+$/, '$1');

function frame(width: number, height: number, title: string, body: string, subtitle?: string): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" ${FONT}>
<rect width="100%" height="100%" fill="#ffffff"/>
<text x="16" y="26" font-size="16" font-weight="600" fill="${COLORS.text}">${esc(title)}</text>
${subtitle ? `<text x="16" y="44" font-size="12" fill="${COLORS.muted}">${esc(subtitle)}</text>` : ''}
${body}
</svg>
`;
}

function niceMax(v: number): number {
  if (v <= 0) return 1;
  const p = 10 ** Math.floor(Math.log10(v));
  return [1, 2, 2.5, 5, 10].map((m) => m * p).find((m) => m >= v)!;
}

function legend(items: { name: string; color: string }[], x: number, y: number): string {
  let cx = x;
  return items
    .map((it) => {
      const s = `<rect x="${cx}" y="${y - 10}" width="12" height="12" rx="2" fill="${it.color}"/><text x="${cx + 17}" y="${y}" font-size="12" fill="${COLORS.text}">${esc(it.name)}</text>`;
      cx += 30 + it.name.length * 7;
      return s;
    })
    .join('');
}

/** Horizontal grouped bars: one row per category, one bar per series. */
export function groupedBars(o: { title: string; subtitle?: string; categories: string[]; series: { name: string; values: number[]; color?: string }[]; unit?: string }): string {
  const labelW = 190;
  const barH = 14;
  const gap = 10;
  const rowH = o.series.length * barH + gap;
  const plotW = 520;
  const top = 70;
  const height = top + o.categories.length * rowH + 40;
  const max = niceMax(Math.max(0, ...o.series.flatMap((s) => s.values)));
  const x = (v: number) => labelW + (v / max) * plotW;
  let body = legend(o.series.map((s, i) => ({ name: s.name, color: s.color ?? PALETTE[i] })), labelW, top - 14);
  for (let t = 0; t <= 4; t++) {
    const v = (max * t) / 4;
    body += `<line x1="${x(v)}" y1="${top}" x2="${x(v)}" y2="${height - 34}" stroke="${COLORS.grid}"/><text x="${x(v)}" y="${height - 18}" font-size="11" text-anchor="middle" fill="${COLORS.muted}">${fmt(v)}</text>`;
  }
  o.categories.forEach((c, i) => {
    const y0 = top + i * rowH;
    body += `<text x="${labelW - 8}" y="${y0 + (o.series.length * barH) / 2 + 4}" font-size="12" text-anchor="end" fill="${COLORS.text}">${esc(c)}</text>`;
    o.series.forEach((s, j) => {
      const v = s.values[i] ?? 0;
      const y = y0 + j * barH;
      body += `<rect x="${labelW}" y="${y}" width="${Math.max(0, x(v) - labelW)}" height="${barH - 2}" fill="${s.color ?? PALETTE[j]}"/><text x="${x(v) + 4}" y="${y + barH - 4}" font-size="10" fill="${COLORS.muted}">${fmt(v)}</text>`;
    });
  });
  if (o.unit) body += `<text x="${labelW + plotW / 2}" y="${height - 4}" font-size="11" text-anchor="middle" fill="${COLORS.muted}">${esc(o.unit)}</text>`;
  return frame(labelW + plotW + 60, height, o.title, body, o.subtitle);
}

/** Point estimates with confidence intervals around a zero line (forest plot). */
export function forest(o: { title: string; subtitle?: string; rows: { label: string; value: number; lo: number; hi: number; note?: string }[]; xLabel: string; positiveLabel?: string; negativeLabel?: string }): string {
  const labelW = 190;
  const plotW = 460;
  const noteW = 150;
  const rowH = 26;
  const top = 76;
  const height = top + o.rows.length * rowH + 44;
  const ext = niceMax(Math.max(0.5, ...o.rows.flatMap((r) => [Math.abs(r.lo), Math.abs(r.hi)]).filter(Number.isFinite)));
  const x = (v: number) => labelW + ((Math.max(-ext, Math.min(ext, v)) + ext) / (2 * ext)) * plotW;
  let body = '';
  for (let t = -2; t <= 2; t++) {
    const v = (ext * t) / 2;
    body += `<line x1="${x(v)}" y1="${top - 6}" x2="${x(v)}" y2="${height - 38}" stroke="${t === 0 ? COLORS.muted : COLORS.grid}"${t === 0 ? ' stroke-dasharray="4 3"' : ''}/><text x="${x(v)}" y="${height - 22}" font-size="11" text-anchor="middle" fill="${COLORS.muted}">${fmt(v)}</text>`;
  }
  if (o.negativeLabel) body += `<text x="${labelW}" y="${top - 12}" font-size="11" fill="${COLORS.loss}">${esc(o.negativeLabel)}</text>`;
  if (o.positiveLabel) body += `<text x="${labelW + plotW}" y="${top - 12}" font-size="11" text-anchor="end" fill="${COLORS.win}">${esc(o.positiveLabel)}</text>`;
  o.rows.forEach((r, i) => {
    const y = top + i * rowH + rowH / 2;
    const color = r.lo > 0 ? COLORS.win : r.hi < 0 ? COLORS.loss : COLORS.muted;
    body += `<text x="${labelW - 8}" y="${y + 4}" font-size="12" text-anchor="end" fill="${COLORS.text}">${esc(r.label)}</text>`;
    if (Number.isFinite(r.value)) {
      body += `<line x1="${x(r.lo)}" y1="${y}" x2="${x(r.hi)}" y2="${y}" stroke="${color}" stroke-width="2"/><line x1="${x(r.lo)}" y1="${y - 5}" x2="${x(r.lo)}" y2="${y + 5}" stroke="${color}" stroke-width="2"/><line x1="${x(r.hi)}" y1="${y - 5}" x2="${x(r.hi)}" y2="${y + 5}" stroke="${color}" stroke-width="2"/><circle cx="${x(r.value)}" cy="${y}" r="5" fill="${color}"/>`;
    }
    body += `<text x="${labelW + plotW + 10}" y="${y + 4}" font-size="11" fill="${COLORS.muted}">${esc(r.note ?? (Number.isFinite(r.value) ? `${fmt(r.value)} [${fmt(r.lo)}, ${fmt(r.hi)}]` : 'no data'))}</text>`;
  });
  body += `<text x="${labelW + plotW / 2}" y="${height - 6}" font-size="11" text-anchor="middle" fill="${COLORS.muted}">${esc(o.xLabel)}</text>`;
  return frame(labelW + plotW + noteW + 20, height, o.title, body, o.subtitle);
}

/** Diverging stacked bars: losses left, ties centre, wins right. */
export function preference(o: { title: string; subtitle?: string; rows: { label: string; wins: number; ties: number; losses: number }[] }): string {
  const labelW = 190;
  const plotW = 520;
  const rowH = 24;
  const top = 76;
  const height = top + o.rows.length * rowH + 30;
  const max = Math.max(1, ...o.rows.map((r) => r.wins + r.ties + r.losses));
  const scale = plotW / max;
  let body = legend([{ name: 'feature on better', color: COLORS.win }, { name: 'tie / inconsistent', color: COLORS.tie }, { name: 'feature off better', color: COLORS.loss }], labelW, top - 16);
  o.rows.forEach((r, i) => {
    const y = top + i * rowH;
    let cx = labelW;
    body += `<text x="${labelW - 8}" y="${y + 15}" font-size="12" text-anchor="end" fill="${COLORS.text}">${esc(r.label)}</text>`;
    for (const [v, c] of [[r.wins, COLORS.win], [r.ties, COLORS.tie], [r.losses, COLORS.loss]] as [number, string][]) {
      if (v > 0) body += `<rect x="${cx}" y="${y + 2}" width="${v * scale}" height="${rowH - 6}" fill="${c}"/>${v * scale > 22 ? `<text x="${cx + (v * scale) / 2}" y="${y + 15}" font-size="10" text-anchor="middle" fill="#fff">${v}</text>` : ''}`;
      cx += v * scale;
    }
  });
  return frame(labelW + plotW + 40, height, o.title, body, o.subtitle);
}

/** Matrix of values with a diverging colour scale (positive = green). */
export function heatmap(o: { title: string; subtitle?: string; rows: string[]; cols: string[]; values: (number | null)[][]; format?: (v: number) => string }): string {
  const labelW = 190;
  const cellW = Math.max(64, Math.min(110, 640 / Math.max(1, o.cols.length)));
  const cellH = 26;
  const top = 84;
  const height = top + o.rows.length * cellH + 20;
  const ext = Math.max(1e-9, ...o.values.flat().filter((v): v is number => v !== null).map(Math.abs));
  const color = (v: number) => {
    const a = Math.min(1, Math.abs(v) / ext);
    const [r, g, b] = v >= 0 ? [22, 163, 74] : [220, 38, 38];
    return `rgba(${r},${g},${b},${(0.1 + 0.8 * a).toFixed(2)})`;
  };
  let body = '';
  o.cols.forEach((c, j) => (body += `<text x="${labelW + j * cellW + cellW / 2}" y="${top - 10}" font-size="11" text-anchor="middle" fill="${COLORS.text}">${esc(c)}</text>`));
  o.rows.forEach((r, i) => {
    const y = top + i * cellH;
    body += `<text x="${labelW - 8}" y="${y + 17}" font-size="12" text-anchor="end" fill="${COLORS.text}">${esc(r)}</text>`;
    o.cols.forEach((_, j) => {
      const v = o.values[i][j];
      const x = labelW + j * cellW;
      body += `<rect x="${x + 1}" y="${y + 1}" width="${cellW - 2}" height="${cellH - 2}" fill="${v === null ? '#f9fafb' : color(v)}"/><text x="${x + cellW / 2}" y="${y + 17}" font-size="11" text-anchor="middle" fill="${COLORS.text}">${v === null ? '·' : esc((o.format ?? fmt)(v))}</text>`;
    });
  });
  return frame(labelW + o.cols.length * cellW + 30, height, o.title, body, o.subtitle);
}

/** Horizontal stacked bars (e.g. error penalty by category). */
export function stackedBars(o: { title: string; subtitle?: string; rows: string[]; segments: { name: string; values: number[] }[]; unit?: string }): string {
  const labelW = 210;
  const plotW = 500;
  const rowH = 22;
  const legendRows = Math.ceil(o.segments.length / 5);
  const top = 64 + legendRows * 18;
  const height = top + o.rows.length * rowH + 40;
  const totals = o.rows.map((_, i) => o.segments.reduce((n, s) => n + (s.values[i] ?? 0), 0));
  const max = niceMax(Math.max(0, ...totals));
  let body = '';
  for (let k = 0; k < legendRows; k++) body += legend(o.segments.slice(k * 5, k * 5 + 5).map((s, j) => ({ name: s.name, color: PALETTE[k * 5 + j] })), labelW, 62 + k * 18);
  for (let t = 0; t <= 4; t++) {
    const x = labelW + (plotW * t) / 4;
    body += `<line x1="${x}" y1="${top}" x2="${x}" y2="${height - 34}" stroke="${COLORS.grid}"/><text x="${x}" y="${height - 18}" font-size="11" text-anchor="middle" fill="${COLORS.muted}">${fmt((max * t) / 4)}</text>`;
  }
  o.rows.forEach((r, i) => {
    const y = top + i * rowH;
    let cx = labelW;
    body += `<text x="${labelW - 8}" y="${y + 14}" font-size="12" text-anchor="end" fill="${COLORS.text}">${esc(r)}</text>`;
    o.segments.forEach((s, j) => {
      const w = ((s.values[i] ?? 0) / max) * plotW;
      if (w > 0) body += `<rect x="${cx}" y="${y + 2}" width="${w}" height="${rowH - 6}" fill="${PALETTE[j % PALETTE.length]}"><title>${esc(`${s.name}: ${fmt(s.values[i])}`)}</title></rect>`;
      cx += w;
    });
    body += `<text x="${cx + 4}" y="${y + 14}" font-size="10" fill="${COLORS.muted}">${fmt(totals[i])}</text>`;
  });
  if (o.unit) body += `<text x="${labelW + plotW / 2}" y="${height - 4}" font-size="11" text-anchor="middle" fill="${COLORS.muted}">${esc(o.unit)}</text>`;
  return frame(labelW + plotW + 60, height, o.title, body, o.subtitle);
}
