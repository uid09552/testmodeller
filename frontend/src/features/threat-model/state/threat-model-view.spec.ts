import { flow, test as secTest, threat } from '../testing/fixtures';
import { component } from '../testing/fixtures';
import {
  csv, freeSpot, highestLevel, isTab, label, layerOf, levelText, regionAt, riskRegisterCsv,
  securityTestsCsv, securityTestsGherkin, targetName,
} from './threat-model-view';

describe('threat-model-view', () => {
  it('labels enum values readably', () => {
    expect(label('information-disclosure')).toBe('Information disclosure');
    expect(label('api')).toBe('REST/GraphQL API');
    expect(label('cicd-pipeline')).toBe('CI/CD pipeline');
    expect(label(null)).toBe('');
  });

  it('knows the tabs and the layers', () => {
    expect(isTab('threats')).toBe(true);
    expect(isTab('nope')).toBe(false);
    expect(layerOf('relational-database')).toBe('data');
    expect(layerOf('human-user')).toBe('external');
  });

  it('names levels with a symbol and a word, and finds the highest', () => {
    expect(levelText('high')).toBe('▆ High');
    expect(levelText(null)).toBe('Not assessed');
    expect(highestLevel(['moderate', null, 'high'])).toBe('high');
    expect(highestLevel([null])).toBeNull();
  });

  it('names flow targets by their components', () => {
    const cs = [component('a', 'Web app'), component('b', 'Customer API')];
    const fs = [flow('f', 'a', 'b')];
    expect(targetName({ kind: 'flow', id: 'f' }, cs, fs)).toBe('Web app → Customer API');
    expect(targetName({ kind: 'component', id: 'gone' }, cs, fs)).toBe('(deleted)');
  });

  it('quotes CSV cells that need it', () => {
    expect(csv([['a,b', 'say "hi"', null]])).toBe('"a,b","say ""hi""",\r\n');
  });

  it('exports only assessed threats in the risk register', () => {
    const out = riskRegisterCsv([
      threat('t1', 1, { inherent: { likelihood: 4, impact: 5 }, inherentScore: 20, inherentLevel: 'critical' }),
      threat('t2', 2),
    ]);
    const lines = out.trim().split('\r\n');
    expect(lines).toHaveLength(2);
    expect(lines[1]).toContain('THR-1');
    expect(lines[1]).toContain('critical');
  });

  it('exports security tests as CSV with steps joined', () => {
    const t = secTest('s1', 1, {
      threatIds: ['t1'], steps: [{ action: 'Call', expected: 'Rejected' }, { action: 'Retry', expected: 'Rejected' }],
    });
    const out = securityTestsCsv([t], [threat('t1', 1)]);
    expect(out).toContain('SEC-1');
    expect(out).toContain('THR-1');
    expect(out).toContain('1. Call => Rejected | 2. Retry => Rejected');
  });

  it('exports Gherkin tagged with the test and its threats', () => {
    const t = secTest('s1', 1, {
      title: 'Access another customer\'s record', threatIds: ['t1'],
      preconditions: ['Two customers exist', 'User 1 is logged in'],
      steps: [{ action: 'GET /customers/2', expected: 'Access is denied' }],
    });
    const out = securityTestsGherkin([t], [threat('t1', 1)], 'Shop');
    expect(out).toContain('Feature: Shop');
    expect(out).toContain('  @sec-s1 @THR-1');
    expect(out).toContain('  Scenario: Access another customer\'s record');
    expect(out).toContain('    Given Two customers exist');
    expect(out).toContain('    And User 1 is logged in');
    expect(out).toContain('    When GET /customers/2');
    expect(out).toContain('    Then Access is denied');
  });

  it('finds the smallest region at a point', () => {
    const regions = new Map([
      ['big', { x: 0, y: 0, w: 500, h: 500 }],
      ['small', { x: 100, y: 100, w: 100, h: 100 }],
    ]);
    expect(regionAt({ x: 150, y: 150 }, regions)).toBe('small');
    expect(regionAt({ x: 400, y: 400 }, regions)).toBe('big');
    expect(regionAt({ x: 900, y: 0 }, regions)).toBeNull();
  });

  it('finds a free spot that overlaps nothing', () => {
    const taken = [{ x: 40, y: 40, w: 150, h: 64 }];
    const p = freeSpot(taken, 150, 64);
    const overlaps = p.x < 40 + 150 + 16 && 40 < p.x + 150 + 16 && p.y < 40 + 64 + 16 && 40 < p.y + 64 + 16;
    expect(overlaps).toBe(false);
  });
});
