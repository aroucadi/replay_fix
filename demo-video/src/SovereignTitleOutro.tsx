import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { ShieldCheck } from "lucide-react";
import { C, MONO, pop, rand } from "./theme";

const LEAVES = 7;
const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

/** Local frame 0 == global 900. */
export const SovereignTitleOutro: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = pop(frame, fps, 0);
  // Tree layout: 7 leaves → 4 → 2 → 1 (RFC 6962 style odd-node promotion)
  const levels = [LEAVES, 4, 2, 1];
  const W = 900, H = 260;
  const pos = (lvl: number, i: number) => {
    const n = levels[lvl];
    return { x: ((i + 0.5) / n) * W, y: H - (lvl / 3) * H + 0 };
  };
  const lock = interpolate(frame, [20, 110], [0, 1], clamp);
  const stamp = pop(frame, fps, 130);
  const heroIn = interpolate(frame, [150, 200], [0, 1], clamp);
  const subIn = interpolate(frame, [205, 245], [0, 1], clamp);
  const tilt = interpolate(frame, [0, 300], [14, 0], clamp);

  return (
    <AbsoluteFill className="items-center justify-center" style={{ fontFamily: MONO, perspective: 1600, opacity: Math.min(1, enter * 1.5) }}>
      {/* particle dust */}
      {Array.from({ length: 60 }).map((_, i) => (
        <div key={i} className="absolute rounded-full" style={{ left: rand(i) * 1920, top: ((rand(i + 100) * 1080 - frame * (0.4 + rand(i + 5))) % 1080 + 1080) % 1080, width: 3, height: 3, background: i % 3 ? C.green : C.cyan, opacity: 0.4 }} />
      ))}
      <div className="flex flex-col items-center" style={{ transform: `rotateX(${tilt}deg) scale(${0.9 + 0.1 * enter})` }}>
        {/* Merkle tree */}
        <svg width={W} height={H + 40} viewBox={`-20 -20 ${W + 40} ${H + 40}`} style={{ overflow: "visible" }}>
          {levels.slice(0, -1).map((n, lvl) =>
            Array.from({ length: n }).map((_, i) => {
              const a = pos(lvl, i), b = pos(lvl + 1, Math.min(Math.floor(i / 2), levels[lvl + 1] - 1));
              const sx = a.x + (rand(i + lvl * 10) - 0.5) * 200 * (1 - lock);
              return <line key={`${lvl}-${i}`} x1={sx} y1={a.y} x2={b.x} y2={b.y} stroke={C.green} strokeWidth={2} opacity={lock} style={{ filter: `drop-shadow(0 0 6px ${C.green})` }} />;
            }),
          )}
          {levels.map((n, lvl) =>
            Array.from({ length: n }).map((_, i) => {
              const p = pos(lvl, i);
              const scatter = (1 - lock) * (rand(i + lvl * 7) - 0.5) * 300;
              const root = lvl === 3;
              return <rect key={`${lvl}-${i}`} x={p.x - 14 + scatter} y={p.y - 14 - (1 - lock) * 80 * rand(i)} width={28} height={28} rx={4} fill={root ? C.gold : "#0a1a14"} stroke={root ? C.gold : C.green} strokeWidth={2} style={{ filter: `drop-shadow(0 0 ${root ? 14 : 8}px ${root ? C.gold : C.green})` }} />;
            }),
          )}
        </svg>
        <div className="mt-2 text-[20px] tracking-widest" style={{ color: C.dim, opacity: lock }}>7 LEAVES · ROOT {lock > 0.95 ? "9c4e…b27a" : "········"}</div>

        {/* seal */}
        <div className="mt-8 flex items-center gap-4 rounded-full px-10 py-4 text-[26px] font-bold tracking-wider" style={{ transform: `scale(${stamp}) rotate(${(1 - stamp) * -8}deg)`, border: `2px solid ${C.gold}`, color: C.gold, background: "#f5c54212", boxShadow: `0 0 50px ${C.gold}55` }}>
          <ShieldCheck size={36} /> RFC 6962 MERKLE PROOF CERTIFIED • Ed25519 SIGNED
        </div>

        {/* hero */}
        <div className="mt-12 max-w-[1700px] text-center text-[58px] font-bold leading-[1.15] text-white" style={{ opacity: heroIn, transform: `translateY(${(1 - heroIn) * 30}px)`, textShadow: `0 0 40px ${C.green}66` }}>
          Turn raw video bug reports into <span style={{ color: C.green }}>verified</span>, cryptographically signed fixes.
        </div>
        <div className="mt-8 text-[24px]" style={{ color: C.cyan, opacity: subIn }}>
          Meta Global AI Developer Hackathon 2026 • Sovereign Autonomous Repair Engine
        </div>
      </div>
    </AbsoluteFill>
  );
};
