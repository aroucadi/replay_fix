import React from "react";
import { AbsoluteFill, Audio, Sequence, interpolate, staticFile, useCurrentFrame } from "remotion";
import { C } from "../theme";
import { ConvergenceFragments } from "./ConvergenceFragments";
import { MerkleTreeAssembly } from "./MerkleTreeAssembly";
import { Ed25519Seal } from "./Ed25519Seal";
import { GateChips } from "./GateChips";
import { HeroPromise } from "./HeroPromise";
import { FinalLockup } from "./FinalLockup";

const FADE = 12;

const FadeWrap: React.FC<{
  dur: number;
  first?: boolean;
  last?: boolean;
  children: React.ReactNode;
}> = ({ dur, first, last, children }) => {
  const f = useCurrentFrame();
  const o = interpolate(
    f,
    [0, FADE, dur - FADE, dur],
    [first ? 1 : 0, 1, 1, last ? 1 : 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );
  return <AbsoluteFill style={{ opacity: o }}>{children}</AbsoluteFill>;
};

const Sfx: React.FC<{ at: number; name: string; volume?: number }> = ({
  at,
  name,
  volume = 0.6,
}) => (
  <Sequence from={at} layout="none">
    <Audio src={staticFile(`audio/sfx/${name}.mp3`)} volume={volume} />
  </Sequence>
);

export const OutroComposition: React.FC = () => {
  const frame = useCurrentFrame();

  // Moving scanline
  const scanY = (frame * 3.5) % 1080;

  // Background music volume curve: fade in 0..30, fade out 560..600
  const musicVol = interpolate(
    frame,
    [0, 30, 560, 600],
    [0, 0.65, 0.65, 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  // Terminal typing key clack frames in Beat 5
  const keyClacks = [556, 560, 564, 568, 572, 576, 580, 584, 588];

  return (
    <AbsoluteFill style={{ background: C.void }}>
      {/* 1. Deep cosmic radial backdrop */}
      <AbsoluteFill
        style={{
          background: `radial-gradient(ellipse at 50% 45%, #0c1424 0%, ${C.void} 75%)`,
        }}
      />

      {/* 2. Cyberpunk matrix grid lines */}
      <AbsoluteFill
        style={{
          backgroundImage: `linear-gradient(${C.green}08 1px, transparent 1px), linear-gradient(90deg, ${C.green}08 1px, transparent 1px)`,
          backgroundSize: "60px 60px",
          opacity: 0.6,
        }}
      />

      {/* BEAT 1: CONVERGENCE (Frames 0..90) */}
      <Sequence from={0} durationInFrames={90}>
        <FadeWrap dur={90} first>
          <ConvergenceFragments />
        </FadeWrap>
      </Sequence>

      {/* BEAT 2: MERKLE ASSEMBLY (Frames 90..240) */}
      <Sequence from={90} durationInFrames={150}>
        <FadeWrap dur={150}>
          <MerkleTreeAssembly />
        </FadeWrap>
      </Sequence>

      {/* BEAT 3: THE SEAL & CHIPS (Frames 240..360) */}
      <Sequence from={240} durationInFrames={120}>
        <FadeWrap dur={120}>
          <Ed25519Seal />
          <GateChips />
        </FadeWrap>
      </Sequence>

      {/* BEAT 4: THE PROMISE (Frames 360..510) */}
      <Sequence from={360} durationInFrames={150}>
        <FadeWrap dur={150}>
          <HeroPromise />
        </FadeWrap>
      </Sequence>

      {/* BEAT 5: LOCKUP & LOOP (Frames 510..600) */}
      <Sequence from={510} durationInFrames={90}>
        <FadeWrap dur={90} last>
          <FinalLockup />
        </FadeWrap>
      </Sequence>

      {/* OVERLAY: Scanlines, vignette, and ambient CRT beam */}
      <AbsoluteFill
        style={{
          backgroundImage:
            "repeating-linear-gradient(0deg, rgba(0,0,0,0.18) 0 2px, transparent 2px 4px)",
          pointerEvents: "none",
        }}
      />
      <div
        className="absolute inset-x-0 pointer-events-none"
        style={{
          top: scanY,
          height: 120,
          background: `linear-gradient(transparent, ${C.green}0e, transparent)`,
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "radial-gradient(ellipse at center, transparent 55%, #000000aa 100%)",
          pointerEvents: "none",
        }}
      />

      {/* AUDIO: Ambient Music Bed */}
      <Audio src={staticFile("audio/outro_music.mp3")} volume={musicVol} />

      {/* AUDIO: SFX Layers */}
      {/* Beat 1 Whoosh */}
      <Sfx at={0} name="whoosh" volume={0.7} />

      {/* Beat 2 Merkle ticks */}
      <Sfx at={120} name="tick" volume={0.5} />
      <Sfx at={155} name="tick" volume={0.5} />
      <Sfx at={195} name="tick" volume={0.5} />
      <Sfx at={220} name="tick" volume={0.6} />

      {/* Beat 3 Impact at Frame 240 */}
      <Sfx at={240} name="impact" volume={0.95} />

      {/* Beat 3 Gate Chips */}
      <Sfx at={264} name="chip" volume={0.55} />
      <Sfx at={288} name="chip" volume={0.55} />
      <Sfx at={312} name="chip" volume={0.55} />

      {/* Beat 5 Terminal typing keys */}
      {keyClacks.map((f) => (
        <Sfx key={f} at={f} name="key" volume={0.4} />
      ))}
    </AbsoluteFill>
  );
};
