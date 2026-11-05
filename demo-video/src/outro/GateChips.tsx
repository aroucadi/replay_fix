import React from "react";
import { AbsoluteFill, useCurrentFrame, useVideoConfig } from "remotion";
import { CheckCircle2, ShieldAlert } from "lucide-react";
import { C, MONO, pop } from "../theme";

interface ChipItem {
  id: string;
  label: string;
  sub: string;
  color: string;
  bg: string;
  delay: number;
  icon: React.ReactNode;
}

export const GateChips: React.FC<{
  centerY?: number;
  scaleOverride?: number;
  opacity?: number;
}> = ({ centerY = 660, scaleOverride, opacity = 1 }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const chips: ChipItem[] = [
    {
      id: "gate1",
      label: "GATE 1 · RED",
      sub: "Causal Red Baseline Verified",
      color: C.red,
      bg: "#2b0a12",
      delay: 24, // local frame
      icon: <ShieldAlert size={26} color={C.red} />,
    },
    {
      id: "gate2",
      label: "GATE 2 · GREEN ×2",
      sub: "2-Run Stability Pass OK",
      color: C.green,
      bg: "#062215",
      delay: 48, // local frame
      icon: <CheckCircle2 size={26} color={C.green} />,
    },
    {
      id: "delta",
      label: "Δrepo = 0",
      sub: "Scratch Isolation Preserved",
      color: C.cyan,
      bg: "#061a29",
      delay: 72, // local frame
      icon: <CheckCircle2 size={26} color={C.cyan} />,
    },
  ];

  return (
    <AbsoluteFill
      style={{
        opacity,
        pointerEvents: "none",
        fontFamily: MONO,
      }}
    >
      <div
        className="absolute inset-x-0 flex justify-center items-center gap-8"
        style={{
          top: centerY,
          transform: scaleOverride ? `scale(${scaleOverride})` : undefined,
        }}
      >
        {chips.map((chip) => {
          const p = pop(frame, fps, chip.delay);
          if (p <= 0.01) return null;

          return (
            <div
              key={chip.id}
              className="flex items-center gap-4 rounded-2xl px-8 py-5 shadow-2xl transition-transform"
              style={{
                transform: `scale(${p})`,
                opacity: Math.min(1, p * 1.5),
                background: `${chip.bg}dd`,
                border: `2px solid ${chip.color}`,
                boxShadow: `0 0 35px ${chip.color}44, inset 0 0 15px ${chip.color}18`,
              }}
            >
              <div
                className="flex h-12 w-12 items-center justify-center rounded-xl"
                style={{ background: `${chip.color}22` }}
              >
                {chip.icon}
              </div>
              <div className="flex flex-col">
                <span
                  className="text-[26px] font-bold tracking-wider"
                  style={{ color: chip.color }}
                >
                  {chip.label} ✔
                </span>
                <span className="text-[14px]" style={{ color: C.dim }}>
                  {chip.sub}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
