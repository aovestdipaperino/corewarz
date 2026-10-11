// The CoreWarz browser front end: warrior editors, battle controls, and a
// canvas view of the core. All simulation happens in WebAssembly; this file
// only paints the buffers the Battle hands back and forwards user input.

import init, {
  Battle,
  sampleNames,
  sampleSource,
  sampleIsExtended,
} from "./pkg/corewarz_web.js";

const MAX_WARRIORS = 6;

// Matches the terminal visualizer's palette, by warrior index.
const COLORS = [
  [34, 211, 238],
  [232, 121, 249],
  [74, 222, 128],
  [250, 204, 21],
  [96, 165, 250],
  [248, 113, 113],
];
const EMPTY = [30, 39, 56];
const EXEC = [255, 255, 255];

// Cycles per second at each speed setting.
const SPEEDS = [1, 5, 25, 100, 500, 2000, 10000, 50000];

// Pixel size of one cell, including a one-pixel gap.
const CELL = 6;

const $ = (id) => document.getElementById(id);
const canvas = $("core");
const ctx = canvas.getContext("2d");

let battle = null;
let running = false;
let budget = 0;
let lastFrame = 0;
let columns = 1;
let image = null;
let hovered = null;

function rgb([r, g, b]) {
  return `rgb(${r}, ${g}, ${b})`;
}

// --- Warrior editors -------------------------------------------------------

function addWarrior(sample) {
  const list = $("warriors");
  if (list.children.length >= MAX_WARRIORS) return;
  const card = $("warrior-template").content.firstElementChild.cloneNode(true);
  const select = card.querySelector(".sample");
  const source = card.querySelector(".source");

  select.append(new Option("custom", ""));
  for (const name of sampleNames()) {
    const label = sampleIsExtended(name) ? `${name} (extended)` : name;
    select.append(new Option(label, name));
  }
  select.addEventListener("change", () => {
    if (!select.value) return;
    source.value = sampleSource(select.value);
    if (sampleIsExtended(select.value)) $("extended").checked = true;
  });
  source.addEventListener("input", () => {
    select.value = "";
  });
  card.querySelector(".remove").addEventListener("click", () => {
    if (list.children.length > 1) {
      card.remove();
      refreshWarriorCards();
    }
  });

  select.value = sample;
  source.value = sampleSource(sample) ?? "";
  list.append(card);
  refreshWarriorCards();
}

function refreshWarriorCards() {
  const cards = [...$("warriors").children];
  cards.forEach((card, index) => {
    card.querySelector(".swatch").style.background = rgb(COLORS[index]);
    card.querySelector(".remove").disabled = cards.length === 1;
  });
  $("add-warrior").disabled = cards.length >= MAX_WARRIORS;
}

// --- Battle lifecycle ------------------------------------------------------

function number(id) {
  return Math.max(0, Math.floor(Number($(id).value) || 0));
}

function load() {
  const sources = [...document.querySelectorAll("#warriors .source")].map(
    (area) => area.value,
  );
  let next;
  try {
    next = new Battle(
      sources,
      $("extended").checked,
      number("core-size"),
      number("max-cycles"),
      number("seed"),
    );
  } catch (error) {
    $("error").textContent = error.message ?? String(error);
    return;
  }
  $("error").textContent = "";
  battle?.free();
  battle = next;
  running = false;
  budget = 0;
  hovered = null;
  layoutCanvas();
  $("play").disabled = false;
  $("step").disabled = false;
  draw();
}

function layoutCanvas() {
  const size = battle.coreSize();
  columns = Math.ceil(Math.sqrt(size * 1.25));
  const rows = Math.ceil(size / columns);
  canvas.width = columns * CELL;
  canvas.height = rows * CELL;
  image = ctx.createImageData(canvas.width, canvas.height);
}

function togglePlay() {
  if (!battle || battle.outcome()) return;
  running = !running;
  budget = 0;
  updateButtons();
}

function step() {
  if (!battle) return;
  running = false;
  battle.run(1);
  draw();
}

function updateButtons() {
  const over = !battle || battle.outcome() !== undefined;
  $("play").textContent = running ? "Pause" : "Run";
  $("play").disabled = over;
  $("step").disabled = over;
}

// --- Drawing ---------------------------------------------------------------

function paintCell(pixels, addr, [r, g, b]) {
  const x0 = (addr % columns) * CELL;
  const y0 = Math.floor(addr / columns) * CELL;
  for (let y = y0; y < y0 + CELL - 1; y++) {
    let i = (y * canvas.width + x0) * 4;
    for (let x = 0; x < CELL - 1; x++, i += 4) {
      pixels[i] = r;
      pixels[i + 1] = g;
      pixels[i + 2] = b;
      pixels[i + 3] = 255;
    }
  }
}

