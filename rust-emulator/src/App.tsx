import { useEffect, useRef, useState } from "react";
import init, { GBAEngine } from "gba-core";

const W = 240;
const H = 160;
const FRAME_MS = 1000 / 59.7275;
const MAX_STEPS_PER_FRAME = 1_000_000;
const DEBUG = false;

const KEY_MAP: Record<string, number> = {
  KeyK: 0, // A
  KeyJ: 1, // B
  KeyM: 2, // Select
  KeyN: 3, // Start
  KeyD: 4, // Right
  KeyA: 5, // Left
  KeyW: 6, // Up
  KeyS: 7, // Down
  KeyI: 8, // R
  KeyU: 9, // L
};

const CONTROLS: { gba: string; key: string }[] = [
  { gba: "Up", key: "W" },
  { gba: "Left", key: "A" },
  { gba: "Down", key: "S" },
  { gba: "Right", key: "D" },
  { gba: "A", key: "K" },
  { gba: "B", key: "J" },
  { gba: "L", key: "U" },
  { gba: "R", key: "I" },
  { gba: "Start", key: "N" },
  { gba: "Select", key: "M" },
];

export default function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const engineRef = useRef<GBAEngine | null>(null);
  const memoryRef = useRef<WebAssembly.Memory | null>(null);
  const rafRef = useRef(0);
  const lastRef = useRef(0);
  const accRef = useRef(0);
  const heldRef = useRef<Set<number>>(new Set());

  const [ready, setReady] = useState(false);
  const [romName, setRomName] = useState("");
  const [paused, setPaused] = useState(false);

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

  // keyboard input
  useEffect(() => {
    function setKey(key: number, released: boolean) {
      const engine = engineRef.current;
      if (!engine) return;

      // only forward actual state changes
      if (released) {
        if (!heldRef.current.delete(key)) return;
      } else {
        if (heldRef.current.has(key)) return;
        heldRef.current.add(key);
      }
      engine.update_key(key, released);
    }

    function onKeyDown(e: KeyboardEvent) {
      const key = KEY_MAP[e.code];
      if (key === undefined) return;
      e.preventDefault();
      if (e.repeat) return;
      setKey(key, false);
    }

    function onKeyUp(e: KeyboardEvent) {
      const key = KEY_MAP[e.code];
      if (key === undefined) return;
      e.preventDefault();
      setKey(key, true);
    }

    // avoid keys getting stuck if the window loses focus while held
    function releaseAll() {
      for (const key of Array.from(heldRef.current)) {
        setKey(key, true);
      }
    }

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", releaseAll);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", releaseAll);
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
        const drawFrame = engine.step();
        if (drawFrame) {
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
    heldRef.current.clear(); // new engine starts with all keys released

    const rom = new Uint8Array(await file.arrayBuffer());
    engineRef.current = new GBAEngine(rom, DEBUG);
    setRomName(file.name);
    setPaused(false);

    accRef.current = 0;
    lastRef.current = performance.now();
    rafRef.current = requestAnimationFrame(frame);
  }

  function togglePause() {
    if (!engineRef.current) return;

    if (paused) {
      // reset timers so the time spent paused isn't "caught up" as a burst of frames
      accRef.current = 0;
      lastRef.current = performance.now();
      rafRef.current = requestAnimationFrame(frame);
    } else {
      cancelAnimationFrame(rafRef.current);
    }
    setPaused(!paused);
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
      <button
        disabled={!romName}
        onClick={togglePause}
        style={{ marginLeft: 12 }}
      >
        {paused ? "Resume" : "Pause"}
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

      <div style={{ marginTop: 16 }}>
        <h3 style={{ margin: "0 0 8px" }}>Controls</h3>
        <table style={{ borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={cellStyle}>GBA</th>
              <th style={cellStyle}>Keyboard</th>
            </tr>
          </thead>
          <tbody>
            {CONTROLS.map(({ gba, key }) => (
              <tr key={gba}>
                <td style={cellStyle}>{gba}</td>
                <td style={cellStyle}>
                  <kbd>{key}</kbd>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

const cellStyle: React.CSSProperties = {
  border: "1px solid #888",
  padding: "4px 12px",
  textAlign: "left",
};
