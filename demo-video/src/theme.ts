import { loadFont } from "@remotion/google-fonts/JetBrainsMono";
import { spring, SpringConfig } from "remotion";

export const { fontFamily: MONO } = loadFont("normal", { weights: ["400", "700"], subsets: ["latin"] });

export const C = {
  void: "#07090e",
  green: "#00ff88",
  cyan: "#00d4ff",
  violet: "#7928ca",
  red: "#ff3b5c",
  gold: "#f5c542",
  dim: "#5b6578",
};

export const SPRING: Partial<SpringConfig> = { damping: 12, mass: 0.5, stiffness: 100 };

export const pop = (frame: number, fps: number, delay = 0) =>
  spring({ frame: frame - delay, fps, config: SPRING });

/** Deterministic pseudo-random in [0,1) for frame-stable effects. */
export const rand = (seed: number) => {
  const x = Math.sin(seed * 127.1 + 311.7) * 43758.5453;
  return x - Math.floor(x);
};
