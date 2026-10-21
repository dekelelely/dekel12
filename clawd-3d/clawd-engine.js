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

  const MOVES = {
    jump: JUMP,
    look: [...run("look-right", 5), ...run("look-left", 5), ...run("default", 1)],
    celebrate: [...JUMP, ...run("default", 3, { offset: 1 })],
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
    ]
  };
  const IDLE = [...run("default", 12), ...run("look-right", 5), ...run("look-left", 5)];
  const ENTRANCES = ["skip", "jump", "look", "spin", "peekaboo", "drop", "waddle", "peek", "wink", "boop", "tap", "sneeze", "turn", "coin-hop"];
  const ON_CLICK = [...ENTRANCES.filter((n) => (MOVES[n][0].x || 0) === 0), "laptop"];
  const ALL = Object.keys(MOVES);

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

  // Camera framing per mode: "stage" shows the whole floor strip; "pet" frames Clawd alone for a small corner window.
  const FRAMING = {
    stage: { target: [-3, 6, 0], theta: 0.38, phi: 1.36, halfH: 12, halfW: 23 },
    pet: { target: [0, 6.8, 0], theta: 0.3, phi: 1.42, halfH: 10.5, halfW: 14 }
  };

  // create({ stage, canvas, fx, mode, on... }) builds the scene in `canvas` (sized to `stage`) and returns a controller,
  // or null when WebGL isn't available.
  //   mode: "stage" (floor strip, drag to orbit) or "pet" (no floor, just a shadow; drag reports onDrag in screen pixels)
  //   onMove(name), onFrame({ name, index, length, frame }), onAuto(on), onHover(overClawd),
  //   onDrag(dx, dy), onDragEnd(), onLongPress(), onDoubleClick()
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

    const SCREEN_TEXT = ["$ claude", "> fix the flaky test", "", "Reading tests/api.spec.ts", "Editing src/retry.ts", "  + await backoff(attempt)", "Running npm test", "  42 passed", "Done."].join("\n");
    let typed = -1, typeTarget = 0;
    function drawScreen(n) {
      if (n === typed) return;
      typed = n;
      const c = screenCtx;
      c.fillStyle = "#11181d";
      c.fillRect(0, 0, screenCanvas.width, screenCanvas.height);
      c.font = "bold 15px Menlo, Consolas, monospace";
      c.textBaseline = "top";
      const lines = SCREEN_TEXT.slice(0, n).split("\n").slice(-9);
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

    // Code that floods out from Clawd, as in the Claude app: a growing disc of text rows with a bright, crisp
    // leading edge and dimmer characters behind it; once it covers the stage it holds, then fades out together.
    const fctx = fx.getContext("2d");
    const GLYPHS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const hash = (i, j, k) => { const v = Math.sin(i * 127.1 + j * 311.7 + k * 74.7) * 43758.5453; return v - Math.floor(v); };
    const RING = { grow: 0.9, hold: 0.35, fade: 1.0, edge: 46, inside: 0.32 };
    let ripple = null, codeColor = "#0e7486";
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
    function drawRipple(dt) {
      if (!ripple) return;
      const s = fx.width / Math.max(1, stage.clientWidth);
      ripple.t += dt;
      const t = ripple.t;
      fctx.clearRect(0, 0, fx.width, fx.height);
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
    const dotGeo = new THREE.BoxGeometry(0.45, 0.06, 0.45);
    const dots = new THREE.Group();
    for (let x = -44; x <= 44; x += 1.6) {
      const d = new THREE.Mesh(dotGeo, dotMat);
      d.position.set(x, 0.03, 7.6);
      d.receiveShadow = true;
      dots.add(d);
    }
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
    const dustGeo = new THREE.BoxGeometry(0.6, 0.6, 0.6);
    const dust = Array.from({ length: 28 }, () => {
      const m = new THREE.Mesh(dustGeo, dustMat);
      m.visible = false;
      m.userData = { v: new THREE.Vector3(), life: 0 };
      scene.add(m);
      return m;
    });
    function puff(kind) {
      const count = kind === "wave" ? 10 : 4;
      const spread = kind === "wave" ? 7 : 4;
      let made = 0;
      for (const m of dust) {
        if (m.visible) continue;
        const side = made % 2 ? 1 : -1;
        m.position.set(root.position.x + side * (BODY_W / 2 - 0.5), 0.35, (Math.random() - 0.5) * DEPTH);
        m.userData.v.set(side * (2 + Math.random() * spread), 1 + Math.random() * 2, (Math.random() - 0.5) * 3);
        m.userData.life = 0.6 + Math.random() * 0.3;
        m.visible = true;
        if (++made === count) break;
      }
    }

    function setColors(c) {
      if (c.floor) floorMat.color.copy(lin(c.floor));
      if (c.dots) dotMat.color.copy(lin(c.dots));
      if (c.dust) dustMat.color.copy(lin(c.dust));
      if (c.code) codeColor = c.code;
    }

    // ---- Turning a frame into 3D targets ----
    const TAU = Math.PI * 2;
    const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
    const wrap = (a) => a - TAU * Math.round(a / TAU);
    const SINK = [0, 0, LEG_H + BODY_H / 2, LEG_H + BODY_H + 1];   // offset 2 leaves only the eyes above the line
    const state = { x: 0, y: 0, ry: 0, crouch: 0, armL: 0, armR: 0, liftL: 0, liftR: 0, eyeX: 0, eyeY: 0, lidL: 0, lidR: 0, squash: 1, laptop: 0, type: 0 };
    const goal = { ...state };

    function targetsFor(f) {
      const p = typeof f.pose === "string" ? POSES[f.pose] : f.pose.facing ? POSES.default : f.pose;
      goal.x = (f.x || 0) * COL;
      goal.ry = f.angle * Math.PI / 180;
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
      goal.eyeX = p.eyes === "left" ? -1 : p.eyes === "right" ? 1 : 0;
      goal.eyeY = p.eyes === "left" || p.eyes === "right" ? 0.8 : 0;
      goal.lidL = p.eyes === "closed" ? 1 : 0;
      goal.lidR = p.eyes === "closed" || p.eyes === "wink" ? 1 : 0;
      goal.laptop = f.laptop || 0;
      goal.type = f.typing ? 1 : 0;
    }

    // ---- Playback ----
    let seq = IDLE, seqName = "idle", idx = 0, acc = 0;
    let auto = false, autoNext = 0;
    let ambient = false, nextAmbient = 0;   // pet mode: a random move every so often, between look-arounds
    let clock = 0;

    function enterFrame() {
      const f = seq[idx];
      targetsFor(f);
      if (f.poof) puff(f.poof);
      if (f.ripple) { startRipple(); typeTarget = 0; }
      if (f.typing) typeTarget = Math.min(SCREEN_TEXT.length, typeTarget + 4);
      if (f.laptop) drawScreen(typeTarget);
      on("onFrame", { name: seqName, index: idx, length: seq.length, frame: f });
    }
    function play(name, { snap = false } = {}) {
      if (name !== "idle" && !MOVES[name]) return;
      seqName = name;
      seq = name === "idle" ? IDLE : MOVES[name];
      idx = 0;
      acc = 0;
      on("onMove", name);
      enterFrame();
      if (snap) for (const prop of Object.keys(state)) state[prop] = goal[prop];
    }
    const pick = (list) => list[Math.floor(Math.random() * list.length)];
    function scheduleAmbient() { nextAmbient = clock + 18 + Math.random() * 27; }
    function finished() {
      if (seqName === "idle" && auto) {
        play(ALL[autoNext]);
        autoNext = (autoNext + 1) % ALL.length;
      } else if (seqName === "idle" && ambient && clock >= nextAmbient) {
        scheduleAmbient();
        play(pick(ON_CLICK));
      } else {
        play("idle");
      }
    }
    function setAuto(value) {
      auto = !!value;
      on("onAuto", auto);
    }
    function setAmbient(value) {
      ambient = !!value;
      scheduleAmbient();
    }

    // ---- Camera, pointer and picking ----
    const raycaster = new THREE.Raycaster();
    const ndc = new THREE.Vector2();
    const target = new THREE.Vector3();
    const view = { theta: 0, phi: 0, tTheta: 0, tPhi: 0, r: 60 };
    let mode = opts.mode === "pet" ? "pet" : "stage";
    let drag = null, hovering = false, longTimer = 0;

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
        setHover(hitsClawd());
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
        if (hitsClawd()) playClickMove();
      }
      toNdc(e);
      setHover(hitsClawd());
    }
    canvas.addEventListener("pointerup", (e) => endDrag(e, false));
    canvas.addEventListener("pointercancel", (e) => endDrag(e, true));
    canvas.addEventListener("pointerleave", () => { if (!drag) setHover(false); });
    canvas.addEventListener("dblclick", (e) => { toNdc(e); if (hitsClawd()) on("onDoubleClick"); });

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
    }
    if (window.ResizeObserver) new ResizeObserver(resize).observe(stage);
    else window.addEventListener("resize", resize);
    setMode(mode);

    // ---- Loop ----
    let last = performance.now();
    function update(dt) {
      clock += dt;
      acc += dt * 1000;
      while (acc >= FRAME_MS) {
        acc -= FRAME_MS;
        idx++;
        if (idx >= seq.length) finished();
        else enterFrame();
      }

      const k = 1 - Math.exp(-dt / 0.045);
      for (const prop of Object.keys(state)) {
        if (prop === "ry") state.ry += wrap(goal.ry - state.ry) * k;
        else state[prop] += (goal[prop] - state[prop]) * k;
      }

      root.position.set(state.x, state.y, 0);
      root.rotation.y = state.ry;
      const legScale = 1 - 0.85 * state.crouch;
      body.position.y = LEG_H * legScale + state.type * 0.25 * (1 - Math.abs(Math.sin(clock * 42)));   // dips into each keystroke
      body.scale.set(1 / Math.sqrt(state.squash), state.squash, 1 / Math.sqrt(state.squash));
      legs.forEach((leg) => {
        leg.scale.y = legScale;
        leg.position.y = LEG_H * legScale + 0.9 * (leg.userData.left ? state.liftL : state.liftR);
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
      });
      const shown = clamp(state.laptop * 2.5, 0, 1);
      laptop.visible = shown > 0.01;
      laptop.scale.setScalar(Math.max(shown, 0.001));
      hinge.rotation.x = clamp((state.laptop - 0.2) / 0.8, 0, 1) * THREE.MathUtils.degToRad(105);
      drawRipple(dt);
      eyes.forEach((eye, i) => {
        const lid = i === 0 ? state.lidL : state.lidR;
        eye.scale.y = 1 - 0.82 * lid;
        eye.position.x = eye.userData.home.x + state.eyeX;
        eye.position.y = eye.userData.home.y + state.eyeY - 0.45 * lid;
      });

      for (const m of dust) {
        if (!m.visible) continue;
        const d = m.userData;
        d.life -= dt * 1.7;
        if (d.life <= 0) { m.visible = false; continue; }
        d.v.y -= 5 * dt;
        d.v.multiplyScalar(1 - 2.4 * dt);
        m.position.addScaledVector(d.v, dt);
        m.position.y = Math.max(0.3, m.position.y);
        m.scale.setScalar(d.life);
      }

      const ease = Math.min(1, dt * 8);
      view.theta += (view.tTheta - view.theta) * ease;
      view.phi += (view.tPhi - view.phi) * ease;
      const sp = Math.sin(view.phi);
      camera.position.set(
        target.x + view.r * sp * Math.sin(view.theta),
        target.y + view.r * Math.cos(view.phi),
        target.z + view.r * sp * Math.cos(view.theta)
      );
      camera.lookAt(target);
    }

    function frame(now) {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      update(dt);
      renderer.render(scene, camera);
      requestAnimationFrame(frame);
    }
    requestAnimationFrame((now) => { last = now; frame(now); });

    return {
      play,
      setAuto,
      get auto() { return auto; },
      get move() { return seqName; },
      setAmbient,
      setMode,
      get mode() { return mode; },
      setColors,
      playRandom: () => play(pick(ON_CLICK)),
      entrance: (opts2) => play(pick(ENTRANCES), opts2)
    };
  }

  window.ClawdEngine = { MOVES, ALL, ENTRANCES, ON_CLICK, describe, create };
})();
