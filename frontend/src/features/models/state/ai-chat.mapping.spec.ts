import {
  inferKind, kindLabel, testCaseToDraft, toCanvasKind, toGraphDraft,
} from './ai-chat.mapping';

describe('inferKind (FR-030 to FR-032)', () => {
  it('asks for a scenario description when the message is about scenarios', () => {
    expect(inferKind('Write a scenario description for login')).toBe('feature-description');
    expect(inferKind('what are the acceptance criteria?')).toBe('feature-description');
  });

  it('asks for a whole model when the message says so', () => {
    expect(inferKind('build the entire model for checkout')).toBe('model');
  });

  it('asks for states and transitions when the message is about the graph', () => {
    expect(inferKind('which transitions are missing?')).toBe('states-and-transitions');
    expect(inferKind('suggest the next steps in this flow')).toBe('states-and-transitions');
  });

  it('defaults to test cases, which is what the editor is for', () => {
    expect(inferKind('cover the empty password')).toBe('test-cases');
    expect(inferKind('propose negative test cases')).toBe('test-cases');
  });

  it('labels every kind', () => {
    expect(kindLabel('test-cases')).toBe('Test cases');
    expect(kindLabel('feature-description')).toBe('Scenario description');
    expect(kindLabel('states-and-transitions')).toBe('States & transitions');
    expect(kindLabel('model')).toBe('Whole model');
  });
});

describe('testCaseToDraft', () => {
  it('maps preconditions to Given and steps to When/Then', () => {
    const draft = testCaseToDraft({
      name: 'Reject an empty password',
      preconditions: 'the login page is open',
      steps: [
        { action: 'leave the password blank', expected: 'the submit button stays disabled' },
        { action: 'submit anyway', expected: 'an error is shown' },
      ],
    });
    expect(draft.given).toBe('the login page is open');
    expect(draft.when).toBe('leave the password blank\nsubmit anyway');
    expect(draft.then).toBe('the submit button stays disabled\nan error is shown');
  });

  it('reads the category and polarity from the tags', () => {
    const draft = testCaseToDraft({
      name: 'x', steps: [], tags: ['Integration', 'negative'],
    });
    expect(draft.category).toBe('integration');
    expect(draft.polarity).toBe('negative');
  });

  it('falls back to a feature-level positive case when the tags say nothing', () => {
    const draft = testCaseToDraft({ name: 'x', steps: [] });
    expect(draft.category).toBe('feature');
    expect(draft.polarity).toBe('positive');
  });

  it('never produces a nameless test case', () => {
    expect(testCaseToDraft({ name: '   ', steps: [] }).name).toBe('Proposed test case');
  });
});

describe('toGraphDraft (FR-031)', () => {
  it('maps API state kinds onto the canvas kinds', () => {
    // The canvas has `decision`, which the contract cannot express.
    expect(toCanvasKind('initial')).toBe('initial');
    expect(toCanvasKind('final')).toBe('final');
    expect(toCanvasKind('normal')).toBe('regular');
  });

  it('resolves transitions that reference states by id', () => {
    const draft = toGraphDraft({
      states: [
        { id: 'a', name: 'Start', kind: 'initial' },
        { id: 'b', name: 'Done', kind: 'final' },
      ],
      transitions: [{ from: 'a', to: 'b', event: 'submit' }],
    });
    expect(draft.transitions).toEqual([
      { from: 'a', to: 'b', event: 'submit', guard: undefined, action: undefined },
    ]);
  });

  it('resolves transitions that reference states by name', () => {
    const draft = toGraphDraft({
      states: [
        { name: 'Start', kind: 'initial' },
        { name: 'Done', kind: 'final' },
      ],
      transitions: [{ from: 'Start', to: 'Done', event: 'submit' }],
    });
    expect(draft.transitions.length).toBe(1);
    expect(draft.transitions[0].from).toBe(draft.states[0].key);
    expect(draft.transitions[0].to).toBe(draft.states[1].key);
  });

  it('drops a transition pointing at a state that is not in the payload', () => {
    // Otherwise accepting it would create a dangling edge on the canvas.
    const draft = toGraphDraft({
      states: [{ id: 'a', name: 'Start', kind: 'initial' }],
      transitions: [{ from: 'a', to: 'nowhere', event: 'submit' }],
    });
    expect(draft.transitions).toEqual([]);
  });

  it('names an unnamed state rather than rendering a blank one', () => {
    const draft = toGraphDraft({
      states: [{ name: '', kind: 'normal' }],
      transitions: [],
    });
    expect(draft.states[0].name).toBe('State 1');
  });
});
