import type { ReactNode } from "react";

// Shared "terminal panel" shell (hairline border, header bar) so the same
// className is not repeated across every view.
export function Panel({
  title,
  badge,
  children,
  className = "",
  bodyClassName = "",
}: {
  title: string;
  badge?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
}) {
  return (
    <div className={`border border-term-line bg-term-panel ${className}`}>
      <h3 className="m-0 px-3 py-1.5 text-[11px] tracking-[0.1em] uppercase text-term-faint border-b border-term-line bg-term-panel-raised flex items-center justify-between font-mono">
        <span>{title}</span>
        {badge}
      </h3>
      <div className={`p-3 ${bodyClassName}`}>{children}</div>
    </div>
  );
}
