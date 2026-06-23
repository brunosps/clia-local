import { useEffect, useRef, useState } from "react";
import cliaSplashLogoUrl from "./assets/brand/clia-dev-splash.svg";
import { useI18n } from "./i18n";

const REVEAL_MS = 260;
const HOLD_MS = 2200; // enough time to read the brand without feeling stuck
const FADE_MS = 400; // overlay fade-out

type Phase = "forming" | "out";

export function SplashScreen({ onDone }: { onDone: () => void }) {
  const { t } = useI18n();
  const [revealed, setRevealed] = useState(false);
  const [phase, setPhase] = useState<Phase>("forming");
  const doneRef = useRef(false);
  const timers = useRef<number[]>([]);

  // Run the animation timeline once on mount. State changes happen inside
  // timer callbacks (not synchronously in the effect body).
  useEffect(() => {
    const clearAll = () => {
      timers.current.forEach((id) => window.clearTimeout(id));
      timers.current = [];
    };
    const finish = () => {
      if (doneRef.current) return;
      doneRef.current = true;
      onDone();
    };

    const reduceMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    if (reduceMotion) {
      timers.current.push(window.setTimeout(() => setRevealed(true), 0));
      timers.current.push(window.setTimeout(() => setPhase("out"), 500));
      timers.current.push(window.setTimeout(finish, 500 + FADE_MS));
      return clearAll;
    }

    timers.current.push(window.setTimeout(() => setRevealed(true), REVEAL_MS));
    const outAt = REVEAL_MS + HOLD_MS;
    timers.current.push(window.setTimeout(() => setPhase("out"), outAt));
    timers.current.push(window.setTimeout(finish, outAt + FADE_MS));

    return clearAll;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Skip with click or Escape: jump to fade-out, then finish.
  function skip() {
    if (doneRef.current || phase === "out") return;
    timers.current.forEach((id) => window.clearTimeout(id));
    timers.current = [];
    setRevealed(true);
    setPhase("out");
    timers.current.push(
      window.setTimeout(() => {
        if (doneRef.current) return;
        doneRef.current = true;
        onDone();
      }, FADE_MS),
    );
  }

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") skip();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div
      className={phase === "out" ? "splash out" : "splash"}
      role="status"
      aria-label={t("app.initializing")}
      onClick={skip}
    >
      <div
        className={["splash-brand", revealed ? "shown" : ""].filter(Boolean).join(" ")}
        aria-hidden="true"
      >
        <img className="splash-logo" src={cliaSplashLogoUrl} alt="" />
      </div>
      <div className="splash-caption">{t("app.tagline")}</div>
    </div>
  );
}
