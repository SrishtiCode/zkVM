import { useArtifact, LoadingOrError } from '../useArtifact';
import { fetchPolynomial, PolynomialExport } from '../data/artifacts';

function BarChart({ values, mod }: { values: number[]; mod: number }) {
  const max = Math.max(...values, 1);
  return (
    <div className="bars">
      {values.map((v, i) => (
        <div key={i} className="bar" style={{ height: `${(v / max) * 100}%` }} title={`x=${i}: ${v} (mod ${mod})`} />
      ))}
    </div>
  );
}

function PolyDetail({ data }: { data: PolynomialExport }) {
  return (
    <>
      <p className="subtitle">
        The trace a_i = 3 + 2i (mod {data.field_modulus}), interpolated to {data.trace_len} coefficients,
        then low-degree-extended {data.blowup_factor}x onto a coset — the same poly::interpolate +
        poly::lde pipeline from Phase 0/1.
      </p>

      <p className="hint" style={{ marginBottom: 6 }}>
        trace values (the {data.trace_len} points being interpolated)
      </p>
      <BarChart values={data.trace_values} mod={data.field_modulus} />

      <p className="hint" style={{ margin: '18px 0 6px' }}>
        interpolated coefficients ({data.coefficients.length} of them). Note these are generically
        all nonzero, even though the trace is a simple arithmetic progression step-by-step — the
        interpolation domain is the 8th roots of unity, not x=0,1,2,..., so "linear in step index"
        doesn't mean "linear in x". It's the later transition-constraint <em>quotient</em> (after
        dividing by the zerofier, not shown here — see the FRI tab) that becomes genuinely
        low-degree; the raw trace polynomial itself doesn't need to be.
      </p>
      <div className="dots">
        {data.coefficients.map((c, i) => (
          <span key={i} className="dot" title={`x^${i} coefficient`}>
            {c}
          </span>
        ))}
      </div>

      <p className="hint" style={{ margin: '18px 0 6px' }}>
        low-degree extension: {data.lde_evaluations.length} evaluations on the coset {data.domain_offset}·H
        (offset {data.domain_offset}, blowup {data.blowup_factor}x)
      </p>
      <div className="scroll-table">
        <table>
          <thead>
            <tr>
              <th>k</th>
              <th>domain point x</th>
              <th>p(x)</th>
            </tr>
          </thead>
          <tbody>
            {data.lde_evaluations.map((v, i) => (
              <tr key={i}>
                <td>{i}</td>
                <td className="mono">{data.lde_domain[i]}</td>
                <td className="mono">{v}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

export default function PolynomialViewer() {
  const state = useArtifact(fetchPolynomial);
  return (
    <section className="panel">
      <h2>Polynomial viewer</h2>
      {state.status === 'ready' ? <PolyDetail data={state.data} /> : <LoadingOrError state={state} />}
    </section>
  );
}
