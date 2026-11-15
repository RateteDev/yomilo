import { homedir } from "node:os";
import { join } from "node:path";

/**
 * `off` fetches every time and keeps nothing, so an edited page or a changed
 * count shows up on the next render. `use` answers from a saved result when
 * there is one; `refresh` fetches again and saves over it.
 */
export type CacheMode = "off" | "use" | "refresh";

export interface FetchCacheOptions {
  /** Defaults to `off`. */
  cache?: CacheMode;
  /** Defaults to `$XDG_CACHE_HOME/yomilo`, or `~/.cache/yomilo`. */
  cacheDir?: string;
}

/** The user's cache directory outlives reinstalls and is shared by every project. */
export function cacheDir(options: FetchCacheOptions, kind: string): string {
  const root =
    options.cacheDir ?? join(process.env.XDG_CACHE_HOME || join(homedir(), ".cache"), "yomilo");
  return join(root, kind);
}
