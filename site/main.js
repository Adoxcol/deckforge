// DeckForge website behaviour: the interactive deck, the editor screen tabs,
// the copy button, and the latest-release lookup. Everything degrades to plain links.

const REPO = "Adoxcol/deckforge";

// Same timings as the app (src-tauri/src/gestures.rs).
const HOLD_MS = 450;
const DOUBLE_MS = 280;
const SWIPE_PX = 36;
const KNOB_PX_PER_DETENT = 12;
const KNOB_DEG_PER_DETENT = 15; // 24 detents per turn

const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

/* ------------------------------------------------------------------ */
/* Interactive deck                                                    */
/* ------------------------------------------------------------------ */

function initDeck() {
  const deck = $("[data-deck]");
  if (!deck) return;
  const strip = $("[data-strip]", deck);
  const out = {
    name: $("[data-r-name]"),
    where: $("[data-r-where]"),
    rows: $("[data-r-rows]"),
    live: $("[data-r-live]"),
  };
  const pageZone = $$(".zone", strip)[1];
  let page = 2;
  const PAGES = 3;

  const rowsFor = (el) => {
    const d = el.dataset;
    if (d.kind === "key") return [["tap", "Tap", d.tap], ["double", "Double-press", d.double], ["hold", "Hold", d.hold]];
    const swipes = [["swipe-left", "Swipe left", "Page: Previous page"], ["swipe-right", "Swipe right", "Page: Next page"]];
    if (d.kind === "strip") return swipes;
    if (d.kind === "zone") return [["tap", "Tap", d.tap], ...swipes];
    return [["cw", "Turn right", d.cw], ["ccw", "Turn left", d.ccw], ["push", "Push", d.push]];
  };

  const show = (el, gesture, detail) => {
    out.name.textContent = el.dataset.name;
    out.where.textContent = el.dataset.where;
    out.rows.replaceChildren(
      ...rowsFor(el).map(([id, label, action]) => {
        const li = document.createElement("li");
        const g = document.createElement("span");
        const a = document.createElement("span");
        g.className = "g";
        g.textContent = label;
        a.className = action ? "a" : "a unset";
        a.textContent = action ? (id === gesture && detail ? `${action} · ${detail}` : action) : "Not set";
        if (id === gesture) li.className = "is-hit";
        li.append(g, a);
        return li;
      }),
    );
  };

  const flash = (el) => {
    el.classList.remove("is-fired");
    void el.offsetWidth; // restart the animation
    el.classList.add("is-fired");
  };

  const fire = (el, gesture, detail) => {
    const row = rowsFor(el).find(([id]) => id === gesture);
    show(el, gesture, detail);
    flash(el.classList.contains("knob") ? $(".knob-dial", el) : el);
    out.live.textContent = `${el.dataset.name}: ${row[1]}, ${row[2]}${detail ? `, ${detail}` : ""}`;
  };

  const setPage = (next) => {
    page = ((next - 1 + PAGES) % PAGES) + 1;
    $(".zone-v", pageZone).textContent = `Page ${page} of ${PAGES}`;
  };

  // Keys: one press can be a tap, a double-press or a hold.
  // As in the app, a tap only waits for a possible double-press when the key has one.
  for (const key of $$(".key", deck)) {
    let holdTimer = 0;
    let tapTimer = 0;
    let held = false;
    let down = false;
    let awaitingSecond = false;

    const press = () => {
      if (down) return;
      down = true;
      held = false;
      key.classList.add("is-down");
      if (key.dataset.hold) {
        holdTimer = setTimeout(() => {
          held = true;
          clearTimeout(tapTimer);
          awaitingSecond = false;
          fire(key, "hold");
        }, HOLD_MS);
      }
    };
    const release = (cancelled = false) => {
      if (!down) return;
      down = false;
      key.classList.remove("is-down");
      clearTimeout(holdTimer);
      if (held || cancelled) return;
      if (!key.dataset.double) return fire(key, "tap");
      if (awaitingSecond) {
        clearTimeout(tapTimer);
        awaitingSecond = false;
        return fire(key, "double");
      }
      awaitingSecond = true;
      tapTimer = setTimeout(() => {
        awaitingSecond = false;
        fire(key, "tap");
      }, DOUBLE_MS);
    };

    key.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      key.setPointerCapture(e.pointerId);
      press();
    });
    key.addEventListener("pointerup", () => release());
    key.addEventListener("pointercancel", () => release(true));
    key.addEventListener("contextmenu", (e) => e.preventDefault());
    key.addEventListener("keydown", (e) => {
      if ((e.key === "Enter" || e.key === " ") && !e.repeat) {
        e.preventDefault();
        press();
      }
    });
    key.addEventListener("keyup", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        release();
      }
    });
    key.addEventListener("blur", () => release(true));
    key.addEventListener("focus", () => key.matches(":focus-visible") && show(key));
  }

  // Touch strip: tap a zone, or drag across the strip to swipe.
  let stripStart = null;
  strip.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    const zone = e.target.closest(".zone");
    stripStart = { x: e.clientX, y: e.clientY, zone, id: e.pointerId };
    zone?.classList.add("is-down");
  });
  strip.addEventListener("pointermove", (e) => {
    if (!stripStart || e.pointerId !== stripStart.id) return;
    if (Math.abs(e.clientX - stripStart.x) > SWIPE_PX / 2) {
      strip.classList.add("is-swiping");
      stripStart.zone?.classList.remove("is-down");
    }
  });
  const endStrip = (e, cancelled) => {
    if (!stripStart || e.pointerId !== stripStart.id) return;
    const { x, y, zone } = stripStart;
    stripStart = null;
    strip.classList.remove("is-swiping");
    zone?.classList.remove("is-down");
    if (cancelled) return;
    const dx = e.clientX - x;
    const dy = e.clientY - y;
    if (Math.abs(dx) >= SWIPE_PX && Math.abs(dx) > Math.abs(dy)) {
      setPage(page + (dx > 0 ? 1 : -1));
      fire(strip, dx > 0 ? "swipe-right" : "swipe-left", `page ${page}`);
    } else if (zone) {
      if (zone === pageZone) setPage(page + 1);
      fire(zone, "tap", zone === pageZone ? `page ${page}` : "");
    }
  };
  strip.addEventListener("pointerup", (e) => endStrip(e, false));
  strip.addEventListener("pointercancel", (e) => endStrip(e, true));
  strip.addEventListener("pointerleave", (e) => e.pointerType === "mouse" && endStrip(e, false));
  for (const zone of $$(".zone", strip)) {
    zone.addEventListener("keydown", (e) => {
      if ((e.key === "Enter" || e.key === " ") && !e.repeat) {
        e.preventDefault();
        if (zone === pageZone) setPage(page + 1);
        fire(zone, "tap", zone === pageZone ? `page ${page}` : "");
      } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
        e.preventDefault();
        const right = e.key === "ArrowRight";
        setPage(page + (right ? 1 : -1));
        fire(strip, right ? "swipe-right" : "swipe-left", `page ${page}`);
      }
    });
    zone.addEventListener("focus", () => zone.matches(":focus-visible") && show(zone));
  }

  // Knobs: drag (or arrow keys) to turn in 24 detents per revolution; click or Enter to push.
  for (const knob of $$(".knob", deck)) {
    const dial = $(".knob-dial", knob);
    const valEl = $(".knob-val", knob);
    const d = knob.dataset;
    let rot = 0;
    let value = d.value ? Number(d.value) : null;
    let drag = null;

    const turn = (dir) => {
      rot += dir * KNOB_DEG_PER_DETENT;
      dial.style.setProperty("--rot", `${rot}deg`);
      let detail = "";
      if (value !== null) {
        value = Math.max(0, Math.min(100, value + dir * Number(d.step)));
        valEl.textContent = `${value}${d.unit}`;
        detail = `${value}${d.unit}`;
      }
      fire(knob, dir > 0 ? "cw" : "ccw", detail);
    };
    const push = () => {
      knob.classList.add("is-down");
      setTimeout(() => knob.classList.remove("is-down"), 160);
      fire(knob, "push");
    };

    knob.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      knob.setPointerCapture(e.pointerId);
      drag = { x: e.clientX, y: e.clientY, acc: 0, moved: false };
    });
    knob.addEventListener("pointermove", (e) => {
      if (!drag) return;
      // Right or up turns clockwise.
      drag.acc += e.clientX - drag.x - (e.clientY - drag.y);
      drag.x = e.clientX;
      drag.y = e.clientY;
      while (Math.abs(drag.acc) >= KNOB_PX_PER_DETENT) {
        const dir = Math.sign(drag.acc);
        drag.acc -= dir * KNOB_PX_PER_DETENT;
        drag.moved = true;
        turn(dir);
      }
    });
    knob.addEventListener("pointerup", () => {
      if (drag && !drag.moved) push();
      drag = null;
    });
    knob.addEventListener("pointercancel", () => (drag = null));
    knob.addEventListener("keydown", (e) => {
      if (e.key === "ArrowRight" || e.key === "ArrowUp") { e.preventDefault(); turn(1); }
      else if (e.key === "ArrowLeft" || e.key === "ArrowDown") { e.preventDefault(); turn(-1); }
      else if ((e.key === "Enter" || e.key === " ") && !e.repeat) { e.preventDefault(); push(); }
    });
    knob.addEventListener("focus", () => knob.matches(":focus-visible") && show(knob));
  }
}

