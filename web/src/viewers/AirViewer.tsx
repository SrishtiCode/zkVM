import { useState } from 'react';
import { useArtifact, LoadingOrError } from '../useArtifact';
import { fetchAir, AirRowJson } from '../data/artifacts';

const REG_NAMES = ['r0', 'r1', 'r2', 'r3', 'r4', 'r5'];
const OPCODES = ['loadimm', 'load', 'store', 'add', 'mul', 'jmp', 'jnz', 'halt'];

function OneHot({ label, bits }: { label: string; bits: number[] }) {
  return (
    <div style={{ marginBottom: 8 }}>
      <span className="hint">{label}: </span>
      <div className="dots" style={{ display: 'inline-flex', marginLeft: 6 }}>
        {bits.map((b, i) => (
          <span
            key={i}
            className="dot"
            style={b === 1 ? { borderColor: 'var(--accent)', color: 'var(--accent)' } : {}}
          >
            {i}
          </span>
        ))}
      </div>
    </div>
  );
}

function AirRowDetail({ row }: { row: AirRowJson }) {
  const failing = row.transition_checks.filter(([, v]) => v !== 0);

  return (
    <div className="two-col">
      <div>
        <table>
          <tbody>
            <tr>
              <td>pc</td>
              <td className="mono">{row.pc}</td>
            </tr>
            <tr>
              <td>opcode</td>
              <td>
                <span className="pill opcode">{row.opcode_name}</span>
              </td>
            </tr>
            <tr>
              <td>addr / target</td>
              <td className="mono">{row.addr}</td>
            </tr>
            <tr>
              <td>imm</td>
              <td className="mono">{row.imm}</td>
            </tr>
            <tr>
              <td>mem read val</td>
              <td className="mono">{row.mem_read_value}</td>
            </tr>
            <tr>
              <td>mem write val</td>
              <td className="mono">{row.mem_write_value}</td>
            </tr>
            <tr>
              <td>jnz_is_zero</td>
              <td className="mono">{row.jnz_is_zero}</td>
            </tr>
            <tr>
              <td>jnz_inv</td>
              <td className="mono">{row.jnz_inv}</td>
            </tr>
          </tbody>
        </table>
        <div style={{ marginTop: 14 }}>
          <OneHot label="write target" bits={row.is_write_r} />
          <OneHot label="read a" bits={row.is_read_a_r} />
          <OneHot label="read b" bits={row.is_read_b_r} />
        </div>
      </div>
      <div>
        <p className="hint" style={{ marginBottom: 6 }}>
          registers
        </p>
        <table style={{ marginBottom: 14 }}>
          <thead>
            <tr>
              {REG_NAMES.map((n) => (
                <th key={n}>{n}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            <tr>
              {row.registers.map((v, i) => (
                <td key={i} className="mono">
                  {v}
                </td>
              ))}
            </tr>
          </tbody>
        </table>
        <p className="hint" style={{ marginBottom: 6 }}>
          transition constraints ({row.transition_checks.length} checked this row)
        </p>
        {failing.length === 0 ? (
          <span className="pill ok">all {row.transition_checks.length} hold</span>
        ) : (
          <>
            <span className="pill bad">{failing.length} failing</span>
            <ul style={{ fontSize: 12, marginTop: 8 }}>
              {failing.map(([name, val], i) => (
                <li key={i} className="mono">
                  {name}: {val}
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
    </div>
  );
}

function AirBrowser({ rows }: { rows: AirRowJson[] }) {
  const [cycle, setCycle] = useState(0);
  const row = rows[cycle];

  return (
    <>
      <p className="subtitle">
        Same run as the execution trace, but through the AIR's eyes: one-hot opcode/register
        selectors and every named transition constraint (Phase 3), evaluated for this row.
      </p>
      <div className="controls">
        <button onClick={() => setCycle((c) => Math.max(0, c - 1))} disabled={cycle === 0}>
          ◀
        </button>
        <span className="cycle-counter">
          row {cycle} / {rows.length - 1}
        </span>
        <button
          onClick={() => setCycle((c) => Math.min(rows.length - 1, c + 1))}
          disabled={cycle === rows.length - 1}
        >
          ▶
        </button>
        <select
          value={cycle}
          onChange={(e) => setCycle(Number(e.target.value))}
          style={{
            background: 'var(--panel-2)',
            color: 'var(--text)',
            border: '1px solid var(--border)',
            borderRadius: 6,
            padding: '5px 8px',
          }}
        >
          {rows.map((r, i) => (
            <option key={i} value={i}>
              row {i} — {OPCODES[r.sel.findIndex((s) => s === 1)] ?? '?'}
            </option>
          ))}
        </select>
      </div>
      <AirRowDetail row={row} />
    </>
  );
}

export default function AirViewer() {
  const state = useArtifact(fetchAir);
  return (
    <section className="panel">
      <h2>AIR visualizer</h2>
      {state.status === 'ready' ? <AirBrowser rows={state.data} /> : <LoadingOrError state={state} />}
    </section>
  );
}
