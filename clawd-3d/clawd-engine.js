// Clawd's 3D scene and move player, shared by the web page and the Windows and Android apps.
// Needs three.js r128 loaded first. Exposes window.ClawdEngine.
(() => {
  // ---- Frame data, as Claude Code defines it ----
  const FRAME_MS = 60;
  const POSES = {
    default: { eyes: "open", arms: "down", feet: "both" },
    "arms-up": { eyes: "open", arms: "up", feet: "both" },
    "look-left": { eyes: "left", arms: "down", feet: "both" },
    "look-right": { eyes: "right", arms: "down", feet: "both" }
  };
  const REST = { pose: "default", offset: 0, x: 0 };
  const run = (pose, n, { offset = 0, x = 0, poof, shadow } = {}) =>
    Array.from({ length: n }, () => ({ pose, offset, x, poof, shadow }));
  const turn = (names, shadow) => names.map((facing) => ({ pose: { facing }, offset: 0, x: 0, shadow }));
  const mood = (eyes, arms = "down", feet = "both") => ({ eyes, arms, feet });
  const crouch = () => [{ pose: "default", offset: 1, x: 0, poof: "dot" }, { pose: "default", offset: 1, x: 0, poof: "wave" }];
  const BLINK = [...run(mood("closed"), 1), REST];
  const JUMP = [...crouch(), ...run("arms-up", 3), ...run("default", 1), ...crouch(), ...run("arms-up", 3), ...run("default", 1)];

  // The ultracode guitar routine as compact entries, each lasting n frames: label, eyes, arms, offset, x (columns),
  // lift, turnTo (degrees), lean, kick, strum, bang, fret, windmill, zap, notes, flash. Choreographed by three
  // designers (jumps, travel, dance) and merged into one song-shaped routine.
  const ULTRA_ROUTINE = [
    { n: 2, label: "headbang stomp", eyes: "closed", offset: 0, x: 0, strum: 1, bang: 1, fret: 0.35, zap: true, notes: "above" },
    { n: 1, label: "headbang stomp", eyes: "closed", offset: 1, x: 0, strum: 1, bang: 1, fret: 0.5, zap: true, notes: "above" },
    { n: 2, label: "headbang stomp", eyes: "closed", offset: 0, x: 0, lean: -0.12, strum: 1, bang: 1, fret: 0.65, zap: true, notes: "above" },
    { n: 1, label: "headbang stomp", eyes: "closed", offset: 1, x: 0, strum: 1, bang: 1, fret: 0.5, zap: true, notes: "above" },
    { n: 2, label: "headbang stomp", eyes: "closed", offset: 0, x: 0, lean: 0.12, strum: 1, bang: 1, fret: 0.35, zap: true, notes: "above" },
    { n: 2, label: "winds up", eyes: "open", offset: 1, x: 0, strum: 0.4, fret: 0.7 },
    { n: 1, label: "launch jump", eyes: "open", offset: 0, x: 0, lift: 0.6, kick: 0.5, strum: 1, fret: 0.7 },
    { n: 1, label: "launch jump", eyes: "open", offset: 0, x: 0, lift: 1.1, kick: 1, strum: 1, fret: 0.7, zap: true },
    { n: 1, label: "launch jump", eyes: "closed", offset: 0, x: 0, lift: 1.4, kick: 1, strum: 1, fret: 0.7, zap: true, notes: "above" },
    { n: 1, label: "launch jump", eyes: "closed", offset: 0, x: 0, lift: 1.4, kick: 1, strum: 1, fret: 0.7, zap: true, notes: "above" },
    { n: 1, label: "launch jump", eyes: "open", offset: 0, x: 0, lift: 1, kick: 0.5, strum: 1, fret: 0.7 },
    { n: 1, label: "launch jump", eyes: "open", offset: 0, x: 0, lift: 0.5, kick: 0, strum: 1, fret: 0.7 },
    { n: 1, label: "lands", eyes: "closed", offset: 1, x: 0, strum: 0.5, fret: 0.6 },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 0, x: 0.5, lift: 0.5, turnTo: 20, kick: 0.3, strum: 1, fret: 0.5 },
    { n: 1, label: "hop hop hop", eyes: "closed", offset: 0, x: 1, lift: 0.8, turnTo: 20, kick: 0.6, strum: 1, fret: 0.5, zap: true, notes: "side" },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 0, x: 1.2, lift: 0.4, turnTo: 20, kick: 0.2, strum: 1, fret: 0.5 },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 1, x: 1.2, strum: 0.5, fret: 0.5 },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 0, x: 1.7, lift: 0.6, turnTo: -20, kick: 0.3, strum: 1, fret: 0.5 },
    { n: 1, label: "hop hop hop", eyes: "closed", offset: 0, x: 2.2, lift: 1, turnTo: -20, kick: 0.6, strum: 1, fret: 0.5, zap: true, notes: "side" },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 0, x: 2.4, lift: 0.5, turnTo: -20, kick: 0.2, strum: 1, fret: 0.5 },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 1, x: 2.4, strum: 0.5, fret: 0.5 },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 0, x: 2.9, lift: 0.7, turnTo: 20, kick: 0.3, strum: 1, fret: 0.5 },
    { n: 1, label: "hop hop hop", eyes: "closed", offset: 0, x: 3.4, lift: 1.2, turnTo: 20, kick: 0.6, strum: 1, fret: 0.5, zap: true, notes: "side" },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 0, x: 3.5, lift: 0.6, turnTo: 20, kick: 0.2, strum: 1, fret: 0.5 },
    { n: 1, label: "hop hop hop", eyes: "open", offset: 1, x: 3.5, strum: 0.5, fret: 0.5 },
    { n: 2, label: "edge shred", eyes: "left", offset: 0, x: 3.5, turnTo: -35, lean: -0.15, strum: 1, bang: 0.6, fret: 0.8, zap: true, notes: "side" },
    { n: 2, label: "edge shred", eyes: "closed", offset: 0, x: 3.5, turnTo: -35, lean: -0.2, strum: 1, bang: 0.9, fret: 0.95, zap: true, notes: "side" },
    { n: 2, label: "crouches", eyes: "open", offset: 1, x: 3.5, turnTo: 0, strum: 0.3, fret: 0.6 },
    { n: 1, label: "spin jump", eyes: "open", offset: 0, x: 3, lift: 0.7, turnTo: 75, kick: 0.3, strum: 0.6, fret: 0.6 },
    { n: 1, label: "spin jump", eyes: "open", offset: 0, x: 2.5, lift: 1.2, turnTo: 150, kick: 0.6, strum: 0.6, fret: 0.6 },
    { n: 1, label: "spin jump", eyes: "open", offset: 0, x: 2, lift: 1.6, turnTo: 225, kick: 0.8, strum: 0.8, fret: 0.6, zap: true },
    { n: 1, label: "spin jump", eyes: "closed", offset: 0, x: 1.5, lift: 1.8, turnTo: 300, kick: 1, strum: 1, fret: 0.6, zap: true },
    { n: 1, label: "spin jump", eyes: "closed", arms: "up", offset: 0, x: 1, lift: 1.8, turnTo: 360, kick: 1, strum: 1, fret: 0.6, zap: true, notes: "above", flash: true },
    { n: 1, label: "spin jump", eyes: "closed", offset: 0, x: 0.6, lift: 1.3, turnTo: 360, kick: 0.5, strum: 1, fret: 0.6, zap: true },
    { n: 1, label: "spin jump", eyes: "open", offset: 0, x: 0.3, lift: 0.6, turnTo: 360, kick: 0.2, strum: 0.6, fret: 0.6 },
    { n: 1, label: "lands", eyes: "open", offset: 1, x: 0, turnTo: 0, strum: 0.4, fret: 0.6 },
    { n: 1, label: "charges", eyes: "open", offset: 0, x: -0.5, turnTo: 0, lean: 0.15, strum: 0.8, fret: 0.6 },
    { n: 1, label: "knee slide", eyes: "closed", offset: 1, x: -1.2, turnTo: 20, lean: -0.2, strum: 1, bang: 0.3, fret: 0.5, zap: true, notes: "side" },
    { n: 1, label: "knee slide", eyes: "closed", offset: 1, x: -1.8, turnTo: 20, lean: -0.26, strum: 1, bang: 0.3, fret: 0.55, zap: true, notes: "side" },
    { n: 1, label: "knee slide", eyes: "closed", offset: 1, x: -2.4, turnTo: 20, lean: -0.3, strum: 1, bang: 0.3, fret: 0.6, zap: true, notes: "side" },
    { n: 1, label: "knee slide", eyes: "closed", offset: 1, x: -2.9, turnTo: 20, lean: -0.33, strum: 1, bang: 0.3, fret: 0.65, zap: true, notes: "side" },
    { n: 1, label: "knee slide", eyes: "closed", offset: 1, x: -3.3, turnTo: 20, lean: -0.35, strum: 1, bang: 0.3, fret: 0.7, zap: true, notes: "side" },
    { n: 1, label: "knee slide", eyes: "closed", offset: 1, x: -3.6, turnTo: 20, lean: -0.35, strum: 1, bang: 0.3, fret: 0.75, zap: true, notes: "side" },
    { n: 1, label: "slide finish", eyes: "closed", offset: 1, x: -3.6, lean: -0.35, strum: 1, fret: 0.9, zap: true, flash: true },
    { n: 2, label: "on his knees", eyes: "closed", offset: 1, x: -3.6, lean: -0.3, strum: 1, bang: 1, fret: 0.8, zap: true, notes: "above" },
    { n: 1, label: "pops up", eyes: "open", offset: 0, x: -3.6, strum: 0.6, fret: 0.6 },
    { n: 1, label: "scissor kick", eyes: "open", offset: 1, x: -3.6, strum: 0.3, fret: 0.6 },
    { n: 1, label: "scissor kick", eyes: "open", offset: 0, x: -3.4, lift: 0.7, lean: 0.1, kick: 0.5, strum: 0.8, fret: 0.6 },
    { n: 1, label: "scissor kick", eyes: "open", offset: 0, x: -3.1, lift: 1.3, lean: 0.2, kick: 1, strum: 1, fret: 0.6, zap: true },
    { n: 1, label: "scissor kick", eyes: "closed", offset: 0, x: -2.8, lift: 1.7, lean: 0.25, kick: 1, strum: 1, fret: 0.6, zap: true, flash: true },
    { n: 1, label: "scissor kick", eyes: "closed", offset: 0, x: -2.5, lift: 1.7, lean: 0.05, kick: 0.1, strum: 1, fret: 0.6, zap: true },
    { n: 1, label: "scissor kick", eyes: "closed", offset: 0, x: -2.3, lift: 1.3, lean: 0.2, kick: 1, strum: 1, fret: 0.6 },
    { n: 1, label: "scissor kick", eyes: "open", offset: 0, x: -2.1, lift: 0.6, lean: 0.1, kick: 0.3, strum: 0.8, fret: 0.6 },
    { n: 1, label: "lands", eyes: "open", offset: 1, x: -2, strum: 0.4, fret: 0.6 },
    { n: 2, label: "duck walk", eyes: "right", offset: 0, x: -1.6, turnTo: 30, lean: 0.15, kick: 0, strum: 1, bang: 0.4, fret: 0.5, notes: "side" },
    { n: 2, label: "duck walk", eyes: "right", offset: 1, x: -1.2, turnTo: 30, lean: -0.1, kick: 1, strum: 1, bang: 0.4, fret: 0.5, zap: true, notes: "side" },
    { n: 2, label: "duck walk", eyes: "closed", offset: 0, x: -0.8, turnTo: 30, lean: 0.15, kick: 0, strum: 1, bang: 0.4, fret: 0.5, notes: "side" },
    { n: 2, label: "duck walk", eyes: "right", offset: 1, x: -0.4, turnTo: 30, lean: -0.1, kick: 1, strum: 1, bang: 0.4, fret: 0.5, zap: true, notes: "side" },
    { n: 2, label: "duck walk", eyes: "right", offset: 0, x: 0, turnTo: 0, lean: 0.15, kick: 0, strum: 1, bang: 0.4, fret: 0.5, notes: "side" },
    { n: 4, label: "windmill", eyes: "closed", arms: "up", offset: 0, x: 0, lean: 0.12, strum: 1, bang: 0.5, fret: 0.4, windmill: true, zap: true, notes: "above" },
    { n: 4, label: "windmill", eyes: "closed", arms: "up", offset: 0, x: 0, lean: -0.12, strum: 1, bang: 0.8, fret: 0.3, windmill: true, zap: true, notes: "above" },
    { n: 2, label: "side sway", eyes: "left", offset: 0, x: -1, turnTo: -20, lean: 0.25, strum: 0.8, bang: 0.3, fret: 0.5, notes: "side" },
    { n: 2, label: "side sway", eyes: "right", offset: 0, x: 1, turnTo: 20, lean: -0.25, strum: 0.8, bang: 0.3, fret: 0.5, notes: "side" },
    { n: 2, label: "side sway", eyes: "left", offset: 0, x: -1, turnTo: -20, lean: 0.25, strum: 0.8, bang: 0.3, fret: 0.5, notes: "side" },
    { n: 2, label: "side sway", eyes: "right", offset: 0, x: 1, turnTo: 20, lean: -0.25, strum: 0.8, bang: 0.3, fret: 0.5, notes: "side" },
    { n: 3, label: "headbang", eyes: "closed", offset: 0, x: 0, strum: 1, bang: 1, fret: 0.3, zap: true, notes: "above" },
    { n: 3, label: "headbang", eyes: "closed", offset: 0, x: 0, strum: 1, bang: 1, fret: 0.9, zap: true, notes: "above" },
    { n: 2, label: "big leap", eyes: "open", offset: 1, x: 0, strum: 0.2, fret: 0.6 },
    { n: 1, label: "big leap", eyes: "open", offset: 0, x: 0, lift: 0.6, strum: 0.8, fret: 0.6 },
    { n: 1, label: "big leap", eyes: "open", offset: 0, x: 0, lift: 1.2, kick: 0.4, strum: 1, fret: 0.6 },
    { n: 1, label: "big leap", eyes: "closed", offset: 0, x: 0, lift: 1.6, kick: 0.8, strum: 1, fret: 0.6, zap: true },
    { n: 1, label: "power chord!", eyes: "closed", arms: "up", offset: 0, x: 0, lift: 1.8, lean: -0.12, kick: 1, strum: 1, fret: 0.6, windmill: true, zap: true, notes: "above", flash: true },
    { n: 1, label: "power chord!", eyes: "closed", arms: "up", offset: 0, x: 0, lift: 1.8, lean: -0.12, kick: 1, strum: 1, fret: 0.6, windmill: true, zap: true, notes: "above" },
    { n: 1, label: "power chord!", eyes: "closed", offset: 0, x: 0, lift: 1.7, lean: -0.06, kick: 0.6, strum: 1, fret: 0.6, zap: true },
    { n: 1, label: "power chord!", eyes: "open", offset: 0, x: 0, lift: 1.2, kick: 0.2, strum: 0.8, fret: 0.6 },
    { n: 1, label: "power chord!", eyes: "open", offset: 0, x: 0, lift: 0.6, strum: 0.6, fret: 0.6 },
    { n: 1, label: "lands", eyes: "open", offset: 1, x: 0, strum: 0.4, fret: 0.6, notes: "above" },
    { n: 3, label: "shreds", eyes: "closed", offset: 0, x: 0, lean: -0.15, strum: 1, bang: 0.6, fret: 0.95, zap: true, notes: "side" },
    { n: 3, label: "rock kick", eyes: "closed", offset: 0, x: 0, lean: 0.2, kick: 1, strum: 1, bang: 0.9, fret: 0.5, zap: true, notes: "above" },
    { n: 2, label: "final pose", eyes: "wink", offset: 0, x: 0, turnTo: 0, lean: 0, kick: 0, strum: 0.3, fret: 0.6, notes: "above" }
  ];
  const fromEntries = (entries) => entries.flatMap(({ n, eyes = "open", arms = "down", ...rest }) =>
    Array.from({ length: n }, () => ({ pose: mood(eyes, arms), offset: 0, x: 0, guitar: 1, ...rest })));

  // A flip: arms swing up, a crouch, one full turn in the air around his middle, a squashy landing and a ta-da.
  // axis "flip" turns him head over heels (dir -1 backwards, 1 forwards) and "roll" sideways like a cartwheel;
  // turnTo turns him away from the camera first so the flip reads clearly. height scales the jump: he is wider than
  // he is tall, so the side flip stays lower to fit the view.
  function flipMove(axis, dir, turnTo, label, height = 1) {
    const air = [0.5, 0.85, 1, 1.05, 1, 0.8, 0.4];   // lift per frame in the air; the last frame completes the turn
    return [
      { pose: "default", offset: 0, x: 0, turnTo: turnTo / 2, label: "gets ready" },
      { pose: mood("open", "up"), offset: 0, x: 0, turnTo, label: "gets ready" },
      { pose: "default", offset: 1, x: 0, turnTo, poof: "dot", label: "crouches" },
      { pose: "default", offset: 1, x: 0, turnTo, label: "crouches" },
      ...air.map((lift, i) => ({
        pose: mood(i > 1 && i < 5 ? "closed" : "open", "up"), offset: 0, x: 0, turnTo, lift: lift * height,
        [axis]: (dir * 360 * (i + 1)) / air.length, label
      })),
      { pose: "default", offset: 1, x: 0, turnTo, poof: "wave", label: "lands" },
      { pose: "default", offset: 1, x: 0, turnTo, label: "lands" },
      { pose: mood("open", "up"), offset: 0, x: 0, turnTo: turnTo / 2, label: "ta-da" },
      { pose: mood("open", "up"), offset: 0, x: 0, turnTo: 0, label: "ta-da" },
      ...run("default", 2), ...BLINK
    ];
  }

  const MOVES = {
    jump: JUMP,
    look: [...run("look-right", 5), ...run("look-left", 5), ...run("default", 1)],
    // In Claude Code, celebrate is just the jump plus a last crouch, so it looks the same as jump. Here it gets its own
    // routine: a spinning jump, a happy hop, and a swaying victory dance, with confetti.
    celebrate: [
      ...crouch(),
      ...[70, 140, 210, 280, 350].map((turnTo, i) => ({
        pose: "arms-up", offset: 0, x: 0, turnTo, label: "spins in the air", ...(i === 2 ? { confetti: 40 } : {})
      })),
      { pose: "default", offset: 1, x: 0, poof: "wave", label: "lands" },
      { pose: "default", offset: 1, x: 0, label: "lands" },
      ...[0.55, 0.9, 0.55].map((lift, i) => ({
        pose: mood("closed", i % 2 ? "one-up" : "up"), offset: 0, x: 0, lift, label: "hops", ...(i === 1 ? { confetti: 30 } : {})
      })),
      { pose: "default", offset: 1, x: 0, poof: "dot", label: "lands" },
      ...Array.from({ length: 12 }, (_, i) => ({
        pose: mood("closed", i % 4 < 2 ? "up" : "one-up"), offset: 0, x: 0, lean: i % 4 < 2 ? 0.14 : -0.14,
        label: "victory dance", ...(i === 0 ? { confetti: 24 } : {})
      })),
      ...run("default", 2), ...BLINK
    ],
    skip: [
      ...run("default", 1, { offset: 1, x: -9 }), ...run("arms-up", 2, { x: -6 }), ...run("default", 1, { x: -6 }),
      ...run("default", 1, { offset: 1, x: -6 }), ...run("arms-up", 2, { x: -3 }), ...run("default", 1, { x: -3 }),
      ...run("default", 1, { offset: 1, x: -3 }), ...run("arms-up", 2), ...crouch(), ...run("default", 1)
    ],
    spin: [...run("look-left", 2), ...run("look-right", 2), ...run("look-left", 2), ...run("arms-up", 3), ...run("default", 1)],
    peekaboo: [
      ...run("default", 1, { offset: 3 }), ...run("default", 3, { offset: 2 }), ...run("look-right", 3, { offset: 2 }),
      ...run("look-left", 3, { offset: 2 }), ...run("default", 2, { offset: 2 }), ...run("default", 1, { offset: 1 }),
      ...run("arms-up", 4), ...run("default", 2), ...BLINK
    ],
    drop: [
      ...run("arms-up", 1, { offset: -3 }), ...run("arms-up", 2, { offset: -2 }), ...run("arms-up", 2, { offset: -1 }),
      ...run("arms-up", 1), ...crouch(), ...run("default", 3), ...BLINK
    ],
    waddle: [
      ...run("look-right", 1, { x: -9 }),
      ...[-6, -5, -4, -3, -2, -1].flatMap((x) => run(mood("right", "down", x % 2 === 0 ? "left" : "right"), 2, { x })),
      ...run("look-right", 2), ...run("default", 2), ...BLINK
    ],
    peek: [
      ...run("look-right", 1, { x: -9 }), ...run("look-right", 5, { x: -6 }), ...run("look-right", 3, { x: -9 }),
      ...run("look-right", 4, { x: -5 }), ...run("arms-up", 2, { x: -3 }), ...run("arms-up", 2), ...crouch(), REST
    ],
    wink: [...run(mood("wink"), 5), REST],
    boop: [
      ...run(mood("closed"), 1, { offset: 1, poof: "dot" }), ...run(mood("closed"), 2, { offset: 1, poof: "wave" }),
      ...run("look-left", 4), ...run("default", 2), ...BLINK
    ],
    tap: [
      ...run(mood("open", "down", "left"), 2), ...run(mood("open", "down", "right"), 2),
      ...run(mood("open", "down", "left"), 2), ...run(mood("open", "down", "right"), 2),
      ...run(mood("open", "down", "left"), 1), ...run(mood("open", "down", "right"), 1),
      ...run("arms-up", 3), REST
    ],
    sneeze: [
      ...run(mood("closed", "up"), 4), ...run(mood("closed"), 2, { offset: 1, poof: "wave" }),
      ...run(mood("closed"), 2), ...run("default", 2), ...BLINK
    ],
    turn: [
      ...turn(["right-12", "right-30", "right-55", "right-75", "edge"]),
      ...turn(["back-105", "back-125", "back-150", "back", "back"]),
      ...turn(["back-150", "back-125", "back-105", "edge"]),
      ...turn(["left-75", "left-55", "left-30", "left-12"]), REST
    ],
    "coin-hop": [
      ...run("default", 2, { offset: 1 }), ...run("arms-up", 1, { shadow: "wide" }),
      ...turn(["right-55", "edge", "back-125", "back"], "narrow"),
      ...turn(["back-125", "edge", "left-55"], "narrow"),
      ...run("arms-up", 1, { shadow: "wide" }), ...crouch(), REST
    ],
    "back-flip": flipMove("flip", -1, -55, "backflip!"),
    "front-flip": flipMove("flip", 1, 85, "front flip!"),
    "side-flip": flipMove("roll", 1, 0, "side flip!", 0.8),
    // Dumbbell curls: he squats to pick up two dumbbells, curls them one arm at a time with his eyes shut at the top
    // of each rep, does a last curl with both, holds it with a wink, and drops them with a thud.
    "dumbbell-curl": [
      { pose: "default", offset: 0, x: 0, squat: 0.5, dumbbell: 0.5, label: "picks up dumbbells" },
      { pose: "default", offset: 0, x: 0, squat: 0.5, dumbbell: 1, label: "picks up dumbbells" },
      { pose: "default", offset: 0, x: 0, dumbbell: 1, label: "picks up dumbbells" },
      ...["curlR", "curlL", "curlR", "curlL", "curlR", "curlL"].flatMap((arm) => {
        const lean = arm === "curlR" ? -0.05 : 0.05;
        return [
          { pose: mood("open"), offset: 0, x: 0, dumbbell: 1, [arm]: 0.5, lean, label: "curls" },
          { pose: mood("closed"), offset: 0, x: 0, dumbbell: 1, [arm]: 1, lean, label: "curls" },
          { pose: mood("closed"), offset: 0, x: 0, dumbbell: 1, [arm]: 1, lean, label: "curls" },
          { pose: mood("open"), offset: 0, x: 0, dumbbell: 1, [arm]: 0.4, label: "curls" },
          { pose: mood("open"), offset: 0, x: 0, dumbbell: 1, label: "curls" }
        ];
      }),
      { pose: mood("open"), offset: 0, x: 0, squat: 0.15, dumbbell: 1, curlL: 0.5, curlR: 0.5, label: "curls both" },
      { pose: mood("closed"), offset: 0, x: 0, dumbbell: 1, curlL: 1, curlR: 1, label: "curls both" },
      ...Array.from({ length: 3 }, () => ({ pose: mood("wink"), offset: 0, x: 0, dumbbell: 1, curlL: 1, curlR: 1, label: "holds it" })),
      { pose: "default", offset: 0, x: 0, squat: 0.3, dumbbell: 1, label: "drops them" },
      { pose: "default", offset: 1, x: 0, dumbbell: 0, poof: "dot", label: "drops them" },
      ...run("default", 2), ...BLINK
    ],
    // Rebuilt from a recording of the Claude app: a look, a ring of code, then Clawd turns to a laptop and types.
    laptop: [
      ...run("look-right", 4),
      { pose: mood("left"), offset: 0, x: 0, turnTo: -25, laptop: 0.35, ripple: true, label: "opens laptop" },
      { pose: mood("left"), offset: 0, x: 0, turnTo: -45, laptop: 0.75, label: "opens laptop" },
      { pose: mood("left"), offset: 0, x: 0, squat: 0.4, turnTo: -55, laptop: 1, label: "opens laptop" },
      ...Array.from({ length: 36 }, () => ({ pose: mood("left"), offset: 0, x: 0, squat: 0.4, turnTo: -55, laptop: 1, typing: true, label: "types" })),
      ...Array.from({ length: 2 }, () => ({ pose: mood("left"), offset: 0, x: 0, squat: 0.4, turnTo: -55, laptop: 0.4, label: "closes laptop" })),
      { pose: "default", offset: 0, x: 0, turnTo: -20, label: "closes laptop" },
      ...run("default", 2), ...BLINK
    ],
    // Ultra effort, after the Claude app's Ultracode effort row: Clawd plays a lavender electric guitar in front of a
    // pulsing pixel wall, with yellow sparks and pixel music notes, and goes all out: a stomping headbang, a launch jump,
    // hops across the stage, a spin jump, a knee slide, a scissor kick, a duck walk, a windmill dance break and a huge
    // final power-chord leap.
    ultracode: [
      ...run("look-right", 2),
      { pose: "default", offset: 0, x: 0, guitar: 0.5, label: "grabs a guitar" },
      { pose: "default", offset: 0, x: 0, guitar: 1, label: "grabs a guitar" },
      ...fromEntries(ULTRA_ROUTINE),
      { pose: "default", offset: 0, x: 0, guitar: 0.4, label: "puts the guitar down" },
      { pose: "default", offset: 0, x: 0, guitar: 0, label: "puts the guitar down" },
      ...run("default", 2), ...BLINK
    ],

    // ---- Moves that aren't in Claude Code ----
    // Says hi: notices you, raises his right arm and waves it, with a wink halfway.
    wave: [
      ...run("look-right", 2),
      { pose: mood("open", "one-up"), offset: 0, x: 0, lean: -0.04, label: "says hi" },
      ...Array.from({ length: 12 }, (_, i) => ({
        pose: mood(i >= 5 && i < 8 ? "wink" : "open", "one-up"), offset: 0, x: 0, lean: -0.04, wave: 1, label: "waves"
      })),
      { pose: mood("open", "one-up"), offset: 0, x: 0, label: "waves" },
      ...run("default", 2), ...BLINK
    ],
    // A little dance: side steps on the beat with his arms going up in turn, a spin, a hop and a ta-da.
    dance: [
      ...[0, 0.5, 0, -0.5, 0, 0.5, 0, -0.5].flatMap((x, beat) => {
        const arms = beat % 2 ? "one-up" : "up";
        const lean = x > 0 ? -0.12 : x < 0 ? 0.12 : 0;
        return [
          { pose: mood("closed", arms, beat % 2 ? "left" : "right"), offset: 0, x, lean, lift: 0.25, label: "dances" },
          { pose: mood("open", arms), offset: 0, x, lean, label: "dances" },
          { pose: mood("open", "down"), offset: 1, x, lean: lean / 2, label: "dances" }
        ];
      }),
      ...[90, 180, 270, 360].map((turnTo) => ({ pose: "arms-up", offset: 0, x: 0, turnTo, lift: 0.6, label: "spins" })),
      { pose: "default", offset: 1, x: 0, turnTo: 0, poof: "wave", label: "lands" },
      ...run("arms-up", 3).map((f) => ({ ...f, label: "ta-da" })),
      ...run("default", 2), ...BLINK
    ],
    // Takes a nap: a big stretch, then he dozes, breathing slowly, with Zs drifting up, until he wakes with a start.
    nap: [
      { pose: mood("closed", "up"), offset: 0, x: 0, squat: 0.1, label: "stretches" },
      ...run(mood("closed", "up"), 4, { offset: 0 }).map((f) => ({ ...f, lift: 0.15, label: "stretches" })),
      { pose: mood("closed"), offset: 0, x: 0, squat: 0.3, label: "dozes off" },
      ...Array.from({ length: 42 }, (_, i) => ({
        pose: mood("closed"), offset: 0, x: 0, squat: i % 14 < 7 ? 0.42 : 0.5, lean: i % 28 < 14 ? 0.03 : -0.03,
        notes: "z", label: "naps"
      })),
      { pose: "default", offset: 0, x: 0, lift: 0.45, label: "wakes with a start" },
      { pose: "arms-up", offset: 0, x: 0, lift: 0.3, label: "wakes with a start" },
      ...run("look-left", 3), ...run("look-right", 3), ...run("default", 2), ...BLINK
    ],
    // Jumping jacks: three reps, eyes shut at the top of each.
    "jumping-jacks": [
      ...Array.from({ length: 3 }, () => [
        { pose: "default", offset: 1, x: 0, poof: "dot", label: "jumping jacks" },
        { pose: mood("open", "up"), offset: 0, x: 0, lift: 0.4, label: "jumping jacks" },
        { pose: mood("closed", "up"), offset: 0, x: 0, lift: 0.55, label: "jumping jacks" },
        { pose: mood("open", "up"), offset: 0, x: 0, lift: 0.25, label: "jumping jacks" }
      ]).flat(),
      { pose: "default", offset: 1, x: 0, poof: "wave", label: "lands" },
      ...run("default", 2), ...BLINK
    ],
    // Takes a bow, eyes shut, then a ta-da with a little confetti.
    bow: [
      { pose: "default", offset: 0, x: 0, flip: 12, label: "bows" },
      { pose: mood("closed"), offset: 0, x: 0, flip: 28, label: "bows" },
      ...run(mood("closed"), 5, { offset: 0 }).map((f) => ({ ...f, flip: 34, label: "bows" })),
      { pose: "default", offset: 0, x: 0, flip: 10, label: "comes up" },
      { pose: "arms-up", offset: 0, x: 0, confetti: 16, label: "ta-da" },
      ...run("arms-up", 3), ...run("default", 2), ...BLINK
    ],
    // The moonwalk: facing one way, he glides the other, feet stepping all the while; then back the other way.
    moonwalk: [
      { pose: "look-right", offset: 0, x: 0, turnTo: 40, label: "turns" },
      ...[-0.4, -0.8, -1.2, -1.6, -2, -2.4, -2.8].flatMap((x, i) => run(mood("right", "down", i % 2 ? "left" : "right"), 2,
        { x }).map((f) => ({ ...f, turnTo: 70, lean: 0.05, label: "moonwalks" }))),
      { pose: "look-left", offset: 0, x: -2.8, turnTo: -40, label: "turns" },
      ...[-2.4, -2, -1.6, -1.2, -0.8, -0.4, 0].flatMap((x, i) => run(mood("left", "down", i % 2 ? "left" : "right"), 2,
        { x }).map((f) => ({ ...f, turnTo: -70, lean: -0.05, label: "moonwalks" }))),
      { pose: mood("open", "one-up"), offset: 0, x: 0, turnTo: 0, lift: 0.3, label: "hee-hee" },
      ...run("default", 2), ...BLINK
    ],
    // Thinks it over (looking up, a hand raised, swaying a little), then has an idea and hops.
    think: [
      ...Array.from({ length: 14 }, (_, i) => ({
        pose: mood("right", "one-up"), offset: 0, x: 0, squat: 0.1, lean: i < 7 ? 0.05 : -0.02, label: "thinks"
      })),
      { pose: mood("closed"), offset: 1, x: 0, label: "has an idea" },
      { pose: "arms-up", offset: 0, x: 0, lift: 0.6, confetti: 10, label: "has an idea" },
      { pose: "arms-up", offset: 0, x: 0, lift: 0.3, label: "has an idea" },
      { pose: "default", offset: 1, x: 0, poof: "dot", label: "lands" },
      ...run("default", 2), ...BLINK
    ]
  };
  const IDLE = [...run("default", 12), ...run("look-right", 5), ...run("look-left", 5)];
  // The laptop move's parts, for working (see work()): where he opens it, and the typing frames he repeats.
  const LAPTOP_OPEN = MOVES.laptop.findIndex((f) => f.label === "opens laptop");
  const TYPING_FROM = MOVES.laptop.findIndex((f) => f.typing);
  const TYPING_TO = MOVES.laptop.length - 1 - [...MOVES.laptop].reverse().findIndex((f) => f.typing);
  const ENTRANCES = ["skip", "jump", "look", "spin", "peekaboo", "drop", "waddle", "peek", "wink", "boop", "tap", "sneeze", "turn", "coin-hop"];
  const ALL = Object.keys(MOVES);
  // Moves that start where he stands, not sliding in from the side: clicks, and everything floating Clawd plays on
  // his own (the 5-second loop and Auto-play), pick from these, since his window only has room around him.
  const ON_CLICK = ALL.filter((n) => (MOVES[n][0].x || 0) === 0);

  // Turn angles: each facing name is an angle; pick the reading that keeps the turn going one way.
  function angleChoices(name) {
    const [side, deg] = name.split("-");
    const n = Number(deg);
    if (name === "edge") return [90, 270];
    if (name === "back") return [180];
    if (side === "right") return [n];
    if (side === "left") return [360 - n];
    return [n, 360 - n];
  }
  for (const frames of [...Object.values(MOVES), IDLE]) {
    let prev = 0;
    let hopRun = 0;
    frames.forEach((f, i) => {
      if (f.turnTo !== undefined) {
        prev = 0;
        f.angle = f.turnTo;
      } else if (typeof f.pose === "object" && f.pose.facing) {
        const options = angleChoices(f.pose.facing);
        prev = options.find((a) => a >= prev) ?? options[options.length - 1];
        f.angle = prev;
      } else {
        prev = 0;
        f.angle = 0;
      }
      // Arms-up right after a crouch reads as a hop; mark the run so the 3D version can leave the ground.
      const before = frames[i - 1];
      if (f.pose === "arms-up" && f.offset === 0 && !f.shadow && before && (before.offset === 1 || hopRun)) hopRun++;
      else hopRun = 0;
      f.hop = hopRun;
    });
    frames.forEach((f, i) => {
      if (!f.hop) return;
      let end = i;
      while (frames[end + 1] && frames[end + 1].hop) end++;
      f.lift = Math.sin(Math.PI * (f.hop - 0.5) / (end - i + f.hop));
    });
  }

  function describe(f) {
    if (f.label) return f.label;
    if (typeof f.pose === "string") return f.pose;
    if (f.pose.facing) return `turn ${f.pose.facing} (${Math.round(f.angle) % 360}°)`;
    const p = f.pose;
    return `eyes ${p.eyes} · arms ${p.arms} · feet ${p.feet}`;
  }

  // What he types. The laptop move types the first session; while he works for Claude Code (work()), each pass of
  // the typing loop types the next one.
  const SCREENS = [
    ["$ claude", "> fix the flaky test", "", "Reading tests/api.spec.ts", "Editing src/retry.ts", "  + await backoff(attempt)", "Running npm test", "  42 passed", "Done."],
    ["> add a dark mode", "", "Reading src/theme.ts", "Editing src/App.tsx", "  + const theme = useTheme()", "Running npm run build", "  built in 1.2s", "Done."],
    ["> why is the build slow?", "", "Reading webpack.config.js", "  - devtool: 'source-map'", "  + devtool: false", "Running npm run build", "  12.4s -> 3.1s", "Done."],
    ["> write tests for the parser", "", "Reading src/parser.rs", "Writing tests/parser.rs", "  + fn parses_empty()", "Running cargo test", "  18 passed", "Done."]
  ].map((lines) => lines.join("\n"));
  // Camera framing per mode: "stage" shows the whole floor strip; "pet" frames Clawd alone for a small corner window.
  const FRAMING = {
    // Headroom above him for the flips; the floor stays where it was.
    stage: { target: [-3, 7, 0], theta: 0.38, phi: 1.36, halfH: 13, halfW: 23 },
    // Room for the guitar routine's travel (about 7 units each way) and leaps (about 6 units up) around him.
    pet: { target: [0, 9, 0], theta: 0.3, phi: 1.42, halfH: 13, halfW: 20 }
  };

  // create({ stage, canvas, fx, mode, on... }) builds the scene in `canvas` (sized to `stage`) and returns a controller,
  // or null when WebGL isn't available.
  //   mode: "stage" (floor strip, drag to orbit) or "pet" (no floor, just a shadow; drag reports onDrag in screen pixels)
  //   onMove(name), onFrame({ name, index, length, frame }), onAuto(on), onWork(on), onHover(overClawd),
  //   onDrag(dx, dy), onDragEnd(), onLongPress(), onDoubleClick(), onTripleClick()
  //   followPointer: false to keep his eyes from following the mouse on the stage
  function create(opts) {
    const THREE = window.THREE;
    const { stage, canvas, fx } = opts;
    const on = (name, ...args) => { if (typeof opts[name] === "function") opts[name](...args); };
    if (!THREE) return null;
    let renderer;
    try {
      renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    } catch (err) { return null; }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    renderer.outputEncoding = THREE.sRGBEncoding;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    renderer.localClippingEnabled = true;
    let dirty = true;   // something besides the moves changed the picture (size, colours, mode): draw the next frame

    const lin = (hex) => new THREE.Color(hex).convertSRGBToLinear();
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(30, 1, 0.5, 400);

    // ---- Model: the sprite's columns, the printed figure's proportions ----
    const COL = 2;                         // one terminal column is two sprite quadrants wide
    const BODY_W = 12, BODY_H = 9, LEG_H = 2.8, DEPTH = 5.5, EYE = 1.4, ARM_LEN = 2.7;
    const floorClip = [new THREE.Plane(new THREE.Vector3(0, 1, 0), 0)];   // the floor line hides whatever sinks below it

    const clayMat = new THREE.MeshStandardMaterial({ color: lin("#d77757"), roughness: 0.62, clippingPlanes: floorClip });
    const legMat = new THREE.MeshStandardMaterial({ color: lin("#c86a4b"), roughness: 0.7, clippingPlanes: floorClip });
    const eyeMat = new THREE.MeshStandardMaterial({ color: lin("#1c1a19"), roughness: 0.4, clippingPlanes: floorClip });
    const pickable = [];
    function box(w, h, d, mat) {
      const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat);
      m.castShadow = true;
      m.receiveShadow = true;
      pickable.push(m);
      return m;
    }

    const root = new THREE.Group();        // place on the floor and turn
    const body = new THREE.Group();        // everything above the legs, lowered when crouching
    scene.add(root);
    root.add(body);
    body.position.y = LEG_H;

    const torso = box(BODY_W, BODY_H, DEPTH, clayMat);
    torso.position.y = BODY_H / 2;
    body.add(torso);

    const eyes = [-3.5, 3.5].map((x) => {
      const pivot = new THREE.Group();
      pivot.position.set(x, BODY_H * 0.74, DEPTH / 2 + 0.25);
      pivot.add(box(EYE, EYE, 0.6, eyeMat));
      pivot.userData.home = pivot.position.clone();
      body.add(pivot);
      return pivot;
    });

    const arms = [-1, 1].map((side) => {
      const pivot = new THREE.Group();
      pivot.position.set(side * BODY_W / 2, BODY_H * 0.5, 0);
      const m = box(ARM_LEN, 2.6, 2.2, clayMat);
      m.position.x = side * ARM_LEN / 2;
      pivot.add(m);
      body.add(pivot);
      return pivot;
    });

    const legs = [-4.5, -2.5, 2.5, 4.5].map((x) => {
      const pivot = new THREE.Group();
      pivot.position.set(x, LEG_H, 0);
      const m = box(1.3, LEG_H, 2.2, legMat);
      m.position.y = -LEG_H / 2;
      pivot.add(m);
      pivot.userData.left = x < 0;
      pivot.userData.outer = Math.abs(x) > 3;
      pivot.userData.homeX = x;
      root.add(pivot);
      return pivot;
    });

    // ---- Laptop for the laptop move: opens in front of Clawd, screen facing it ----
    const LT = { W: 8.5, D: 5, BASE: 1.75, LID: 0.25 };   // base height puts the keys just under Clawd's level hands
    const laptopMat = new THREE.MeshStandardMaterial({ color: lin("#d5d8dd"), roughness: 0.45, metalness: 0.15, clippingPlanes: floorClip });
    const keysMat = new THREE.MeshStandardMaterial({ color: lin("#3a3f46"), roughness: 0.8, clippingPlanes: floorClip });
    const screenCanvas = document.createElement("canvas");
    screenCanvas.width = 384;
    screenCanvas.height = 176;
    const screenCtx = screenCanvas.getContext("2d");
    const screenTex = new THREE.CanvasTexture(screenCanvas);
    screenTex.encoding = THREE.sRGBEncoding;

    const laptop = new THREE.Group();
    laptop.position.set(0, 0, DEPTH / 2 + 0.15 + LT.D / 2);
    const base = new THREE.Mesh(new THREE.BoxGeometry(LT.W, LT.BASE, LT.D), laptopMat);
    base.position.y = LT.BASE / 2;
    const keys = new THREE.Mesh(new THREE.BoxGeometry(LT.W - 1.4, 0.04, LT.D * 0.5), keysMat);
    keys.position.set(0, LT.BASE + 0.02, LT.D * 0.1);
    const hinge = new THREE.Group();
    hinge.position.set(0, LT.BASE, LT.D / 2);
    const lid = new THREE.Mesh(new THREE.BoxGeometry(LT.W, LT.LID, LT.D), laptopMat);
    lid.position.set(0, LT.LID / 2, -LT.D / 2);
    const screen = new THREE.Mesh(
      new THREE.PlaneGeometry(LT.W - 0.8, LT.D - 0.6),
      new THREE.MeshBasicMaterial({ map: screenTex, clippingPlanes: floorClip })
    );
    screen.rotation.set(Math.PI / 2, 0, Math.PI);   // faces Clawd once open, top edge at the lid's free end
    screen.position.set(0, -0.01, -LT.D / 2);
    for (const m of [base, lid]) { m.castShadow = true; m.receiveShadow = true; }
    hinge.add(lid, screen);
    laptop.add(base, keys, hinge);
    laptop.visible = false;
    root.add(laptop);

    let screenText = SCREENS[0], screenNo = 0;
    let typed = -1, typeTarget = 0;
    function nextScreen() {
      screenNo = (screenNo + 1) % SCREENS.length;
      screenText = SCREENS[screenNo];
      typeTarget = 0;
      typed = -1;
    }
    function drawScreen(n) {
      if (n === typed) return;
      typed = n;
      const c = screenCtx;
      c.fillStyle = "#11181d";
      c.fillRect(0, 0, screenCanvas.width, screenCanvas.height);
      c.font = "bold 15px Menlo, Consolas, monospace";
      c.textBaseline = "top";
      const lines = screenText.slice(0, n).split("\n").slice(-9);
      lines.forEach((line, i) => {
        c.fillStyle = line.startsWith("  +") ? "#7fe0a0" : line.startsWith(">") ? "#e8a07f" : "#5fd3e4";
        c.fillText(line, 12, 10 + i * 18);
      });
      const lastLine = lines[lines.length - 1] || "";
      c.fillStyle = "#5fd3e4";
      c.fillRect(12 + c.measureText(lastLine).width + 2, 10 + (lines.length - 1) * 18, 8, 15);
      screenTex.needsUpdate = true;
    }
    drawScreen(0);

    // ---- Electric guitar for ultra effort: dark body, lavender face and headstock, as in the Claude app ----
    const darkMat = new THREE.MeshStandardMaterial({ color: lin("#23202c"), roughness: 0.55, clippingPlanes: floorClip });
    const lavMat = new THREE.MeshStandardMaterial({ color: lin("#a49cf0"), roughness: 0.4, clippingPlanes: floorClip });
    const boardMat = new THREE.MeshStandardMaterial({ color: lin("#7d74d6"), roughness: 0.6, clippingPlanes: floorClip });
    const stringMat = new THREE.MeshStandardMaterial({ color: lin("#e4e0ff"), roughness: 0.3, metalness: 0.6, clippingPlanes: floorClip });
    function part(w, h, d, mat, x, y, z) {
      const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat);
      m.position.set(x, y, z);
      m.castShadow = true;
      m.receiveShadow = true;
      return m;
    }
    const guitar = new THREE.Group();     // neck along +x, strings facing +z
    guitar.add(
      part(3.8, 4.2, 0.8, darkMat, 0, 0, 0),               // body
      part(1.3, 1.1, 0.8, darkMat, 2.3, 1.45, 0),          // upper horn
      part(1.0, 1.0, 0.8, darkMat, 2.15, -1.5, 0),         // lower horn
      part(3.0, 3.4, 0.1, lavMat, 0.1, 0.1, 0.44),         // lavender face
      part(0.32, 1.5, 0.14, darkMat, -0.8, 0, 0.52),       // pickups
      part(0.32, 1.5, 0.14, darkMat, 0.0, 0, 0.52),
      part(0.4, 1.7, 0.16, stringMat, -1.3, 0, 0.5),       // bridge
      part(6.6, 0.75, 0.42, darkMat, 5.0, 0, 0.12),        // neck
      part(6.0, 0.6, 0.08, boardMat, 4.9, 0, 0.37),        // fretboard
      part(1.7, 1.1, 0.32, lavMat, 9.1, 0.1, 0.08)         // headstock
    );
    for (let i = 0; i < 3; i++) {                           // tuning pegs
      guitar.add(part(0.3, 0.3, 0.3, darkMat, 8.5 + i * 0.55, 0.85, 0.08));
      guitar.add(part(0.3, 0.3, 0.3, darkMat, 8.5 + i * 0.55, -0.65, 0.08));
    }
    for (let i = 0; i < 7; i++) guitar.add(part(0.06, 0.62, 0.1, stringMat, 2.3 + i * 0.85, 0, 0.42));
    for (const y of [-0.2, -0.07, 0.07, 0.2]) guitar.add(part(10.2, 0.04, 0.04, stringMat, 3.6, y, 0.54));
    // Held low across the front, body over his right legs, neck rising steeply past his left shoulder.
    const GUITAR_AT = { x: 0.8, y: 0.6, tilt: 0.9 };
    const guitarPivot = new THREE.Group();
    guitarPivot.position.set(GUITAR_AT.x, GUITAR_AT.y, DEPTH / 2 + 0.75);
    guitarPivot.rotation.z = GUITAR_AT.tilt;
    guitarPivot.add(guitar);
    guitarPivot.visible = false;
    body.add(guitarPivot);
    // A point along the guitar (distance from the body's centre toward the headstock) in body coordinates.
    const onGuitar = (d) => [GUITAR_AT.x + d * Math.cos(GUITAR_AT.tilt), GUITAR_AT.y + d * Math.sin(GUITAR_AT.tilt)];

    // ---- Dumbbells for the curls: one in each fist, the bar running front to back with an iron plate at each end ----
    const ironMat = new THREE.MeshStandardMaterial({ color: lin("#4a505a"), roughness: 0.5, metalness: 0.3, clippingPlanes: floorClip });
    const barMat = new THREE.MeshStandardMaterial({ color: lin("#d3d7de"), roughness: 0.3, metalness: 0.6, clippingPlanes: floorClip });
    const dumbbells = arms.map((arm, i) => {
      const side = i ? 1 : -1;
      const d = new THREE.Group();
      d.position.set(side * (ARM_LEN + 0.1), 0, 0);   // in his fist, just past the end of the arm
      d.add(part(0.7, 0.7, 5.4, barMat, 0, 0, 0));
      for (const z of [-2.05, 2.05]) d.add(part(2.5, 2.5, 0.9, ironMat, 0, 0, z), part(1.5, 1.5, 0.35, barMat, 0, 0, z + Math.sign(z) * 0.6));
      d.visible = false;
      arm.add(d);
      return d;
    });

    // Particles (sparks, confetti, dust) are pools of little boxes, each pool drawn as one instanced mesh: one draw call
    // however many are flying. A particle has a position, velocity, rotation, spin, scale, life and colour.
    const placer = new THREE.Object3D();
    function particlePool(count, geo, mat, colors) {
      const mesh = new THREE.InstancedMesh(geo, mat, count);
      mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
      if (colors) mesh.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(count * 3), 3);
      mesh.count = 0;
      mesh.visible = false;
      mesh.frustumCulled = false;   // the instances fly far from the mesh's own origin
      scene.add(mesh);
      const parts = Array.from({ length: count }, (_, i) => ({
        live: false, pos: new THREE.Vector3(), v: new THREE.Vector3(), rot: new THREE.Euler(), spin: new THREE.Vector3(),
        scale: 1, life: 0, phase: 0, color: colors ? colors[i % colors.length] : null
      }));
      return { mesh, parts, live: 0 };
    }
    function spawn(pool, count, init) {
      let made = 0;
      for (const p of pool.parts) {
        if (p.live) continue;
        p.live = true;
        p.scale = 1;
        init(p);
        if (++made === count) break;
      }
      pool.live += made;
    }
    // Writes the live particles into the first instances and draws only those.
    function placeParticles(pool) {
      let n = 0;
      for (const p of pool.parts) {
        if (!p.live) continue;
        placer.position.copy(p.pos);
        placer.rotation.copy(p.rot);
        placer.scale.setScalar(p.scale);
        placer.updateMatrix();
        pool.mesh.setMatrixAt(n, placer.matrix);
        if (p.color) pool.mesh.setColorAt(n, p.color);
        n++;
      }
      if (n || pool.mesh.count) {
        pool.mesh.instanceMatrix.needsUpdate = true;
        if (pool.mesh.instanceColor) pool.mesh.instanceColor.needsUpdate = true;
      }
      pool.mesh.count = n;
      pool.mesh.visible = n > 0;   // no draw call at all for an empty pool
      pool.live = n;
    }

    // Yellow lightning bits that crackle off the strings.
    const sparks = particlePool(48, new THREE.BoxGeometry(0.22, 0.75, 0.22), new THREE.MeshBasicMaterial(),
      [lin("#ffd23f"), lin("#ffe680")]);
    const sparkFrom = new THREE.Vector3();
    // burst: all around (a power chord); otherwise off to his left (the viewer's right), like the app.
    function emitSparks(count, burst) {
      guitar.getWorldPosition(sparkFrom);
      spawn(sparks, count, (p) => {
        const a = burst ? Math.random() * Math.PI * 2 : -0.4 + Math.random() * 1.6;
        const sp = burst ? 10 + Math.random() * 8 : 6 + Math.random() * 6;
        p.pos.copy(sparkFrom);
        p.v.set(Math.cos(a) * sp, Math.sin(a) * sp + (burst ? 0 : 2), (Math.random() - 0.3) * 3);
        p.life = 0.35 + Math.random() * 0.3;
        p.rot.set(0, 0, Math.random() * Math.PI);
      });
    }

    // Confetti for celebrate: little coloured squares thrown up from over his head that flutter and spin down.
    const confetti = particlePool(90, new THREE.BoxGeometry(0.6, 0.6, 0.06), new THREE.MeshBasicMaterial({ side: THREE.DoubleSide }),
      ["#ffd23f", "#4cc9db", "#ff6fa8", "#a49cf0", "#7fe0a0", "#d77757"].map(lin));
    function emitConfetti(count) {
      spawn(confetti, count, (p) => {
        p.pos.set(root.position.x + (Math.random() - 0.5) * 6, root.position.y + 12, (Math.random() - 0.5) * 3);
        p.v.set((Math.random() - 0.5) * 16, 5 + Math.random() * 6, (Math.random() - 0.5) * 8);   // peaks within the frame
        p.spin.set(Math.random() * 10 - 5, Math.random() * 10 - 5, Math.random() * 10 - 5);
        p.life = 2 + Math.random() * 0.8;
        p.phase = Math.random() * Math.PI * 2;
        p.rot.set(Math.random() * 3, Math.random() * 3, Math.random() * 3);
      });
    }

    // A wall of pixel cells behind him: lavender blocks pulse outward from Clawd over deep purple, and flash on a chord.
    const WALL = { cols: 18, rows: 12, cell: 20, w: 36, h: 24 };
    const wallCanvas = document.createElement("canvas");
    wallCanvas.width = WALL.cols * WALL.cell;
    wallCanvas.height = WALL.rows * WALL.cell;
    const wallCtx = wallCanvas.getContext("2d");
    const wallTex = new THREE.CanvasTexture(wallCanvas);
    wallTex.encoding = THREE.sRGBEncoding;
    wallTex.magFilter = THREE.NearestFilter;
    const wall = new THREE.Mesh(
      new THREE.PlaneGeometry(WALL.w, WALL.h),
      new THREE.MeshBasicMaterial({ map: wallTex, transparent: true, depthWrite: false, clippingPlanes: floorClip })
    );
    wall.position.set(0, 9, -5);
    wall.visible = false;
    scene.add(wall);
    function drawWall(t, flashLevel) {
      const c = wallCtx, { cols, rows, cell } = WALL;
      c.clearRect(0, 0, wallCanvas.width, wallCanvas.height);
      const cx = cols / 2, cy = rows / 2 + 1.5;          // Clawd's middle, a little below the wall's
      const front = ((t * 2.2) % 1) * 11;                 // a ring of light stepping outward on each beat
      for (let j = 0; j < rows; j++) {
        for (let i = 0; i < cols; i++) {
          const dx = i + 0.5 - cx, dy = (j + 0.5 - cy) * 1.15;
          const d = Math.max(Math.abs(dx), Math.abs(dy)) * 0.6 + Math.hypot(dx, dy) * 0.4;
          const edge = clamp(1 - d / 7.5, 0, 1);              // fades out well before the wall ends
          if (edge <= 0) continue;
          let level = d < front && d > front - 3.5 ? 1 - (front - d) / 3.5 : 0;
          level = Math.max(level, 0.45 * clamp(1 - d / 4.5, 0, 1), flashLevel);
          level = Math.round(level * edge * 3) / 3;      // stepped, like the app's pixel blocks
          c.fillStyle = `rgba(58, 36, 150, ${0.9 * Math.min(1, edge * 1.6)})`;
          c.fillRect(i * cell, j * cell, cell, cell);
          if (level > 0) {
            c.fillStyle = `rgba(190, 182, 255, ${0.85 * level})`;
            c.fillRect(i * cell + 1, j * cell + 1, cell - 2, cell - 2);
          }
          c.strokeStyle = `rgba(36, 22, 110, ${0.7 * edge})`;
          c.strokeRect(i * cell + 0.5, j * cell + 0.5, cell - 1, cell - 1);
        }
      }
      wallTex.needsUpdate = true;
    }

    // Pixel music notes drawn over the scene: yellow ones over his head, grey ones beside him.
    const NOTE_SHAPES = [
      ["..##.", "..#.#", "..#..", "###..", "###.."],
      [".####", ".#..#", ".#..#", "##.##", "##.##"]
    ];
    const Z_SHAPE = ["#####", "...#.", "..#..", ".#...", "#####"];
    const notes = [];
    let noteSlot = 0, zSlot = 0;
    // where: "above" (a yellow note over his head), "side" (a grey one beside him) or "z" (napping: Zs that drift up
    // to his right, each a little higher).
    function addNote(where) {
      if (where === "z") {
        const n = zSlot++ % 3;
        notes.push({ pos: new THREE.Vector3(root.position.x + 6.5 + 1.6 * n, 13 + 1.2 * n, 1), color: "#c9d6ff", shape: Z_SHAPE, life: 1 });
        return;
      }
      const above = where === "above";
      notes.push({
        pos: new THREE.Vector3(
          root.position.x + (above ? -3.5 + 3.5 * (noteSlot++ % 3) : 9 + Math.random() * 1.5),
          above ? 13 : 3 + Math.random() * 4,
          above ? 1 : 2
        ),
        color: above ? "#ffd23f" : "#8b8a9c",
        shape: NOTE_SHAPES[Math.random() < 0.5 ? 0 : 1],
        life: 1
      });
    }
    const notePoint = new THREE.Vector3();
    function drawNotes(dt) {
      for (let n = notes.length - 1; n >= 0; n--) {
        const note = notes[n];
        note.life -= dt * 1.3;
        note.pos.y += dt * 1.5;
        if (note.life <= 0) { notes.splice(n, 1); continue; }
        notePoint.copy(note.pos).project(camera);
        const px = (notePoint.x + 1) / 2 * fx.width, py = (1 - notePoint.y) / 2 * fx.height;
        const size = Math.max(2, Math.round(fx.height / 110));
        fctx.globalAlpha = Math.min(1, note.life * 2);
        fctx.fillStyle = note.color;
        note.shape.forEach((row, r) => [...row].forEach((ch, k) => {
          if (ch === "#") fctx.fillRect(Math.round(px + (k - 2.5) * size), Math.round(py + (r - 2.5) * size), size, size);
        }));
      }
      fctx.globalAlpha = 1;
    }

    // Code that floods out from Clawd, as in the Claude app: a growing disc of text rows with a bright, crisp
    // leading edge and dimmer characters behind it; once it covers the stage it holds, then fades out together.
    const fctx = fx.getContext("2d");
    const GLYPHS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const hash = (i, j, k) => { const v = Math.sin(i * 127.1 + j * 311.7 + k * 74.7) * 43758.5453; return v - Math.floor(v); };
    const RING = { grow: 0.9, hold: 0.35, fade: 1.0, edge: 46, inside: 0.32 };
    let ripple = null, codeColor = "#0e7486", fxDirty = false;
    function startRipple() {
      const p = new THREE.Vector3(root.position.x, 6, 0).project(camera);
      ripple = { t: 0, x: (p.x + 1) / 2 * fx.width, y: (1 - p.y) / 2 * fx.height };
    }
    function glyphAt(i, j, t) {
      // Rows read like lines of code: runs of characters broken by spaces, each cell changing now and then.
      if (hash(Math.floor(i / 5), j, 9) < 0.18 || hash(i, j, 3) < 0.12) return null;
      const k = Math.floor((t + hash(i, j, 1) * 0.6) / 0.35);
      return GLYPHS[Math.floor(hash(i, j, k + 17) * 997) % GLYPHS.length];
    }
    // The overlay canvas holds the laptop's code and the guitar's notes; clear it only while something is on it.
    function drawFx(dt) {
      const busy = !!ripple || notes.length > 0;
      if (!busy && !fxDirty) return;
      fxDirty = busy;
      fctx.clearRect(0, 0, fx.width, fx.height);
      drawRipple(dt);
      drawNotes(dt);
    }
    function drawRipple(dt) {
      if (!ripple) return;
      const s = fx.width / Math.max(1, stage.clientWidth);
      ripple.t += dt;
      const t = ripple.t;
      const far = Math.hypot(Math.max(ripple.x, fx.width - ripple.x), Math.max(ripple.y, fx.height - ripple.y));
      const R = far * Math.min(1, Math.pow(t / RING.grow, 0.85)) + RING.edge * s;
      const fade = 1 - clamp((t - RING.grow - RING.hold) / RING.fade, 0, 1);
      if (fade <= 0) { ripple = null; return; }
      const edge = RING.edge * s, cw = 8.5 * s, ch = 13 * s;
      const cols = Math.ceil(fx.width / cw), rows = Math.ceil(fx.height / ch);
      fctx.fillStyle = codeColor;
      fctx.textAlign = "center";
      fctx.textBaseline = "middle";
      // Two passes so each needs only one font: the dim fill behind the edge, then the bold edge itself.
      for (const rim of [false, true]) {
        fctx.font = `${rim ? 700 : 500} ${11 * s}px "Martian Mono", Menlo, Consolas, monospace`;
        for (let j = 0; j < rows; j++) {
          const y = (j + 0.5) * ch, dy = y - ripple.y;
          if (Math.abs(dy) > R) continue;
          for (let i = 0; i < cols; i++) {
            const x = (i + 0.5) * cw, d = Math.hypot(x - ripple.x, dy);
            if (d > R) continue;
            const onEdge = d > R - edge && R < far + edge;
            if (onEdge !== rim) continue;
            const g = glyphAt(i, j, t);
            if (!g) continue;
            const shade = 0.55 + 0.45 * hash(i, j, 5);
            fctx.globalAlpha = fade * shade * (rim ? 1 : RING.inside);
            fctx.fillText(g, x, y);
          }
        }
      }
      fctx.globalAlpha = 1;
    }

    // ---- Stage: a floor strip with the dotted line from the welcome screen, or in pet mode just a shadow ----
    const floorMat = new THREE.MeshStandardMaterial({ color: lin("#b8bec6"), roughness: 0.9 });
    const floor = new THREE.Mesh(new THREE.BoxGeometry(90, 0.8, 18), floorMat);
    floor.position.y = -0.4;
    floor.receiveShadow = true;
    scene.add(floor);
    const dotMat = new THREE.MeshStandardMaterial({ color: lin("#8d949e"), roughness: 0.9 });
    const DOT_XS = [];
    for (let x = -44; x <= 44; x += 1.6) DOT_XS.push(x);
    const dots = new THREE.InstancedMesh(new THREE.BoxGeometry(0.45, 0.06, 0.45), dotMat, DOT_XS.length);   // one draw call
    DOT_XS.forEach((x, i) => {
      placer.position.set(x, 0.03, 7.6);
      placer.updateMatrix();
      dots.setMatrixAt(i, placer.matrix);
    });
    dots.receiveShadow = true;
    dots.frustumCulled = false;
    scene.add(dots);
    const shadowOnly = new THREE.Mesh(new THREE.PlaneGeometry(60, 30), new THREE.ShadowMaterial({ opacity: 0.3 }));
    shadowOnly.rotation.x = -Math.PI / 2;
    shadowOnly.receiveShadow = true;
    scene.add(shadowOnly);

    scene.add(new THREE.HemisphereLight(lin("#ffffff"), lin("#6f625a"), 0.5));
    const sun = new THREE.DirectionalLight(lin("#fff3e8"), 0.95);
    sun.position.set(14, 26, 18);
    sun.target.position.set(-6, 3, 0);
    sun.castShadow = true;
    sun.shadow.mapSize.set(2048, 2048);
    Object.assign(sun.shadow.camera, { left: -32, right: 24, top: 24, bottom: -24, near: 1, far: 90 });
    sun.shadow.bias = -0.0006;
    sun.shadow.normalBias = 0.03;
    scene.add(sun, sun.target);
    const fill = new THREE.DirectionalLight(lin("#cbd8ff"), 0.35);
    fill.position.set(-18, 10, -4);
    scene.add(fill);
    const rim = new THREE.DirectionalLight(lin("#ffffff"), 0.35);
    rim.position.set(0, 14, -22);
    scene.add(rim);

    // Puffs at the feet: "·" is a small one, "~" a wider one.
    const dustMat = new THREE.MeshStandardMaterial({ color: lin("#a9b0ba"), roughness: 1 });
    const dust = particlePool(28, new THREE.BoxGeometry(0.6, 0.6, 0.6), dustMat);
    function puff(kind) {
      const spread = kind === "wave" ? 7 : 4;
      let made = 0;
      spawn(dust, kind === "wave" ? 10 : 4, (p) => {
        const side = made++ % 2 ? 1 : -1;
        p.pos.set(root.position.x + side * (BODY_W / 2 - 0.5), 0.35, (Math.random() - 0.5) * DEPTH);
        p.v.set(side * (2 + Math.random() * spread), 1 + Math.random() * 2, (Math.random() - 0.5) * 3);
        p.life = 0.6 + Math.random() * 0.3;
        p.rot.set(0, 0, 0);
      });
    }

    function setColors(c) {
      if (c.floor) floorMat.color.copy(lin(c.floor));
      if (c.dots) dotMat.color.copy(lin(c.dots));
      if (c.dust) dustMat.color.copy(lin(c.dust));
      if (c.code) codeColor = c.code;
      dirty = true;
    }

    // ---- Turning a frame into 3D targets ----
    const TAU = Math.PI * 2;
    const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
    const wrap = (a) => a - TAU * Math.round(a / TAU);
    const SINK = [0, 0, LEG_H + BODY_H / 2, LEG_H + BODY_H + 1];   // offset 2 leaves only the eyes above the line
    const state = { x: 0, y: 0, ry: 0, crouch: 0, armL: 0, armR: 0, liftL: 0, liftR: 0, eyeX: 0, eyeY: 0, lidL: 0, lidR: 0, squash: 1, laptop: 0, type: 0, guitar: 0, strum: 0, bang: 0, fret: 0.3, lean: 0, kick: 0, windmill: 0, flip: 0, roll: 0, dumbbell: 0, curlL: 0, curlR: 0, wave: 0 };
    const goal = { ...state };
    const KEYS = Object.keys(state);
    const IS_ANGLE = KEYS.map((prop) => prop === "ry" || prop === "flip" || prop === "roll");
    const SETTLED = 0.004;   // closer than this to its goal, a value has arrived (well under a pixel at any size)

    // Where he looks when the frame leaves his eyes open and straight ahead: a point on the page in CSS pixels (it may
    // lie outside the page, like the mouse elsewhere on the desktop), or null to look straight ahead.
    let gazeAt = null, eyesFree = true;
    const eyeBase = { x: 0, y: 0 };
    const eyePoint = new THREE.Vector3();
    function applyGaze() {
      goal.eyeX = eyeBase.x;
      goal.eyeY = eyeBase.y;
      if (!gazeAt || !eyesFree) return;
      const w = stage.clientWidth || 1, h = stage.clientHeight || 1;
      eyePoint.set(root.position.x, root.position.y + LEG_H + BODY_H * 0.74, DEPTH / 2).project(camera);
      const ex = (eyePoint.x + 1) / 2 * w, ey = (1 - eyePoint.y) / 2 * h;
      const reach = Math.max(80, 0.6 * Math.min(w, h));   // this far away, he looks as far as his eyes go
      goal.eyeX = clamp((gazeAt.x - ex) / reach, -1, 1);
      goal.eyeY = 0.7 * clamp((ey - gazeAt.y) / reach, -1, 1);
    }
    function setGaze(x, y) {
      gazeAt = typeof x === "number" && typeof y === "number" && isFinite(x) && isFinite(y) ? { x, y } : null;
      applyGaze();
    }

    function targetsFor(f) {
      const p = typeof f.pose === "string" ? POSES[f.pose] : f.pose.facing ? POSES.default : f.pose;
      goal.x = (f.x || 0) * COL;
      goal.ry = f.angle * Math.PI / 180;
      goal.flip = (f.flip || 0) * Math.PI / 180;
      goal.roll = (f.roll || 0) * Math.PI / 180;
      goal.crouch = f.offset === 1 ? 1 : f.squat || 0;   // a squat bends the legs but keeps them in view
      let y = 0;
      if (f.offset >= 2) y = -SINK[Math.min(f.offset, 3)];
      if (f.offset < 0) y = -f.offset * BODY_H / 2;
      if (f.shadow === "wide") y += 3;
      if (f.shadow === "narrow") y += 6;
      if (f.lift) y += 3.2 * f.lift;
      goal.y = y;
      goal.squash = f.offset === 1 ? 0.94 : f.lift || f.shadow ? 1.04 : 1;
      goal.armL = p.arms === "up" ? 0.95 : 0;
      goal.armR = p.arms === "up" || p.arms === "one-up" ? 0.95 : 0;
      goal.liftL = p.feet === "right" ? 1 : 0;
      goal.liftR = p.feet === "left" ? 1 : 0;
      eyeBase.x = p.eyes === "left" ? -1 : p.eyes === "right" ? 1 : 0;
      eyeBase.y = p.eyes === "left" || p.eyes === "right" ? 0.8 : 0;
      eyesFree = p.eyes === "open";
      applyGaze();
      goal.lidL = p.eyes === "closed" ? 1 : 0;
      goal.lidR = p.eyes === "closed" || p.eyes === "wink" ? 1 : 0;
      goal.laptop = f.laptop || 0;
      goal.type = f.typing ? 1 : 0;
      goal.guitar = f.guitar || 0;
      goal.strum = f.strum || 0;
      goal.bang = f.bang || 0;
      if (f.fret !== undefined) goal.fret = f.fret;
      goal.lean = f.lean || 0;
      goal.kick = f.kick || 0;
      goal.windmill = f.windmill ? 1 : 0;
      goal.dumbbell = f.dumbbell || 0;
      goal.curlL = f.curlL || 0;
      goal.curlR = f.curlR || 0;
      goal.wave = f.wave || 0;
      zapping = !!f.zap;
      // While ultra effort is on, Clawd keeps the guitar between moves and jams quietly.
      if (ultra && seqName === "idle") {
        goal.guitar = 1;
        goal.strum = 0.35;
        goal.bang = 0.25;
        goal.fret = 0.35;
        zapping = false;
      }
    }

    // ---- Playback ----
    let seq = IDLE, seqName = "idle", idx = 0, acc = 0;
    let auto = false;
    let following = false;   // another view (floating Clawd) picks the random moves and this one plays them
    // Pet mode loops on its own: a random move AMBIENT_GAP seconds after the last one ends. Tapping takes over,
    // and the loop picks up again once he has finished and AMBIENT_GAP seconds have passed since the last tap.
    let ambient = false, nextAmbient = 0, lastTapAt = -Infinity;
    const AMBIENT_GAP = 5;
    let ultra = false, zapping = false, sparkClock = 0, flash = 0, noteClock = 0;
    let clock = 0;

    function enterFrame() {
      const f = seq[idx];
      targetsFor(f);
      if (f.poof) puff(f.poof);
      if (f.ripple) { startRipple(); typeTarget = 0; }
      if (f.notes && idx % (f.notes === "z" ? 6 : 3) === 0) addNote(f.notes);
      if (f.flash) { flash = 1; emitSparks(28, true); }
      if (f.confetti) emitConfetti(f.confetti);
      if (f.typing) typeTarget = Math.min(screenText.length, typeTarget + 4);
      if (f.laptop) drawScreen(typeTarget);
      on("onFrame", { name: seqName, index: idx, length: seq.length, frame: f });
    }
    // from: the frame to start at (working skips the laptop move's opening look).
    function play(name, { snap = false, from = 0 } = {}) {
      if (name !== "idle" && !MOVES[name]) return;
      seqName = name;
      seq = name === "idle" ? IDLE : MOVES[name];
      idx = Math.min(Math.max(0, from), seq.length - 1);
      acc = 0;
      on("onMove", name);
      enterFrame();
      if (snap) for (const prop of KEYS) state[prop] = goal[prop];
    }
    const pick = (list) => list[Math.floor(Math.random() * list.length)];
    const pickOther = (list) => pick(list.filter((name) => name !== seqName));
    const autoMoves = () => (mode === "pet" ? ON_CLICK : ALL);
    function scheduleAmbient() { nextAmbient = clock + AMBIENT_GAP; }
    function finished() {
      if (working) play("laptop", { from: LAPTOP_OPEN });
      // Auto-play: a random move straight after each one ends, never the same one twice in a row.
      else if (auto && !following) play(pickOther(autoMoves()));
      else {
        if (seqName !== "idle") scheduleAmbient();   // a move just ended: the next random one is due in 5 s
        play("idle");
      }
    }
    function setAuto(value) {
      auto = !!value;
      on("onAuto", auto);
      if (auto && !following && !working && seqName === "idle") play(pickOther(autoMoves()));
    }
    function setFollowing(value) {
      following = !!value;
    }
    // Switching to ultra effort starts the guitar move right away, like changing the setting in Claude Code.
    function setUltra(value) {
      ultra = !!value;
      on("onUltra", ultra);
      if (ultra) play("ultracode");
      else if (seqName === "idle") targetsFor(seq[idx]);
    }
    function setAmbient(value) {
      ambient = !!value;
      scheduleAmbient();
    }
    // Working, for Claude Code: he opens the laptop and types, one session after another, until work(false), when he
    // closes it. A move started meanwhile (a tap) plays out, and then he goes back to the laptop.
    let working = false;
    function work(value) {
      if (!!value === working) return;
      working = !!value;
      on("onWork", working);
      if (working) {
        if (seqName === "idle") play("laptop", { from: LAPTOP_OPEN });
      } else if (seqName === "laptop" && idx >= TYPING_FROM && idx <= TYPING_TO) {
        idx = TYPING_TO + 1;   // close the laptop now
        acc = 0;
        enterFrame();
      }
    }
    // Holds the current frame ms longer, e.g. so a move lines up with sound that reaches the speakers late.
    function hold(ms) {
      if (ms > 0) acc -= ms;
    }

    // ---- Camera, pointer and picking ----
    const raycaster = new THREE.Raycaster();
    const ndc = new THREE.Vector2();
    const target = new THREE.Vector3();
    const view = { theta: 0, phi: 0, tTheta: 0, tPhi: 0, r: 60 };
    let mode = opts.mode === "pet" ? "pet" : "stage";
    let drag = null, hovering = false, longTimer = 0, lastTap = 0, lastTapX = 0, lastTapY = 0, taps = 0, tapTimer = 0;
    const MULTI_TAP_MS = 400;   // the most time between taps of a double or triple tap
    // Right after a tap, the spot keeps taking clicks (pet mode on Windows) so a double tap lands even if the move it
    // started has carried him away.
    const tapping = () => performance.now() - lastTap < MULTI_TAP_MS;

    function frameFor(m) {
      const fr = FRAMING[m];
      target.set(...fr.target);
      view.theta = view.tTheta = fr.theta;
      view.phi = view.tPhi = fr.phi;
      resize();
    }
    function setMode(m) {
      mode = m === "pet" ? "pet" : "stage";
      floor.visible = dots.visible = mode === "stage";
      shadowOnly.visible = mode === "pet";
      // The floating window is small: a quarter of the shadow map's texels still give more than one per pixel.
      const shadowSize = mode === "pet" ? 1024 : 2048;
      if (sun.shadow.mapSize.x !== shadowSize) {
        sun.shadow.mapSize.set(shadowSize, shadowSize);
        if (sun.shadow.map) {
          sun.shadow.map.dispose();
          sun.shadow.map = null;
        }
      }
      frameFor(mode);
    }

    function toNdc(e) {
      const r = canvas.getBoundingClientRect();
      ndc.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
    }
    function hitsClawd() {
      raycaster.setFromCamera(ndc, camera);
      return raycaster.intersectObjects(pickable, false).some((h) => h.point.y >= 0);
    }
    function setHover(over) {
      if (over === hovering) return;
      hovering = over;
      canvas.classList.toggle("over", over);
      on("onHover", over);
    }
    function playClickMove() {
      // Floating: every tap plays a new move right away, even mid-move.
      if (ambient) {
        lastTapAt = clock;
        setAuto(false);
        play(pickOther(ON_CLICK));
        return;
      }
      // As in the terminal, a click starts a random move only while Clawd is idle.
      if (seqName !== "idle") return;
      setAuto(false);
      play(pick(ON_CLICK));
    }
    canvas.addEventListener("pointerdown", (e) => {
      if (e.button !== undefined && e.button !== 0) return;
      toNdc(e);
      drag = { x: e.clientX, y: e.clientY, sx: e.screenX, sy: e.screenY, px: e.screenX, py: e.screenY, cx: e.clientX, cy: e.clientY, moved: false, long: false };
      try { canvas.setPointerCapture(e.pointerId); } catch (err) {}
      clearTimeout(longTimer);
      if (hitsClawd()) longTimer = setTimeout(() => { if (drag && !drag.moved) { drag.long = true; on("onLongPress"); } }, 650);
    });
    canvas.addEventListener("pointermove", (e) => {
      toNdc(e);
      if (drag) {
        if (Math.hypot(e.clientX - drag.cx, e.clientY - drag.cy) > 6 || Math.hypot(e.screenX - drag.sx, e.screenY - drag.sy) > 6) {
          drag.moved = true;
          clearTimeout(longTimer);
          canvas.classList.add("dragging");
        }
        if (mode === "pet") {
          // Screen coordinates, so moving the window under the pointer doesn't feed back into the drag.
          if (drag.moved) on("onDrag", e.screenX - drag.px, e.screenY - drag.py);
          drag.px = e.screenX;
          drag.py = e.screenY;
        } else {
          const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
          view.tTheta -= dx * 0.008;
          if (e.pointerType === "mouse") view.tPhi = clamp(view.tPhi - dy * 0.006, 0.85, 1.48);
        }
        drag.x = e.clientX;
        drag.y = e.clientY;
      } else {
        setHover(hitsClawd() || tapping());
      }
      // On the stage his eyes follow the mouse (in the floating window, the app passes him the desktop's pointer).
      if (mode === "stage" && e.pointerType === "mouse" && opts.followPointer !== false) {
        const r = canvas.getBoundingClientRect();
        setGaze(e.clientX - r.left, e.clientY - r.top);
      }
    });
    function endDrag(e, cancelled) {
      clearTimeout(longTimer);
      if (!drag) return;
      const d = drag;
      drag = null;
      canvas.classList.remove("dragging");
      if (d.moved && mode === "pet") on("onDragEnd");
      if (!cancelled && !d.moved && !d.long) {
        toNdc(e);
        // The first tap on Clawd plays a move right away. Taps that follow within MULTI_TAP_MS, near the one before,
        // are counted even if he has moved away; once they stop, two make a double tap and three or more a triple
        // tap (touch and mouse alike).
        const now = performance.now();
        const followUp = tapping() && Math.hypot(e.clientX - lastTapX, e.clientY - lastTapY) < 40;
        if (followUp || hitsClawd()) {
          if (followUp) taps++;
          else {
            taps = 1;
            playClickMove();
          }
          lastTap = now;
          lastTapX = e.clientX;
          lastTapY = e.clientY;
          clearTimeout(tapTimer);
          tapTimer = setTimeout(() => {
            if (taps >= 3 && opts.onTripleClick) on("onTripleClick");
            else if (taps >= 2) on("onDoubleClick");
            taps = 0;
            setHover(hitsClawd());
          }, MULTI_TAP_MS);
        }
      }
      toNdc(e);
      setHover(hitsClawd() || tapping());
    }
    canvas.addEventListener("pointerup", (e) => endDrag(e, false));
    canvas.addEventListener("pointercancel", (e) => endDrag(e, true));
    // A drag whose pointerup never came (the window lost the pointer) ends too, so the floating window doesn't keep
    // taking every click.
    canvas.addEventListener("lostpointercapture", (e) => endDrag(e, true));
    canvas.addEventListener("pointerleave", () => {
      if (!drag) setHover(false);
      if (mode === "stage") setGaze(null);
    });

    // Where Clawd's body is on screen, in CSS pixels (for the Android bubble's touch pad).
    const bounds = new THREE.Box3(), corner = new THREE.Vector3();
    function screenRect() {
      bounds.makeEmpty();
      for (const m of pickable) bounds.expandByObject(m);
      if (bounds.isEmpty()) return null;
      const w = stage.clientWidth, h = stage.clientHeight;
      let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
      for (let i = 0; i < 8; i++) {
        corner.set(i & 1 ? bounds.max.x : bounds.min.x, i & 2 ? bounds.max.y : bounds.min.y, i & 4 ? bounds.max.z : bounds.min.z).project(camera);
        const x = (corner.x + 1) / 2 * w, y = (1 - corner.y) / 2 * h;
        x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y);
      }
      x0 = clamp(x0, 0, w); x1 = clamp(x1, 0, w); y0 = clamp(y0, 0, h); y1 = clamp(y1, 0, h);
      return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }

    // Each box of his body on screen (torso, eyes, arms, legs), in CSS pixels: tighter than screenRect() around his
    // outline, for hosts that work out themselves whether the pointer is over him (the desktop app's click-through).
    const corner2 = new THREE.Vector3();
    function screenRects() {
      const w = stage.clientWidth, h = stage.clientHeight;
      root.updateMatrixWorld(true);
      const rects = [];
      for (const m of pickable) {
        const g = m.geometry;
        if (!g.boundingBox) g.computeBoundingBox();
        const b = g.boundingBox;
        let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
        for (let i = 0; i < 8; i++) {
          corner2.set(i & 1 ? b.max.x : b.min.x, i & 2 ? b.max.y : b.min.y, i & 4 ? b.max.z : b.min.z)
            .applyMatrix4(m.matrixWorld)
            .project(camera);
          const x = (corner2.x + 1) / 2 * w, y = (1 - corner2.y) / 2 * h;
          x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y);
        }
        x0 = clamp(x0, 0, w); x1 = clamp(x1, 0, w); y0 = clamp(y0, 0, h); y1 = clamp(y1, 0, h);
        if (x1 - x0 >= 1 && y1 - y0 >= 1) rects.push({ x: Math.floor(x0), y: Math.floor(y0), w: Math.ceil(x1 - x0), h: Math.ceil(y1 - y0) });
      }
      return rects;
    }

    function resize() {
      const w = stage.clientWidth, h = stage.clientHeight;
      if (!w || !h) return;
      renderer.setSize(w, h, false);
      fx.width = Math.round(w * renderer.getPixelRatio());
      fx.height = Math.round(h * renderer.getPixelRatio());
      camera.aspect = w / h;
      const fr = FRAMING[mode];
      const tanHalf = Math.tan(THREE.MathUtils.degToRad(camera.fov / 2));
      view.r = Math.max(fr.halfH / tanHalf, fr.halfW / (tanHalf * camera.aspect));
      camera.updateProjectionMatrix();
      dirty = true;   // resizing clears the canvas
    }
    if (window.ResizeObserver) new ResizeObserver(resize).observe(stage);
    else window.addEventListener("resize", resize);
    setMode(mode);

    // ---- Loop ----
    const FLIP_MIDDLE = new THREE.Vector3(0, LEG_H + BODY_H / 2, 0), UP = new THREE.Vector3(0, 1, 0);
    const flipShift = new THREE.Vector3(), flipTurn = new THREE.Euler();
    let last = performance.now();
    function update(dt) {
      clock += dt;
      acc += dt * 1000;
      while (acc >= FRAME_MS) {
        acc -= FRAME_MS;
        idx++;
        if (working && seqName === "laptop" && idx > TYPING_TO) {   // keep typing: the next session
          idx = TYPING_FROM;
          nextScreen();
        }
        if (idx >= seq.length) finished();
        else enterFrame();
      }
      // Pet mode: once he has idled AMBIENT_GAP seconds since his last move and the last tap, start a random one
      // (mid look-around is fine).
      if (ambient && !auto && !following && !working && seqName === "idle" && clock >= nextAmbient && clock >= lastTapAt + AMBIENT_GAP) play(pick(ON_CLICK));

      const k = 1 - Math.exp(-dt / 0.045);
      let moving = false;
      for (let i = 0; i < KEYS.length; i++) {
        const prop = KEYS[i];
        const d = IS_ANGLE[i] ? wrap(goal[prop] - state[prop]) : goal[prop] - state[prop];
        if (Math.abs(d) < SETTLED) state[prop] += d;   // arrived: land exactly, so he can come to rest
        else {
          state[prop] += d * k;
          moving = true;
        }
      }

      // Flips turn him around his middle, not his feet: shift him back by however far the turn moves his middle.
      flipTurn.set(state.flip, 0, state.roll);
      flipShift.copy(FLIP_MIDDLE).applyEuler(flipTurn).sub(FLIP_MIDDLE).negate().applyAxisAngle(UP, state.ry);
      root.position.set(state.x + flipShift.x, state.y + flipShift.y, flipShift.z);
      root.rotation.set(state.flip, state.ry, state.lean + state.roll, "YXZ");
      const legScale = 1 - 0.85 * state.crouch;
      body.position.y = LEG_H * legScale + state.type * 0.25 * (1 - Math.abs(Math.sin(clock * 42)));   // dips into each keystroke
      body.scale.set(1 / Math.sqrt(state.squash), state.squash, 1 / Math.sqrt(state.squash));
      body.rotation.x = state.bang * 0.14 * Math.sin(clock * 13);   // headbanging
      legs.forEach((leg) => {
        leg.scale.y = legScale;
        const stomp = state.strum * 0.55 * Math.max(0, Math.sin(clock * 6.5 + (leg.userData.left ? 0 : Math.PI)));
        const kick = leg.userData.left ? 0 : state.kick * (leg.userData.outer ? 1 : 0.5);   // a rock kick with his left legs
        leg.position.x = leg.userData.homeX + 1.2 * kick;
        leg.position.y = LEG_H * legScale + 0.9 * (leg.userData.left ? state.liftL : state.liftR) + stomp + 1.3 * kick;
      });
      // Typing, as in the recording: the arms reach straight forward and stay level (no rotation);
      // the hands move only by sliding: down onto the keys in turn, side to side, and in and out.
      arms.forEach((arm, i) => {
        const side = i ? 1 : -1;
        const press = Math.max(0, Math.sin(clock * 42 + i * Math.PI));
        const slide = Math.sin(clock * 9 + i * 2.1);
        arm.position.x = side * (BODY_W / 2 - 2.6 * state.type) + state.type * 0.8 * slide;
        arm.position.y = BODY_H * 0.5 - state.type * (BODY_H * 0.5 - 1.9) - state.type * 0.75 * press;   // lifts high, comes down onto the keys
        const reach = 0.5 + 0.5 * Math.sin(clock * 12 + i * 2.4);   // pulls back into the body and pushes out again
        arm.position.z = state.type * (DEPTH / 2 + 0.1 - 0.9 * reach);
        arm.rotation.y = -side * Math.PI / 2 * state.type;
        arm.rotation.z = side * (i ? state.armR : state.armL);
        if (i) arm.rotation.z += state.wave * 0.45 * Math.sin(clock * 15);   // waving his raised right arm
        // Holding the guitar: the right hand (viewer's left) strums over the pickups, the left frets up the neck.
        const hold = clamp(state.guitar * 1.5 - 0.3, 0, 1);
        if (hold > 0) {
          const [gx, gy] = i
            ? onGuitar(3 + 4 * clamp(state.fret + (zapping ? 0.08 * Math.sin(clock * 9) : 0), 0, 1))
            : onGuitar(-0.3);
          let tx = gx, ty = gy + (i ? 0 : state.strum * 0.9 * Math.sin(clock * 24));
          if (!i && state.windmill > 0.01) {
            // Windmill: the strumming hand swings in a big circle, high over the strings and back down.
            const a = clock * 13;
            tx += state.windmill * (2.6 * Math.cos(a) - 0.6);
            ty += state.windmill * (3.2 + 3.2 * Math.sin(a));
          }
          arm.position.x += (tx - arm.position.x) * hold;
          arm.position.y += (ty - arm.position.y) * hold;
          arm.position.z += (DEPTH / 2 - 0.3 - arm.position.z) * hold;
          arm.rotation.y += (-side * Math.PI / 2 - arm.rotation.y) * hold;
          arm.rotation.z *= 1 - hold;
        }
        // Dumbbell curls: each arm swings a weight from beside his hip up to beside his head, out at his side where
        // the curl reads from the front.
        const grip = clamp(state.dumbbell * 1.5 - 0.3, 0, 1);
        if (grip > 0) {
          const curl = i ? state.curlR : state.curlL;   // 0 hanging down, 1 curled up
          arm.rotation.z += (side * (-1.2 + 2.45 * curl) - arm.rotation.z) * grip;
          arm.rotation.y += (-side * 0.3 - arm.rotation.y) * grip;
        }
      });
      const dShown = clamp(state.dumbbell * 2, 0, 1);
      dumbbells.forEach((d) => {
        d.visible = dShown > 0.01;
        d.scale.setScalar(Math.max(dShown, 0.001));
      });
      const gShown = clamp(state.guitar * 2, 0, 1);
      guitarPivot.visible = gShown > 0.01;
      guitarPivot.scale.setScalar(Math.max(gShown, 0.001));
      flash = Math.max(0, flash - dt * 2.5);
      wall.visible = state.guitar > 0.02;
      if (wall.visible) {
        wall.position.x = root.position.x;
        wall.material.opacity = clamp(state.guitar * 1.2, 0, 1);
        drawWall(clock, flash);
      }
      if (zapping) {
        sparkClock += dt;
        while (sparkClock > 0.08) { sparkClock -= 0.08; emitSparks(2); }
      }
      // While he keeps the guitar between moves, a note drifts up now and then.
      if (ultra && seqName === "idle") {
        noteClock += dt;
        if (noteClock > 1.1) { noteClock = 0; addNote(Math.random() < 0.5 ? "above" : "side"); }
      }
      if (sparks.live) {
        for (const p of sparks.parts) {
          if (!p.live) continue;
          p.life -= dt * 1.4;
          if (p.life <= 0) { p.live = false; continue; }
          p.v.multiplyScalar(1 - 3 * dt);
          p.pos.addScaledVector(p.v, dt);
          p.rot.z += dt * 9;
          p.scale = Math.min(1, p.life * 3);
        }
        placeParticles(sparks);
      }
      const shown = clamp(state.laptop * 2.5, 0, 1);
      laptop.visible = shown > 0.01;
      laptop.scale.setScalar(Math.max(shown, 0.001));
      hinge.rotation.x = clamp((state.laptop - 0.2) / 0.8, 0, 1) * THREE.MathUtils.degToRad(105);
      drawFx(dt);
      eyes.forEach((eye, i) => {
        const lid = i === 0 ? state.lidL : state.lidR;
        eye.scale.y = 1 - 0.82 * lid;
        eye.position.x = eye.userData.home.x + state.eyeX;
        eye.position.y = eye.userData.home.y + state.eyeY - 0.45 * lid;
      });

      if (confetti.live) {
        for (const p of confetti.parts) {
          if (!p.live) continue;
          p.life -= dt;
          if (p.life <= 0) { p.live = false; continue; }
          p.v.y = Math.max(p.v.y - 16 * dt, -3.2);          // falls fast at first, then flutters down
          p.v.x *= 1 - 1.2 * dt;
          p.v.z *= 1 - 1.2 * dt;
          p.pos.addScaledVector(p.v, dt);
          p.pos.x += Math.sin(clock * 5 + p.phase) * 1.6 * dt;
          if (p.pos.y <= 0.05) {                              // settles on the floor
            p.pos.y = 0.05;
            p.v.set(0, 0, 0);
            p.spin.set(0, 0, 0);
            p.rot.x = -Math.PI / 2;
          }
          p.rot.x += p.spin.x * dt;
          p.rot.y += p.spin.y * dt;
          p.rot.z += p.spin.z * dt;
          p.scale = Math.min(1, p.life * 2);
        }
        placeParticles(confetti);
      }
      if (dust.live) {
        for (const p of dust.parts) {
          if (!p.live) continue;
          p.life -= dt * 1.7;
          if (p.life <= 0) { p.live = false; continue; }
          p.v.y -= 5 * dt;
          p.v.multiplyScalar(1 - 2.4 * dt);
          p.pos.addScaledVector(p.v, dt);
          p.pos.y = Math.max(0.3, p.pos.y);
          p.scale = p.life;
        }
        placeParticles(dust);
      }

      const ease = Math.min(1, dt * 8);
      view.theta += (view.tTheta - view.theta) * ease;
      view.phi += (view.tPhi - view.phi) * ease;
      const turning = Math.abs(view.tTheta - view.theta) > 1e-4 || Math.abs(view.tPhi - view.phi) > 1e-4;
      if (!turning) {
        view.theta = view.tTheta;
        view.phi = view.tPhi;
      }
      const sp = Math.sin(view.phi);
      camera.position.set(
        target.x + view.r * sp * Math.sin(view.theta),
        target.y + view.r * Math.cos(view.phi),
        target.z + view.r * sp * Math.cos(view.theta)
      );
      camera.lookAt(target);

      // Whether this frame looks any different from the last: he or the camera is moving, something animates by the
      // clock (typing, strumming, headbanging, the guitar's wall and sparks), or particles are flying.
      return moving || turning || state.type > 0 || state.strum > 0 || state.bang > 0 || state.windmill > 0 || state.wave > 0 ||
        zapping || flash > 0 || wall.visible || sparks.live > 0 || confetti.live > 0 || dust.live > 0;
    }

    // Draws only when the picture changed. Between moves he often stands perfectly still, and then nothing is drawn
    // at all: no GPU work, and the browser has no new frame to put on screen. Floating and idle (just looking
    // around), 30 frames a second are plenty; his moves get every frame.
    let lastDrawn = 0, behind = false;   // behind: the picture changed since it was last drawn
    function frame(now) {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      if (update(dt)) behind = true;
      const calm = mode === "pet" && seqName === "idle" && !ultra && !working;
      if (dirty || (behind && (!calm || now - lastDrawn > 30))) {
        dirty = behind = false;
        lastDrawn = now;
        renderer.render(scene, camera);
      }
      requestAnimationFrame(frame);
    }
    requestAnimationFrame((now) => { last = now; frame(now); });

    return {
      play,
      setAuto,
      get auto() { return auto; },
      get move() { return seqName; },
      setAmbient,
      setFollowing,
      setUltra,
      get ultra() { return ultra; },
      setMode,
      get mode() { return mode; },
      setColors,
      playRandom: () => play(pick(ON_CLICK)),
      tap: () => playClickMove(),
      work,
      get working() { return working; },
      hold,
      setGaze,
      screenRect,
      screenRects,
      // Whether a tap just landed: for a little while the spot keeps taking clicks, so a double or triple tap lands
      // even if the move the first tap started carried him away.
      get tapping() { return tapping(); },
      entrance: (opts2) => play(pick(ENTRANCES), opts2)
    };
  }

  // IDLE, FRAMING, SCREENS and the laptop indices are read by the Rust port's export (rust/tools/export-moves.mjs).
  window.ClawdEngine = {
    MOVES, ALL, ENTRANCES, ON_CLICK, IDLE, FRAME_MS, FRAMING, SCREENS, LAPTOP_OPEN, TYPING_FROM, TYPING_TO, describe, create
  };
})();
