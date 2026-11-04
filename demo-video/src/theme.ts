import { loadFont as loadMono } from "@remotion/google-fonts/JetBrainsMono";
import { loadFont as loadSans } from "@remotion/google-fonts/Inter";
import { spring, SpringConfig, random } from "remotion";

export const { fontFamily: MONO } = loadMono("normal", {
  weights: ["400", "700"],
  subsets: ["latin"],
});

export const { fontFamily: SANS } = loadSans("normal", {
  weights: ["400", "600", "700", "800"],
  subsets: ["latin"],
});

export const C = {
  void: "#07090e",
  green: "#00ff88",
  cyan: "#00d4ff",
  violet: "#7928ca",
  red: "#ff3b5c",
  gold: "#f5c542",
  dim: "#5b6578",
  white: "#ffffff",
  border: "#1f293d",
};

export const SPRING: Partial<SpringConfig> = {
  damping: 12,
  mass: 0.5,
  stiffness: 100,
};

export const IMPACT_SPRING: Partial<SpringConfig> = {
  damping: 8,
  mass: 0.8,
  stiffness: 180,
};

export const pop = (frame: number, fps: number, delay = 0) =>
  spring({ frame: frame - delay, fps, config: SPRING });

export const popImpact = (frame: number, fps: number, delay = 0) =>
  spring({ frame: frame - delay, fps, config: IMPACT_SPRING });

/** Deterministic pseudo-random in [0,1) for frame-stable effects. */
export const rand = (seed: number) => {
  return random(seed);
};
