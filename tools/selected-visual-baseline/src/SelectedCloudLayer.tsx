import { Suspense, memo, useEffect, useMemo, useRef } from "react";
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import { Cloud, Clouds } from "@selected/drei-cloud";
import { MathUtils, type Group } from "three";

interface SelectedCloudLayerProps {
  enabled: boolean;
  dispersing: boolean;
  intensity: number;
  muted: boolean;
  reducedMotion: boolean;
  lowPerformance: boolean;
  foreground: boolean;
  foregroundStrength: number;
  clipToComponents?: boolean;
}

interface CloudFieldProps {
  dispersing: boolean;
  muted: boolean;
  reducedMotion: boolean;
  lowPerformance: boolean;
}

const CLOUD_LIMIT = 72;
const LOW_CLOUD_LIMIT = 42;
const CLOUD_TEXTURE_URL = "/assets/drei-cloud.png";
const CLOUD_FRAME_INTERVAL = 1000 / 60;

function CloudFrameDriver({ active }: { active: boolean }) {
  const invalidate = useThree((state) => state.invalidate);

  useEffect(() => {
    // Always publish one frame after a state transition. When the cloud field
    // is parked this becomes its final, fully resolved image.
    invalidate();
    if (!active) return undefined;

    let frame = 0;
    let last = performance.now();
    let accumulated = CLOUD_FRAME_INTERVAL;
    const tick = (now: number) => {
      accumulated += Math.min(100, now - last);
      last = now;
      if (accumulated >= CLOUD_FRAME_INTERVAL) {
        // Preserve elapsed time instead of resetting it. This produces an
        // even 60Hz cadence on 120/144/240Hz displays rather than aliasing to
        // an integer divisor of the monitor refresh rate.
        accumulated %= CLOUD_FRAME_INTERVAL;
        invalidate();
      }
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(frame);
  }, [active, invalidate]);

  return null;
}

interface CloudMotion {
  originX: number;
  originY: number;
  phase: number;
  speed: number;
  driftX: number;
  driftY: number;
  driftCrossX: number;
  driftCrossY: number;
  scatterX: number;
  scatterY: number;
}

const TAU = Math.PI * 2;

function createCloudMotion(originX: number, originY: number, outwardSide: -1 | 1): CloudMotion {
  const driftAngle = Math.random() * TAU;
  const driftAmplitude = 0.16 + (Math.random() * 0.32);
  const scatterAngle = (Math.random() - 0.5) * 0.72;
  const scatterDistance = 3.7 + (Math.random() * 2.1);

  return {
    originX,
    originY,
    phase: Math.random() * TAU,
    // Deliberately slow: this is a background wind, not a foreground
    // animation competing with the research surface.
    speed: 0.045 + (Math.random() * 0.065),
    driftX: Math.cos(driftAngle) * driftAmplitude,
    driftY: Math.sin(driftAngle) * driftAmplitude * 0.42,
    driftCrossX: Math.sin(driftAngle) * driftAmplitude * 0.34,
    driftCrossY: Math.cos(driftAngle) * driftAmplitude * 0.18,
    // Preserve the outward gesture while allowing each cloud to choose a
    // slightly different vertical/diagonal exit direction.
    scatterX: outwardSide * Math.cos(scatterAngle) * scatterDistance,
    scatterY: Math.sin(scatterAngle) * scatterDistance * 0.42,
  };
}

function CloudField({ dispersing, muted, reducedMotion, lowPerformance }: CloudFieldProps) {
  const leftCloudRef = useRef<Group>(null);
  const rightCloudRef = useRef<Group>(null);
  const dispersalRef = useRef(dispersing ? 1 : 0);
  const motionTimeRef = useRef(0);
  const cloudMotion = useMemo(
    () => ({
      right: createCloudMotion(1, 0, 1),
      left: createCloudMotion(-4.4, -0.2, -1),
    }),
    [],
  );

  // Reuse R3F's existing render loop: the clouds physically part instead of
  // merely fading as one flat layer, without creating another window RAF.
  useFrame((_, delta) => {
    motionTimeRef.current += Math.min(delta, 0.1);
    const target = dispersing ? 1 : 0;
    const progress = MathUtils.damp(dispersalRef.current, target, dispersing ? 2.7 : 3.6, delta);
    dispersalRef.current = progress;

    const time = motionTimeRef.current;
    const drift = (motion: CloudMotion) => ({
      x: (Math.sin((time * motion.speed) + motion.phase) * motion.driftX)
        + (Math.cos((time * motion.speed * 0.57) + (motion.phase * 0.71)) * motion.driftCrossX),
      y: (Math.cos((time * motion.speed * 0.83) + motion.phase) * motion.driftY)
        + (Math.sin((time * motion.speed * 0.49) + (motion.phase * 0.53)) * motion.driftCrossY),
    });

    const left = leftCloudRef.current;
    const right = rightCloudRef.current;
    if (left) {
      const leftDrift = drift(cloudMotion.left);
      left.position.x = cloudMotion.left.originX + leftDrift.x + (progress * cloudMotion.left.scatterX);
      left.position.y = cloudMotion.left.originY + leftDrift.y + (progress * cloudMotion.left.scatterY);
      left.scale.set(1 + (progress * 0.22), 1 - (progress * 0.18), 1);
    }
    if (right) {
      const rightDrift = drift(cloudMotion.right);
      right.position.x = cloudMotion.right.originX + rightDrift.x + (progress * cloudMotion.right.scatterX);
      right.position.y = cloudMotion.right.originY + rightDrift.y + (progress * cloudMotion.right.scatterY);
      right.scale.set(1 + (progress * 0.28), 1 - (progress * 0.2), 1);
    }
  });

  const cloudMotionPaused = reducedMotion || muted;

  return (
    <group position={[0, 2.65, 0]} scale={[1.15, 1.15, 1.15]}>
      {/*
       * Keep every cloud in one provider. Drei's <Clouds> owns one
       * instanced mesh and one material; using a provider per cloud group
       * duplicates the draw submission and texture bookkeeping without
       * improving the image.
       */}
      <Clouds
        texture={CLOUD_TEXTURE_URL}
        limit={lowPerformance ? LOW_CLOUD_LIMIT : CLOUD_LIMIT}
        frustumCulled
      >
        <group ref={rightCloudRef}>
          {/* Back shell: a wide, low-density volume that establishes depth. */}
          <Cloud
            seed={4}
            segments={lowPerformance ? 8 : 16}
            bounds={[5.8, 1.8, 2.8]}
            volume={2.5}
            smallestVolume={0.38}
            color="#a9c1ba"
            opacity={0.34}
            fade={18}
            speed={cloudMotionPaused ? 0 : 0.038}
            growth={0.52}
          />
          {/* Main billow: compact enough to read as a cloud body, not a fog
           * sheet; the larger Y/Z bounds provide visible thickness. */}
          <Cloud
            seed={17}
            segments={lowPerformance ? 12 : 22}
            bounds={[4.4, 2.4, 2.8]}
            volume={2.25}
            smallestVolume={0.46}
            color="#e4f2ed"
            opacity={0.76}
            fade={16}
            speed={cloudMotionPaused ? 0 : 0.052}
            growth={0.72}
            position={[0.1, 0.12, 0.05]}
          />
          {/* Front puffs catch the light and make the cloud read in depth. */}
          <Cloud
            seed={31}
            segments={lowPerformance ? 6 : 12}
            bounds={[2.7, 1.8, 2.3]}
            volume={1.65}
            smallestVolume={0.42}
            color="#f1fbf7"
            opacity={0.56}
            fade={14}
            speed={cloudMotionPaused ? 0 : 0.067}
            growth={0.48}
            position={[1.05, 0.25, 0.85]}
          />
        </group>
        <group ref={leftCloudRef} position={[-4.4, -0.2, -1]}>
          {/* A cooler, smaller counter-volume keeps the horizon balanced. */}
          <Cloud
            seed={9}
            segments={lowPerformance ? 4 : 7}
            bounds={[3.8, 1.4, 2.1]}
            volume={2.1}
            smallestVolume={0.36}
            color="#8da9a1"
            opacity={0.28}
            fade={18}
            speed={cloudMotionPaused ? 0 : 0.028}
            growth={0.42}
          />
          <Cloud
            seed={23}
            segments={lowPerformance ? 7 : 10}
            bounds={[2.9, 1.9, 2.1]}
            volume={1.85}
            smallestVolume={0.4}
            color="#c6ddd5"
            opacity={0.56}
            fade={15}
            speed={cloudMotionPaused ? 0 : 0.043}
            growth={0.5}
            position={[-0.35, 0.18, 0.1]}
          />
          <Cloud
            seed={41}
            segments={lowPerformance ? 3 : 5}
            bounds={[1.8, 1.2, 1.5]}
            volume={1.3}
            smallestVolume={0.36}
            color="#edf8f3"
            opacity={0.4}
            fade={13}
            speed={cloudMotionPaused ? 0 : 0.058}
            growth={0.34}
            position={[-1.1, 0.15, 0.72]}
          />
        </group>
      </Clouds>
    </group>
  );
}

function SelectedCloudLayer({
  enabled,
  dispersing,
  intensity,
  muted,
  reducedMotion,
  lowPerformance,
  foreground,
  foregroundStrength,
  clipToComponents = false,
}: SelectedCloudLayerProps) {
  if (!enabled) {
    return <div className={`cloud-layer ${foreground ? "is-foreground" : ""}`} data-source="drei-disabled" aria-hidden="true" />;
  }

  // Keep the cloud field visible while it physically disperses. The CSS
  // transition on `.is-dispersing` fades the already-separated field only
  // after the motion has begun, avoiding a hard disappearance.
  // Cloud sprites have much softer source alpha than the raw rain texture.
  // A perceptual curve keeps both effects visible at the same restrained
  // foreground setting without forcing the rain to become too dense.
  const placementStrength = foreground ? Math.sqrt(foregroundStrength) * 0.78 : 1;
  const opacity = enabled ? intensity * placementStrength * (muted ? 0.18 : 1) : 0;

  return (
    <div
      className={`cloud-layer ${foreground ? "is-foreground" : ""} ${dispersing ? "is-dispersing" : ""}`}
      data-liquid-media-only="true"
      data-source="drei-ffa15b956e32"
      data-rain-composition={foreground ? (clipToComponents ? "surface" : "foreground") : "behind"}
      style={{
        opacity,
        clipPath: foreground && clipToComponents ? "url(#mindscape-rain-surface-clip)" : undefined,
      }}
      aria-hidden="true"
    >
      <Canvas
        camera={{ position: [0, 0, 9], fov: 42 }}
        dpr={lowPerformance ? 1 : [1, 1.5]}
        // Focused reading/input already fades the atmosphere. Stop the
        // simulation while it is visually de-emphasised; the next visible
        // frame is still rendered at the original DPR and segment count.
        // A dispersal is a real positional animation, so keep R3F's loop
        // alive even though focused mode normally parks the cloud simulation.
        // Drei updates and depth-sorts every cloud segment on each rendered
        // frame. Demand mode plus the driver below preserves a full 60Hz
        // image while preventing 120/144/240Hz monitors from repeating the
        // same slow cloud motion multiple times per visual frame.
        frameloop="demand"
        // Cloud sprites already carry a soft alpha edge. MSAA does not
        // improve that texture, but it does add a full-screen resolve pass.
        // Keep the original texture/segment quality and let the browser pick
        // the high-performance adapter when one is available.
        gl={{
          alpha: true,
          antialias: false,
          powerPreference: "high-performance",
          // LiquidGlass samples this WebGL canvas on its own staggered render
          // cadence. Without a preserved back buffer, that read can land
          // after browser presentation has cleared the cloud frame, causing
          // transparent/complete samples to alternate inside glass panels.
          preserveDrawingBuffer: true,
        }}
      >
        <CloudFrameDriver active={!reducedMotion && (!muted || dispersing)} />
        <ambientLight intensity={1.9} />
        <directionalLight position={[-4, 5, 6]} intensity={3.2} color="#f2fff9" />
        <Suspense fallback={null}>
          <CloudField
            dispersing={dispersing}
            muted={muted}
            reducedMotion={reducedMotion}
            lowPerformance={lowPerformance}
          />
        </Suspense>
      </Canvas>
    </div>
  );
}

export default memo(SelectedCloudLayer);
