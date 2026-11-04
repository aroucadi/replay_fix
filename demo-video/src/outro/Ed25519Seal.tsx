import React from "react";
import { AbsoluteFill, interpolate, random, useCurrentFrame, useVideoConfig } from "remotion";
import { ShieldCheck, Lock } from "lucide-react";
import { C, MONO, SANS, popImpact } from "../theme";
import { MERKLE_DATA } from "./data";

export const Ed25519Seal: React.FC<{
  centerX?: number;
  centerY?: number;
  scaleOverride?: number;
  opacity?: number;
}> = ({ centerX = 960, centerY = 340, scaleOverride, opacity = 1 }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // Impact spring from scale 2.4 down to 1.0 with overshoot
  const impactProgress = popImpact(frame, fps, 0);
  const sealScale = scaleOverride ?? interpolate(impactProgress, [0, 1], [2.4, 1.0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // 6-frame deterministic camera shake (frames 0..6)
  const shakeIntensity = Math.max(0, 6 - frame);
  const shakeX = (random(frame * 17 + 1) - 0.5) * shakeIntensity * 7;
  const shakeY = (random(frame * 31 + 2) - 0.5) * shakeIntensity * 7;

  // Radial shockwave ring expanding outward from the impact
  const shockProgress = interpolate(frame, [0, 45], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const shockRadius = interpolate(shockProgress, [0, 1], [140, 750]);
  const shockOpacity = interpolate(shockProgress, [0, 0.2, 1], [0.9, 0.7, 0]);

  // Second delayed shockwave ring
  const shockProgress2 = interpolate(frame, [8, 55], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const shockRadius2 = interpolate(shockProgress2, [0, 1], [100, 600]);
  const shockOpacity2 = interpolate(shockProgress2, [0, 0.2, 1], [0.8, 0.5, 0]);

  // Flash bloom at frame 0..18
  const flash = interpolate(frame, [0, 3, 18], [0, 0.85, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Micro-text ring rotation (degrees)
  const rotation = frame * 0.45;

  const pubKey = MERKLE_DATA.attestation.public_key;
  const pubTrunc = `${pubKey.slice(0, 6)}…${pubKey.slice(-6)}`;
  const sig = MERKLE_DATA.attestation.signature;
  const sigTrunc = `${sig.slice(0, 6)}…${sig.slice(-6)}`;
  const rootTrunc = `${MERKLE_DATA.merkle_root.slice(0, 8)}…${MERKLE_DATA.merkle_root.slice(-8)}`;

  return (
    <AbsoluteFill
      style={{
        opacity,
        transform: `translate(${shakeX}px, ${shakeY}px)`,
        pointerEvents: "none",
      }}
    >
      {/* Green / Cyan Flash Bloom */}
      {flash > 0 && (
        <AbsoluteFill
          style={{
            background: `radial-gradient(circle at ${centerX}px ${centerY}px, ${C.green}cc 0%, ${C.cyan}66 35%, transparent 70%)`,
            opacity: flash,
            mixBlendMode: "screen",
          }}
        />
      )}

      {/* Expanding Shockwaves */}
      {shockOpacity > 0 && (
        <div
          className="absolute rounded-full"
          style={{
            left: centerX - shockRadius,
            top: centerY - shockRadius,
            width: shockRadius * 2,
            height: shockRadius * 2,
            border: `3px solid ${C.green}`,
            boxShadow: `0 0 40px ${C.green}, inset 0 0 20px ${C.green}44`,
            opacity: shockOpacity,
          }}
        />
      )}
      {shockOpacity2 > 0 && (
        <div
          className="absolute rounded-full"
          style={{
            left: centerX - shockRadius2,
            top: centerY - shockRadius2,
            width: shockRadius2 * 2,
            height: shockRadius2 * 2,
            border: `2px solid ${C.gold}`,
            boxShadow: `0 0 30px ${C.gold}`,
            opacity: shockOpacity2,
          }}
        />
      )}

      {/* The Central Ed25519 Seal */}
      <div
        className="absolute flex flex-col items-center justify-center"
        style={{
          left: centerX - 320,
          top: centerY - 240,
          width: 640,
          height: 480,
          transform: `scale(${sealScale})`,
        }}
      >
        {/* SVG Seal Medallion */}
        <div className="relative flex items-center justify-center">
          <svg width="340" height="340" viewBox="-170 -170 340 340" className="overflow-visible">
            <defs>
              <path
                id="textPathCircle"
                d="M 0, -125 A 125,125 0 1,1 0, 125 A 125,125 0 1,1 0, -125"
              />
              <filter id="seal-glow" x="-30%" y="-30%" width="160%" height="160%">
                <feGaussianBlur stdDeviation="8" result="blur" />
                <feMerge>
                  <feMergeNode in="blur" />
                  <feMergeNode in="SourceGraphic" />
                </feMerge>
              </filter>
            </defs>

            {/* Outer golden gear / dashed rim */}
            <circle
              r="150"
              fill="none"
              stroke={C.gold}
              strokeWidth="2"
              strokeDasharray="8 6"
              strokeOpacity="0.85"
            />
            <circle
              r="140"
              fill="#080d14ee"
              stroke={C.gold}
              strokeWidth="3"
              filter="url(#seal-glow)"
            />

            {/* Rotating Micro-Text Ring */}
            <g transform={`rotate(${rotation})`}>
              <text
                fill={C.gold}
                fontSize="11.5"
                fontFamily={MONO}
                letterSpacing="2.8"
                fontWeight="bold"
              >
                <textPath href="#textPathCircle">
                  RFC 6962 MERKLE PROOF · ED25519 CRYPTOGRAPHIC ATTESTATION · REPLAYFIX v2.0 ·
                </textPath>
              </text>
            </g>

            {/* Inner Ring with hex tick markers */}
            <circle
              r="105"
              fill="#0c1424"
              stroke={C.green}
              strokeWidth="2"
              strokeDasharray="4 4"
            />

            {/* Core Shield Emblem */}
            <circle r="75" fill="#060910" stroke={C.gold} strokeWidth="2.5" />
          </svg>

          {/* Central Lucide Shield Icon */}
          <div className="absolute flex flex-col items-center justify-center text-center">
            <ShieldCheck size={76} color={C.gold} style={{ filter: `drop-shadow(0 0 16px ${C.gold})` }} />
            <div
              className="mt-1 font-bold tracking-widest text-[13px]"
              style={{ color: C.green, fontFamily: MONO }}
            >
              SEALED
            </div>
          </div>
        </div>

        {/* Seal Label Banner */}
        <div
          className="mt-6 flex items-center gap-3 rounded-full px-8 py-3 text-[24px] font-bold tracking-wider"
          style={{
            fontFamily: MONO,
            border: `2px solid ${C.gold}`,
            color: C.gold,
            background: "#120f06f0",
            boxShadow: `0 0 45px ${C.gold}55`,
          }}
        >
          <Lock size={26} color={C.gold} />
          <span>Ed25519 SIGNED · RFC 6962 VERIFIED</span>
        </div>

        {/* Attestation Micro-Badges */}
        <div
          className="mt-3 flex items-center gap-4 text-[14px] font-mono tracking-wider"
          style={{ color: C.dim, fontFamily: MONO }}
        >
          <span>PUB <span style={{ color: "#e2e8f0" }}>{pubTrunc}</span></span>
          <span>•</span>
          <span>SIG <span style={{ color: "#e2e8f0" }}>{sigTrunc}</span></span>
          <span>•</span>
          <span>ROOT <span style={{ color: C.green }}>{rootTrunc}</span></span>
        </div>
      </div>
    </AbsoluteFill>
  );
};
