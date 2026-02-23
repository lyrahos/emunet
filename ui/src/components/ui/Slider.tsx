import { useState } from "react";

interface SliderProps {
  value: number;
  onChange: (value: number) => void;
  levels: { label: string; emoji: string; description: string }[];
  showCustom?: boolean;
}

const defaultLevels = [
  { label: "Low", emoji: "\u{1F331}", description: "Minimal resources" },
  { label: "Medium", emoji: "\u{1F33F}", description: "Balanced allocation" },
  { label: "High", emoji: "\u{1F333}", description: "Maximum contribution" },
];

export default function Slider({
  value,
  onChange,
  levels = defaultLevels,
  showCustom = true,
}: SliderProps) {
  const [isCustom, setIsCustom] = useState(false);
  const totalSteps = levels.length - 1;

  const handlePresetClick = (index: number) => {
    setIsCustom(false);
    onChange(index);
  };

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseFloat(e.target.value);
    const isExactStep = levels.some((_, i) => Math.abs(val - i) < 0.01);
    setIsCustom(!isExactStep);
    onChange(val);
  };

  const currentLevel = isCustom
    ? { label: "Custom", emoji: "\u2699\uFE0F", description: "Custom allocation" }
    : levels[Math.round(value)] ?? levels[0];

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-2xl">{currentLevel.emoji}</span>
          <div>
            <p className="font-semibold text-[var(--color-text)]">
              {currentLevel.label}
            </p>
            <p className="text-sm text-[var(--color-text-secondary)]">
              {currentLevel.description}
            </p>
          </div>
        </div>
      </div>

      <input
        type="range"
        min={0}
        max={totalSteps}
        step={showCustom ? 0.01 : 1}
        value={value}
        onChange={handleSliderChange}
        className="w-full h-2 rounded-full appearance-none bg-[var(--color-border)]
          [&::-webkit-slider-thumb]:appearance-none
          [&::-webkit-slider-thumb]:w-5
          [&::-webkit-slider-thumb]:h-5
          [&::-webkit-slider-thumb]:rounded-full
          [&::-webkit-slider-thumb]:bg-[var(--color-accent)]
          [&::-webkit-slider-thumb]:cursor-pointer
          [&::-webkit-slider-thumb]:shadow-md
          [&::-webkit-slider-thumb]:transition-transform
          [&::-webkit-slider-thumb]:hover:scale-110"
      />

      <div className="flex justify-between">
        {levels.map((level, i) => (
          <button
            key={level.label}
            onClick={() => handlePresetClick(i)}
            className={`text-xs px-2 py-1 rounded-md transition-colors
              ${
                !isCustom && Math.round(value) === i
                  ? "text-[var(--color-accent)] font-semibold"
                  : "text-[var(--color-text-secondary)] hover:text-[var(--color-text)]"
              }`}
          >
            {level.emoji} {level.label}
          </button>
        ))}
        {showCustom && (
          <button
            onClick={() => setIsCustom(true)}
            className={`text-xs px-2 py-1 rounded-md transition-colors
              ${
                isCustom
                  ? "text-[var(--color-accent)] font-semibold"
                  : "text-[var(--color-text-secondary)] hover:text-[var(--color-text)]"
              }`}
          >
            \u2699\uFE0F Custom
          </button>
        )}
      </div>
    </div>
  );
}
