// Synthesizes outro SFX and ambient bed, then converts to MP3 using ffmpeg.
import { writeFileSync, mkdirSync } from "node:fs";
import { execSync } from "node:child_process";

const SR = 44100;
mkdirSync("public/audio/sfx", { recursive: true });

function writeWav(filename, seconds, fn) {
  const n = Math.floor(SR * seconds);
  const buf = Buffer.alloc(44 + n * 2);
  buf.write("RIFF", 0);
  buf.writeUInt32LE(36 + n * 2, 4);
  buf.write("WAVE", 8);
  buf.write("fmt ", 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(1, 22);
  buf.writeUInt32LE(SR, 24);
  buf.writeUInt32LE(SR * 2, 28);
  buf.writeUInt16LE(2, 32);
  buf.writeUInt16LE(16, 34);
  buf.write("data", 36);
  buf.writeUInt32LE(n * 2, 40);

  let seed = 42;
  const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296) * 2 - 1;

  for (let i = 0; i < n; i++) {
    const t = i / SR;
    const v = Math.max(-1, Math.min(1, fn(t, seconds, rnd)));
    buf.writeInt16LE(Math.round(v * 32767 * 0.85), 44 + i * 2);
  }
  writeFileSync(filename, buf);
}

const env = (t, a, d) => (t < a ? t / a : Math.exp(-(t - a) / d));

// 1. Whoosh (convergence sweep)
writeWav("public/audio/sfx/whoosh.wav", 1.5, (t, s, r) => {
  const p = t / s;
  const freq = 120 + 600 * Math.pow(p, 2);
  const noise = r() * 0.4;
  const swell = Math.sin(Math.PI * p);
  return (Math.sin(2 * Math.PI * freq * t) * 0.6 + noise) * swell;
});

// 2. Tick (Merkle pair merge)
writeWav("public/audio/sfx/tick.wav", 0.08, (t, _s, r) => {
  return (Math.sin(2 * Math.PI * 2600 * t) * 0.7 + r() * 0.3) * env(t, 0.001, 0.015);
});

// 3. Impact (Frame 240 Ed25519 seal impact)
writeWav("public/audio/sfx/impact.wav", 2.2, (t, _s, r) => {
  const sub = Math.sin(2 * Math.PI * (45 * Math.exp(-t * 1.5)) * t) * env(t, 0.002, 0.6);
  const mid = Math.sin(2 * Math.PI * 180 * t) * env(t, 0.001, 0.15);
  const crack = r() * env(t, 0.0005, 0.03) * 0.8;
  const shimmer = Math.sin(2 * Math.PI * 1760 * t) * env(t, 0.05, 0.7) * 0.25;
  return sub * 0.7 + mid * 0.3 + crack + shimmer;
});

// 4. Chip (Gate chips snap in)
writeWav("public/audio/sfx/chip.wav", 0.12, (t) => {
  const f = t < 0.04 ? 880 : 1320;
  return Math.sin(2 * Math.PI * f * t) * env(t, 0.002, 0.03) * 0.6;
});

// 5. Key (Terminal typing)
writeWav("public/audio/sfx/key.wav", 0.06, (t, _s, r) => {
  return (r() * 0.6 + Math.sin(2 * Math.PI * 350 * t) * 0.4) * env(t, 0.0005, 0.009);
});

// 6. Outro music ambient bed (10.5 seconds)
writeWav("public/audio/outro_music.wav", 10.5, (t, s) => {
  // Deep cyberpunk bass drone (55Hz A1, 110Hz A2) + fifth (165Hz E3) + shimmer
  const swell = t < 1 ? t : t > s - 1 ? (s - t) : 1;
  const d1 = Math.sin(2 * Math.PI * 55 * t);
  const d2 = Math.sin(2 * Math.PI * 110 * t + 0.2);
  const d3 = Math.sin(2 * Math.PI * 165 * t + 0.4) * 0.4;
  const pad = Math.sin(2 * Math.PI * 440 * (1 + 0.005 * Math.sin(t * 1.5)) * t) * 0.15;
  const pulse = (0.9 + 0.1 * Math.sin(t * 3));
  return (d1 * 0.5 + d2 * 0.3 + d3 + pad) * swell * pulse * 0.5;
});

// Convert all generated wavs to mp3 using ffmpeg
const files = [
  ["public/audio/sfx/whoosh.wav", "public/audio/sfx/whoosh.mp3"],
  ["public/audio/sfx/tick.wav", "public/audio/sfx/tick.mp3"],
  ["public/audio/sfx/impact.wav", "public/audio/sfx/impact.mp3"],
  ["public/audio/sfx/chip.wav", "public/audio/sfx/chip.mp3"],
  ["public/audio/sfx/key.wav", "public/audio/sfx/key.mp3"],
  ["public/audio/outro_music.wav", "public/audio/outro_music.mp3"]
];

for (const [wavPath, mp3Path] of files) {
  try {
    execSync(`ffmpeg -y -v error -i "${wavPath}" -b:a 192k "${mp3Path}"`);
    console.log(`Generated ${mp3Path}`);
  } catch (e) {
    console.error(`Failed to convert ${wavPath}:`, e.message);
  }
}

console.log("Outro audio generation complete.");
