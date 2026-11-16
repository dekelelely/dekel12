// A generated rock riff timed to Clawd's ultracode routine (60 ms frames), built from oscillators, distortion and noise
// with the Web Audio API: no samples, nothing downloaded. A preview for now; not wired into the apps.
// ClawdRiff.schedule(ctx, destination, startTime) plays it on any AudioContext or OfflineAudioContext.
(() => {
  const F = 0.06;                                    // one frame of the routine
  const DURATION = 8.6;
  const N = {
    E2: 82.41, G2: 98.0, A2: 110.0, B2: 123.47, D3: 146.83, E3: 164.81, Fs3: 185.0, G3: 196.0,
    E4: 329.63, G4: 392.0, A4: 440.0, B4: 493.88, D5: 587.33, E5: 659.25, G5: 783.99, A5: 880.0
  };
  const power = (root) => [root, root * 1.4983, root * 2];          // root, fifth, octave
  const big = (root) => [root, root * 1.4983, root * 2, root * 2.9966, root * 4];

  // Soft-clipping curve for the amp's distortion. An odd length puts an exact zero in the middle, so silence in
  // gives silence out (an even length would turn silence into a small constant offset).
  const CURVE = (() => {
    const n = 2049, c = new Float32Array(n), k = 60;
    for (let i = 0; i < n; i++) {
      const x = (i * 2) / (n - 1) - 1;
      c[i] = ((1 + k) * x) / (1 + k * Math.abs(x));
    }
    return c;
  })();

  function schedule(ctx, destination, t0) {
    // Live playback builds each sound just before it is due (a phone can't run every note's nodes at once);
    // an OfflineAudioContext builds them all up front.
    const live = !(typeof OfflineAudioContext !== "undefined" && ctx instanceof OfflineAudioContext);
    const queue = [];
    const defer = (fn) => (t, ...args) => {
      if (live && t > ctx.currentTime + 0.5) queue.push({ t, run: () => fn(t, ...args) });
      else fn(t, ...args);
    };
    if (live) {
      const timer = setInterval(() => {
        queue.sort((a, b) => a.t - b.t);
        while (queue.length && queue[0].t <= ctx.currentTime + 0.5) queue.shift().run();
        if (!queue.length) clearInterval(timer);
      }, 100);
    }

    const dcBlock = ctx.createBiquadFilter();       // removes any constant offset before the compressor
    dcBlock.type = "highpass";
    dcBlock.frequency.value = 25;
    const master = ctx.createDynamicsCompressor();
    dcBlock.connect(master);
    master.threshold.value = -16;
    master.ratio.value = 4;
    master.attack.value = 0.003;
    master.release.value = 0.12;
    const out = ctx.createGain();
    out.gain.value = 0.5;   // keeps the peaks under full scale when played live
    master.connect(out);
    out.connect(destination);
    const guitarBus = ctx.createGain();
    guitarBus.gain.value = 0.85;
    guitarBus.connect(dcBlock);
    const drumBus = ctx.createGain();
    drumBus.gain.value = 0.55;
    drumBus.connect(dcBlock);

    const noise = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate);
    const nd = noise.getChannelData(0);
    for (let i = 0; i < nd.length; i++) nd[i] = Math.random() * 2 - 1;
    const at = (frame) => t0 + frame * F;

    const noiseHit = defer(function (t, { type = "highpass", freq = 5000, q = 0.7, level = 0.3, decay = 0.3, bus = drumBus, sweepTo = 0 }) {
      const src = ctx.createBufferSource();
      src.buffer = noise;
      const filter = ctx.createBiquadFilter();
      filter.type = type;
      filter.frequency.setValueAtTime(freq, t);
      if (sweepTo) filter.frequency.exponentialRampToValueAtTime(sweepTo, t + decay);
      filter.Q.value = q;
      const env = ctx.createGain();
      env.gain.setValueAtTime(level, t);
      env.gain.exponentialRampToValueAtTime(0.0001, t + decay);
      src.connect(filter);
      filter.connect(env);
      env.connect(bus);
      src.start(t);
      src.stop(t + decay + 0.05);
    });
    const crash = (t) => noiseHit(t, { freq: 4000, level: 0.32, decay: 1.4 });
    const hat = (t) => noiseHit(t, { freq: 8000, level: 0.1, decay: 0.035 });
    const snare = defer(function (t) {
      noiseHit(t, { type: "bandpass", freq: 1800, q: 0.9, level: 0.5, decay: 0.16 });
      const o = ctx.createOscillator(), g = ctx.createGain();
      o.type = "triangle";
      o.frequency.setValueAtTime(190, t);
      g.gain.setValueAtTime(0.35, t);
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.09);
      o.connect(g);
      g.connect(drumBus);
      o.start(t);
      o.stop(t + 0.12);
    });
    const kick = defer(function (t) {
      const o = ctx.createOscillator(), g = ctx.createGain();
      o.frequency.setValueAtTime(140, t);
      o.frequency.exponentialRampToValueAtTime(42, t + 0.11);
      g.gain.setValueAtTime(1, t);
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.28);
      o.connect(g);
      g.connect(drumBus);
      o.start(t);
      o.stop(t + 0.3);
    });

    // One pick of the guitar: detuned sawtooth voices into distortion, a tone filter and a cabinet bump.
    // mute: palm-muted chug (dark, short). bend: pitch ratio reached over bendDur. vibrato: depth in cents.
    const guitar = defer(function (t, freqs, dur, { mute = false, level = 0.3, tone = 3200, bend = 1, bendAt = 0, bendDur = 0.2, vibrato = 0 } = {}) {
      const pre = ctx.createGain();
      pre.gain.value = 2.2;
      const shaper = ctx.createWaveShaper();
      shaper.curve = CURVE;
      shaper.oversample = "2x";
      const lp = ctx.createBiquadFilter();
      lp.type = "lowpass";
      lp.frequency.value = mute ? 700 : tone;
      lp.Q.value = 0.8;
      const cab = ctx.createBiquadFilter();
      cab.type = "peaking";
      cab.frequency.value = mute ? 220 : 1600;
      cab.gain.value = mute ? 6 : 4;
      const env = ctx.createGain();
      env.gain.value = 0;
      pre.connect(shaper);
      shaper.connect(lp);
      lp.connect(cab);
      cab.connect(env);
      env.connect(guitarBus);
      const end = t + dur;
      env.gain.setValueAtTime(0.0001, t);
      env.gain.exponentialRampToValueAtTime(level, t + 0.005);
      if (mute || dur < 0.25) {
        env.gain.exponentialRampToValueAtTime(0.0001, t + (mute ? Math.min(dur, 0.11) : dur));
      } else {
        env.gain.exponentialRampToValueAtTime(level * 0.55, t + 0.15);
        env.gain.exponentialRampToValueAtTime(level * (dur > 0.8 ? 0.12 : 0.35), end - 0.08);   // rings down
        env.gain.exponentialRampToValueAtTime(0.0001, end);
      }
      let wobble = null;
      if (vibrato) {
        const lfo = ctx.createOscillator();
        lfo.frequency.value = 6;
        wobble = ctx.createGain();
        wobble.gain.value = vibrato;
        lfo.connect(wobble);
        lfo.start(t + 0.12);
        lfo.stop(end + 0.05);
      }
      for (const f of freqs) {
        for (const detune of [-7, 7]) {
          const o = ctx.createOscillator();
          o.type = "sawtooth";
          o.detune.value = detune;
          o.frequency.setValueAtTime(f, t);
          if (bend !== 1) {
            o.frequency.setValueAtTime(f, t + bendAt);
            o.frequency.exponentialRampToValueAtTime(f * bend, t + bendAt + bendDur);
          }
          if (wobble) wobble.connect(o.detune);
          o.connect(pre);
          o.start(t);
          o.stop(end + 0.05);
        }
      }
    });
    const chug = (frame) => guitar(at(frame), power(N.E2), F, { mute: true });
    const stab = (frame, root, frames = 2) => guitar(at(frame), power(root), frames * F, { level: 0.28 });
    const lead = (frame, note, frames = 0.5) => guitar(at(frame), [note], frames * F, { level: 0.22, tone: 4500 });
    function slam(frame, frames, root = N.E2) {   // a big power chord with a crash and a kick
      guitar(at(frame), big(root), frames * F, { level: 0.34, vibrato: 12 });
      crash(at(frame));
      kick(at(frame));
    }

    // Drums: a steady rock beat (a quarter note is 8 frames, 125 bpm): kick, snare, eighth-note hi-hats.
    for (let f = 4; f < 116; f += 2) hat(at(f));
    for (let f = 4; f < 116; f += 16) {
      kick(at(f));
      snare(at(f + 8));
    }

    // Grabs the guitar: a pick scrape and a slide up.
    noiseHit(at(1), { type: "bandpass", freq: 6000, q: 2, level: 0.35, decay: 0.16, bus: guitarBus, sweepTo: 900 });
    guitar(at(2), [N.E3], 2 * F, { level: 0.24, bend: 2, bendDur: 0.1 });
    // Headbang stomp: palm-muted chugs.
    for (let f = 4; f < 12; f++) chug(f);
    // Winds up, then the launch jump: an open E bent up at the top.
    stab(12, N.G2, 1);
    stab(13, N.A2, 1);
    guitar(at(14), power(N.E2), 6 * F, { level: 0.32, bend: 1.122, bendAt: 2 * F, bendDur: 0.15, vibrato: 10 });
    kick(at(14));
    // Hop, hop, hop: rising stabs, a chug on each landing.
    [20, 24, 28, 32].forEach(chug);
    stab(22, N.G2);
    stab(26, N.A2);
    stab(30, N.B2);
    // Edge shred: a fast E minor pentatonic run up.
    [N.E4, N.G4, N.A4, N.B4, N.D5, N.E5, N.G5, N.A5].forEach((note, i) => lead(33 + i * 0.5, note));
    [37, 38].forEach(chug);
    // Spin jump: a chord rising with him, landing on a slam at the top.
    guitar(at(39), power(N.D3), 4 * F, { level: 0.28, bend: 1.5, bendDur: 4 * F });
    slam(43, 3);
    [46, 47].forEach(chug);
    // Knee slide: the chord slides down while feedback squeals on top.
    guitar(at(48), power(N.A2), 6 * F, { level: 0.3, bend: 0.75, bendDur: 6 * F });
    guitar(at(48), [N.E5], 7 * F, { level: 0.12, tone: 5000, vibrato: 30 });
    slam(54, 1);
    [55, 56].forEach(chug);
    stab(57, N.G2, 1);
    // Scissor kick.
    stab(58, N.A2, 3);
    slam(61, 3);
    chug(65);
    // Duck walk: a Chuck Berry boogie on A.
    for (let i = 0; i < 10; i++) {
      guitar(at(66 + i), i % 2 ? [N.A2, N.Fs3] : [N.A2, N.E3], F * 0.9, { mute: true, level: 0.32 });
    }
    // Windmill: two huge strummed chords.
    slam(76, 4);
    guitar(at(80), big(N.E2), 4 * F, { level: 0.32, vibrato: 12 });
    // Side sway: chug, chug, stab.
    [84, 85, 88, 89].forEach(chug);
    stab(86, N.G2);
    stab(90, N.A2);
    // Headbang: fast chugs.
    for (let f = 92; f < 98; f++) chug(f);
    // Big leap: a rising slide into the power chord.
    guitar(at(98), power(N.B2), 5 * F, { level: 0.3, bend: 1.335, bendDur: 5 * F });
    slam(103, 5);
    kick(at(108));
    // Shreds: a quick run back down.
    [N.E5, N.D5, N.B4, N.A4, N.G4, N.E4].forEach((note, i) => lead(109 + i * 0.5, note));
    // Rock kick, then the final chord rings out.
    stab(112, N.G2, 1);
    stab(113, N.A2, 1);
    guitar(at(115), big(N.E2), 1.5, { level: 0.34, vibrato: 14 });
    crash(at(115));
    kick(at(115));

    return t0 + DURATION;
  }

  window.ClawdRiff = { duration: DURATION, schedule };
})();
