import { useArtifact, LoadingOrError } from '../useArtifact';
import { fetchFri, FriExport, FriRoundExport } from '../data/artifacts';

function Round({ round }: { round: FriRoundExport }) {
  return (
    <div className="fri-round">
      <div className="head">
        <strong>round {round.round}</strong>
        <span className="size">domain size {round.domain_size}</span>
      </div>
      <div className="dots" style={{ marginBottom: 8 }}>
        {round.evaluations.map((v, i) => (
          <span key={i} className="dot" title={`x=${round.domain[i]}: ${v}`}>
            {v}
          </span>
        ))}
      </div>
      <table style={{ fontSize: 12 }}>
        <tbody>
          <tr>
            <td>Merkle root</td>
            <td className="mono">{round.merkle_root ?? '—'}</td>
          </tr>
          {round.beta !== null && (
            <tr>
              <td>fold challenge β</td>
              <td className="mono">{round.beta}</td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

function FriDetail({ data }: { data: FriExport }) {
  return (
    <>
      <p className="subtitle">
        Folding the composition polynomial in half each round — p'(y) = p_even(y) + β·p_odd(y) —
        until it's small enough to send outright. Each round is committed with a Merkle root before
        the next round's β is revealed.
      </p>
      {data.rounds.map((r) => (
        <Round key={r.round} round={r} />
      ))}
      <div className="fri-round" style={{ borderColor: 'var(--accent)' }}>
        <div className="head">
          <strong>final polynomial</strong>
          <span className="size">{data.final_poly_coeffs.length} coefficient(s)</span>
        </div>
        <div className="dots">
          {data.final_poly_coeffs.map((c, i) => (
            <span key={i} className="dot" style={{ borderColor: 'var(--accent)', color: 'var(--accent)' }}>
              {c}
            </span>
          ))}
        </div>
        <p className="hint" style={{ marginTop: 8, marginBottom: 0 }}>
          Sent directly — fewer coefficients than the final layer's points, so this check has real
          redundancy (see toystark/fri's docs on why zero redundancy here is a soundness bug).
        </p>
      </div>
    </>
  );
}

export default function FriViewer() {
  const state = useArtifact(fetchFri);
  return (
    <section className="panel">
      <h2>FRI visualizer</h2>
      {state.status === 'ready' ? <FriDetail data={state.data} /> : <LoadingOrError state={state} />}
    </section>
  );
}
