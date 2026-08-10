import { useArtifact, LoadingOrError } from '../useArtifact';
import { fetchProof, ProofExport, QueryJson } from '../data/artifacts';

function MerklePathView({ query }: { query: QueryJson }) {
  return (
    <div className="merkle-path">
      <div className="step">
        <span className="node target">
          leaf[{query.trace_path.leaf_index}] = {query.trace_value}
        </span>
      </div>
      {query.trace_path.siblings.map((s, i) => (
        <div className="step" key={i}>
          <span className="hint">↑ hash with sibling</span>
          <span className="node">{s}</span>
        </div>
      ))}
      <div className="step">
        <span className="hint">↑ =</span>
        <span className="node target">root</span>
      </div>
    </div>
  );
}

function ProofDetail({ data }: { data: ProofExport }) {
  return (
    <>
      <p className="subtitle">{data.statement}</p>

      <table style={{ marginBottom: 16 }}>
        <tbody>
          <tr>
            <td>public claim</td>
            <td className="mono">
              a_0={data.start}, a_last={data.end}
            </td>
          </tr>
          <tr>
            <td>trace Merkle root</td>
            <td className="mono">{data.trace_root}</td>
          </tr>
          <tr>
            <td>FRI layer roots</td>
            <td className="mono">{data.fri_layer_roots.join(', ')}</td>
          </tr>
          <tr>
            <td>FRI final polynomial</td>
            <td className="mono">[{data.fri_final_poly_coeffs.join(', ')}]</td>
          </tr>
          <tr>
            <td>verifier result</td>
            <td>
              <span className={`pill ${data.accepted ? 'ok' : 'bad'}`}>
                {data.accepted ? 'ACCEPTED' : 'REJECTED'}
              </span>
            </td>
          </tr>
        </tbody>
      </table>

      <p className="hint" style={{ marginBottom: 8 }}>
        {data.num_queries} random query openings — each one a Merkle authentication path from an
        opened trace value up to the committed root.
      </p>
      {data.queries.map((q) => (
        <div key={q.index} className="query-card">
          <div className="hint" style={{ marginBottom: 6 }}>
            query at index {q.index}
          </div>
          <MerklePathView query={q} />
        </div>
      ))}
    </>
  );
}

export default function ProofViewer() {
  const state = useArtifact(fetchProof);
  return (
    <section className="panel">
      <h2>Proof explorer</h2>
      {state.status === 'ready' ? <ProofDetail data={state.data} /> : <LoadingOrError state={state} />}
    </section>
  );
}
