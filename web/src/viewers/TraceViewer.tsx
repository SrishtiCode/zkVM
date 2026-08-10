import { useArtifact, LoadingOrError } from '../useArtifact';
import { fetchTrace, CpuExport } from '../data/artifacts';

function TraceTable({ data }: { data: CpuExport }) {
  return (
    <>
      <p className="subtitle">
        Every cycle of the same run, all at once — the table `air::build_rows` turns into columns
        for interpolation. {data.trace.length} rows.
      </p>
      <div className="scroll-table">
        <table>
          <thead>
            <tr>
              <th>cycle</th>
              <th>pc</th>
              <th>instruction</th>
              <th>r0</th>
              <th>r1</th>
              <th>r2</th>
              <th>r3</th>
              <th>r4</th>
              <th>r5</th>
              <th>mem read</th>
              <th>mem write</th>
            </tr>
          </thead>
          <tbody>
            {data.trace.map((row) => (
              <tr key={row.cycle}>
                <td>{row.cycle}</td>
                <td>{row.pc}</td>
                <td className="mono" style={{ textAlign: 'left' }}>
                  {data.program[row.pc]?.text ?? 'halt'}
                </td>
                {row.registers.map((v, i) => (
                  <td key={i} className="mono">
                    {v}
                  </td>
                ))}
                <td className="mono">{row.mem_read ? `[${row.mem_read[0]}]=${row.mem_read[1]}` : '—'}</td>
                <td className="mono">{row.mem_write ? `[${row.mem_write[0]}]=${row.mem_write[1]}` : '—'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

export default function TraceViewer() {
  const state = useArtifact(fetchTrace);
  return (
    <section className="panel">
      <h2>Execution trace</h2>
      {state.status === 'ready' ? <TraceTable data={state.data} /> : <LoadingOrError state={state} />}
    </section>
  );
}
