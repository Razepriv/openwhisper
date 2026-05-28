/**
 * Sidebar "General" icon — a 3-bar audio waveform.
 *
 * Replaces the prior hand glyph (now renamed in spirit — the component is
 * still called HandyHand so Sidebar.tsx imports don't churn, but it now
 * renders the soundwave that matches the OpenVoice brand mark).
 */
const HandyHand = ({
  width,
  height,
}: {
  width?: number | string;
  height?: number | string;
}) => (
  <svg
    width={width || 16}
    height={height || 16}
    viewBox="0 0 24 24"
    className="stroke-text"
    fill="currentColor"
    xmlns="http://www.w3.org/2000/svg"
  >
    {/* Three rounded bars in an equalizer pattern — short, tall, medium.
        Matches the brand-mark SVG used elsewhere (just without the
        rounded-square background). */}
    <rect x="6" y="9" width="2.5" height="6" rx="1.25" />
    <rect x="10.75" y="5" width="2.5" height="14" rx="1.25" />
    <rect x="15.5" y="7" width="2.5" height="10" rx="1.25" />
  </svg>
);

export default HandyHand;
