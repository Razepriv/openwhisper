import React from "react";

/**
 * OpenVoice brand mark — a 3-bar audio waveform inside a violet rounded
 * square, paired with the wordmark set in Inter.
 *
 * Replaces the prior hand-drawn "handy" wordmark (paths in a 930×328 box).
 * The new mark scales cleanly at any size because it's three rounded
 * rectangles + a square background, and the wordmark uses the system
 * font instead of vector paths so the file ships much smaller.
 *
 * The component keeps the original `HandyTextLogo` name so existing
 * imports in Sidebar.tsx etc. don't need touching — only the rendered
 * output changes.
 */
const HandyTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  // Output aspect ratio ~3.6:1 matches the sidebar's old hand-drawn
  // wordmark, so swapping in here doesn't shift layout. Width drives
  // the mark size; the wordmark scales relative to it.
  const w = typeof width === "number" ? width : 96;
  const h = typeof height === "number" ? height : Math.round(w / 3.6);
  const markSize = h;
  const fontSize = Math.round(h * 0.72);

  return (
    <span
      className={className}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: `${Math.round(h * 0.32)}px`,
        width: `${w}px`,
        height: `${h}px`,
        color: "var(--color-text)",
        fontFamily:
          'Inter, "Segoe UI", system-ui, -apple-system, sans-serif',
        fontWeight: 600,
        fontSize: `${fontSize}px`,
        letterSpacing: "-0.02em",
        lineHeight: 1,
        whiteSpace: "nowrap",
      }}
    >
      <svg
        width={markSize}
        height={markSize}
        viewBox="0 0 32 32"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
        style={{ flexShrink: 0 }}
      >
        <rect
          width="32"
          height="32"
          rx="8"
          className="logo-primary"
        />
        {/* Three white bars — audio-equalizer pattern. Heights chosen so the
            middle bar reads as the strongest peak (typical waveform shape). */}
        <rect x="9" y="13" width="3" height="6" rx="1.5" fill="white" />
        <rect x="14.5" y="9" width="3" height="14" rx="1.5" fill="white" />
        <rect x="20" y="11" width="3" height="10" rx="1.5" fill="white" />
      </svg>
      <span>OpenVoice</span>
    </span>
  );
};

export default HandyTextLogo;
