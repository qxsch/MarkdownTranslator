import { describe, expect, it } from 'vitest';
import { binomialP, bootstrapReduction, cohenKappa, fleissKappa, holm, majority, wilson } from '../eval/stats.js';
import { parseJsonAnswer } from '../eval/judges.js';

describe('evaluation statistics', () => {
  it('sign test', () => {
    expect(binomialP(0, 0)).toBe(1);
    expect(binomialP(6, 6)).toBeCloseTo(0.03125, 5);
    expect(binomialP(185, 313)).toBeLessThan(0.002);
    expect(binomialP(28, 56)).toBe(1);
    expect(binomialP(900, 1000)).toBeLessThan(1e-100);
  });

  it('Wilson interval', () => {
    const [lo, hi] = wilson(76, 76);
    expect(hi).toBeCloseTo(1, 9);
    expect(lo).toBeGreaterThan(0.95);
    const [a, b] = wilson(50, 100);
    expect(a).toBeCloseTo(0.4038, 3);
    expect(b).toBeCloseTo(0.5962, 3);
  });

  it('Holm correction is monotone and bounded', () => {
    expect(holm([0.01, 0.04, 0.03, 0.5])).toEqual([0.04, 0.09, 0.09, 0.5]);
  });

  it('paired bootstrap brackets the estimate and is reproducible', () => {
    const items = Array.from({ length: 50 }, (_, i) => ({ words: 20, on: i % 3, off: (i % 3) + 1 }));
    const r = bootstrapReduction(items);
    expect(r.estimate).toBeCloseTo(5, 5);
    expect(r.lo).toBeLessThanOrEqual(r.estimate);
    expect(r.hi).toBeGreaterThanOrEqual(r.estimate);
    expect(bootstrapReduction(items)).toEqual(r);
  });

  it('agreement statistics', () => {
    expect(cohenKappa([['on', 'on'], ['off', 'off'], ['tie', 'tie']])).toBe(1);
    expect(cohenKappa([['on', 'off'], ['off', 'on']])).toBeLessThan(0);
    expect(fleissKappa([['on', 'on', 'on'], ['off', 'off', 'off']])).toBe(1);
    expect(majority(['on', 'on', 'off'])).toBe('on');
    expect(majority(['on', 'off', 'tie'])).toBe('tie');
  });

  it('parses judge answers with fences or prose around the JSON', () => {
    expect(parseJsonAnswer('```json\n{"results": []}\n```')).toEqual({ results: [] });
    expect(parseJsonAnswer('Here you go: {"a": {"b": 1}} done')).toEqual({ a: { b: 1 } });
    expect(() => parseJsonAnswer('no json')).toThrow();
  });
});
