export async function withTimeout<T>(promise: Promise<T>, ms: number, what: string): Promise<T> {
  let timer: Timer | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error(`${what} timed out after ${ms} ms`)), ms);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

export interface WaitOptions {
  timeoutMs?: number;
  intervalMs?: number;
}

/**
 * Calls `check` until it returns a value other than `undefined`. For state inside the VM,
 * which the host cannot subscribe to; each call is a round trip over SSH.
 */
export async function waitFor<T>(
  what: string,
  check: () => Promise<T | undefined>,
  { timeoutMs = 30_000, intervalMs = 250 }: WaitOptions = {},
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const value = await check();
    if (value !== undefined) return value;
    if (Date.now() >= deadline)
      throw new Error(`timed out after ${timeoutMs} ms waiting for ${what}`);
    await Bun.sleep(intervalMs);
  }
}
