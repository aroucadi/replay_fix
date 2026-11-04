import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { C, MONO, pop } from "./theme";

const BANNER = [
  "██████╗ ███████╗██████╗ ██╗      █████╗ ██╗   ██╗███████╗██╗██╗  ██╗",
  "██╔══██╗██╔════╝██╔══██╗██║     ██╔══██╗╚██╗ ██╔╝██╔════╝██║╚██╗██╔╝",
  "██████╔╝█████╗  ██████╔╝██║     ███████║ ╚████╔╝ █████╗  ██║ ╚███╔╝ ",
  "██╔══██╗██╔══╝  ██╔═══╝ ██║     ██╔══██║  ╚██╔╝  ██╔══╝  ██║ ██╔██╗ ",
  "██║  ██║███████╗██║     ███████╗██║  ██║   ██║   ██║     ██║██╔╝ ██╗",
  "╚═╝  ╚═╝╚══════╝╚═╝     ╚══════╝╚═╝  ╚═╝   ╚═╝   ╚═╝     ╚═╝╚═╝  ╚═╝",
];
const CMD = "replayfix v2.0.0 --repro --video crash.mp4 --repo ./counter-app";

/** Local frame 0 == global 240. */
export const TuiTerminalBoot: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const open = pop(frame, fps, 0);
  const typed = Math.floor(interpolate(frame, [20, 90], [0, CMD.length], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }));
  const bannerChars = interpolate(frame, [95, 160], [0, 70], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const ff = interpolate(frame, [170, 300], [0, 100], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const rows = [
    { at: 160, text: "tokio runtime", val: `${Math.min(8, Math.floor((frame - 160) / 4))} / 8 async workers ready`, ok: frame > 192 },
    { at: 190, text: "ephemeral port", val: "Allocated 127.0.0.1:54321  (bind :0)", ok: frame > 215 },
    { at: 215, text: "scratch worktree", val: "isolated clone · Δrepo = 0", ok: frame > 240 },
  ];
  const caret = Math.floor(frame / 15) % 2 === 0;
  const spin = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"[Math.floor(frame / 4) % 10];

  return (
    <AbsoluteFill className="items-center justify-center" style={{ fontFamily: MONO }}>
      <div className="overflow-hidden rounded-xl" style={{ width: 1500, height: 860, transform: `scale(${0.6 + 0.4 * open})`, opacity: Math.min(1, open * 2), background: "#05070bf2", border: `1px solid ${C.green}55`, boxShadow: `0 0 120px ${C.green}25, inset 0 0 60px #00ff8808` }}>
        <div className="flex h-12 items-center gap-2 px-5" style={{ background: "#0d121c", borderBottom: `1px solid ${C.green}22` }}>
          {["#ff5f56", "#ffbd2e", "#27c93f"].map((c) => <div key={c} className="h-3.5 w-3.5 rounded-full" style={{ background: c }} />)}
          <span className="ml-4 text-[18px]" style={{ color: C.dim }}>replayfix — tui</span>
        </div>
        <div className="p-10 text-[26px] leading-[1.5]">
          <div><span style={{ color: C.green }}>❯ </span><span className="text-white">{CMD.slice(0, typed)}</span>{caret && frame < 100 && <span style={{ background: C.green }}>&nbsp;</span>}</div>
          <pre className="mt-6 text-[19px] leading-[1.15]" style={{ color: C.green, textShadow: `0 0 14px ${C.green}` }}>
            {BANNER.map((l) => l.slice(0, Math.floor(bannerChars))).join("\n")}
          </pre>
          <div className="mt-8 space-y-1">
            {rows.map((r) => frame > r.at && (
              <div key={r.text} className="flex gap-4">
                <span style={{ color: r.ok ? C.green : C.cyan, width: 30 }}>{r.ok ? "✔" : spin}</span>
                <span className="w-[300px]" style={{ color: C.dim }}>{r.text}</span>
                <span style={{ color: C.cyan }}>{r.val}</span>
              </div>
            ))}
            {frame > 240 && (
              <div className="flex gap-4">
                <span style={{ color: ff >= 100 ? C.green : C.cyan, width: 30 }}>{ff >= 100 ? "✔" : spin}</span>
                <span className="w-[300px]" style={{ color: C.dim }}>ffmpeg keyframes</span>
                <span className="text-white">[{"█".repeat(Math.floor(ff / 100 * 36)).padEnd(36, "░")}]</span>
                <span style={{ color: C.green }}>{Math.floor(ff)}%</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </AbsoluteFill>
  );
};
