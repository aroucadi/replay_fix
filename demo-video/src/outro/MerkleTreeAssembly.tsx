import React from "react";
import { AbsoluteFill, interpolate, random, useCurrentFrame, useVideoConfig } from "remotion";
import { C, MONO, pop } from "../theme";
import { buildMerkleTreeStructure, TreeNode, TreeEdge, MERKLE_DATA } from "./data";

const { levels, edges } = buildMerkleTreeStructure(MERKLE_DATA);

// Geometry config
const W = 1920;
const H = 1080;

// Y coordinate per tree level (bottom up)
const LEVEL_Y = [760, 560, 370, 180];

function getNodePos(lvl: number, idx: number, totalInLvl: number) {
  const marginX = 160;
  const usableWidth = W - marginX * 2;
  const x = marginX + ((idx + 0.5) / totalInLvl) * usableWidth;
  const y = LEVEL_Y[lvl];
  return { x, y };
}

function scrambleHex(realHex: string, frame: number, seed: number, progress: number): string {
  if (progress >= 1) return realHex;
  const chars = "0123456789abcdef";
  const len = realHex.length;
  const fixedCount = Math.floor(progress * len);
  let res = realHex.slice(0, fixedCount);
  for (let i = fixedCount; i < len; i++) {
    const r = Math.floor(random(seed * 1000 + frame * 17 + i) * 16);
    res += chars[r];
  }
  return res;
}

