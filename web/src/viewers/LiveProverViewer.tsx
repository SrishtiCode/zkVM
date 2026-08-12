import { useState } from 'react';
import { provePower, MAX_EXPONENT, PowerProofResult } from '../wasmProver';

type Status =
  | { kind: 'idle' }
  | { kind: 'proving' }
  | { kind: 'done'; result: PowerProofResult; base: bigint; exponent: bigint }
  | { kind: 'error'; message: string };

const inputStyle = {
  width: 60,
  background: 'var(--panel-2)',
  color: 'var(--text)',
  border: '1px solid var(--border)',
  borderRadius: 6,
  padding: '5px 8px',
};

export default function LiveProverViewer() {
  const [base, setBase] = useState('3');
  const [exponent, setExponent] = useState('4');
  const [status, setStatus] = useState<Status>({ kind: 'idle' });

  const exponentNum = Number(exponent);
  const invalidInput =
    !/^\d+$/.test(base) || !/^\d+$/.test(exponent) || exponentNum > MAX_EXPONENT || exponentNum < 0;

  async function run() {
    setStatus({ kind: 'proving' });
    try {
      const b = BigInt(base);
      const e = BigInt(exponent);
      const result = await provePower(b, e);
      setStatus({ kind: 'done', result, base: b, exponent: e });
    } catch (err) {
      setStatus({ kind: 'error', message: err instanceof Error ? err.message : String(err) });
    }
  }

  return (
    <section className="panel">
      <h2>Live prover</h2>
      <p className="subtitle">
        Phase 9: the real prover and verifier, compiled to WebAssembly, running the same
        base^exponent CPU program as the CPU simulator tab — right here in your browser, no server
        round trip. This computes a genuine STARK: interpolation, low-degree extension, Merkle
        commitments, FRI folding, Fiat-Shamir — over a 128-cycle trace, so it takes several seconds.
      </p>

      <div className="controls" style={{ alignItems: 'center' }}>
        <label className="hint">
          base: <input value={base} onChange={(e) => setBase(e.target.value)} style={inputStyle} />
        </label>
        <label className="hint">
          exponent (0–{MAX_EXPONENT}):{' '}
          <input value={exponent} onChange={(e) => setExponent(e.target.value)} style={inputStyle} />
        </label>
        <button onClick={run} disabled={invalidInput || status.kind === 'proving'}>
          {status.kind === 'proving' ? 'proving…' : 'prove it'}
        </button>
      </div>

      {invalidInput && (
        <p className="hint" style={{ color: 'var(--warn)' }}>
          base and exponent must be non-negative integers; exponent must be ≤ {MAX_EXPONENT} (fixed
          128-cycle trace length for this demo).
        </p>
      )}

      {status.kind === 'proving' && (
        <p className="hint">
          Proving <code>{base}^{exponent}</code> in your browser — running the real Rust
          prover/verifier compiled to WASM, this genuinely takes a few seconds...
        </p>
      )}

      {status.kind === 'error' && (
        <div className="error-box">
          <strong>Couldn't run the live prover.</strong>
          <p>{status.message}</p>
        </div>
      )}

      {status.kind === 'done' && (
        <table style={{ marginTop: 12 }}>
          <tbody>
            <tr>
              <td>statement</td>
              <td className="mono">
                {status.base.toString()}^{status.exponent.toString()} mod p
              </td>
            </tr>
            <tr>
              <td>computed result</td>
              <td className="mono">{status.result.resultValue.toString()}</td>
            </tr>
            <tr>
              <td>trace length</td>
              <td className="mono">{status.result.traceLen}</td>
            </tr>
            <tr>
              <td>query count</td>
              <td className="mono">{status.result.numQueries}</td>
            </tr>
            <tr>
              <td>trace Merkle root (low 64 bits)</td>
              <td className="mono">{status.result.traceRootLow.toString()}</td>
            </tr>
            <tr>
              <td>verifier result</td>
              <td>
                <span className={`pill ${status.result.accepted ? 'ok' : 'bad'}`}>
                  {status.result.accepted ? 'ACCEPTED' : 'REJECTED'}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      )}
    </section>
  );
}
