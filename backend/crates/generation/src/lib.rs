//! Coverage-based test generation. See docs/specification/04-test-generation.md.
//!
//! Paths are searched breadth-first over `(state, variable environment)` so that guards are
//! respected; unsatisfiable targets are reported as skipped. Tie-breaking between equally short
//! paths depends only on the seed, making generation deterministic (FR-021).

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use tm_domain::expr::{self, Assign, Env, Expr};
use tm_domain::validation::{has_errors, validate};
use tm_domain::{
    AssignmentTarget, CoverageCriterion, ModelGraph, StateKind, TestCaseData, TestCaseStatus,
    TestStep, ValidationIssue,
};
use uuid::Uuid;

mod simulation;
mod staleness;
pub use simulation::{simulate, SimError, SimStep, SimTransition};
pub use staleness::{check_path, StaleCode, StaleReason};

/// Upper bound on search nodes per query, protecting against state explosion through variables.
const MAX_SEARCH_NODES: usize = 200_000;
/// Upper bound on enumerated paths for `bounded-paths`.
pub const MAX_BOUNDED_PATHS: usize = 1_000;
/// Upper bound on reported skipped paths.
const MAX_SKIPPED_REPORTS: usize = 100;
/// Upper bound on prime paths produced for `prime-paths`.
pub const MAX_PRIME_PATHS: usize = 1_000;
/// Upper bound on simple paths examined while looking for prime paths.
const MAX_SIMPLE_PATHS: usize = 3_000;
/// Upper bound on random walks tried for `random`.
pub const MAX_RANDOM_WALKS: usize = 1_000;
/// Walks made by `random` when no stop condition is given.
pub const DEFAULT_RANDOM_WALKS: usize = 10;
/// Maximum length of a generated test case name.
const MAX_NAME_CHARS: usize = 300;

/// Errors preventing generation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GenerationError {
    /// The model has validation errors.
    #[error("model has validation errors")]
    InvalidModel(Vec<ValidationIssue>),
    /// `bounded-paths` requires `maxPathLength`.
    #[error("maxPathLength is required for bounded-paths")]
    MissingMaxPathLength,
    /// `reach-element` requires `targetId`.
    #[error("targetId is required for reach-element")]
    MissingTarget,
    /// `targetId` is neither a state nor a transition of the model.
    #[error("targetId is not a state or transition of this model")]
    UnknownTarget,
}

/// Generation parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationOptions {
    /// Criterion to satisfy.
    pub criterion: CoverageCriterion,
    /// Seed for tie-breaking.
    pub seed: u64,
    /// Maximum path length (required for bounded-paths, optional bound otherwise).
    pub max_path_length: Option<usize>,
    /// State or transition to reach (`reach-element`).
    pub target: Option<Uuid>,
    /// Stop a `random` run after this many walks.
    pub stop_after_tests: Option<usize>,
    /// Stop a `random` run once this percentage of transitions is covered.
    pub stop_at_coverage: Option<u8>,
}

impl GenerationOptions {
    /// Options for `criterion` with only a seed; the rest at their defaults.
    pub fn new(criterion: CoverageCriterion, seed: u64) -> Self {
        Self {
            criterion,
            seed,
            max_path_length: None,
            target: None,
            stop_after_tests: None,
            stop_at_coverage: None,
        }
    }
}

/// A path from the initial state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedPath {
    /// Transitions taken, in order.
    pub transitions: Vec<Uuid>,
    /// States visited, starting with the initial state (`transitions.len() + 1` entries).
    pub states: Vec<Uuid>,
}

/// A target or path that could not be covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedPath {
    /// Transitions involved.
    pub transition_ids: Vec<Uuid>,
    /// Why it was skipped.
    pub reason: String,
}

/// Covered/total counts with uncovered elements.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoverageReport {
    /// Covered states.
    pub states_covered: usize,
    /// All states.
    pub states_total: usize,
    /// Covered transitions.
    pub transitions_covered: usize,
    /// All transitions.
    pub transitions_total: usize,
    /// States not covered, in model order.
    pub uncovered_state_ids: Vec<Uuid>,
    /// Transitions not covered, in model order.
    pub uncovered_transition_ids: Vec<Uuid>,
}

impl CoverageReport {
    /// Computes coverage of a graph given the covered element ids.
    pub fn compute(
        graph: &ModelGraph,
        covered_states: &HashSet<Uuid>,
        covered_transitions: &HashSet<Uuid>,
    ) -> Self {
        let uncovered_state_ids: Vec<Uuid> = graph
            .states
            .iter()
            .map(|s| s.id)
            .filter(|id| !covered_states.contains(id))
            .collect();
        let uncovered_transition_ids: Vec<Uuid> = graph
            .transitions
            .iter()
            .map(|t| t.id)
            .filter(|id| !covered_transitions.contains(id))
            .collect();
        Self {
            states_covered: graph.states.len() - uncovered_state_ids.len(),
            states_total: graph.states.len(),
            transitions_covered: graph.transitions.len() - uncovered_transition_ids.len(),
            transitions_total: graph.transitions.len(),
            uncovered_state_ids,
            uncovered_transition_ids,
        }
    }

    /// Adds another report's counts and uncovered elements (for aggregation).
    pub fn merge(&mut self, other: CoverageReport) {
        self.states_covered += other.states_covered;
        self.states_total += other.states_total;
        self.transitions_covered += other.transitions_covered;
        self.transitions_total += other.transitions_total;
        self.uncovered_state_ids.extend(other.uncovered_state_ids);
        self.uncovered_transition_ids
            .extend(other.uncovered_transition_ids);
    }
}

/// Result of a generation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationOutput {
    /// Generated paths.
    pub paths: Vec<GeneratedPath>,
    /// Coverage the paths achieve.
    pub coverage: CoverageReport,
    /// Uncoverable targets and infeasible paths.
    pub skipped: Vec<SkippedPath>,
}

