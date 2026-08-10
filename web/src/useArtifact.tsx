import { useEffect, useState } from 'react';

type AsyncState<T> =
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | { status: 'ready'; data: T };

/** Fetches an artifact once on mount, exposing loading/error/ready states
 * so every viewer can show the same simple "loading… / error / content"
 * shape without repeating the boilerplate. */
export function useArtifact<T>(fetcher: () => Promise<T>): AsyncState<T> {
  const [state, setState] = useState<AsyncState<T>>({ status: 'loading' });

  useEffect(() => {
    let cancelled = false;
    setState({ status: 'loading' });
    fetcher()
      .then((data) => {
        if (!cancelled) setState({ status: 'ready', data });
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setState({ status: 'error', message: err instanceof Error ? err.message : String(err) });
        }
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return state;
}

export function LoadingOrError({ state }: { state: { status: 'loading' } | { status: 'error'; message: string } }) {
  if (state.status === 'loading') {
    return <p className="hint">loading…</p>;
  }
  return (
    <div className="error-box">
      <strong>Couldn't load this artifact.</strong>
      <p>{state.message}</p>
    </div>
  );
}
