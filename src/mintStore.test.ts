import { describe, it, expect } from 'vitest';
import { MemoryMintStore, RedisMintStore, TRY_RECORD_LUA } from './mintStore';

describe('MemoryMintStore', () => {
  it('enforces the limit atomically inside tryRecord', async () => {
    const store = new MemoryMintStore();
    const now = 1_000_000;
    const cutoff = now - 60_000;

    expect(await store.tryRecord('a', now, cutoff, 2)).toEqual({ allowed: true, count: 1 });
    expect(await store.tryRecord('a', now + 1, cutoff, 2)).toEqual({ allowed: true, count: 2 });
    expect(await store.tryRecord('a', now + 2, cutoff, 2)).toEqual({
      allowed: false,
      oldestRecent: now,
    });
  });

  it('prunes timestamps at or before the cutoff', async () => {
    const store = new MemoryMintStore();
    await store.tryRecord('a', 1000, 0, 5);
    await store.tryRecord('a', 2000, 0, 5);

    expect(await store.getRecent('a', 1500)).toEqual([2000]);
  });

  it('counts two records with the same timestamp as two mints', async () => {
    const store = new MemoryMintStore();
    const now = 1_000_000;
    const cutoff = now - 60_000;

    expect(await store.tryRecord('a', now, cutoff, 5)).toEqual({ allowed: true, count: 1 });
    expect(await store.tryRecord('a', now, cutoff, 5)).toEqual({ allowed: true, count: 2 });
    expect(await store.getRecent('a', cutoff)).toEqual([now, now]);
  });

  it('records same-millisecond mints with unique members in Redis', async () => {
    // ZADD with the timestamp as member collapses same-ms mints into one;
    // the script must use a unique member with the timestamp as score.
    expect(TRY_RECORD_LUA).toContain('ARGV[5]');
    expect(TRY_RECORD_LUA).not.toMatch(/ZADD.*ARGV\[2\],\s*ARGV\[2\]/);

    const store = new RedisMintStore('redis://127.0.0.1:6379') as unknown as {
      client: { eval: (...args: unknown[]) => Promise<[number, number]> };
    };
    const seenMembers = new Set<string>();
    store.client = {
      eval: (async (_script: unknown, opts: { arguments: string[] }) => {
        seenMembers.add(opts.arguments[4]);
        return [1, seenMembers.size];
      }) as never,
    };
    // Bypass connection handling for the unit test.
    const proto = RedisMintStore.prototype as unknown as { ensureConnected: () => Promise<void> };
    const orig = proto.ensureConnected;
    proto.ensureConnected = async () => undefined;
    try {
      const now = 1_000_000;
      await (store as unknown as RedisMintStore).tryRecord('a', now, now - 60_000, 5);
      await (store as unknown as RedisMintStore).tryRecord('a', now, now - 60_000, 5);
      expect(seenMembers.size).toBe(2);
      for (const m of seenMembers) {
        expect(m.startsWith(`${now}-`)).toBe(true);
      }
    } finally {
      proto.ensureConnected = orig;
    }
  });
});
