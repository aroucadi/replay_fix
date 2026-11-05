import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { Cpu, Crosshair } from "lucide-react";
import { C, MONO, pop } from "./theme";

const LOG: { at: number; text: string; color?: string }[] = [
  { at: 70, text: "muse-spark-1.3 ▸ deliberating…", color: C.violet },
  { at: 90, text: "  turn 1: counter state read as undefined" },
  { at: 125, text: "  turn 2: onClick shadows `state` binding" },
  { at: 160, text: "● GATE 1 · RED — bug reproduced", color: C.red },
  { at: 195, text: "muse-wonder ▸ AST search/replace patch", color: C.violet },
  { at: 215, text: "- const n = state.count + 1", color: C.red },
  { at: 230, text: "+ const n = (state?.count ?? 0) + 1", color: C.green },
  { at: 265, text: "tests frozen · sha256 a3f9…c01e", color: C.dim },
  { at: 295, text: "Pass 1/2: OK", color: C.green },
  { at: 325, text: "Pass 2/2: OK", color: C.green },
  { at: 355, text: "● GATE 2 · GREEN — stability pass", color: C.green },
];

/** Local frame 0 == global 540. */
export const NeuralTelemetryDashboard: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = pop(frame, fps, 0);
  const snap = interpolate(frame, [40, 100], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  // bbox starts large/offset and snaps to button
  const bx = 330 + (1 - snap) * -140, by = 310 + (1 - snap) * -110;
  const bw = 220 + (1 - snap) * 200, bh = 70 + (1 - snap) * 120;
  const locked = frame > 100;
  const sweep = (frame * 6) % 360;
  const tok = Math.floor(interpolate(frame, [70, 360], [0, 1840], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }));
  const gate2 = frame > 355;

  return (
    <AbsoluteFill className="items-center justify-center" style={{ fontFamily: MONO, opacity: Math.min(1, enter * 1.5) }}>
      <div className="grid gap-8" style={{ gridTemplateColumns: "820px 820px", transform: `scale(${0.94 + 0.06 * enter})` }}>
        {/* LEFT — SAM 3.1 */}
        <div className="relative h-[820px] overflow-hidden rounded-xl" style={{ background: "#05070b", border: `1px solid ${C.cyan}55`, boxShadow: `0 0 80px ${C.cyan}18` }}>
          <div className="flex items-center justify-between px-6 py-4 text-[20px]" style={{ borderBottom: `1px solid ${C.cyan}22`, color: C.cyan }}>
            <span className="flex items-center gap-2"><Crosshair size={22} /> SAM 3.1 · PERCEPTION</span>
            <span style={{ color: C.dim }}>1920x1080 @ 2x DPR</span>
          </div>
          <div className="relative m-6 h-[560px] overflow-hidden rounded-lg" style={{ background: "#0d1220", backgroundImage: `linear-gradient(${C.cyan}12 1px,transparent 1px),linear-gradient(90deg,${C.cyan}12 1px,transparent 1px)`, backgroundSize: "40px 40px" }}>
            <div className="absolute w-full text-center text-[64px] font-bold text-white/60" style={{ top: 90 }}>Count: NaN</div>
            <div className="absolute flex items-center justify-center rounded-md text-[22px] font-bold" style={{ left: 340, top: 320, width: 200, height: 50, background: C.cyan, color: C.void }}>+ Increment</div>
            {/* radar sweep */}
            <div className="absolute rounded-full" style={{ left: 440 - 150, top: 345 - 150, width: 300, height: 300, border: `1px solid ${C.cyan}44`, background: `conic-gradient(from ${sweep}deg, ${C.cyan}55, transparent 25%)`, opacity: locked ? 0.15 : 0.9 }} />
            {/* bbox */}
            <div className="absolute" style={{ left: bx, top: by, width: bw, height: bh, border: `3px solid ${locked ? C.green : C.cyan}`, boxShadow: `0 0 24px ${locked ? C.green : C.cyan}` }}>
              <div className="absolute -top-8 left-0 whitespace-nowrap text-[17px]" style={{ color: locked ? C.green : C.cyan }}>
                {locked ? "LOCK · <button id=\"counter-btn\">" : "segmenting…"}
              </div>
            </div>
            {locked && <div className="absolute h-3 w-3 rounded-full" style={{ left: 440 - 6, top: 345 - 6, background: C.green, boxShadow: `0 0 16px ${C.green}` }} />}
          </div>
          <div className="px-6 text-[19px] leading-8">
            <div style={{ color: C.dim }}>click (482, 310) → calibrated css (241, 155)</div>
            <div style={{ color: locked ? C.green : C.dim }}>centroid snap ▸ {locked ? "(440, 345) · Δ = 0 px · conf 0.987" : "searching…"}</div>
          </div>
        </div>
        {/* RIGHT — Muse Spark */}
        <div className="relative h-[820px] overflow-hidden rounded-xl" style={{ background: "#05070b", border: `1px solid ${C.violet}88`, boxShadow: `0 0 80px ${C.violet}30` }}>
          <div className="flex items-center justify-between px-6 py-4 text-[20px]" style={{ borderBottom: `1px solid ${C.violet}44`, color: "#b57bff" }}>
            <span className="flex items-center gap-2"><Cpu size={22} /> META MUSE SPARK 1.3</span>
            <span style={{ color: C.green }}>{frame > 70 ? "305 tok/s" : "idle"} · {tok} tok</span>
          </div>
          <div className="space-y-2 p-6 text-[22px] leading-8">
            {LOG.map((l) => frame >= l.at && (
              <div key={l.at} style={{ color: l.color ?? "#e6edf3", opacity: interpolate(frame, [l.at, l.at + 8], [0, 1], { extrapolateRight: "clamp" }) }}>
                {l.text.slice(0, Math.floor((frame - l.at) * 2.2) + 1)}
              </div>
            ))}
          </div>
          <div className="absolute bottom-6 left-6 right-6 flex gap-4">
            {[{ n: "GATE 1", s: "RED", on: frame > 160, c: C.red }, { n: "GATE 2", s: "GREEN", on: gate2, c: C.green }].map((g) => (
              <div key={g.n} className="flex-1 rounded-lg py-4 text-center text-[26px] font-bold" style={{ border: `2px solid ${g.on ? g.c : C.dim + "55"}`, color: g.on ? g.c : C.dim, background: g.on ? `${g.c}15` : "transparent", boxShadow: g.on ? `0 0 30px ${g.c}55` : "none" }}>
                {g.n} · {g.on ? g.s : "···"}
              </div>
            ))}
          </div>
        </div>
      </div>
    </AbsoluteFill>
  );
};
