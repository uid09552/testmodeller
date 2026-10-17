import { control, test, threat } from '../testing/fixtures';
import { idsIn, inferKind } from './ai-intent';

describe('reading a chat message', () => {
  it.each([
    ['what could go wrong with the payment API?', 'tm-threats'],
    ['suggest controls for THR-4', 'tm-controls'],
    ['how could we mitigate this', 'tm-controls'],
    ['rate THR-2', 'tm-ratings'],
    ['what are likelihood and impact here?', 'tm-ratings'],
    ['write security tests for the login flow', 'tm-security-tests'],
    ['what is missing in this model?', 'tm-review'],
    ['review the model for gaps', 'tm-review'],
    ['we have a React SPA, an API gateway and a Postgres database', 'tm-architecture'],
    ['add a message broker component', 'tm-architecture'],
    ['hello', 'tm-threats'],
  ])('"%s" asks for %s', (text, kind) => {
    expect(inferKind(text)).toBe(kind);
  });

  it('finds the threats, controls and tests a message names, ignoring leading zeros and case', () => {
    const t = [threat('t4', 4), threat('t5', 5)];
    const c = [control('c1', 1)];
    const s = [test('s2', 2)];
    expect(idsIn('suggest controls for THR-0004 and thr-5', t, c, s)).toEqual(['t4', 't5']);
    expect(idsIn('verify CTL-1 with SEC-2', t, c, s)).toEqual(['c1', 's2']);
    expect(idsIn('THR-4 THR-4', t, c, s)).toEqual(['t4']);
    expect(idsIn('THR-99 is not there', t, c, s)).toEqual([]);
    expect(idsIn('nothing', t, c, s)).toEqual([]);
  });
});
