/**
 * Maps internal protocol terms to user-friendly display names.
 * Follows the HIS Section 1 terminology mapping.
 *
 * Protocol term -> User-facing term
 */
const termMap: Record<string, string> = {
  group: "Space",
  groups: "Spaces",
  group_id: "Space ID",
  owner: "Host",
  owner_role: "Host",
  pik: "identity",
  pik_hash: "identity fingerprint",
  nullifier: "", // hidden from user
  micro_seeds: "Seeds",
  "micro-seeds": "Seeds",
  seed: "Seed",
  seeds: "Seeds",
  blob: "content",
  blob_hash: "content hash",
  relay: "relay node",
  whisper: "Whisper",
  guardian: "recovery contact",
  share: "recovery share",
  abr: "contribution metrics",
  storefront: "Storefront",
  forum: "Forum",
  newsfeed: "News Feed",
  gallery: "Gallery",
  library: "Library",
};

/**
 * Translate a protocol term to its user-facing equivalent.
 * Returns the original term if no mapping exists.
 */
export function t(protocolTerm: string): string {
  const lower = protocolTerm.toLowerCase();
  if (lower in termMap) {
    const mapped = termMap[lower];
    // Empty string means "hidden" (e.g., nullifier)
    return mapped || protocolTerm;
  }
  return protocolTerm;
}

/**
 * Check if a protocol term should be hidden from the user.
 */
export function isHiddenTerm(protocolTerm: string): boolean {
  const lower = protocolTerm.toLowerCase();
  return lower in termMap && termMap[lower] === "";
}

/**
 * Role display names with consistent capitalization.
 */
export const roleLabels: Record<string, string> = {
  owner: "Host",
  host: "Host",
  creator: "Creator",
  moderator: "Moderator",
  member: "Member",
};
