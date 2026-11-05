import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { ShieldCheck } from "lucide-react";
import { C, MONO, SANS, pop } from "../theme";
import { MERKLE_DATA } from "./data";

export const HeroPromise: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // Words for the hero promise
  const words = [
    { text: "Give", highlight: false },
    { text: "it", highlight: false },
    { text: "a", highlight: false },
    { text: "video.", highlight: false, breakAfter: true },
    { text: "Give", highlight: false },
    { text: "it", highlight: false },
    { text: "a", highlight: false },
    { text: "repo.", highlight: false, breakAfter: true },
    { text: "Get", highlight: false },
    { text: "a", highlight: false },
    { text: "verified", highlight: true },
    { text: "fix.", highlight: false },
  ];

  // Each word appears sequentially between frame 5 and 65
  const wordDelay = 4.8;

  // ReplayFix wordmark entrance at frame 70
  const wordmarkPop = pop(frame, fps, 70);

  // Chromatic aberration offset settling to 0
  const caOffset = interpolate(frame, [70, 95], [6, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Sub-line fade in between frame 95 and 125
  const sublineOpacity = interpolate(frame, [95, 125], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Top corner mini Merkle proof glyph
  const glyphPop = pop(frame, fps, 0);
  const rootTrunc = `${MERKLE_DATA.merkle_root.slice(0, 12)}…${MERKLE_DATA.merkle_root.slice(-8)}`;

  return (
    <AbsoluteFill className="items-center justify-center">
      {/* Top corner glyph */}
      <div
        className="absolute top-12 left-16 flex items-center gap-3 rounded-full px-6 py-2.5 shadow-lg"
        style={{
          fontFamily: MONO,
          transform: `scale(${glyphPop})`,
          opacity: glyphPop,
          border: `1.5px solid ${C.gold}88`,
          background: "#080d16cc",
        }}
      >
        <ShieldCheck size={22} color={C.gold} />
        <span className="text-[16px] text-white/90">
          RFC 6962 ROOT: <span style={{ color: C.green }}>{rootTrunc}</span> · Ed25519 SEALED
        </span>
      </div>

      {/* Main Container */}
      <div className="flex flex-col items-center text-center max-w-[1720px] px-8">
        {/* Hero Promise Text (≥96px) */}
        <div
          className="flex flex-wrap justify-center items-baseline gap-x-6 gap-y-2 text-[98px] font-extrabold tracking-tight leading-[1.12]"
          style={{ fontFamily: SANS, color: "#ffffff" }}
        >
          {words.map((w, idx) => {
            const startFrame = 5 + idx * wordDelay;
            const wProg = interpolate(frame, [startFrame, startFrame + 6], [0, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            });
            const translateY = (1 - wProg) * 24;

            return (
              <React.Fragment key={idx}>
                <span
                  style={{
                    opacity: wProg,
                    transform: `translateY(${translateY}px)`,
                    color: w.highlight ? C.green : "#ffffff",
                    textShadow: w.highlight
                      ? `0 0 50px ${C.green}aa, 0 0 100px ${C.green}44`
                      : "0 4px 20px rgba(0,0,0,0.8)",
                  }}
                >
                  {w.text}
                </span>
                {w.breakAfter && <div className="basis-full h-0" />}
              </React.Fragment>
            );
          })}
        </div>

        {/* Wordmark with Chromatic Aberration Settle */}
        {wordmarkPop > 0.01 && (
          <div
            className="relative mt-12 flex items-center justify-center select-none"
            style={{
              transform: `scale(${0.9 + 0.1 * wordmarkPop})`,
              opacity: Math.min(1, wordmarkPop * 1.5),
            }}
          >
            {/* Cyan layer offset */}
            <div
              className="absolute font-black tracking-widest text-[84px] uppercase"
              style={{
                fontFamily: SANS,
                color: C.cyan,
                transform: `translate(${-caOffset}px, ${-caOffset * 0.5}px)`,
                opacity: caOffset > 0.5 ? 0.8 : 0,
                mixBlendMode: "screen",
              }}
            >
              ReplayFix
            </div>

            {/* Red / Violet layer offset */}
            <div
              className="absolute font-black tracking-widest text-[84px] uppercase"
              style={{
                fontFamily: SANS,
                color: C.red,
                transform: `translate(${caOffset}px, ${caOffset * 0.5}px)`,
                opacity: caOffset > 0.5 ? 0.8 : 0,
                mixBlendMode: "screen",
              }}
            >
              ReplayFix
            </div>

            {/* Crisp center wordmark */}
            <div
              className="font-black tracking-widest text-[84px] uppercase text-white"
              style={{
                fontFamily: SANS,
                textShadow: `0 0 35px ${C.cyan}66`,
              }}
            >
              ReplayFix
            </div>
          </div>
        )}

        {/* Sub-line */}
        <div
          className="mt-6 flex items-center gap-3 text-[26px] font-bold tracking-wider"
          style={{
            fontFamily: MONO,
            opacity: sublineOpacity,
            color: C.cyan,
            textShadow: `0 0 20px ${C.cyan}44`,
          }}
        >
          <span>Powered by</span>
          <span style={{ color: C.violet }}>Meta Muse Spark</span>
          <span style={{ color: C.dim }}>·</span>
          <span style={{ color: C.violet }}>Muse Wonder</span>
          <span style={{ color: C.dim }}>·</span>
          <span style={{ color: C.cyan }}>SAM 3.1</span>
        </div>
      </div>
    </AbsoluteFill>
  );
};
