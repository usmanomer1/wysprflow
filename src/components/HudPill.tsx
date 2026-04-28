import { useEffect, useRef, useState } from "react";
import { Mic, AlertCircle, Pencil } from "lucide-react";
import { events, type HudStateEvent } from "@/lib/tauri";
import { cn } from "@/lib/utils";

type HudPhase = "idle" | "initializing" | "listening" | "processing" | "error";

const PHASE_WIDTHS: Record<HudPhase, number> = {
  idle: 92,
  initializing: 92,
  listening: 92,
  processing: 132,
  error: 92,
};

export function HudPill() {
  const [event, setEvent] = useState<HudStateEvent>({ state: "idle" });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await events.onHudState(setEvent);
    })();
    return () => unlisten?.();
  }, []);

  const phase: HudPhase = (event.state as HudPhase) ?? "idle";
  const width = PHASE_WIDTHS[phase] ?? 92;

  return (
    <div className="flex h-full w-full items-start justify-center">
      <div
        className="flex h-[38px] items-center justify-center overflow-hidden bg-black px-3 text-white shadow-[0_8px_24px_rgba(0,0,0,0.35)] transition-[width] duration-200 ease-out rounded-b-[12px]"
        style={{ width: `${width}px` }}
      >
        {phase === "idle" && <IdleHint />}
        {phase === "initializing" && <InitializingDots />}
        {phase === "listening" && <Waveform level={event.level ?? 0} pulsing />}
        {phase === "processing" && <ProcessingIndicator message={event.message} />}
        {phase === "error" && <ErrorIndicator message={event.message} />}
      </div>
    </div>
  );
}

function IdleHint() {
  return (
    <div className="flex items-center gap-1.5 text-white/55">
      <Mic className="size-3" />
      <span className="text-[10px] font-medium tracking-wide">Hold Fn</span>
    </div>
  );
}

function InitializingDots() {
  const [active, setActive] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setActive((a) => (a + 1) % 3), 500);
    return () => clearInterval(t);
  }, []);
  return (
    <div className="flex items-center gap-1">
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          className={cn(
            "size-[4.5px] rounded-full transition-opacity duration-300",
            active === i ? "bg-white/90" : "bg-white/25",
          )}
        />
      ))}
    </div>
  );
}

const BAR_COUNT = 9;
// Center-weighted multipliers from FreeFlow — center bar tallest, edges shortest.
const MULTIPLIERS = [0.35, 0.55, 0.75, 0.9, 1.0, 0.9, 0.75, 0.55, 0.35];
const MAX_BAR = 18;
const MIN_BAR = 2;

/** Audio-driven 9-bar waveform with traveling-wave + shimmer shimmer overlay. */
function Waveform({ level, pulsing = false }: { level: number; pulsing?: boolean }) {
  const time = useAnimationClock(pulsing);
  return (
    <div className="flex h-5 items-center gap-[2.5px]">
      {Array.from({ length: BAR_COUNT }).map((_, i) => {
        const baseAmp = Math.min(level * MULTIPLIERS[i], 1);
        let amp = baseAmp;
        if (pulsing) {
          const traveling = 0.5 + 0.5 * Math.sin(time * 6.2 - i * 0.78);
          const shimmer = 0.5 + 0.5 * Math.sin(time * 3.1 + i * 0.5);
          const pulse = traveling * 0.22 + shimmer * 0.06;
          const sat = baseAmp * (0.74 + pulse);
          const quiet = (1 - baseAmp) * (0.04 + pulse * 0.28);
          amp = Math.min(sat + quiet, 1);
        }
        const h = MIN_BAR + (MAX_BAR - MIN_BAR) * amp;
        return (
          <span
            key={i}
            style={{ height: `${h}px` }}
            className="w-[3px] rounded-full bg-white"
          />
        );
      })}
    </div>
  );
}

/** Sine-driven shimmer for the "thinking / cleaning up" phase, no audio input. */
function ProcessingIndicator({ message }: { message?: string }) {
  const label = message?.trim() || "Working";
  return (
    <div className="flex items-center gap-2">
      <ProcessingWaveform />
      <span className="text-[10px] font-medium text-white/80">{label}</span>
    </div>
  );
}

function ProcessingWaveform() {
  const time = useAnimationClock(true);
  return (
    <div className="flex h-5 items-center gap-[2.5px]">
      {Array.from({ length: BAR_COUNT }).map((_, i) => {
        const wave = 0.5 + 0.5 * Math.sin(time * 5.6 - i * 0.5);
        const shimmer = 0.5 + 0.5 * Math.sin(time * 2.8 + i * 0.75);
        const amp = Math.min(0.16 + wave * MULTIPLIERS[i] * 0.52 + shimmer * 0.08, 1);
        const h = MIN_BAR + (MAX_BAR - MIN_BAR) * amp;
        const opacity = 0.45 + wave * 0.5;
        return (
          <span
            key={i}
            style={{ height: `${h}px`, opacity }}
            className="w-[3px] rounded-full bg-white"
          />
        );
      })}
    </div>
  );
}

function ErrorIndicator({ message }: { message?: string }) {
  return (
    <div className="flex items-center gap-1.5 text-red-200">
      <AlertCircle className="size-3" />
      <span className="text-[10px] font-medium">{message ?? "Error"}</span>
    </div>
  );
}

/** Reserved for command/edit-mode HUD content. Used in Phase 3. */
export function CommandModeIndicator() {
  return <Pencil className="size-3 text-white/90" />;
}

/** rAF-driven monotonic clock in seconds. Pauses when the page is hidden. */
function useAnimationClock(running: boolean) {
  const [t, setT] = useState(0);
  const rafRef = useRef<number>(0);

  useEffect(() => {
    if (!running) return;
    const start = performance.now();
    const tick = (now: number) => {
      setT((now - start) / 1000);
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [running]);

  return t;
}
