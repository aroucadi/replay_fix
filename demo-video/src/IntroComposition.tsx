import { AbsoluteFill, Audio, Sequence, interpolate, staticFile, useCurrentFrame } from "remotion";
import { C, rand } from "./theme";
import { GlitchVideoProblem } from "./GlitchVideoProblem";
import { TuiTerminalBoot } from "./TuiTerminalBoot";
import { NeuralTelemetryDashboard } from "./NeuralTelemetryDashboard";
import { SovereignTitleOutro } from "./SovereignTitleOutro";

const FADE = 20;

/** Cross-fades a phase in/out over its [from, from+dur) window. */
const Phase: React.FC<{ from: number; dur: number; first?: boolean; last?: boolean; children: React.ReactNode }> = ({ from, dur, first, last, children }) => (
  <Sequence from={from} durationInFrames={dur}>
    <Fade dur={dur} first={first} last={last}>{children}</Fade>
  </Sequence>
);

const Fade: React.FC<{ dur: number; first?: boolean; last?: boolean; children: React.ReactNode }> = ({ dur, first, last, children }) => {
  const f = useCurrentFrame();
  const o = interpolate(f, [0, FADE, dur - FADE, dur], [first ? 1 : 0, 1, 1, last ? 1 : 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return <AbsoluteFill style={{ opacity: o }}>{children}</AbsoluteFill>;
};

const Sfx: React.FC<{ at: number; name: string; volume?: number }> = ({ at, name, volume = 0.6 }) => (
  <Sequence from={at} layout="none"><Audio src={staticFile(`sfx/${name}.wav`)} volume={volume} /></Sequence>
);

export const IntroComposition: React.FC = () => {
  const frame = useCurrentFrame();
  const scan = (frame * 3) % 1080;
  // typing clacks during terminal command (global 260–330) and log stream
  const clacks = Array.from({ length: 22 }, (_, i) => 262 + i * 3 + Math.floor(rand(i) * 2));
  return (
    <AbsoluteFill style={{ background: C.void }}>
      <AbsoluteFill style={{ background: `radial-gradient(ellipse at 50% 40%, #0e1626 0%, ${C.void} 70%)` }} />
      <AbsoluteFill style={{ backgroundImage: `linear-gradient(${C.green}08 1px,transparent 1px),linear-gradient(90deg,${C.green}08 1px,transparent 1px)`, backgroundSize: "60px 60px" }} />

      <Phase from={0} dur={260} first><GlitchVideoProblem /></Phase>
      <Phase from={240} dur={320}><TuiTerminalBoot /></Phase>
      <Phase from={540} dur={430}><NeuralTelemetryDashboard /></Phase>
      <Phase from={900} dur={300} last><SovereignTitleOutro /></Phase>

      {/* scanlines + vignette */}
      <AbsoluteFill style={{ backgroundImage: "repeating-linear-gradient(0deg, rgba(0,0,0,0.18) 0 2px, transparent 2px 4px)", pointerEvents: "none" }} />
      <div className="absolute inset-x-0" style={{ top: scan, height: 120, background: `linear-gradient(transparent, ${C.green}0c, transparent)` }} />
      <AbsoluteFill style={{ background: "radial-gradient(ellipse at center, transparent 55%, #000000aa 100%)" }} />

      {/* Sound design */}
      <Sfx at={0} name="drop" volume={0.7} />
      <Sfx at={50} name="hum" volume={0.5} />
      <Sfx at={72} name="glitch" />
      <Sfx at={130} name="beep" />
      <Sfx at={152} name="glitch" volume={0.8} />
      <Sfx at={240} name="drop" volume={0.6} />
      {clacks.map((a) => <Sfx key={a} at={a} name="clack" volume={0.35} />)}
      <Sfx at={440} name="beep" volume={0.4} />
      <Sfx at={600} name="hum" volume={0.5} />
      <Sfx at={640} name="beep" volume={0.7} />
      <Sfx at={700} name="beep" volume={0.4} />
      <Sfx at={835} name="beep" volume={0.5} />
      <Sfx at={865} name="beep" volume={0.5} />
      <Sfx at={895} name="drop" volume={0.7} />
      <Sfx at={1030} name="stamp" volume={0.9} />
    </AbsoluteFill>
  );
};
