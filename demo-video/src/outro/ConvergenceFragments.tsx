import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { AlertTriangle, Terminal, Crosshair, Cpu } from "lucide-react";
import { C, MONO, pop } from "../theme";

export const ConvergenceFragments: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // Progress 0 -> 1 over frames 0..90
  const t = interpolate(frame, [0, 85], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Compression spring: accelerates rapidly into the center singularity
  const springPull = pop(frame, fps, 0);

  // Center coordinate
  const cx = 960;
  const cy = 540;

  // Singularity collapse flash near frame 85-90
  const flash = interpolate(frame, [80, 85, 90], [0, 1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Scale of fragments shrinking toward zero
  const fragScale = interpolate(t, [0, 0.4, 0.85, 1], [0.8, 0.7, 0.25, 0.02], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const fragOpacity = interpolate(t, [0, 0.1, 0.8, 1], [0, 1, 0.9, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Fragment starting positions (outer edges)
  const frags = [
    {
      id: "video",
      title: "crash.mp4 · PLAYWRIGHT",
      icon: <AlertTriangle size={18} color={C.red} />,
      startX: 120,
      startY: 120,
      color: C.red,
      body: (
        <div className="space-y-1 text-[13px] text-white/80">
          <div className="flex justify-between text-red-400">
            <span>● ERROR</span>
            <span>(482, 310)</span>
          </div>
          <div className="truncate text-white/70">Uncaught TypeError: count undefined</div>
        </div>
      ),
    },
    {
      id: "tui",
      title: "replayfix repro --video",
      icon: <Terminal size={18} color={C.green} />,
      startX: 120,
      startY: 840,
      color: C.green,
      body: (
        <div className="space-y-1 text-[13px]">
          <div className="text-emerald-400">❯ ephemeral port :54321</div>
          <div style={{ color: C.dim }}>Δrepo = 0 · isolated worktree</div>
        </div>
      ),
    },
    {
      id: "sam",
      title: "SAM 3.1 PERCEPTION",
      icon: <Crosshair size={18} color={C.cyan} />,
      startX: 1600,
      startY: 140,
      color: C.cyan,
      body: (
        <div className="space-y-1 text-[13px]">
          <div className="text-cyan-400">LOCK · &lt;button id="counter-btn"&gt;</div>
          <div style={{ color: C.dim }}>centroid (440, 345) conf 0.987</div>
        </div>
      ),
    },
    {
      id: "muse",
      title: "MUSE SPARK 1.3",
      icon: <Cpu size={18} color={C.violet} />,
      startX: 1600,
      startY: 840,
      color: C.violet,
      body: (
        <div className="space-y-1 text-[13px]">
          <div className="text-purple-300">STREAMING · 305 tok/s</div>
          <div className="text-emerald-400">Pass 2/2: OK · Gate 2 GREEN</div>
        </div>
      ),
    },
  ];

  return (
    <AbsoluteFill style={{ fontFamily: MONO }}>
      {/* Warping grid background */}
      <div
        className="absolute inset-0"
        style={{
          backgroundImage: `radial-gradient(circle at center, rgba(0,255,136,${0.08 * (1 - t * 0.5)}) 1px, transparent 1px)`,
          backgroundSize: `${40 - t * 15}px ${40 - t * 15}px`,
          transform: `scale(${1 + t * 0.2})`,
          opacity: 0.7,
        }}
      />

      {/* Trajectory light trails & converging cards */}
      {frags.map((f, i) => {
        // Bézier curve pulling from start toward center (cx, cy)
        const curX = interpolate(t, [0, 1], [f.startX, cx]);
        const curY = interpolate(t, [0, 1], [f.startY, cy]);

        // Trail positions
        const trailT1 = Math.max(0, t - 0.08);
        const trailT2 = Math.max(0, t - 0.16);
        const tx1 = interpolate(trailT1, [0, 1], [f.startX, cx]);
        const ty1 = interpolate(trailT1, [0, 1], [f.startY, cy]);
        const tx2 = interpolate(trailT2, [0, 1], [f.startX, cx]);
        const ty2 = interpolate(trailT2, [0, 1], [f.startY, cy]);

        return (
          <React.Fragment key={f.id}>
            {/* Trail line */}
            <svg className="absolute inset-0 pointer-events-none" width="1920" height="1080">
              <line
                x1={f.startX}
                y1={f.startY}
                x2={curX}
                y2={curY}
                stroke={f.color}
                strokeWidth={3}
                strokeOpacity={fragOpacity * 0.4}
                strokeDasharray="6 6"
              />
            </svg>

            {/* Ghost trails */}
            <div
              className="absolute pointer-events-none rounded-xl"
              style={{
                left: tx2 - 160,
                top: ty2 - 70,
                width: 320,
                height: 140,
                background: `${f.color}10`,
                border: `1px solid ${f.color}22`,
                transform: `scale(${fragScale * 0.85})`,
                opacity: fragOpacity * 0.2,
                filter: "blur(4px)",
              }}
            />
            <div
              className="absolute pointer-events-none rounded-xl"
              style={{
                left: tx1 - 160,
                top: ty1 - 70,
                width: 320,
                height: 140,
                background: `${f.color}15`,
                border: `1px solid ${f.color}44`,
                transform: `scale(${fragScale * 0.92})`,
                opacity: fragOpacity * 0.4,
                filter: "blur(2px)",
              }}
            />

            {/* Main converging fragment card */}
            <div
              className="absolute overflow-hidden rounded-xl p-4 shadow-2xl backdrop-blur-md"
              style={{
                left: curX - 160,
                top: curY - 70,
                width: 320,
                height: 140,
                background: "#080d1aee",
                border: `1.5px solid ${f.color}`,
                boxShadow: `0 0 30px ${f.color}44`,
                transform: `scale(${fragScale}) rotate(${(1 - t) * (i % 2 === 0 ? 8 : -8)}deg)`,
                opacity: fragOpacity,
              }}
            >
              <div
                className="mb-2 flex items-center justify-between border-b pb-2 text-[14px] font-bold"
                style={{ borderColor: `${f.color}33`, color: f.color }}
              >
                <span className="flex items-center gap-2">
                  {f.icon} {f.title}
                </span>
                <span className="text-[11px] opacity-70">SYNC</span>
              </div>
              {f.body}
            </div>
          </React.Fragment>
        );
      })}

      {/* Central compression singularity glow */}
      <div
        className="absolute rounded-full"
        style={{
          left: cx - 120,
          top: cy - 120,
          width: 240,
          height: 240,
          background: `radial-gradient(circle, ${C.green}88 0%, ${C.cyan}44 40%, transparent 70%)`,
          transform: `scale(${interpolate(t, [0.3, 0.9, 1], [0.1, 1.8, 3.5], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })})`,
          opacity: interpolate(t, [0.4, 0.85, 1], [0, 0.8, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }),
          filter: "blur(20px)",
        }}
      />

      {/* Singularity impact flash */}
      {flash > 0 && (
        <AbsoluteFill
          style={{
            background: "#ffffff",
            opacity: flash * 0.85,
            mixBlendMode: "overlay",
          }}
        />
      )}
    </AbsoluteFill>
  );
};