/// SplitMix64: tiny, deterministic, dependency-free PRNG for tie-breaking.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

struct CompiledTransition {
    from: usize,
    to: usize,
    guard: Option<Expr>,
    action: Vec<Assign>,
}

struct Machine<'a> {
    graph: &'a ModelGraph,
    transitions: Vec<CompiledTransition>,
    /// Outgoing transition indices per state index, in seeded order.
    out: Vec<Vec<usize>>,
    initial: usize,
    initial_env: Env,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Node {
    state: usize,
    env: Env,
    last: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Target {
    State(usize),
    Transition(usize),
    Pair(usize, usize),
}

enum Step {
    Taken(Env),
    /// Why the transition cannot be taken (shown by simulation).
    Blocked(String),
}

impl<'a> Machine<'a> {
    fn new(graph: &'a ModelGraph, seed: u64) -> Option<Self> {
        let index: HashMap<Uuid, usize> = graph
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id, i))
            .collect();
        let initial_id = graph.initial_state()?.id;
        let mut transitions = Vec::with_capacity(graph.transitions.len());
        for t in &graph.transitions {
            let guard = match t.guard.as_deref().filter(|g| !g.trim().is_empty()) {
                Some(g) => Some(expr::check_guard(g, &graph.variables).ok()?),
                None => None,
            };
            let action = match t.action.as_deref() {
                Some(a) => expr::check_action(a, &graph.variables).ok()?,
                None => Vec::new(),
            };
            transitions.push(CompiledTransition {
                from: *index.get(&t.from)?,
                to: *index.get(&t.to)?,
                guard,
                action,
            });
        }
        let mut out = vec![Vec::new(); graph.states.len()];
        for (i, t) in transitions.iter().enumerate() {
            out[t.from].push(i);
        }
        let mut rng = SplitMix64(seed);
        for edges in &mut out {
            rng.shuffle(edges);
        }
        Some(Self {
            graph,
            transitions,
            out,
            initial: *index.get(&initial_id)?,
            initial_env: expr::initial_env(&graph.variables),
        })
    }

    fn start(&self) -> Node {
        Node {
            state: self.initial,
            env: self.initial_env.clone(),
            last: None,
        }
    }

    fn step(&self, env: &Env, t: usize) -> Step {
        let tr = &self.transitions[t];
        let source = &self.graph.transitions[t];
        if let Some(g) = &tr.guard {
            let text = source.guard.as_deref().unwrap_or_default();
            match expr::eval_guard(g, env) {
                Ok(true) => {}
                Ok(false) => return Step::Blocked(format!("guard `{text}` is false")),
                Err(e) => return Step::Blocked(format!("guard `{text}` cannot be evaluated: {e}")),
            }
        }
        match expr::apply_action(&tr.action, env) {
            Ok(env) => Step::Taken(env),
            Err(e) => Step::Blocked(format!(
                "action `{}` cannot be applied: {e}",
                source.action.as_deref().unwrap_or_default()
            )),
        }
    }

    fn is_terminal(&self, node: &Node) -> bool {
        self.graph.states[node.state].kind == StateKind::Final
            || self.out[node.state]
                .iter()
                .all(|&t| matches!(self.step(&node.env, t), Step::Blocked(_)))
    }

    /// Shortest transition sequence from `start` to a node satisfying `goal`. The goal receives
    /// the previous node's last transition so that transition pairs can be matched on edges.
    fn search(
        &self,
        start: &Node,
        max_depth: usize,
        goal: impl Fn(Option<usize>, &Node) -> bool,
    ) -> Option<(Vec<usize>, Node)> {
        if goal(None, start) {
            return Some((Vec::new(), start.clone()));
        }
        let mut parent: HashMap<Node, (Node, usize)> = HashMap::new();
        let mut seen: HashSet<Node> = HashSet::from([start.clone()]);
        let mut queue = VecDeque::from([(start.clone(), 0usize)]);
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth || seen.len() > MAX_SEARCH_NODES {
                continue;
            }
            for &t in &self.out[node.state] {
                let Step::Taken(env) = self.step(&node.env, t) else {
                    continue;
                };
                let next = Node {
                    state: self.transitions[t].to,
                    env,
                    last: Some(t),
                };
                if goal(node.last, &next) {
                    let mut path = vec![t];
                    let mut cur = node.clone();
                    while let Some((prev, pt)) = parent.get(&cur) {
                        path.push(*pt);
                        cur = prev.clone();
                    }
                    path.reverse();
                    return Some((path, next));
                }
                if seen.insert(next.clone()) {
                    parent.insert(next.clone(), (node.clone(), t));
                    queue.push_back((next, depth + 1));
                }
            }
        }
        None
    }

    fn to_path(&self, transitions: &[usize]) -> GeneratedPath {
        let mut states = vec![self.graph.states[self.initial].id];
        states.extend(
            transitions
                .iter()
                .map(|&t| self.graph.states[self.transitions[t].to].id),
        );
        GeneratedPath {
            transitions: transitions
                .iter()
                .map(|&t| self.graph.transitions[t].id)
                .collect(),
            states,
        }
    }

    fn targets_of(&self, criterion: CoverageCriterion, path: &[usize]) -> BTreeSet<Target> {
        let mut set = BTreeSet::new();
        match criterion {
            CoverageCriterion::State => {
                set.insert(Target::State(self.initial));
                set.extend(path.iter().map(|&t| Target::State(self.transitions[t].to)));
            }
            CoverageCriterion::Transition => {
                set.extend(path.iter().map(|&t| Target::Transition(t)));
            }
            CoverageCriterion::TransitionPair => {
                set.extend(path.windows(2).map(|w| Target::Pair(w[0], w[1])));
            }
            // These do not minimize by element targets.
            CoverageCriterion::BoundedPaths
            | CoverageCriterion::PrimePaths
            | CoverageCriterion::ReachElement
            | CoverageCriterion::Random => {}
        }
        set
    }

    fn all_targets(&self, criterion: CoverageCriterion) -> Vec<Target> {
        match criterion {
            CoverageCriterion::State => (0..self.graph.states.len()).map(Target::State).collect(),
            CoverageCriterion::Transition => (0..self.transitions.len())
                .map(Target::Transition)
                .collect(),
            CoverageCriterion::TransitionPair => {
                let mut v = Vec::new();
                for (a, ta) in self.transitions.iter().enumerate() {
                    for (b, tb) in self.transitions.iter().enumerate() {
                        if ta.to == tb.from {
                            v.push(Target::Pair(a, b));
                        }
                    }
                }
                v
            }
            CoverageCriterion::BoundedPaths
            | CoverageCriterion::PrimePaths
            | CoverageCriterion::ReachElement
            | CoverageCriterion::Random => Vec::new(),
        }
    }

    fn target_reached(&self, target: Target, node: &Node, prev_last: Option<usize>) -> bool {
        match target {
            Target::State(s) => node.state == s,
            Target::Transition(t) => node.last == Some(t),
            Target::Pair(a, b) => prev_last == Some(a) && node.last == Some(b),
        }
    }

    fn target_transition_ids(&self, target: Target) -> Vec<Uuid> {
        let id = |t: usize| self.graph.transitions[t].id;
        match target {
            Target::State(_) => Vec::new(),
            Target::Transition(t) => vec![id(t)],
            Target::Pair(a, b) => vec![id(a), id(b)],
        }
    }

    fn target_label(&self, target: Target) -> String {
        match target {
            Target::State(s) => format!("state '{}'", self.graph.states[s].name),
            Target::Transition(t) => {
                format!("transition '{}'", self.graph.transitions[t].event)
            }
            Target::Pair(a, b) => format!(
                "transition pair '{}' -> '{}'",
                self.graph.transitions[a].event, self.graph.transitions[b].event
            ),
        }
    }

    fn cover_greedily(
        &self,
        criterion: CoverageCriterion,
        max_depth: usize,
    ) -> (Vec<Vec<usize>>, Vec<SkippedPath>) {
        let mut covered: HashSet<Target> = HashSet::new();
        let mut paths: Vec<Vec<usize>> = Vec::new();
        let mut skipped = Vec::new();
        for target in self.all_targets(criterion) {
            if covered.contains(&target) {
                continue;
            }
            match self.path_through(target, max_depth) {
                Ok(path) => {
                    covered.extend(self.targets_of(criterion, &path));
                    paths.push(path);
                }
                Err(skip) => skipped.push(skip),
            }
        }
        (self.minimize(criterion, paths), skipped)
    }

    /// The shortest path that reaches `target`, continued to a final state or dead end.
    fn path_through(&self, target: Target, max_depth: usize) -> Result<Vec<usize>, SkippedPath> {
        let found = self.search(&self.start(), max_depth, |prev, n| {
            self.target_reached(target, n, prev)
        });
        let Some((mut path, end)) = found else {
            return Err(SkippedPath {
                transition_ids: self.target_transition_ids(target),
                reason: format!(
                    "{} is unreachable or infeasible within {max_depth} steps",
                    self.target_label(target)
                ),
            });
        };
        let remaining = max_depth.saturating_sub(path.len());
        if let Some((tail, _)) = self.search(&end, remaining, |_, n| self.is_terminal(n)) {
            path.extend(tail);
        }
        Ok(path)
    }

    /// Drops paths whose targets are all covered by other kept paths.
    fn minimize(&self, criterion: CoverageCriterion, paths: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
        let targets: Vec<BTreeSet<Target>> = paths
            .iter()
            .map(|p| self.targets_of(criterion, p))
            .collect();
        let mut counts: HashMap<Target, usize> = HashMap::new();
        for t in targets.iter().flatten() {
            *counts.entry(*t).or_default() += 1;
        }
        let mut keep = vec![true; paths.len()];
        for (i, own) in targets.iter().enumerate() {
            if own.iter().all(|t| counts.get(t).is_some_and(|&c| c > 1)) {
                keep[i] = false;
                for t in own {
                    if let Some(c) = counts.get_mut(t) {
                        *c -= 1;
                    }
                }
            }
        }
        paths
            .into_iter()
            .zip(keep)
            .filter_map(|(p, k)| k.then_some(p))
            .collect()
    }

    /// Prime paths of the state graph (guards ignored): maximal simple paths, where a cycle
    /// may return to its start. Returns them, longest first, and whether the search was cut off.
    fn prime_paths(&self) -> (Vec<Vec<usize>>, bool) {
        let mut simple: Vec<Vec<usize>> = Vec::new();
        let mut cut = false;
        for start in 0..self.graph.states.len() {
            let mut visited = vec![false; self.graph.states.len()];
            visited[start] = true;
            // Depth-first, in the seeded order of outgoing transitions.
            let mut stack: Vec<(Vec<usize>, usize)> = self.out[start]
                .iter()
                .rev()
                .map(|&t| (vec![t], self.transitions[t].to))
                .collect();
            // `visited` is per path, so rebuild it from the path on each pop.
            while let Some((path, last)) = stack.pop() {
                if simple.len() >= MAX_SIMPLE_PATHS {
                    cut = true;
                    break;
                }
                simple.push(path.clone());
                if last == start {
                    continue; // a cycle ends where it began
                }
                let on_path: HashSet<usize> = std::iter::once(start)
                    .chain(path.iter().map(|&t| self.transitions[t].to))
                    .collect();
                for &t in self.out[last].iter().rev() {
                    let to = self.transitions[t].to;
                    if to == start || !on_path.contains(&to) {
                        let mut next = path.clone();
                        next.push(t);
                        stack.push((next, to));
                    }
                }
            }
            if cut {
                break;
            }
        }
        // A path is prime when no longer simple path contains it; checking against the
        // maximal ones found so far is enough, as containment is transitive.
        let mut order: Vec<usize> = (0..simple.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(simple[i].len()));
        let mut prime: Vec<Vec<usize>> = Vec::new();
        for i in order {
            let p = &simple[i];
            let inside = prime
                .iter()
                .any(|q| q.len() > p.len() && q.windows(p.len()).any(|w| w == p.as_slice()));
            if !inside {
                prime.push(p.clone());
            }
        }
        (prime, cut)
    }

    /// One test path per prime path: the shortest route to its first state, the prime path,
    /// then the shortest route to an end. Prime paths already inside a produced path are not
    /// repeated.
    fn cover_prime_paths(&self, max_depth: usize) -> (Vec<Vec<usize>>, Vec<SkippedPath>) {
        let (prime, cut) = self.prime_paths();
        let mut paths: Vec<Vec<usize>> = Vec::new();
        let mut skipped = Vec::new();
        let total = prime.len();
        for (n, p) in prime.into_iter().enumerate() {
            if n >= MAX_PRIME_PATHS {
                skipped.push(SkippedPath {
                    transition_ids: Vec::new(),
                    reason: format!(
                        "prime path limit of {MAX_PRIME_PATHS} reached; {} more omitted",
                        total - n
                    ),
                });
                break;
            }
            if paths
                .iter()
                .any(|q| q.len() >= p.len() && q.windows(p.len()).any(|w| w == p.as_slice()))
            {
                continue;
            }
            let first_state = self.transitions[p[0]].from;
            let label = || {
                p.iter()
                    .map(|&t| self.graph.transitions[t].event.as_str())
                    .collect::<Vec<_>>()
                    .join("' -> '")
            };
            let ids = || p.iter().map(|&t| self.graph.transitions[t].id).collect();
            let Some((mut path, mut node)) =
                self.search(&self.start(), max_depth, |_, n| n.state == first_state)
            else {
                skipped.push(SkippedPath {
                    transition_ids: ids(),
                    reason: format!(
                        "prime path '{}' is unreachable from the initial state",
                        label()
                    ),
                });
                continue;
            };
            let mut blocked = None;
            for &t in &p {
                match self.step(&node.env, t) {
                    Step::Taken(env) => {
                        node = Node {
                            state: self.transitions[t].to,
                            env,
                            last: Some(t),
                        };
                        path.push(t);
                    }
                    Step::Blocked(why) => {
                        blocked = Some(why);
                        break;
                    }
                }
            }
            if let Some(why) = blocked {
                skipped.push(SkippedPath {
                    transition_ids: ids(),
                    reason: format!(
                        "prime path '{}' is infeasible after the shortest route: {why}",
                        label()
                    ),
                });
                continue;
            }
            let remaining = max_depth.saturating_sub(path.len());
            if let Some((tail, _)) = self.search(&node, remaining, |_, n| self.is_terminal(n)) {
                path.extend(tail);
            }
            paths.push(path);
        }
        if cut {
            skipped.push(SkippedPath {
                transition_ids: Vec::new(),
                reason: format!(
                    "search for prime paths stopped after {MAX_SIMPLE_PATHS} simple paths; the model has too many"
                ),
            });
        }
        (paths, skipped)
    }

    /// Seeded random walks from the initial state, each step chosen by transition weight.
    fn random_walks(
        &self,
        seed: u64,
        max_len: usize,
        stop_after: Option<usize>,
        stop_at_coverage: Option<u8>,
    ) -> (Vec<Vec<usize>>, Vec<SkippedPath>) {
        let mut rng = SplitMix64(seed ^ 0xA076_1D64_78BD_642F);
        let want_tests = match (stop_after, stop_at_coverage) {
            (None, None) => Some(DEFAULT_RANDOM_WALKS),
            (n, _) => n,
        };
        let total = self.transitions.len();
        let mut covered: HashSet<usize> = HashSet::new();
        let mut paths: Vec<Vec<usize>> = Vec::new();
        let mut seen: HashSet<Vec<usize>> = HashSet::new();
        let mut skipped = Vec::new();
        let done = |paths: &Vec<Vec<usize>>, covered: &HashSet<usize>| {
            want_tests.is_some_and(|n| paths.len() >= n)
                || stop_at_coverage
                    .is_some_and(|c| total > 0 && covered.len() * 100 >= usize::from(c) * total)
        };
        for _ in 0..MAX_RANDOM_WALKS {
            if done(&paths, &covered) {
                return (paths, skipped);
            }
            let mut node = self.start();
            let mut path: Vec<usize> = Vec::new();
            while path.len() < max_len && self.graph.states[node.state].kind != StateKind::Final {
                let moves: Vec<(usize, Env, u32)> = self.out[node.state]
                    .iter()
                    .filter_map(|&t| match self.step(&node.env, t) {
                        Step::Taken(env) => {
                            Some((t, env, self.graph.transitions[t].weight.unwrap_or(1)))
                        }
                        Step::Blocked(_) => None,
                    })
                    .collect();
                let sum: u64 = moves.iter().map(|m| u64::from(m.2)).sum();
                if sum == 0 {
                    break;
                }
                let mut pick = rng.next() % sum;
                let (t, env, _) = moves
                    .into_iter()
                    .find(|m| {
                        let w = u64::from(m.2);
                        if pick < w {
                            true
                        } else {
                            pick -= w;
                            false
                        }
                    })
                    .expect("a pick below the sum lands on a move");
                node = Node {
                    state: self.transitions[t].to,
                    env,
                    last: Some(t),
                };
                path.push(t);
            }
            if path.is_empty() {
                skipped.push(SkippedPath {
                    transition_ids: Vec::new(),
                    reason: "no transition can be taken from the initial state (guards or weights)"
                        .into(),
                });
                return (paths, skipped);
            }
            if seen.insert(path.clone()) {
                covered.extend(path.iter().copied());
                paths.push(path);
            }
        }
        if !done(&paths, &covered) {
            skipped.push(SkippedPath {
                transition_ids: Vec::new(),
                reason: format!(
                    "stopped after {MAX_RANDOM_WALKS} walks with {} distinct test case(s) covering {} of {total} transitions",
                    paths.len(),
                    covered.len()
                ),
            });
        }
        (paths, skipped)
    }

    fn enumerate_bounded(&self, max_len: usize) -> (Vec<Vec<usize>>, Vec<SkippedPath>) {
        let mut paths = Vec::new();
        let mut skipped = Vec::new();
        let mut stack = vec![(self.start(), Vec::<usize>::new())];
        let mut truncated = false;
        while let Some((node, path)) = stack.pop() {
            if paths.len() >= MAX_BOUNDED_PATHS {
                truncated = true;
                break;
            }
            if path.len() >= max_len || self.graph.states[node.state].kind == StateKind::Final {
                paths.push(path);
                continue;
            }
            let mut extended = false;
            // Reverse so that the stack yields transitions in seeded order.
            for &t in self.out[node.state].iter().rev() {
                let mut next_path = path.clone();
                next_path.push(t);
                match self.step(&node.env, t) {
                    Step::Taken(env) => {
                        extended = true;
                        let next = Node {
                            state: self.transitions[t].to,
                            env,
                            last: Some(t),
                        };
                        stack.push((next, next_path));
                    }
                    Step::Blocked(_) => {
                        if skipped.len() < MAX_SKIPPED_REPORTS {
                            skipped.push(SkippedPath {
                                transition_ids: next_path
                                    .iter()
                                    .map(|&i| self.graph.transitions[i].id)
                                    .collect(),
                                reason: format!(
                                    "guard of '{}' is unsatisfiable on this path",
                                    self.graph.transitions[t].event
                                ),
                            });
                        }
                    }
                }
            }
            if !extended {
                paths.push(path);
            }
        }
        if truncated {
            skipped.push(SkippedPath {
                transition_ids: Vec::new(),
                reason: format!(
                    "path limit of {MAX_BOUNDED_PATHS} reached; remaining paths omitted"
                ),
            });
        }
        (paths, skipped)
    }
}

