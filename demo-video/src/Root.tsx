import { Composition } from "remotion";
import { IntroComposition } from "./IntroComposition";
import "./index.css";

export const FPS = 60;
export const DURATION = 1200;

export const Root: React.FC = () => (
  <Composition
    id="IntroComposition"
    component={IntroComposition}
    durationInFrames={DURATION}
    fps={FPS}
    width={1920}
    height={1080}
  />
);
