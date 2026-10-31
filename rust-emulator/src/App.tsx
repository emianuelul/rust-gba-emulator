import { useEffect, useRef, useState } from "react";
import init, { GBAEngine } from "gba-core";

const W = 240;
const H = 160;
const FRAME_MS = 1000 / 59.7275;
const MAX_STEPS_PER_FRAME = 1_000_000;

export default function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const engineRef = useRef<GBAEngine | null>(null);
  const memoryRef = useRef<WebAssembly.Memory | null>(null);
  const rafRef = useRef(0);
  const lastRef = useRef(0);
  const accRef = useRef(0);

  const [ready, setReady] = useState(false);
  const [romName, setRomName] = useState("");

  useEffect(() => {
    let cancelled = false;
    init().then((wasm) => {
      if (cancelled) return;
      memoryRef.current = wasm.memory;
      setReady(true);
    });
    return () => {
      cancelled = true;
      cancelAnimationFrame(rafRef.current);
      engineRef.current?.free();
      engineRef.current = null;
    };
  }, []);

  function frame(now: number) {
    const engine = engineRef.current;
    const memory = memoryRef.current;
    const ctx = canvasRef.current?.getContext("2d");
    if (!engine || !memory || !ctx) return;

    accRef.current += Math.min(now - lastRef.current, 100);
    lastRef.current = now;

    let drew = false;
    while (accRef.current >= FRAME_MS) {
      accRef.current -= FRAME_MS;
      for (let i = 0; i < MAX_STEPS_PER_FRAME; i++) {
        if (engine.step()) {
          drew = true;
          break;
        }
      }
    }

    if (drew) {
      const pixels = new Uint8ClampedArray(
        memory.buffer,
        engine.get_screen_pointer(),
        W * H * 4,
      );
      ctx.putImageData(new ImageData(pixels, W, H), 0, 0);
    }

    rafRef.current = requestAnimationFrame(frame);
  }

  async function onFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file || !memoryRef.current) return;

    cancelAnimationFrame(rafRef.current);
    engineRef.current?.free();
    engineRef.current = null;

    const rom = new Uint8Array(await file.arrayBuffer());
    engineRef.current = new GBAEngine(rom);
    setRomName(file.name);

    accRef.current = 0;
    lastRef.current = performance.now();
    rafRef.current = requestAnimationFrame(frame);
  }

  return (
    <div style={{ padding: 16 }}>
      <input
        ref={fileRef}
        type="file"
        accept=".gba"
        onChange={onFile}
        style={{ display: "none" }}
      />
      <button disabled={!ready} onClick={() => fileRef.current?.click()}>
        {romName ? "Load another ROM" : "Select ROM"}
      </button>
      {romName && <span style={{ marginLeft: 12 }}>{romName}</span>}

      <div style={{ marginTop: 16 }}>
        <canvas
          ref={canvasRef}
          width={W}
          height={H}
          style={{ width: W * 3, height: H * 3, imageRendering: "pixelated", background: "#000" }}
        />
      </div>
    </div>
  );
}
