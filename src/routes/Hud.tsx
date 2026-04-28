import { useEffect } from "react";
import { HudPill } from "@/components/HudPill";

export default function Hud() {
  useEffect(() => {
    document.body.classList.add("hud");
    return () => document.body.classList.remove("hud");
  }, []);
  return <HudPill />;
}
