import { useSyncExternalStore } from "react";

type Listener = () => void;
type SetState<State> = (update: (state: State) => State) => void;

interface StoreHook<State> {
  <Selected>(selector: (state: State) => Selected): Selected;
  getState(): State;
  subscribe(listener: Listener): () => void;
}

/**
 * A minimal external store bound to React through `useSyncExternalStore`. The
 * returned value is both a selector hook and a plain store handle, so listeners
 * outside the React tree (the drag-and-drop subscription) can push state in
 * without re-rendering themselves.
 */
export function createStore<State>(
  initialize: (setState: SetState<State>) => State,
): StoreHook<State> {
  const listeners = new Set<Listener>();
  let state: State;

  const setState: SetState<State> = (update) => {
    const nextState = update(state);
    if (Object.is(nextState, state)) {
      return;
    }

    state = nextState;
    listeners.forEach((listener) => listener());
  };

  state = initialize(setState);

  const subscribe = (listener: Listener) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  };

  const useStore = <Selected>(selector: (currentState: State) => Selected) =>
    useSyncExternalStore(
      subscribe,
      () => selector(state),
      () => selector(state),
    );

  useStore.getState = () => state;
  useStore.subscribe = subscribe;

  return useStore;
}