/// Generates paths covering the criterion. Deterministic for graph, options and seed (FR-020, FR-021).
pub fn generate(
    graph: &ModelGraph,
    options: GenerationOptions,
) -> Result<GenerationOutput, GenerationError> {
    let issues = validate(graph);
    if has_errors(&issues) {
        return Err(GenerationError::InvalidModel(issues));
    }
    let Some(machine) = Machine::new(graph, options.seed) else {
        return Err(GenerationError::InvalidModel(issues));
    };
    let default_depth = (graph.states.len() + graph.transitions.len()).max(32) * 2;
    let (paths, skipped) = match options.criterion {
        CoverageCriterion::BoundedPaths => {
            let n = options
                .max_path_length
                .ok_or(GenerationError::MissingMaxPathLength)?;
            machine.enumerate_bounded(n)
        }
        CoverageCriterion::PrimePaths => {
            machine.cover_prime_paths(options.max_path_length.unwrap_or(default_depth))
        }
        CoverageCriterion::ReachElement => {
            let id = options.target.ok_or(GenerationError::MissingTarget)?;
            let target = if let Some(i) = graph.states.iter().position(|s| s.id == id) {
                Target::State(i)
            } else if let Some(i) = graph.transitions.iter().position(|t| t.id == id) {
                Target::Transition(i)
            } else {
                return Err(GenerationError::UnknownTarget);
            };
            match machine.path_through(target, options.max_path_length.unwrap_or(default_depth)) {
                Ok(path) => (vec![path], Vec::new()),
                Err(skip) => (Vec::new(), vec![skip]),
            }
        }
        CoverageCriterion::Random => machine.random_walks(
            options.seed,
            options.max_path_length.unwrap_or(default_depth),
            options.stop_after_tests,
            options.stop_at_coverage,
        ),
        criterion => {
            machine.cover_greedily(criterion, options.max_path_length.unwrap_or(default_depth))
        }
    };
    let paths: Vec<GeneratedPath> = paths.iter().map(|p| machine.to_path(p)).collect();
    let covered_states: HashSet<Uuid> = paths.iter().flat_map(|p| p.states.clone()).collect();
    let covered_transitions: HashSet<Uuid> =
        paths.iter().flat_map(|p| p.transitions.clone()).collect();
    Ok(GenerationOutput {
        coverage: CoverageReport::compute(graph, &covered_states, &covered_transitions),
        paths,
        skipped,
    })
}

