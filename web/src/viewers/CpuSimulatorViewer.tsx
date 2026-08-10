import { useState } from 'react';
import { useArtifact, LoadingOrError } from '../useArtifact';
import { fetchTrace, CpuExport } from '../data/artifacts';

const REG_NAMES = ['r0', 'r1', 'r2', 'r3', 'r4', 'r5'];

function Simulator({ data }: { data: CpuExport }) {
  const [cycle, setCycle] = useState(0);
  const row = data.trace[cycle];
  const prevRow = cycle > 0 ? data.trace[cycle - 1] : null;
  const atEnd = cycle >= data.trace.length - 1;
  const atStart = cycle === 0;

  return (
    <>
      <p className="subtitle">
        3^4 mod {data.field_modulus} via repeated multiplication — step through the actual CPU
        simulator from Phase 2, one instruction at a time.
      </p>

      <div className="controls">
        <button onClick={() => setCycle(0)} disabled={atStart}>
          ⏮ reset
        </button>
        <button onClick={() => setCycle((c) => Math.max(0, c - 1))} disabled={atStart}>
          ◀ step back
        </button>
        <button onClick={() => setCycle((c) => Math.min(data.trace.length - 1, c + 1))} disabled={atEnd}>
          step forward ▶
        </button>
        <button onClick={() => setCycle(data.trace.length - 1)} disabled={atEnd}>
          run to end ⏭
        </button>
        <span className="cycle-counter">
          cycle {cycle} / {data.trace.length - 1}
        </span>
      </div>

      <div className="register-grid">
        {REG_NAMES.map((name, i) => (
          <div
            key={name}
            className={`register-cell ${prevRow && prevRow.registers[i] !== row.registers[i] ? 'changed' : ''}`}
          >
            <span className="label">{name}</span>
            <span className="value">{row.registers[i]}</span>
          </div>
        ))}
      </div>

      <div className="two-col">
        <div>
          <p className="hint" style={{ marginBottom: 6 }}>
            program (pc = {row.pc})
          </p>
          <div className="program-list">
            {data.program.map((instr, i) => (
              <div key={i} className={`line ${i === row.pc ? 'current' : ''}`}>
                {i === row.pc ? '→ ' : '\u00A0\u00A0'}
                {String(i).padStart(2, '0')}: {instr.text}
              </div>
            ))}
          </div>
        </div>
        <div>
          <p className="hint" style={{ marginBottom: 6 }}>
            this cycle
          </p>
          <table>
            <tbody>
              <tr>
                <td>instruction</td>
                <td className="mono">{data.program[row.pc]?.text ?? '(halted)'}</td>
              </tr>
              <tr>
                <td>memory read</td>
                <td className="mono">{row.mem_read ? `mem[${row.mem_read[0]}] = ${row.mem_read[1]}` : '—'}</td>
              </tr>
              <tr>
                <td>memory write</td>
                <td className="mono">{row.mem_write ? `mem[${row.mem_write[0]}] = ${row.mem_write[1]}` : '—'}</td>
              </tr>
            </tbody>
          </table>
          {atEnd && (
            <p className="hint" style={{ marginTop: 12 }}>
              halted. final r0 = <strong className="mono">{data.final_registers[0]}</strong>
            </p>
          )}
        </div>
      </div>
    </>
  );
}

export default function CpuSimulatorViewer() {
  const state = useArtifact(fetchTrace);
  return (
    <section className="panel">
      <h2>CPU simulator</h2>
      {state.status === 'ready' ? <Simulator data={state.data} /> : <LoadingOrError state={state} />}
    </section>
  );
}