function cellColor(owner, heat) {
  if (owner === 0) return EMPTY;
  // Owned cells glow at 40% of their warrior's colour and flare towards
  // white when freshly written.
  const base = COLORS[owner - 1];
  const t = heat / 255;
  const level = 0.4 + 0.6 * t;
  return base.map((c, i) => {
    const lit = EMPTY[i] + (c - EMPTY[i]) * level;
    return Math.round(lit + (255 - lit) * 0.35 * t * t);
  });
}

function draw() {
  const owners = battle.owners();
  const heat = battle.heat();
  const pixels = image.data;
  for (let addr = 0; addr < owners.length; addr++) {
    paintCell(pixels, addr, cellColor(owners[addr], heat[addr]));
  }
  for (let w = 0; w < battle.warriorCount(); w++) {
    const pc = battle.nextPc(w);
    if (pc !== undefined) paintCell(pixels, pc, EXEC);
  }
  ctx.putImageData(image, 0, 0);
  drawStatus();
  drawScoreboard();
  drawInspect();
  updateButtons();
}

function drawStatus() {
  const outcome = battle.outcome();
  const progress = `cycle ${battle.cycle()} / ${battle.maxCycles()}`;
  $("status").textContent = outcome ? `${progress}: ${outcome}` : progress;
}

function drawScoreboard() {
  const table = $("scoreboard");
  const rows = ["<tr><th></th><th>warrior</th><th>processes</th><th>deaths</th><th>next</th></tr>"];
  for (let w = 0; w < battle.warriorCount(); w++) {
    const pc = battle.nextPc(w);
    const next = pc === undefined ? "dead" : `${pc}: ${battle.instructionAt(pc)}`;
    rows.push(
      `<tr class="${pc === undefined ? "dead" : ""}">` +
        `<td><span class="swatch" style="display:inline-block;background:${rgb(COLORS[w])}"></span></td>` +
        `<td>${escape(battle.warriorName(w))}</td>` +
        `<td class="num">${battle.processCount(w)}</td>` +
        `<td class="num">${battle.deaths(w)}</td>` +
        `<td class="code">${escape(next)}</td></tr>`,
    );
  }
  table.innerHTML = rows.join("");
}

function drawInspect() {
  if (hovered === null || !battle || hovered >= battle.coreSize()) return;
  const owner = battle.owners()[hovered];
  const by = owner ? ` (last written by ${battle.warriorName(owner - 1)})` : "";
  $("inspect").textContent = `${hovered}: ${battle.instructionAt(hovered)}${by}`;
}

function escape(text) {
  return text.replace(/[&<>"]/g, (c) => `&#${c.charCodeAt(0)};`);
}

// --- Animation loop --------------------------------------------------------

function frame(now) {
  const elapsed = Math.min((now - lastFrame) / 1000, 0.1);
  lastFrame = now;
  if (battle) {
    if (running) {
      budget += SPEEDS[$("speed").value] * elapsed;
      const cycles = Math.floor(budget);
      budget -= cycles;
      if (cycles > 0 && battle.run(cycles)) running = false;
    }
    draw();
    battle.decay();
  }
  requestAnimationFrame(frame);
}

function showSpeed() {
  $("speed-label").textContent = `${SPEEDS[$("speed").value]} cycles/s`;
}

// --- Wiring ----------------------------------------------------------------

function bindEvents() {
  $("load").addEventListener("click", load);
  $("play").addEventListener("click", togglePlay);
  $("step").addEventListener("click", step);
  $("add-warrior").addEventListener("click", () => addWarrior("imp"));
  $("speed").addEventListener("input", showSpeed);

  canvas.addEventListener("mousemove", (event) => {
    if (!battle) return;
    const rect = canvas.getBoundingClientRect();
    const x = Math.floor(((event.clientX - rect.left) / rect.width) * canvas.width / CELL);
    const y = Math.floor(((event.clientY - rect.top) / rect.height) * canvas.height / CELL);
    hovered = y * columns + x;
    drawInspect();
  });

  document.addEventListener("keydown", (event) => {
    const tag = event.target.tagName;
    if (tag === "TEXTAREA" || tag === "INPUT" || tag === "SELECT") return;
    const speed = $("speed");
    if (event.key === " ") {
      event.preventDefault();
      togglePlay();
    } else if (event.key === "s") {
      step();
    } else if (event.key === "+" || event.key === "=") {
      speed.value = Math.min(Number(speed.value) + 1, SPEEDS.length - 1);
      showSpeed();
    } else if (event.key === "-") {
      speed.value = Math.max(Number(speed.value) - 1, 0);
      showSpeed();
    }
  });
}

await init();
bindEvents();
showSpeed();
addWarrior("imp");
addWarrior("dwarf");
load();
requestAnimationFrame(frame);
