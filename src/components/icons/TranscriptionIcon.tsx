import React from "react";

/**
 * Widget transcribing-state icon — three horizontal dots that universally
 * read as "processing / loading". Pairs with the equalizer bars used for
 * the active recording state so users can tell at a glance which phase
 * the pipeline is in: bars = listening, dots = working on it.
 *
 * Component name kept as TranscriptionIcon so RecordingOverlay.tsx
 * imports don't need touching.
 */
interface TranscriptionIconProps {
  width?: number;
  height?: number;
  color?: string;
  className?: string;
}

const TranscriptionIcon: React.FC<TranscriptionIconProps> = ({
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
      {/* Three dots — left fades a little so the row reads as "loading",
          matching the pulsing "Transcribing… 3s" copy beside it. */}
      <circle cx="6" cy="12" r="1.75" opacity="0.55" />
      <circle cx="12" cy="12" r="1.75" opacity="0.8" />
      <circle cx="18" cy="12" r="1.75" />
    </svg>
  );
};

export default TranscriptionIcon;
