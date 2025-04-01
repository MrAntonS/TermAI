// src/lib/ConnectionTab/DummyConnectionAgent.ts
export function connect(): Promise<string> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve("Connected to dummy connection!");
    }, 1000);
  });
}