fn truncate_chars(s: String, max: usize) -> String {
    if s.chars().count() <= max {
        return s;
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Converts a generated path into test case data plus its assignment targets
/// (each with an optional 1-based step order).
pub fn path_to_test_case(
    graph: &ModelGraph,
    model_name: &str,
    criterion: CoverageCriterion,
    index: usize,
    path: &GeneratedPath,
) -> (TestCaseData, Vec<(AssignmentTarget, Option<i32>)>) {
    let state_name = |id: Uuid| graph.state(id).map_or("?", |s| s.name.as_str());
    let route: Vec<&str> = path.states.iter().map(|&id| state_name(id)).collect();
    let name = truncate_chars(
        format!("{model_name} #{}: {}", index + 1, route.join(" → ")),
        MAX_NAME_CHARS,
    );

    let mut preconditions = format!("System is in state '{}'", route.first().unwrap_or(&"?"));
    if !graph.variables.is_empty() {
        let env = expr::initial_env(&graph.variables);
        let vars: Vec<String> = env.iter().map(|(k, v)| format!("{k} = {v}")).collect();
        preconditions.push_str(&format!(" with {}", vars.join(", ")));
    }

    let mut steps = Vec::with_capacity(path.transitions.len());
    let mut targets: Vec<(AssignmentTarget, Option<i32>)> = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |targets: &mut Vec<_>, target: AssignmentTarget, step: Option<i32>| {
        if seen.insert(target) {
            targets.push((target, step));
        }
    };
    if let Some(&first) = path.states.first() {
        push(&mut targets, AssignmentTarget::State(first), None);
    }
    for (i, (&tid, &to)) in path
        .transitions
        .iter()
        .zip(path.states.iter().skip(1))
        .enumerate()
    {
        let order = i as i32 + 1;
        let (action, expected) = match graph.transition(tid) {
            Some(t) => {
                let action = match t.guard.as_deref().filter(|g| !g.trim().is_empty()) {
                    Some(g) => format!("{} [when {g}]", t.event),
                    None => t.event.clone(),
                };
                let expected = t
                    .expected
                    .clone()
                    .filter(|e| !e.trim().is_empty())
                    .unwrap_or_else(|| format!("State '{}' is reached", state_name(to)));
                (action, expected)
            }
            None => (String::from("?"), String::from("?")),
        };
        steps.push(TestStep {
            order,
            action,
            expected,
        });
        // Every step's transition is assigned, even one the path takes again, so
        // the path can be rebuilt from the assignments (stale-test detection).
        targets.push((AssignmentTarget::Transition(tid), Some(order)));
        push(&mut targets, AssignmentTarget::State(to), Some(order));
    }

    let data = TestCaseData {
        name,
        description: Some(format!(
            "Generated from model '{model_name}' for {} coverage.",
            criterion.as_str()
        )),
        preconditions: Some(preconditions),
        priority: None,
        status: TestCaseStatus::Draft,
        tags: vec![criterion.as_str().to_owned()],
        steps,
        implementation_url: None,
        backlog_url: None,
    };
    (data, targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tm_domain::{State, Transition, Variable, VariableType};

    fn state(name: &str, kind: StateKind) -> State {
        State {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            kind,
            position: None,
        }
    }

    fn tr(from: &State, to: &State, event: &str) -> Transition {
        Transition {
            id: Uuid::new_v4(),
            from: from.id,
            to: to.id,
            event: event.into(),
            guard: None,
            action: None,
            expected: None,
            weight: None,
        }
    }

    /// Login model: Start -> LoggedOut <-> LoggedIn -> Done, with a retry counter.
    fn login_model() -> ModelGraph {
        let start = state("Start", StateKind::Initial);
        let out = state("LoggedOut", StateKind::Normal);
        let inn = state("LoggedIn", StateKind::Normal);
        let locked = state("Locked", StateKind::Final);
        let done = state("Done", StateKind::Final);
        let mut fail = tr(&out, &out, "wrong password");
        fail.guard = Some("attempts < 2".into());
        fail.action = Some("attempts = attempts + 1".into());
        let mut lock = tr(&out, &locked, "wrong password");
        lock.guard = Some("attempts >= 2".into());
        ModelGraph {
            variables: vec![Variable {
                name: "attempts".into(),
                var_type: VariableType::Integer,
                initial: None,
            }],
            transitions: vec![
                tr(&start, &out, "open app"),
                tr(&out, &inn, "login"),
                fail,
                lock,
                tr(&inn, &out, "logout"),
                tr(&inn, &done, "close app"),
            ],
            states: vec![start, out, inn, locked, done],
        }
    }

    fn opts(criterion: CoverageCriterion, seed: u64) -> GenerationOptions {
        GenerationOptions::new(criterion, seed)
    }

    #[test]
    fn fr020_transition_coverage_respects_guards() {
        let g = login_model();
        let out = generate(&g, opts(CoverageCriterion::Transition, 1)).unwrap();
        assert_eq!(out.coverage.transitions_covered, g.transitions.len());
        assert_eq!(out.coverage.states_covered, g.states.len());
        assert!(out.skipped.is_empty());
        // Every path starts at the initial state.
        for p in &out.paths {
            assert_eq!(p.states[0], g.states[0].id);
        }
    }

    #[test]
    fn fr020_state_and_pair_coverage() {
        let g = login_model();
        let out = generate(&g, opts(CoverageCriterion::State, 3)).unwrap();
        assert_eq!(out.coverage.states_covered, g.states.len());
        let out = generate(&g, opts(CoverageCriterion::TransitionPair, 3)).unwrap();
        assert_eq!(out.coverage.transitions_covered, g.transitions.len());
    }

    #[test]
    fn infeasible_targets_are_reported() {
        let mut g = login_model();
        g.transitions[3].guard = Some("attempts > 100".into());
        let out = generate(
            &g,
            GenerationOptions {
                max_path_length: Some(10),
                ..opts(CoverageCriterion::Transition, 0)
            },
        )
        .unwrap();
        assert_eq!(out.skipped.len(), 1);
        assert_eq!(out.skipped[0].transition_ids, vec![g.transitions[3].id]);
        assert_eq!(
            out.coverage.uncovered_transition_ids,
            vec![g.transitions[3].id]
        );
    }

    #[test]
    fn bounded_paths_requires_length_and_enumerates() {
        let g = login_model();
        assert_eq!(
            generate(&g, opts(CoverageCriterion::BoundedPaths, 0)),
            Err(GenerationError::MissingMaxPathLength)
        );
        let out = generate(
            &g,
            GenerationOptions {
                max_path_length: Some(3),
                ..opts(CoverageCriterion::BoundedPaths, 0)
            },
        )
        .unwrap();
        assert!(out.paths.iter().all(|p| p.transitions.len() <= 3));
        assert!(out.paths.len() >= 3);
    }

    /// Start -> A <-> B, A -> End: a loop between A and B.
    fn loop_model() -> ModelGraph {
        let start = state("Start", StateKind::Initial);
        let a = state("A", StateKind::Normal);
        let b = state("B", StateKind::Normal);
        let end = state("End", StateKind::Final);
        ModelGraph {
            variables: vec![],
            transitions: vec![
                tr(&start, &a, "begin"),
                tr(&a, &b, "ab"),
                tr(&b, &a, "ba"),
                tr(&a, &end, "finish"),
            ],
            states: vec![start, a, b, end],
        }
    }

    fn events(g: &ModelGraph, p: &GeneratedPath) -> Vec<String> {
        p.transitions
            .iter()
            .map(|id| g.transition(*id).unwrap().event.clone())
            .collect()
    }

    #[test]
    fn prime_paths_cover_the_loop_once_each() {
        let g = loop_model();
        let (prime, cut) = Machine::new(&g, 0).unwrap().prime_paths();
        assert!(!cut);
        // Maximal simple paths: begin-ab, begin-finish... and the two cycles.
        let named: HashSet<String> = prime
            .iter()
            .map(|p| {
                p.iter()
                    .map(|&t| g.transitions[t].event.as_str())
                    .collect::<Vec<_>>()
                    .join(">")
            })
            .collect();
        assert!(named.contains("begin>ab"), "{named:?}");
        assert!(named.contains("begin>finish"), "{named:?}");
        assert!(named.contains("ab>ba"), "{named:?}");
        assert!(named.contains("ba>ab"), "{named:?}");
        assert!(named.contains("ba>finish"), "{named:?}");
        // A prime path is never part of a longer one.
        for p in &prime {
            assert!(!prime
                .iter()
                .any(|q| q.len() > p.len() && q.windows(p.len()).any(|w| w == p.as_slice())));
        }
        let out = generate(&g, opts(CoverageCriterion::PrimePaths, 1)).unwrap();
        assert!(out.skipped.is_empty(), "{:?}", out.skipped);
        assert_eq!(out.coverage.transitions_covered, 4);
        // Every prime path appears inside some generated test path.
        for p in &prime {
            let ids: Vec<Uuid> = p.iter().map(|&t| g.transitions[t].id).collect();
            assert!(
                out.paths.iter().any(|q| q
                    .transitions
                    .windows(ids.len())
                    .any(|w| w == ids.as_slice())),
                "prime path {ids:?} not covered"
            );
        }
        // Same seed, same result.
        assert_eq!(
            out,
            generate(&g, opts(CoverageCriterion::PrimePaths, 1)).unwrap()
        );
    }

    #[test]
    fn a_prime_path_with_an_unsatisfiable_guard_is_skipped_and_reported() {
        let mut g = loop_model();
        g.transitions[1].guard = Some("flag".into());
        g.variables = vec![Variable {
            name: "flag".into(),
            var_type: VariableType::Boolean,
            initial: None,
        }];
        let out = generate(&g, opts(CoverageCriterion::PrimePaths, 0)).unwrap();
        assert!(
            out.skipped.iter().any(|s| s.reason.contains("infeasible")),
            "{:?}",
            out.skipped
        );
        assert!(!out.paths.is_empty());
    }

    #[test]
    fn reach_element_finds_the_shortest_path_to_a_state_or_transition() {
        let g = loop_model();
        let mut o = opts(CoverageCriterion::ReachElement, 0);
        assert_eq!(generate(&g, o), Err(GenerationError::MissingTarget));
        o.target = Some(Uuid::new_v4());
        assert_eq!(generate(&g, o), Err(GenerationError::UnknownTarget));
        // The state B: Start -> A -> B.
        o.target = Some(g.states[2].id);
        let out = generate(&g, o).unwrap();
        assert_eq!(out.paths.len(), 1);
        assert_eq!(&events(&g, &out.paths[0])[..2], ["begin", "ab"]);
        // The transition `finish`.
        o.target = Some(g.transitions[3].id);
        let out = generate(&g, o).unwrap();
        assert_eq!(events(&g, &out.paths[0]), ["begin", "finish"]);
    }

    #[test]
    fn reach_element_reports_an_unreachable_target() {
        let mut g = loop_model();
        g.transitions[1].guard = Some("flag".into());
        g.variables = vec![Variable {
            name: "flag".into(),
            var_type: VariableType::Boolean,
            initial: None,
        }];
        let mut o = opts(CoverageCriterion::ReachElement, 0);
        o.target = Some(g.states[2].id);
        let out = generate(&g, o).unwrap();
        assert!(out.paths.is_empty());
        assert_eq!(out.skipped.len(), 1);
    }

    #[test]
    fn random_walks_are_deterministic_and_stop_by_count() {
        let g = loop_model();
        let mut o = opts(CoverageCriterion::Random, 42);
        o.stop_after_tests = Some(4);
        let a = generate(&g, o).unwrap();
        assert_eq!(a, generate(&g, o).unwrap());
        assert!(a.paths.len() <= 4 && !a.paths.is_empty());
        // Each walk starts at the initial state and ends in a final state or at the length cap.
        for p in &a.paths {
            assert_eq!(p.states[0], g.states[0].id);
        }
        // Without a stop condition: the default number of walks, or all distinct ones.
        let d = generate(&g, opts(CoverageCriterion::Random, 1)).unwrap();
        assert!(d.paths.len() <= DEFAULT_RANDOM_WALKS);
    }

    #[test]
    fn random_walks_follow_the_weights() {
        let mut g = loop_model();
        // From A: never loop back (ab weight 0), always finish.
        g.transitions[1].weight = Some(0);
        let mut o = opts(CoverageCriterion::Random, 7);
        o.stop_after_tests = Some(5);
        let out = generate(&g, o).unwrap();
        assert_eq!(
            out.paths.len(),
            1,
            "only one distinct walk exists: {:?}",
            out.paths
        );
        assert_eq!(events(&g, &out.paths[0]), ["begin", "finish"]);
        // Mostly finish: a heavy `finish` shows up in the first walk.
        let mut g = loop_model();
        g.transitions[3].weight = Some(1000);
        let out = generate(&g, o).unwrap();
        assert!(events(&g, &out.paths[0]).contains(&"finish".to_string()));
    }

    #[test]
    fn random_walks_stop_at_a_coverage_target() {
        let g = loop_model();
        let mut o = opts(CoverageCriterion::Random, 3);
        o.stop_at_coverage = Some(100);
        let out = generate(&g, o).unwrap();
        assert_eq!(out.coverage.transitions_covered, 4, "{:?}", out.skipped);
        // Reaching it stops the run early, rather than making 1 000 walks.
        assert!(out.paths.len() < 50);
    }

    #[test]
    fn random_walks_say_when_no_transition_can_be_taken() {
        let mut g = loop_model();
        g.transitions[0].weight = Some(0);
        let out = generate(&g, opts(CoverageCriterion::Random, 0)).unwrap();
        assert!(out.paths.is_empty());
        assert_eq!(out.skipped.len(), 1);
    }

    #[test]
    fn invalid_model_is_rejected() {
        let g = ModelGraph {
            states: vec![state("A", StateKind::Normal)],
            ..Default::default()
        };
        assert!(matches!(
            generate(&g, opts(CoverageCriterion::State, 0)),
            Err(GenerationError::InvalidModel(_))
        ));
    }

    #[test]
    fn converts_paths_to_test_cases_with_assignments() {
        let g = login_model();
        let out = generate(&g, opts(CoverageCriterion::Transition, 1)).unwrap();
        let path = &out.paths[0];
        let (tc, targets) = path_to_test_case(&g, "Login", CoverageCriterion::Transition, 0, path);
        assert_eq!(tc.steps.len(), path.transitions.len());
        assert!(tc.name.starts_with("Login #1: Start"));
        assert_eq!(targets[0], (AssignmentTarget::State(g.states[0].id), None));
        assert!(targets.contains(&(AssignmentTarget::Transition(path.transitions[0]), Some(1))));
    }

    #[test]
    fn a_repeated_transition_is_assigned_at_every_step() {
        let g = login_model();
        let fail = g.transitions[2].id;
        let path = GeneratedPath {
            transitions: vec![g.transitions[0].id, fail, fail],
            states: vec![
                g.states[0].id,
                g.states[1].id,
                g.states[1].id,
                g.states[1].id,
            ],
        };
        let (_, targets) = path_to_test_case(&g, "Login", CoverageCriterion::Transition, 0, &path);
        assert!(targets.contains(&(AssignmentTarget::Transition(fail), Some(2))));
        assert!(targets.contains(&(AssignmentTarget::Transition(fail), Some(3))));
    }

    #[test]
    fn nfr001_large_model_is_fast() {
        let mut states = vec![state("S0", StateKind::Initial)];
        for i in 1..200 {
            let kind = if i == 199 {
                StateKind::Final
            } else {
                StateKind::Normal
            };
            states.push(state(&format!("S{i}"), kind));
        }
        let mut transitions = Vec::new();
        for i in 0..199 {
            transitions.push(tr(&states[i], &states[i + 1], "next"));
            transitions.push(tr(&states[i + 1], &states[i / 2], "back"));
        }
        let g = ModelGraph {
            states,
            transitions,
            variables: vec![],
        };
        let started = std::time::Instant::now();
        let out = generate(&g, opts(CoverageCriterion::Transition, 7)).unwrap();
        assert_eq!(out.coverage.transitions_covered, g.transitions.len());
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    proptest! {
        /// FR-021: same model, criterion and seed yield identical output.
        #[test]
        fn fr021_generation_is_deterministic(seed in any::<u64>(), crit in 0usize..3) {
            let g = login_model();
            let criterion = [CoverageCriterion::State, CoverageCriterion::Transition, CoverageCriterion::TransitionPair][crit];
            let a = generate(&g, opts(criterion, seed)).unwrap();
            let b = generate(&g, opts(criterion, seed)).unwrap();
            prop_assert_eq!(a, b);
        }

        /// Transition coverage is complete on fully feasible random graphs.
        #[test]
        fn transition_coverage_is_complete(n in 2usize..12, edges in proptest::collection::vec((0usize..12, 0usize..12), 1..30), seed in any::<u64>()) {
            let mut states: Vec<State> = (0..n).map(|i| state(&format!("S{i}"), StateKind::Normal)).collect();
            states[0].kind = StateKind::Initial;
            let mut transitions: Vec<Transition> = (1..n).map(|i| tr(&states[i - 1], &states[i], "chain")).collect();
            transitions.extend(edges.iter().map(|&(a, b)| tr(&states[a % n], &states[b % n], "edge")));
            let g = ModelGraph { states, transitions, variables: vec![] };
            let out = generate(&g, opts(CoverageCriterion::Transition, seed)).unwrap();
            prop_assert_eq!(out.coverage.transitions_covered, g.transitions.len());
            prop_assert!(out.skipped.is_empty());
        }
    }
}