/* ------------------------------------------------------------------ */
/* Editor screen tabs                                                  */
/* ------------------------------------------------------------------ */

function initTabs() {
  const tabs = $$('[data-tabs] [role="tab"]');
  if (!tabs.length) return;

  const select = (tab, focus) => {
    for (const t of tabs) {
      const on = t === tab;
      t.setAttribute("aria-selected", String(on));
      t.tabIndex = on ? 0 : -1;
      document.getElementById(t.getAttribute("aria-controls")).hidden = !on;
    }
    if (focus) tab.focus();
    tab.scrollIntoView({ block: "nearest", inline: "nearest" });
  };

  tabs.forEach((tab, i) => {
    tab.addEventListener("click", () => select(tab, false));
    tab.addEventListener("keydown", (e) => {
      const moves = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: tabs.length - 1 };
      if (!(e.key in moves)) return;
      e.preventDefault();
      select(tabs[(moves[e.key] + tabs.length) % tabs.length], true);
    });
  });

  // Warm the other screens once the page has settled, so switching tabs is instant.
  const warm = () =>
    $$('[data-tabs] img[loading="lazy"]').forEach((img) => {
      const pre = new Image();
      pre.sizes = img.sizes;
      pre.srcset = img.srcset;
      pre.src = img.src;
    });
  if ("requestIdleCallback" in window) requestIdleCallback(warm, { timeout: 4000 });
  else setTimeout(warm, 2500);
}

