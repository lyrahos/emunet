const MICRO_SEEDS_PER_SEED = 100_000_000;

/**
 * Format micro-seeds amount to a human-readable "X.XX Seeds" string.
 * Examples:
 *   formatSeeds(100_000_000)  -> "1.00 Seeds"
 *   formatSeeds(50_000_000)   -> "0.50 Seeds"
 *   formatSeeds(1_234_567)    -> "0.01 Seeds"
 *   formatSeeds(0)            -> "0.00 Seeds"
 */
export function formatSeeds(microSeeds: number): string {
  const seeds = microSeeds / MICRO_SEEDS_PER_SEED;
  if (seeds >= 1_000_000) {
    return `${(seeds / 1_000_000).toFixed(2)}M Seeds`;
  }
  if (seeds >= 1_000) {
    return `${(seeds / 1_000).toFixed(2)}K Seeds`;
  }
  return `${seeds.toFixed(2)} Seeds`;
}

/**
 * Format micro-seeds as a short string for compact displays.
 */
export function formatSeedsShort(microSeeds: number): string {
  const seeds = microSeeds / MICRO_SEEDS_PER_SEED;
  if (seeds >= 1_000_000) return `${(seeds / 1_000_000).toFixed(1)}M`;
  if (seeds >= 1_000) return `${(seeds / 1_000).toFixed(1)}K`;
  if (seeds >= 1) return seeds.toFixed(2);
  if (seeds > 0) return `< 0.01`;
  return "0";
}

/**
 * Format a timestamp to relative time (e.g., "2m ago", "3h ago", "5d ago").
 */
export function formatRelativeTime(epochMs: number): string {
  const now = Date.now();
  const diff = now - epochMs;

  if (diff < 0) return "just now";

  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return "just now";

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;

  const weeks = Math.floor(days / 7);
  if (weeks < 4) return `${weeks}w ago`;

  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;

  const years = Math.floor(days / 365);
  return `${years}y ago`;
}

/**
 * Format a byte count to human-readable string.
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

/**
 * Truncate a hash for display (first 8 + last 4 chars).
 */
export function truncateHash(hash: string, prefixLen = 8, suffixLen = 4): string {
  if (hash.length <= prefixLen + suffixLen + 3) return hash;
  return `${hash.slice(0, prefixLen)}...${hash.slice(-suffixLen)}`;
}