export const MerkleTreeAssembly: React.FC<{ compact?: boolean; scale?: number }> = ({ compact = false }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // Timing breakdown (local frames 0..150):
  // 0..25: Leaf burst & position settle
  // 25..65: Level 0 -> Level 1 pulse & resolution
  // 65..105: Level 1 -> Level 2 pulse & resolution
  // 105..145: Level 2 -> Level 3 (Root) pulse & root typing

  const leafBurst = pop(frame, fps, 0);

  // Pulse timings for each tier transition
  const pulseL0to1 = interpolate(frame, [25, 55], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const pulseL1to2 = interpolate(frame, [65, 95], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const pulseL2to3 = interpolate(frame, [105, 135], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  // Node resolution progress
  const resolveL1 = interpolate(frame, [45, 65], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const resolveL2 = interpolate(frame, [85, 105], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const resolveL3 = interpolate(frame, [125, 145], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  const rootTyping = Math.floor(interpolate(frame, [120, 150], [0, 64], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }));

  return (
    <AbsoluteFill style={{ fontFamily: MONO }}>
      {/* Background radial highlight */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background: `radial-gradient(circle at 50% 30%, ${C.cyan}12 0%, transparent 65%)`,
        }}
      />

      <svg width={W} height={H} className="absolute inset-0 overflow-visible">
        <defs>
          <filter id="glow-cyan" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="6" result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
          <filter id="glow-green" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="8" result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {/* Tree Edges */}
        {edges.map((e) => {
          const fromPos = getNodePos(e.fromLevel, e.fromIndex, levels[e.fromLevel].length);
          const toPos = getNodePos(e.toLevel, e.toIndex, levels[e.toLevel].length);

          let tierProgress = 0;
          if (e.fromLevel === 0) tierProgress = pulseL0to1;
          else if (e.fromLevel === 1) tierProgress = pulseL1to2;
          else if (e.fromLevel === 2) tierProgress = pulseL2to3;

          const edgeLength = Math.hypot(toPos.x - fromPos.x, toPos.y - fromPos.y);
          const dashOffset = edgeLength * (1 - tierProgress);

          // Animated pulse particle coordinate along the line
          const px = fromPos.x + (toPos.x - fromPos.x) * tierProgress;
          const py = fromPos.y + (toPos.y - fromPos.y) * tierProgress;

          return (
            <g key={e.id}>
              {/* Base faint edge */}
              <line
                x1={fromPos.x}
                y1={fromPos.y}
                x2={toPos.x}
                y2={toPos.y}
                stroke={C.dim}
                strokeWidth={2}
                strokeOpacity={0.25}
              />
              {/* Active pulsing connecting edge */}
              {tierProgress > 0 && (
                <line
                  x1={fromPos.x}
                  y1={fromPos.y}
                  x2={toPos.x}
                  y2={toPos.y}
                  stroke={e.toLevel === 3 ? C.gold : C.cyan}
                  strokeWidth={3}
                  strokeDasharray={edgeLength}
                  strokeDashoffset={dashOffset}
                  filter="url(#glow-cyan)"
                  strokeOpacity={0.85}
                />
              )}
              {/* Pulse head particle */}
              {tierProgress > 0 && tierProgress < 1 && (
                <circle
                  cx={px}
                  cy={py}
                  r={5}
                  fill={C.green}
                  filter="url(#glow-green)"
                />
              )}
            </g>
          );
        })}
      </svg>

      {/* Tree Nodes */}
      {levels.map((lvlNodes, lvl) => {
        return lvlNodes.map((node, idx) => {
          const pos = getNodePos(lvl, idx, lvlNodes.length);

          // Node appearance timing
          let nodePop = 1;
          let resolveProg = 1;
          let isResolved = true;

          if (lvl === 0) {
            nodePop = leafBurst;
            resolveProg = 1;
          } else if (lvl === 1) {
            nodePop = interpolate(frame, [30, 50], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
            resolveProg = resolveL1;
            isResolved = frame >= 65;
          } else if (lvl === 2) {
            nodePop = interpolate(frame, [70, 90], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
            resolveProg = resolveL2;
            isResolved = frame >= 105;
          } else if (lvl === 3) {
            nodePop = interpolate(frame, [110, 130], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
            resolveProg = resolveL3;
            isResolved = frame >= 145;
          }

          if (nodePop <= 0.01) return null;

          const displayHash = scrambleHex(node.shortHash, frame, lvl * 10 + idx, resolveProg);
          const isRoot = node.isRoot;
          const nodeWidth = isRoot ? 540 : lvl === 0 ? 210 : 220;
          const nodeHeight = isRoot ? 72 : 56;

          return (
            <div
              key={node.id}
              className="absolute flex flex-col justify-center rounded-lg border px-3 transition-transform"
              style={{
                left: pos.x - nodeWidth / 2,
                top: pos.y - nodeHeight / 2,
                width: nodeWidth,
                height: nodeHeight,
                transform: `scale(${nodePop})`,
                opacity: Math.min(1, nodePop * 1.5),
                background: isRoot ? "#131008ee" : "#080c16ee",
                borderColor: isRoot ? C.gold : isResolved ? C.green : C.cyan,
                boxShadow: isRoot
                  ? `0 0 35px ${C.gold}66, inset 0 0 15px ${C.gold}22`
                  : isResolved
                  ? `0 0 20px ${C.green}33`
                  : `0 0 15px ${C.cyan}25`,
              }}
            >
              <div className="flex items-center justify-between text-[13px]">
                <span
                  className="font-bold tracking-wider"
                  style={{ color: isRoot ? C.gold : isResolved ? C.green : C.cyan }}
                >
                  {isRoot ? "RFC 6962 ROOT" : lvl === 0 ? `0x00 · ${node.label}` : `0x01 · ${node.label}`}
                </span>
                <span className="text-[11px]" style={{ color: C.dim }}>
                  {lvl === 0 ? "LEAF" : isRoot ? "SHA-256" : "NODE"}
                </span>
              </div>
              <div
                className="font-mono text-[16px] font-semibold tracking-wide"
                style={{ color: isRoot ? "#ffffff" : isResolved ? "#e2e8f0" : C.cyan }}
              >
                {isRoot
                  ? `${node.fullHash.slice(0, rootTyping)}${rootTyping < 64 ? "…" : ""}`
                  : `${displayHash}…`}
              </div>
            </div>
          );
        });
      })}

      {/* Top Banner indicating RFC 6962 Verification Progress */}
      <div
        className="absolute top-10 inset-x-0 flex justify-center text-[22px] tracking-widest uppercase font-bold"
        style={{
          color: frame >= 135 ? C.gold : C.cyan,
          opacity: interpolate(frame, [10, 30], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }),
        }}
      >
        <span>
          {frame < 60
            ? "1. HASHING EVIDENCE LEAVES (RFC 6962 0x00)"
            : frame < 110
            ? "2. LEVEL-BY-LEVEL INTERNAL MERKLE PAIRING (0x01)"
            : "3. RFC 6962 MERKLE ROOT CALCULATED"}
        </span>
      </div>
    </AbsoluteFill>
  );
};
