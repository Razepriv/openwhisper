import React from "react";

/**
 * Widget mic-state icon — three rounded bars in the equalizer pattern that
 * matches the OpenVoice brand mark (favicon, .ico, taskbar, HandyTextLogo).
 *
 * The component name is kept as `MicrophoneIcon` so existing imports in
 * RecordingOverlay.tsx don't need touching — only the rendered glyph
 * changes. Visually consistent with the violet rounded-square logo on the
 * landing page and Windows shell.
 */
interface MicrophoneIconProps {
  width?: number;
  height?: number;
  color?: string;
  className?: string;
}

const MicrophoneIcon: React.FC<MicrophoneIconProps> = ({
  width = 24,
  height = 24,
  color = "#a78bfa",
  className = "",
}) => {
  return (
    <svg
      width={width}
      height={height}
      viewBox="0 0 24 24"
      fill={color}
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      {/* Three rounded bars — short, tall, medium — same proportions as the
          brand mark (just sized up from a 32-viewbox to a 24-viewbox). */}
      <rect x="5" y="9.5" width="2.5" height="5" rx="1.25" />
      <rect x="10.75" y="5.5" width="2.5" height="13" rx="1.25" />
      <rect x="16.5" y="7.5" width="2.5" height="9" rx="1.25" />
    </svg>
  );
};

export default MicrophoneIcon;
