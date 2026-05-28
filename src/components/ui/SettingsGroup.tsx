import React from "react";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

/**
 * SettingsGroup — editorial card container for a labelled group of
 * settings rows.
 *
 * Design refresh (Phase Final.UI): switched from the inline-pink-fill
 * style to a quieter editorial card:
 *
 *  - Title sits ABOVE the card in a small caps style (typographic
 *    "section header") rather than competing with the card chrome.
 *  - Card uses `bg-background-elevated` so it stands above the page
 *    background by a single shade — readable in both light + dark.
 *  - Hairline border (`border-border`) instead of a soft tint —
 *    matches the Linear/Vercel/Stripe aesthetic.
 *  - Rows are separated by `divide-border` at 1px so the boundaries
 *    feel intentional rather than spilling-into-each-other.
 */
export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <section className="space-y-3">
      {title && (
        <header className="px-1">
          <h2 className="text-[11px] font-semibold text-text-muted uppercase tracking-[0.08em]">
            {title}
          </h2>
          {description && (
            <p className="text-xs text-text-muted mt-1.5 leading-relaxed">
              {description}
            </p>
          )}
        </header>
      )}
      <div className="bg-background-elevated border border-border rounded-xl overflow-visible shadow-[0_1px_2px_rgba(0,0,0,0.04)]">
        <div className="divide-y divide-border">{children}</div>
      </div>
    </section>
  );
};
