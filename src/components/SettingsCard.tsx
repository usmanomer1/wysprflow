import * as React from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

interface SettingsCardProps {
  title: string;
  icon?: LucideIcon;
  description?: string;
  comingSoon?: string;
  children?: React.ReactNode;
  className?: string;
}

export function SettingsCard({
  title,
  icon: Icon,
  description,
  comingSoon,
  children,
  className,
}: SettingsCardProps) {
  const isPlaceholder = !!comingSoon && !children;
  return (
    <section
      className={cn(
        "rounded-lg border border-border/60 bg-card/40 p-4 transition-colors",
        isPlaceholder && "opacity-60",
        className,
      )}
    >
      <header className="mb-2 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          {Icon ? <Icon className="size-4 text-foreground" /> : null}
          <h3 className="text-sm font-semibold tracking-tight">{title}</h3>
        </div>
        {comingSoon ? (
          <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
            {comingSoon}
          </span>
        ) : null}
      </header>
      {description ? (
        <p className={cn("text-xs text-muted-foreground", children && "mb-3")}>
          {description}
        </p>
      ) : null}
      {children}
    </section>
  );
}
