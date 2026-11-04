// Synthesizes the SFX used by the intro (no external assets, fully deterministic).
import { writeFileSync, mkdirSync } from "node:fs";

const SR = 44100;
mkdirSync("public/sfx", { recursive: true });

function wav(name, seconds, fn) {
  const n = Math.floor(SR * seconds);
  const buf = Buffer.alloc(44 + n * 2);
  buf.write("RIFF", 0); buf.writeUInt32LE(36 + n * 2, 4); buf.write("WAVE", 8);
  buf.write("fmt ", 12); buf.writeUInt32LE(16, 16); buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(1, 22); buf.writeUInt32LE(SR, 24); buf.writeUInt32LE(SR * 2, 28);
  buf.writeUInt16LE(2, 32); buf.writeUInt16LE(16, 34); buf.write("data", 36);
  buf.writeUInt32LE(n * 2, 40);
  let seed = 12345;
  const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296) * 2 - 1;
  for (let i = 0; i < n; i++) {
    const t = i / SR;
    const v = Math.max(-1, Math.min(1, fn(t, seconds, rnd)));
    buf.writeInt16LE(Math.round(v * 32767 * 0.8), 44 + i * 2);
  }
  writeFileSync(`public/sfx/${name}.wav`, buf);
}

const env = (t, a, d) => (t < a ? t / a : Math.exp(-(t - a) / d));

// Key clack: short noise burst + low thump
wav("clack", 0.08, (t, _s, r) => r() * env(t, 0.001, 0.008) * 0.6 + Math.sin(2 * Math.PI * 180 * t) * env(t, 0.001, 0.02) * 0.5);
// Sub-bass drop: falling sine
wav("drop", 1.6, (t) => Math.sin(2 * Math.PI * (30 + 90 * Math.exp(-t * 3)) * t) * env(t, 0.01, 0.6));
// Scan hum: 110Hz + harmonics with slow swell
wav("hum", 4, (t, s) => {
  const sw = Math.sin((Math.PI * t) / s);
  return (Math.sin(2 * Math.PI * 110 * t) * 0.5 + Math.sin(2 * Math.PI * 220 * t + Math.sin(t * 6)) * 0.25) * sw * 0.5;
});
// Lock-on beep
wav("beep", 0.25, (t) => Math.sin(2 * Math.PI * 1320 * t) * env(t, 0.002, 0.08));
// Glitch: bit-crushed noise
wav("glitch", 0.4, (t, _s, r) => Math.round(r() * 4) / 4 * env(t, 0.001, 0.12) * (Math.sin(t * 90) > 0 ? 1 : 0.2));
// Stamp: impact + shimmer
wav("stamp", 1.5, (t, _s, r) => Math.sin(2 * Math.PI * 55 * t) * env(t, 0.002, 0.25) + r() * env(t, 0.001, 0.03) * 0.5 + Math.sin(2 * Math.PI * 1760 * t) * env(t, 0.05, 0.5) * 0.15);
console.log("sfx generated");
