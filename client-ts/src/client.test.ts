import { createNoAuthClient } from "./client";

function to_bytes(text: string): Uint8Array {
  return Uint8Array.from(Array.from(text).map(letter => letter.charCodeAt(0)));
}

test("Service", async () => {
  const PORT = 50051;

  const client = createNoAuthClient(`localhost:${PORT}`, "test-app");

  try {
    await client.initApp();
  } catch (e) {
    console.error(e);
  }
  const checkpoints = await client.checkpoints();
  expect(checkpoints.length).toEqual(0);
  await client.set({ "a": to_bytes("1") });
  expect(await client.get(["a", "b"])).toEqual({ "a": to_bytes("1") });
  await client.set({ "a": to_bytes("2") });
  const checkpoint0 = await client.create_checkpoint("0");
  await client.set({ "a": to_bytes("3"), "b": to_bytes("3") });
  expect(await client.get(["a"])).toEqual({ "a": to_bytes("3") });
  await client.revert(checkpoint0);
  expect(await client.get(["a"])).toEqual({ "a": to_bytes("2") });
  const checkpoint1 = await client.create_checkpoint("1");
  await client.cleanup(checkpoint1);
  await client.set({ "a": to_bytes("2") });
  await client.reset();
  expect(await client.get(["a", "b"])).toEqual({});

  // expect(await client.revert(checkpoint0)).toThrow();
  client.revert(checkpoint0);
});
