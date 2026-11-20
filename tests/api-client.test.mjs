import { test } from "node:test";
import assert from "node:assert/strict";
import { apiRequest } from "../src/api.js";

test("returns committed backend data without generating local task records", async (t) => {
  t.mock.method(
    globalThis,
    "fetch",
    async () =>
      new Response(JSON.stringify({ id: "server-id", revision: 3 }), {
        status: 201,
      }),
  );
  assert.deepEqual(
    await apiRequest("/api/tasks", { method: "POST", body: "{}" }),
    { id: "server-id", revision: 3 },
  );
});

test("preserves a backend conflict message instead of reporting success", async (t) => {
  t.mock.method(
    globalThis,
    "fetch",
    async () =>
      new Response(
        JSON.stringify({
          error: { code: "conflict", message: "The local worker is busy." },
        }),
        { status: 409 },
      ),
  );
  await assert.rejects(
    apiRequest("/api/tasks/id/runs", { method: "POST", body: "{}" }),
    /local worker is busy/,
  );
});

test("reports an unavailable proxy or HTML response clearly", async (t) => {
  t.mock.method(
    globalThis,
    "fetch",
    async () => new Response("<html>Unavailable</html>", { status: 502 }),
  );
  await assert.rejects(
    apiRequest("/api/workspace"),
    /backend did not return a valid response/,
  );
});

test("does not retry a failed mutation automatically", async (t) => {
  const spy = t.mock.method(globalThis, "fetch", async () => {
    throw new TypeError("Network failure");
  });
  await assert.rejects(
    apiRequest("/api/tasks", { method: "POST", body: "{}" }),
    /Network failure/,
  );
  assert.equal(spy.mock.callCount(), 1);
});
