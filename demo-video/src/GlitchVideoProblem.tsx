import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { AlertTriangle, Play, MousePointer2 } from "lucide-react";
import { C, MONO, pop, rand } from "./theme";

const TARGET = { x: 482, y: 310 };

export const GlitchVideoProblem: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = pop(frame, fps, 0);
  const glitchOn = frame > 70 && (frame % 37 < 6 || frame > 150);
  const shift = glitchOn ? (rand(frame) - 0.5) * 28 : 0;
  const progress = interpolate(frame, [0, 240], [0.18, 0.46]);
  const scanY = interpolate(frame, [50, 130], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const scanX = interpolate(frame, [50, 130], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const clickT = interpolate(frame, [130, 150], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const errT = pop(frame, fps, 150);
  // Player viewport (scaled-down): 1280x720 shown in 1280x720 box; target in video coords 960x540 → scale
  const W = 1280, H = 720;
  const tx = (TARGET.x / 960) * W, ty = (TARGET.y / 540) * H;

  return (
    <AbsoluteFill style={{ fontFamily: MONO }}>
      <div className="absolute inset-0 flex flex-col items-center justify-center" style={{ transform: `scale(${0.9 + 0.1 * enter})`, opacity: enter }}>
        <div className="mb-5 flex w-[1280px] items-center justify-between text-[20px]" style={{ color: C.dim }}>
          <span>▶ crash.mp4 — user bug report</span>
          <span style={{ color: C.red }}>● REC 00:0{Math.min(9, Math.floor(frame / 30))}</span>
        </div>
        <div className="relative overflow-hidden rounded-xl" style={{ width: W, height: H, border: `1px solid ${C.dim}55`, background: "#0d1220", boxShadow: `0 0 80px ${C.red}22` }}>
          {/* mock app */}
          <div style={{ transform: `translateX(${shift}px)`, filter: glitchOn ? `hue-rotate(${rand(frame + 3) * 90}deg)` : "none" }} className="absolute inset-0">
            <div className="flex h-14 items-center px-8 text-[20px] text-white/70" style={{ background: "#121a2e" }}>counter-app · localhost</div>
            <div className="absolute left-0 right-0 top-[200px] text-center text-[96px] font-bold text-white/90">Count: {frame > 150 ? "NaN" : 7}</div>
            <div className="absolute flex items-center justify-center rounded-lg text-[24px] font-bold" style={{ left: tx - 90, top: ty - 28, width: 180, height: 56, background: C.cyan, color: C.void }}>+ Increment</div>
          </div>
          {glitchOn && (
            <>
              <div className="absolute inset-x-0" style={{ top: rand(frame) * H, height: 14, background: `${C.red}55`, mixBlendMode: "screen" }} />
              <div className="absolute inset-x-0" style={{ top: rand(frame + 9) * H, height: 6, background: `${C.cyan}66` }} />
            </>
          )}
          {/* scan lines tracing click coordinate */}
          <div className="absolute inset-y-0 w-px" style={{ left: tx, background: C.green, opacity: scanY > 0 ? 0.7 : 0, clipPath: `inset(0 0 ${100 - scanY * 100}% 0)`, boxShadow: `0 0 12px ${C.green}` }} />
          <div className="absolute inset-x-0 h-px" style={{ top: ty, background: C.green, opacity: scanX > 0 ? 0.7 : 0, clipPath: `inset(0 ${100 - scanX * 100}% 0 0)`, boxShadow: `0 0 12px ${C.green}` }} />
          {clickT > 0 && (
            <>
              <div className="absolute rounded-full" style={{ left: tx - 40 * clickT, top: ty - 40 * clickT, width: 80 * clickT, height: 80 * clickT, border: `2px solid ${C.green}`, opacity: 1 - clickT * 0.3 }} />
              <MousePointer2 className="absolute" size={34} color="#fff" style={{ left: tx, top: ty }} />
              <div className="absolute rounded px-3 py-1 text-[18px]" style={{ left: tx + 56, top: ty - 70, background: "#000a", color: C.green, border: `1px solid ${C.green}` }}>
                click (x: {TARGET.x}, y: {TARGET.y})
              </div>
            </>
          )}
          {/* exception overlay */}
          {frame > 150 && (
            <div className="absolute left-1/2 top-[110px] w-[760px] -translate-x-1/2 rounded-lg p-6" style={{ background: "#2a0710ee", border: `2px solid ${C.red}`, transform: `translateX(-50%) scale(${errT})`, boxShadow: `0 0 60px ${C.red}66` }}>
              <div className="flex items-center gap-3 text-[26px] font-bold" style={{ color: C.red }}><AlertTriangle size={30} /> Uncaught TypeError</div>
              <div className="mt-3 text-[20px] text-white/80">Cannot read properties of undefined (reading 'count')</div>
              <div className="mt-1 text-[17px]" style={{ color: C.dim }}>at onClick (counter.tsx:42:17)</div>
            </div>
          )}
          {/* scrub bar */}
          <div className="absolute bottom-0 flex h-14 w-full items-center gap-4 px-6" style={{ background: "#000b" }}>
            <Play size={22} color="#fff" />
            <div className="relative h-1.5 flex-1 rounded bg-white/20"><div className="h-full rounded" style={{ width: `${progress * 100}%`, background: C.red }} /></div>
          </div>
        </div>
        <div className="mt-6 text-[24px]" style={{ color: C.dim, opacity: interpolate(frame, [170, 200], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }) }}>
          A video bug report. No repro steps. No stack trace you can trust.
        </div>
      </div>
    </AbsoluteFill>
  );
};
