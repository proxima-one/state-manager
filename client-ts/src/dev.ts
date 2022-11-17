import { createNoAuthClient } from "./client";

function range(length: number) {
  return Array.from({ length }, (_, i) => i);
}

function toBytes(text: string): Uint8Array {
  return Uint8Array.from(Array.from(text).map(letter => letter.charCodeAt(0)));
}

const ZERO = '0'.charCodeAt(0);

function randomBytes(n: number) {
  return Uint8Array.from(new Array(n).map(_ => Math.floor(Math.random() * 256)));
}

async function main() {
  const PORT = 50051;

  const clients = await Promise.all(range(20).map(async i => {
    const client = createNoAuthClient(`localhost:${PORT}`, `test-app-${i}`);
    await client.initApp();
    return client;
  }));

  for (let i = 0; i < 100000; ++i) {
    if (i % 100 == 0) {
      console.log(`Iteration ${i}`);
    }
    const client = clients[i % 20];
    await client.set({ "global.state": randomBytes(i * 100) });
    // await client.set({});
    await client.get(["non-existing", "global.state"]);
    await client.get([]);
    const checkpoint = await client.create_checkpoint(i.toString());
  }
}

main();