/* ------------------------------------------------------------------ */
/* Copy button                                                         */
/* ------------------------------------------------------------------ */

function initCopy() {
  for (const btn of $$("[data-copy]")) {
    const label = $("span", btn);
    btn.addEventListener("click", async () => {
      const text = $(btn.dataset.copy).innerText
        .split("\n")
        .filter((line) => !line.trimStart().startsWith("#"))
        .join("\n")
        .trim();
      try {
        await navigator.clipboard.writeText(text);
        label.textContent = "Copied";
        btn.classList.add("is-done");
      } catch {
        label.textContent = "Select and copy";
      }
      setTimeout(() => {
        label.textContent = "Copy";
        btn.classList.remove("is-done");
      }, 1800);
    });
  }
}

/* ------------------------------------------------------------------ */
/* Latest release: point Download straight at the installer            */
/* ------------------------------------------------------------------ */

async function initRelease() {
  const CACHE_KEY = "deckforge-release";
  const CACHE_MS = 60 * 60 * 1000;
  let release = null;

  try {
    const cached = JSON.parse(sessionStorage.getItem(CACHE_KEY) ?? "null");
    if (cached && Date.now() - cached.at < CACHE_MS) release = cached.release;
  } catch {}

  if (!release) {
    try {
      const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
        headers: { Accept: "application/vnd.github+json" },
      });
      if (!res.ok) return;
      const data = await res.json();
      const installer = data.assets?.find((a) => /-setup\.exe$/i.test(a.name));
      release = {
        tag: data.tag_name,
        url: data.html_url,
        installer: installer ? { url: installer.browser_download_url, size: installer.size } : null,
      };
      try {
        sessionStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), release }));
      } catch {}
    } catch {
      return; // Offline or rate-limited: the static links to the releases page still work.
    }
  }

  if (!release?.tag) return;
  const link = $("[data-release-link]");
  if (link) {
    link.href = release.url;
    $("[data-release-text]", link).textContent = `${release.tag} is out · release notes`;
  }
  if (release.installer) {
    const mb = (release.installer.size / 1e6).toFixed(1);
    for (const a of $$("[data-download]")) a.href = release.installer.url;
    for (const p of $$("[data-download-meta]")) p.textContent = `${release.tag} · ${mb} MB · 64-bit Windows installer · GPL-3.0`;
  }
}

initDeck();
initTabs();
initCopy();
initRelease();
