import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { Award, GitBranch } from "lucide-react";
import { C, MONO, SANS, pop } from "../theme";

const REPO_URL = "github.com/aroucadi/replay_fix";
const TERMINAL_CMD = "$ replayfix repro --video bug.mp4 --repo .";

const GithubIcon: React.FC<{ size?: number; color?: string }> = ({ size = 26, color = "#fff" }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke={color}
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
    <path d="M9 18c-4.51 2-5-2-7-2" />
  </svg>
);

export const FinalLockup: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // Entrance of final card during frames 0..30
  const cardPop = pop(frame, fps, 0);

  // Fade out of the card heading elements into pitch black during frames 32..50
  const cardFadeOut = interpolate(frame, [32, 50], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Terminal command typing between frame 46 and 80
  const typedChars = Math.floor(
    interpolate(frame, [46, 80], [0, TERMINAL_CMD.length], {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
    })
  );

  // Blinking cursor
  const showCursor = Math.floor(frame / 12) % 2 === 0;

  // Terminal line entrance opacity
  const termOpacity = interpolate(frame, [42, 48], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill className="items-center justify-center" style={{ fontFamily: MONO }}>
      {/* 1. Outro metadata card (fades to black) */}
      <div
        className="flex flex-col items-center text-center space-y-6"
        style={{
          transform: `scale(${0.92 + 0.08 * cardPop})`,
          opacity: Math.min(cardPop, cardFadeOut),
          pointerEvents: "none",
        }}
      >
        <div
          className="text-[64px] font-black tracking-widest text-white uppercase"
          style={{ fontFamily: SANS }}
        >
          ReplayFix <span style={{ color: C.green }}>v2.0.0</span>
        </div>

        <div className="flex items-center gap-3 rounded-full border border-white/20 bg-white/5 px-8 py-3 text-[24px] text-white/90">
          <GithubIcon size={28} color="#ffffff" />
          <span style={{ color: C.cyan }}>{REPO_URL}</span>
        </div>

        <div
          className="flex items-center gap-2 text-[22px] tracking-wider font-semibold"
          style={{ color: C.gold }}
        >
          <Award size={24} color={C.gold} />
          <span>Meta Global AI Developer Hackathon 2026</span>
        </div>
      </div>

      {/* 2. Terminal Command typing loop (persists until end) */}
      {termOpacity > 0 && (
        <div
          className="absolute flex items-center justify-center px-12 py-6 rounded-xl"
          style={{
            opacity: termOpacity,
            fontFamily: MONO,
            fontSize: "36px",
            color: C.green,
            textShadow: `0 0 25px ${C.green}aa`,
            background: "rgba(5, 7, 11, 0.9)",
            border: `1.5px solid ${C.green}44`,
            boxShadow: `0 0 60px ${C.green}22`,
          }}
        >
          <span>{TERMINAL_CMD.slice(0, typedChars)}</span>
          {showCursor && (
            <span
              className="inline-block ml-2 w-4 h-9"
              style={{ background: C.green, verticalAlign: "middle" }}
            />
          )}
        </div>
      )}
    </AbsoluteFill>
  );
};
