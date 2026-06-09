# Project Timeline

We want to maintain a consistent steady development timeline which allows us to keep development fun, rewarding and production.

The proposed timeline is as follows:

## Phase 1 - Project Setup

Focused initialization of documentation, best practices, contribution, and so
forth.

## Phase 2 - Stateful Graph Engine

### Description

- Define `State` and `StateDelta`
- Define `Node` trait
- Define graph
  - Fixed edges
  - Conditional routing
  - What do `START` and `END` look like
  - Runtime loop

### Acceptance Criteria

Working stateful graph implementation with a few examples of linear, branching,
and looping graphs with some logic fleshed out.

## Phase 3 - Asynchronous Graph Engine

### Description

- Convert `Node` trait to async
- Figure out async further
- Add `tokio` runtime
- Add concurrent runnable-node set
- Think about pregel-esque supersteps, execution limits, cancellation
- Tracing and tests

### Acceptance Criteria

Same as phase 2 but async. Able to run nodes concurrently.

Phase 3 and 4 have overlap due to reducers being a common theme.

## Phase 4 - Merging

### Description

- Implementation of a merge trait that determines how we manage conflicts of
state.

### Acceptance Criteria

User should be able to define reducers.

## Phase 5 - Model Integration

### Description

- Define `Model` trait independent of provider
- Provider adapter
