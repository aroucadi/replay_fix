import { Composition } from "remotion";
import { IntroComposition } from "./IntroComposition";
import { OutroComposition } from "./outro/OutroComposition";
import "./index.css";

export const FPS = 60;
export const INTRO_DURATION = 1200; // 20s
export const OUTRO_DURATION = 600;  // 10s

export const Root: React.FC = () => (
  <>
    <Composition
      id="IntroComposition"
      component={IntroComposition}
      durationInFrames={INTRO_DURATION}
      fps={FPS}
      width={1920}
      height={1080}
    />
    <Composition
      id="OutroComposition"
      component={OutroComposition}
      durationInFrames={OUTRO_DURATION}
      fps={FPS}
      width={1920}
      height={1080}
    />
  </>
);
