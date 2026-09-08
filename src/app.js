/* Chrome Extension Cleaner frontend (no dependencies, inline SVG icons only). */
const SVG = {
copy:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>',
folder:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>',
trash:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 11v6"/><path d="M14 11v6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M3 6h18"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>',
check:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>',
warn:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>',
moon:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401"/></svg>',
sun:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>',
done:'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.801 10A10 10 0 1 1 17 3.335"/><path d="m9 11 3 3L22 4"/></svg>',
find:'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21 21-4.34-4.34"/><circle cx="11" cy="11" r="8"/></svg>',
funnel:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3z"/></svg>',
/* fold / unfold-all: a single "chevrons up-down" glyph — the same icon is used
   for BOTH states (collapse-all and expand-all) since it only signals "toggle
   vertical density of all rows"; the label (collapse / expand) carries the meaning. */
fold:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/></svg>',
/* select-all button: the double-check glyph (two overlapping checks, NO box) —
   the canonical "select all" mark. Used for BOTH states so the icon never
   implies a particular action; only the label (select all / deselect all) carries meaning. */
sel:'<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 7 17l-5-5"/><path d="m22 10-7.5 7.5L13 16"/></svg>'};

/* Real OS, published by the Rust init script (window-chrome fusion, Phase M5).
   That script may only touch `window`: it runs at document start and on Windows
   (WebView2) <html> does not exist yet, so the previous one-liner writing
   documentElement.dataset threw a TypeError the runtime swallowed — `data-os`
   was never set and the self-drawn caption stayed at display:none. index.html
   copies the value onto <html>; this re-applies it so the JS path is correct
   even if that copy is skipped. Empty outside Tauri (no init script), where
   CSS keeps the caption hidden because there is no window to drive. */
const OS = window.__CEC_OS__ || "";
if (OS) document.documentElement.dataset.os = OS;

/* UI strings live in i18n.json so they can be edited, diffed and handed to
   translators without touching this file. JSON cannot hold functions, so the
   former template-literal entries are now {placeholder} strings and callers
   use fmt(T(key), {vars}) instead of T(key)(args). */
let I18N = {en:{}, "zh-CN":{}};
async function loadI18n(){
  const r = await fetch(new URL("i18n.json", document.baseURI));
  if (!r.ok) throw new Error(`i18n.json: HTTP ${r.status}`);
  I18N = await r.json();
}
const fmt = (tpl, vars) =>
  String(tpl).replace(/\{(\w+)\}/g, (m, k) => (k in vars ? vars[k] : m));

/* The 9 UI languages. `code` is the BCP-47 tag (matches Chrome extension
   _locales dirs); `label` is the full name shown in the language dropdown;
   `short` is the glyph shown on the collapsed switcher button. */
const LANGS = [
  {code:"zh-CN", label:"简体中文", short:"简"},
  {code:"zh-TW", label:"繁體中文", short:"繁"},
  {code:"en",    label:"English",  short:"EN"},
  {code:"ko",    label:"한국어",    short:"한"},
  {code:"ja",    label:"日本語",    short:"日"},
  {code:"de",    label:"Deutsch",  short:"DE"},
  {code:"fr",    label:"Français", short:"FR"},
  {code:"es",    label:"Español",  short:"ES"},
  {code:"pt-BR", label:"Português (Brasil)", short:"PT"},
  {code:"ru",    label:"Русский", short:"RU"},
];
const SYS_CODES = LANGS.map(l => l.code.toLowerCase());
/* Map the browser's BCP-47 tag onto a supported UI code. Falls back to "en"
   (T() also has an en fallback, so any locale we don't ship a UI for still
   renders in English). */
function detectSysLang(){
  const nav = (navigator.language || "en").toLowerCase();
  const hit = SYS_CODES.find(c => nav === c || nav.startsWith(c + "-"));
  return hit ? LANGS.find(l => l.code.toLowerCase() === hit).code : "en";
}
const langShort = code => (LANGS.find(l => l.code === code) || {short:code}).short;

/* Tauri IPC: scan + chrome_status + prepare/delete + reveal. */
const { invoke } = window.__TAURI__.core;
let LANG = localStorage.getItem("cec-lang") || detectSysLang();
let DATA = [], LABELS = {};
const STATUS_VALS = ["", "on", "off"];
let STATUS_IDX = 0, SCOPE_FULL = false;
/* Sort cycles through three frontend orderings; the backend only guarantees
   the default (waste desc). dir is -1 = desc, 1 = asc. Status sort is
   intentionally omitted: the On/Off filter already covers that grouping. */
const SORTS = [{key:"waste", dir:-1}, {key:"name", dir:1}, {key:"total", dir:-1}];
let SORT_IDX = 0;
/* Selection lives here, keyed by candidate id. Rendering only reflects it. */
const SELECTED = new Set();
/* Minimum visible duration for a scan, so the spinner / progress bar always
   register even when the backend answers instantly. Also deters double-clicks
   on Rescan. */
const SCAN_MIN_MS = 2000;
const CIDINFO = new Map();
const T = k => ((I18N[LANG] && I18N[LANG][k]) ?? (I18N.en && I18N.en[k]) ?? k);
// The machine's OS language (what Chrome itself uses to localize extension
// names), normalized to a supported code. `name` in the scan payload is
// resolved here as a fallback; the per-locale `names` map covers every UI
// language so the display name follows LANG (the app UI language), not this.
const SYS_LOCALE = detectSysLang();
// Active locale for resolving extension names. Fixed at startup to the OS
// language in production.
let NAME_LOCALE = SYS_LOCALE;
/* ---- feature gates (false = hidden; flip to true to surface for debugging) ----
   Hidden UI is kept in the DOM behind these flags rather than HTML-commented
   out, so re-enabling never requires digging through git history. */
const SHOW_SLOGAN = false;            // header subtitle line
const SHOW_OS_PREVIEW = false;        // TEMP os switcher (Mac/Win/Lin preview)
const SHOW_CONSENT_PREVIEW = false;   // TEMP consent preview key (gated off)
const SHOW_FTMETA = false;            // header OS / Chrome version + run-state
const SHOW_QCOUNT = false;            // queue header "N ext · selected · SIZE"
const SHOW_NAME_LOCALE_TEST = false;  // TEST: extension-name locale switcher
const SHOW_ANCHOR_BAR = false;        // multi-profile jump anchor strip
// Pick the display name in the current UI language. The backend returns a
// `names` map covering every supported UI locale, so switching language
// repaints the list without a rescan and search matches any spelling. Falls
// back to a bare `name` (OS-locale resolved) for any snapshot served by an
// older backend.
const dispName = x => {
  if (!x) return "";
  // Extension names are resolved for every supported UI locale at scan time
  // (info.names), so the display language follows the app UI language (LANG)
  // and switches live with it — no re-scan needed. `name` is the OS-locale
  // fallback for any locale the extension itself doesn't ship.
  const names = x.names || {};
  return names[LANG] || x.name || "";
};
/* Version directory of the icon to show: the active version, else the first. */
const activeDir = e => {
  const a = e.versions.find(v => v.is_active);
  return (a && a.dir) || (e.versions[0] && e.versions[0].dir) || "";
};
const $ = id => document.getElementById(id);
/* Format a raw byte count into a human string with the smallest sensible
   unit. The backend only ever sends bytes; unit selection lives here so a
   tiny extension (e.g. 40 KB) shows "40 KB" instead of rounding to "0 MB".
   Returns e.g. "512 B", "40.0 KB", "1.5 MB", "2.31 GB". */
const fmtSize = b => {
  if (!b || b < 0) return "0 B";
  const KB = 1024, MB = 1048576, GB = 1073741824;
  if (b < KB) return `${b} B`;
  if (b < MB) return `${(b / KB).toFixed(1)} KB`;
  if (b < GB) return `${(b / MB).toFixed(1)} MB`;
  return `${(b / GB).toFixed(2)} GB`;
};
/* Split form for the summary cards, which keep the unit in a separate span. */
const fmtSizeParts = b => {
  const s = fmtSize(b);
  const i = s.lastIndexOf(" ");
  return { num: s.slice(0, i), unit: s.slice(i + 1) };
};

function esc(s){return String(s ?? "").replace(/[&<>"]/g,
  c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c]))}
/* 2026-09-08: the footer status line is GONE (#log removed). Every message it
   used to show was already on screen somewhere else — scanning→scanBar,
   scanned→summary cards, rvTitle→#reviewBar, deletedSub→done page,
   logNoChrome→the .tech banner — and the error paths below all reach
   showFatal() anyway. So there is no log()/logMsg()/renderLog() any more:
   transient feedback is the toast, hard failures are the fatal overlay. */
let toastTimer = null;
let toastVisible = false;     // a temporary toast currently owns the slot
let warnDismissed = false;    // user closed the persistent Chrome-running warning
let chromeStatus = "unknown"; // running | stopped | unknown, last set by renderEnv/probe
function hideToast(){
  const slot = $("notifSlot");
  slot.classList.remove("state-toast");
  toastVisible = false;
  $("toast").classList.remove("err");
  clearTimeout(toastTimer);
  syncWarn();              // flip the warn back, unless dismissed
}
/* The notification slot (a centred overlay above the actionbar since 2026-09-08)
   holds exactly one chip at a time: the persistent
   Chrome-running warning OR a temporary toast. While a toast is up the warn is
   flipped away (CSS `.state-toast`); once it clears the warn flips back unless
   the user dismissed it. `no-warn` hides the warn for stopped/undetected/dismissed. */
function syncWarn(){
  const slot = $("notifSlot");
  slot.classList.toggle("no-warn", warnDismissed || chromeStatus === "stopped");
}
function toast(html, isErr){
  const slot = $("notifSlot");
  $("toastBody").innerHTML = html;
  $("toast").classList.toggle("err", !!isErr);
  slot.classList.add("state-toast");   // flip the warn away, show the toast
  toastVisible = true;
  syncWarn();
  clearTimeout(toastTimer);
  toastTimer = setTimeout(hideToast, 3000);
}

/* Fatal error surface. Hard-coded English on purpose: it must render even when
   i18n failed to load. Any uncaught error or unhandled promise rejection is
   routed here by the window-level listeners registered at boot. */
const GITHUB_REPO = "https://github.com/yingw/chrome-ext-cleaner";
const GITHUB_ISSUES = GITHUB_REPO + "/issues/new";
function showFatal(detail){
  if (!$("fatal")) return;
  const msg = String(detail ?? "Unknown error");
  $("fatalMsg").textContent = msg;
  $("fatal").classList.add("show");
  // #fatal carries an inline `display:none` in the markup (so the overlay can
  // never flash before CSS loads). An inline style outranks any class rule,
  // including `#fatal.show{display:flex}`, so the class alone is not enough —
  // set display explicitly or the overlay stays invisible and a crash looks
  // like a dead, silent UI.
  $("fatal").style.display = "flex";
  // Move focus into the dialog so keyboard / screen-reader users land on it.
  $("btnFatalClose").focus();
}
/* Open an external URL via Tauri's opener plugin. A plain <a href> does not
   work in Tauri — the webview blocks external navigation. Route through the
   plugin and open it directly. On failure we let the error surface: the
   window-level error handlers turn it into the fatal overlay. We do NOT fall
   back to copying the link to the clipboard — a button whose whole job is to
   report an error (e.g. "Report on GitHub") must surface its own failure,
   not pretend success by stashing a URL in the clipboard. */
async function openExternal(url){
  await invoke("plugin:opener|open_url", { url, with: null });
}
async function openGitHub(){
  await openExternal(GITHUB_ISSUES);
}
$("btnGitHub").onclick = async (e) => {
  e.preventDefault();
  try {
    await openExternal(GITHUB_REPO);
  } catch (err) {
    showFatal(err);
  }
};

function applyTheme(){
  const th = localStorage.getItem("cec-theme") ||
    (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
  document.documentElement.dataset.theme = th;
  $("btnTheme").innerHTML = th === "dark" ? SVG.sun : SVG.moon;
}
function applyLang(){
  document.documentElement.lang = LANG;
  const sh = $("langShort"); if (sh) sh.textContent = langShort(LANG);
  // mark the active entry in the language dropdown
  document.querySelectorAll(".lang-opt").forEach(b => b.classList.toggle("on", b.dataset.code === LANG));
  document.querySelectorAll("[data-i18n]").forEach(el => {
    const v = T(el.dataset.i18n); if (typeof v === "string") el.textContent = v;});
  document.querySelectorAll("[data-i18n-ph]").forEach(el => {
    const v = T(el.dataset.i18nPh); if (typeof v === "string") el.placeholder = v;});
  $("toTop").title = T("backTop");
  /* These two titles were hard-coded English in index.html and never followed
     the language switch (unlike #toTop above). #btnSelAll's title is handled
     in updateSelAllLabel() because it also flips with select/deselect state. */
  $("btnTheme").title = T("tipTheme");
  $("warnX").title = T("tipDismiss");
  updateCycleLabels();
  updateSortLabel();
  render();
  // Everything below builds strings at runtime and carries no data-i18n, so
  // it is repainted here instead. Each is a pure function of cached state
  // (last scan payload / plan / result) and a no-op before that state exists.
  // Fold label goes after render(): it reads the freshly rendered rows.
  updateFoldLabel();
  renderCards();
  renderEnv();
  renderReview();
  loadExtIcons();
  renderDone();
  updateSelAllLabel();
  // Slogan is a gate (SHOW_SLOGAN), not an HTML comment — one flip re-shows it.
  $("slogan").hidden = !SHOW_SLOGAN;
  syncCaptionLabels();
  syncConsentTexts();
}
const profLabel = p => {
  const l = LABELS[p]; return l ? `${l} · ${p}` : p;};

/* ---------- scan & list ---------- */
/* Reset every transient view when a scan starts. The summary cards are NOT
   reset: they are a session dashboard (total size, reclaimable, freed-this-
   session) that must persist across rescans, so the user sees the running
   totals update rather than flicker back to a blank slate. While the scan
   runs the cards are dimmed via the .refreshing class (a "stale, updating"
   cue, not a "new value is here" cue), then repopulated by finishScan().
   The list, filters, sort, scope, selection and the Clean button are reset
   and disabled in one synchronous pass, so nothing stale stays on screen
   while the backend works. finishScan() is the exact mirror image — it
   repopulates all of it in one tick when the scan resolves.
   Deliberately NOT reset: the card values, #tSession (cumulative counter, not
   a scan result), and the scroll position. */
function setScanning(on){
  const cards = document.querySelector(".cards");
  if (cards) cards.classList.toggle("refreshing", on);
  $("btnScan").disabled = on;
  const rm = $("btnRecheck"); if (rm) rm.disabled = on;
  $("scanBar").classList.toggle("show", on);
  if (!on) return;
  $("queue").innerHTML = "";
  // Use the class, not an inline style: refreshEmpty() shows this box again
  // with .empty.show{display:block}, and an inline display:none would outrank
  // that rule and leave the empty state permanently invisible after the
  // first scan.
  $("empty").classList.remove("show");
  // $("qCount").textContent = "";   // hidden 2026-09-06, see updCounts()
  updateSelAllLabel();
  updateFoldLabel();
  const cleanBtn = $("btnGoClean");
  cleanBtn.disabled = true;
  $("cleanLabel").textContent = T("cleanBtn");
  $("fProfile").value = "";
  $("fKw").value = "";
  STATUS_IDX = 0; SORT_IDX = 0; SCOPE_FULL = false;
  // qCount is now gated in updCounts(); no need to clear it here.
  updateCycleLabels();
  updateSortLabel();
  // Hide the warn for the duration of the scan. Visibility is owned by syncWarn()
  // via the .no-warn CLASS now — NOT an inline style. The old inline display:none
  // here outranked the `.notif-slot > .warn` rule and pinned the warn hidden forever,
  // because renderEnv() only toggles the class and never touches the inline style.
  // Clear any stale inline override too, in case one lingered.
  $("chromeWarn").style.display = "";
  $("notifSlot").classList.add("no-warn");
}

async function scan(){
  const t0 = performance.now();
  setScanning(true);
  // j must be declared OUTSIDE the try: the finally block reads it, and a
  // `let` inside the try block is not in scope there (that was a hard
  // ReferenceError on every scan, leaving the UI stuck mid-scan).
  let j = null;
  try {
    j = await invoke("scan", {locale: NAME_LOCALE});
  } catch(e){
    showFatal(e);
    return;
  } finally {
    await resolveScan(t0, j);
  }
}

/* Single exit point for every scan. Two rules:
   1. A successful scan repopulates ALL data views in one tick, after the
      minimum visible duration — the user never sees results trickle in.
   2. A failure (or "Chrome not detected") returns control to the user
      immediately rather than sitting out the delay. */
async function resolveScan(t0, j){
  if (j && j.chrome_detected){
    const wait = SCAN_MIN_MS - (performance.now() - t0);
    if (wait > 0) await sleep(wait);
    try { await finishScan(j); }
    catch(e){ showFatal(e); }
    finally { setScanning(false); }
  } else {
    if (j) showNoPrereq(j);
    setScanning(false);
  }
}

// Chrome user-data directory missing: the scan never ran, so show the
// standalone prerequisite page INSTEAD of the list. No cards, no filters, no
// queue, no Clean button, no green hero — nothing here may imply a completed
// scan. The technical path stays collapsed; the page itself carries the
// explanation, so no separate status line is needed (the footer one is gone).
function showNoPrereq(j){
  SELECTED.clear(); CIDINFO.clear();
  window._scan = j;
  $("nopreqPath").textContent = j.expected_dir || "";
  renderEnv();
  showView("nopreq");
}

// Populate every data view from a completed scan. Called only after the
// minimum visible scan duration has elapsed, so the result appears as one
// atomic update rather than trickling in as the backend answers.
async function finishScan(j){
  showView("list");
  // Snapshot first: render() reads DATA/LABELS, and everything below must
  // land in the same tick so the list and the cards never disagree.
  DATA = j.extensions; LABELS = j.profile_labels || {};
  SELECTED.clear(); CIDINFO.clear(); OPEN_STATE.clear();
  j.extensions.forEach(e => e.versions.forEach(v => {
    if (v.deletable){ SELECTED.add(v.cid); CIDINFO.set(v.cid, v.size); }
  }));
  // Total stale versions found, independent of selection. Both the status line
  // and the toast derive from this one number so they can never disagree.
  const oldCount = j.extensions.reduce((a, e) => a + e.versions.filter(v => v.deletable).length, 0);
  window._scan = j;
  const tt = fmtSizeParts(j.total_bytes);
  $("tTotal").textContent = tt.num; $("uTotal").textContent = tt.unit;
  const tw = fmtSizeParts(j.waste_bytes);
  $("tWaste").textContent = tw.num; $("uWaste").textContent = tw.unit;
  // Card 3: session freed total + folder count, backfilled from the scan so it
  // has one source of truth (no reliance on DOM-leftover values).
  const ts = fmtSizeParts(j.session_freed_bytes);
  $("tSession").textContent = ts.num; $("uSession").textContent = ts.unit;
  $("tSessionCount").textContent = j.session_cleaned_count;
  // Card frames are conditional state indicators, not decoration: Card 2 turns
  // amber while reclaimable space exists (to-do), Card 3 turns green once the
  // session has actually freed something (achievement). With no data they drop
  // to the neutral .card frame — an amber "all clear" or green "0 achieved"
  // would mislead. Toggled here (and after each clean in startClean) so the
  // frame always tracks the numbers; left untouched at scan start so the stale
  // frame stays visible, dimmed, until finishScan repaints it.
  $("cardWaste").classList.toggle("warn", j.waste_bytes > 0);
  $("cardSession").classList.toggle("good", j.session_freed_bytes > 0);
  // Status comes from the scan response itself (the backend already probed
  // once inside scan). No second live probe: it only spawned a duplicate
  // tasklist on Windows for a seconds-fresh value.
  window._chromeSt = j.chrome_status || "unknown";
  renderCards();
  renderEnv();
  render();
  toast(fmt(T("toastScanned"), {old: oldCount, size: fmtSize(j.waste_bytes)}));
}

/* Filter + frontend sort. The backend only guarantees the default (waste desc)
   ordering; everything else is a pure view concern. Ties on numeric keys fall
   back to name asc so the queue is deterministic across renders. */
function applyFiltersAndSort(list){
  const pf = $("fProfile").value, st = STATUS_VALS[STATUS_IDX];
  const kw = $("fKw").value.toLowerCase();
  const r = list.filter(e =>
    (!pf || e.profile === pf) &&
    (!st || (st === "on" ? e.enabled : !e.enabled)) &&
    (!SCOPE_FULL && e.waste_bytes > 0 || SCOPE_FULL) &&
    (!kw || (e.name || "").toLowerCase().includes(kw) || Object.values(e.names || {}).some(n => (n || "").toLowerCase().includes(kw)) || e.id.includes(kw)));
  const s = SORTS[SORT_IDX];
  r.sort((a, b) => {
    if (s.key === "name") return dispName(a).localeCompare(dispName(b), LANG, {sensitivity:"base"});
    const av = s.key === "waste" ? a.waste_bytes : a.total_bytes;
    const bv = s.key === "waste" ? b.waste_bytes : b.total_bytes;
    if (av !== bv) return s.dir * (av - bv);
    return dispName(a).localeCompare(dispName(b), LANG, {sensitivity:"base"});
  });
  return r;
}

/* Profile grouping (multi-profile, unfiltered view). Group ORDER is fixed by
   profile name — "Default" always first, then "Profile N" numerically — so the
   layout is predictable and matches how Chrome order profiles on disk (the
   number is creation order). This is independent of the sort dropdown, which
   still governs the order of extensions WITHIN each group. */
function profileSortKey(p){
  const l = (p || "").toLowerCase();
  if (l === "default") return [0, 0];
  const m = l.match(/(\d+)/);
  const n = m ? parseInt(m[1], 10) : NaN;
  return [1, isNaN(n) ? l : n];
}
function orderedProfiles(){
  return ((window._scan && window._scan.profiles) || []).slice()
    .sort((a, b) => {
      const ka = profileSortKey(a), kb = profileSortKey(b);
      if (ka[0] !== kb[0]) return ka[0] - kb[0];
      if (typeof ka[1] === "number" && typeof kb[1] === "number") return ka[1] - kb[1];
      return String(ka[1]).localeCompare(String(kb[1]));
    });
}
const slug = s => (s || "").replace(/[^a-zA-Z0-9_-]/g, "_");

function profileAggInfo(list){
  let ext = list.length, old = 0, waste = 0;
  for (const e of list){
    const olds = e.versions.filter(v => v.deletable);
    old += olds.length;
    waste += olds.reduce((a, v) => a + v.size, 0);
  }
  return {ext, old, waste};
}
/* Top-of-list profile jump bar (multi-profile view): a "N profiles" count
   label on the left, plus a row of clickable profile-name chips on the right
   that scroll to each profile's section (id="prof-<slug>"). Links are built
   only from profiles that actually render, so they never point at a missing
   section. */
function anchorStrip(profiles){
  const d = document.createElement("div");
  d.id = "profiles-anchor";
  d.className = "profiles-anchor";
  const label = document.createElement("span");
  label.className = "profiles-anchor-label";
  label.textContent = fmt(T("profilesAnchor"), {n: profiles.length});
  d.appendChild(label);
  const nav = document.createElement("span");
  nav.className = "profile-jump";
  for (const p of profiles){
    const a = document.createElement("a");
    a.className = "pjump";
    a.href = "#prof-" + slug(p);
    a.dataset.target = "prof-" + slug(p);
    a.textContent = profLabel(p);
    nav.appendChild(a);
  }
  d.appendChild(nav);
  return d;
}
function profileGroup(p, list, showOcc){
  const sec = document.createElement("section");
  sec.className = "pgroup";
  sec.id = "prof-" + slug(p);
  const {ext, old, waste} = profileAggInfo(list);
  const head = document.createElement("div");
  head.className = "phead";
  head.innerHTML = `<span class="pname">${esc(profLabel(p))}</span>` +
    /* <span class="pmeta">${fmt(T("groupHeader"), {ext, old, waste: fmtSize(waste)})}</span> */ "";
  /* Per-profile bulk selection. The global "select all" spans every profile,
     so cleaning exactly one profile used to mean ticking row by row or
     switching to the profile filter first. Only added when the group actually
     has deletable versions. */
  if (old > 0){
    const b = document.createElement("button");
    b.type = "button";
    b.className = "psel";
    b.dataset.groupSel = "1";
    b.onclick = () => {
      const cbs = [...sec.querySelectorAll("details.qrow .one")];
      if (!cbs.length) return;
      const on = !cbs.every(c => c.checked);   // fill, or clear if already full
      for (const c of cbs){
        if (c.checked === on) continue;
        c.checked = on;
        flip(c.dataset.cid, on);   // same path as a manual tick
        paintChip(c);
      }
      // syncRow() re-derives each row's own checkbox from its versions.
      sec.querySelectorAll("details.qrow").forEach(r => syncRow(r));
      updBar(); updCounts();
    };
    head.appendChild(b);
  }
  sec.appendChild(head);
  for (const e of list) sec.appendChild(buildRow(e, showOcc));
  return sec;
}
/* Placeholder for a profile with nothing to clean. Without it such profiles
   vanished from the grouped view entirely, so the user could not confirm the
   scan really covered them. Only rendered when no filter is active — see the
   `unfiltered` guard in render(). */
/* Group select buttons flip between "select this profile" and "clear this
   profile". They are plain runtime text (no data-i18n) and depend on live
   checkbox state, so they are repainted from updCounts() — which already runs
   on every selection change — rather than only on render(). */
function updateGroupSelLabels(){
  document.querySelectorAll("#queue .pgroup").forEach(g => {
    const b = g.querySelector("[data-group-sel]");
    if (!b) return;
    const cbs = [...g.querySelectorAll("details.qrow .one")];
    b.textContent = (cbs.length > 0 && cbs.every(c => c.checked))
      ? T("deselGroup") : T("selGroup");
  });
}

/* One extension row (<details>). `.tag` (per-row profile name) is intentionally
   omitted: redundant in single/filtered views, replaced by the group header in
   grouped views. `showOcc` is true only when the user sorts by size: the
   total-size ("X MB kept") field is then rendered so the sort column is
   meaningful; otherwise it is omitted entirely to keep rows uncluttered. */
function buildRow(e, showOcc){
  const olds = e.versions.filter(v => v.deletable);
  const canDel = olds.length > 0;
  const waste = olds.reduce((a, v) => a + v.size, 0);
  const det = document.createElement("details");
  det.className = "qrow";
  const rkey = e.profile + "|" + e.id;
  const wasOpen = OPEN_STATE.has(rkey) ? OPEN_STATE.get(rkey) : false;
  det.open = wasOpen;
  OPEN_STATE.set(rkey, wasOpen);
  det.dataset.profile = e.profile; det.dataset.eid = e.id;
  det.innerHTML =
    `<summary>${canDel
        ? `<span class="ck-slot"><input type="checkbox" class="rowck" ${olds.every(v => SELECTED.has(v.cid)) ? "checked" : ""}></span>`
        : `<span class="ck-slot"></span>`}
      <img class="ext-ico" alt="" style="display:none" data-profile="${esc(e.profile)}" data-eid="${esc(e.id)}" data-dir="${esc(activeDir(e))}"><span><span class="nm" title="${esc(dispName(e))}">${esc(dispName(e))}</span>${iconButtons(e)}</span>
      <span class="rmeta">
        <span class="rline">
          ${canDel ? `<span class="badge b-old">${olds.length} · ${T("oldWord")}</span>` : ""}
          ${waste > 0 ? `<span class="rwaste nonzero">${fmtSize(waste)}</span>` : `<span class="badge b-pure">${T("pureWord")}</span>`}
          ${showOcc ? `<span class="rkeep">${fmtSize(e.total_bytes)}</span>` : ""}
        </span>
        ${"" /* severity bar disabled 2026-09-05 (pending, restorable) */}
      </span>
      <span class="chev">›</span></summary>
    <div class="vlist">
      ${e.versions.map(v => {
        if (v.unknown)
          return `<div class="vline"><span class="ck-slot"></span><span class="ver">${esc(v.dir)}</span><span class="badge b-bad">?</span></div>`;
        if (v.is_active)
          return `<div class="vline"><span class="ck-slot"></span><span class="ver">${esc(v.version)}</span>${statusBadge(e)}<span class="sz">${fmtSize(v.size)}</span></div>`;
        if (v.deletable)
          return `<div class="vline"><span class="ck-slot"><input type="checkbox" class="one" data-cid="${esc(v.cid)}" ${SELECTED.has(v.cid) ? "checked" : ""}></span><span class="ver" title="${esc(v.reason)}">${esc(v.version)}</span><span class="badge b-old" data-role="stale">${SELECTED.has(v.cid) ? T("staleWord") : T("oldWord")}</span><span class="sz can-clear">${fmtSize(v.size)}</span></div>`;
        return `<div class="vline"><span class="ck-slot"></span><span class="ver" title="${esc(v.reason)}">${esc(v.version)}</span><span class="badge b-protected">${T("protectedWord")}</span></div>`;
      }).join("")}
      ${!e.verified ? `<div class="vline"><span class="ck-slot"></span><span class="badge b-bad">${SVG.warn} ${T("unverified")}</span></div>` : ""}
    </div>`;
  return det;
}

function statusBadge(e){
  return e.enabled ? `<span class="badge b-on">${T("on")}</span>`
    : `<span class="badge b-off">${T("off")}</span>`;
}

function iconButtons(e){
  const url = `chrome://extensions/?id=${e.id}`;
  /* Icon-only buttons need an aria-label, not just a title, so screen readers
     announce their action (title is not reliably exposed as the accessible name). */
  return `<button class="icon-btn" title="${T("gotoUninstall")}" aria-label="${T("gotoUninstall")}" data-copy="${esc(url)}">${SVG.copy}</button>` +
    `<button class="icon-btn" title="${T("openDir")}" aria-label="${T("openDir")}" data-reveal="${esc(e.id)}:${esc(e.profile)}">${SVG.folder}</button>`;
}

/* Per-row expand/collapse state, keyed by "profile|eid". render() rebuilds the
   whole #queue (filter/sort/select-all/scopes), which would otherwise reset
   every <details> and wipe the user's manual fold state.
   Rows never seen before default to COLLAPSED, so both a fresh app start and
   the end of a scan present a compact list (OPEN_STATE.clear() in loadList
   restores that same "all collapsed" default on every rescan). */
const OPEN_STATE = new Map();

/* Visible-first selection (soft clip): SELECTED keeps the user's intent across
   filter changes and is never pruned. Each render records the visible deletable
   cids in VISIBLE; the red Clean button and openClean() intersect the two, so
   "what you see is what gets deleted" while hidden ticks survive. */
const VISIBLE = new Set();
function noteVisible(list){
  list.forEach(e => e.versions.forEach(v => {
    if (v.deletable) VISIBLE.add(v.cid);
  }));
}

function render(){
  VISIBLE.clear();
  const box = $("queue"); box.innerHTML = "";
  // Anchor strip lives in its own bar above the qhead (swapped rows 2/3 per
  // user request), so clear it here on every re-render to avoid stale chips.
  const anchorBar = document.getElementById("profilesAnchorBar");
  anchorBar.innerHTML = "";
  const profiles = (window._scan && window._scan.profiles) || [];
  const pf = $("fProfile").value;
  const single = profiles.length <= 1 || pf !== "";
  const showOcc = SORTS[SORT_IDX].key === "total";
  // Single-profile environments have no meaningful profile selector.
  $("fProfile").style.display = profiles.length <= 1 ? "none" : "";
  if (single){
    const list = applyFiltersAndSort(DATA);
    noteVisible(list);
    /* filtered to one profile: omit the group header — the profile name is
       already shown in the #fProfile dropdown, so a header row would only
       repeat it (same as the single-profile view, which also has no header). */
    for (const e of list) box.appendChild(buildRow(e, showOcc));
  } else {
    /* Multi-profile view drops any profile that has nothing cleanable — no
       empty group, no "nothing to clean" row. A profile emptied by a search/
       status filter was always noise; and when unfiltered we also skip empty
       profiles so the list shows only sections that actually hold deletable
       extensions (the profile dropdown already confirms every profile scanned). */
    const shown = orderedProfiles()
      .map(p => ({p, plist: applyFiltersAndSort(DATA.filter(e => e.profile === p))}))
      .filter(x => x.plist.length > 0);
    shown.forEach(x => noteVisible(x.plist));
    // A jump bar only makes sense with 2+ sections. With one it rendered the
    // self-contradictory "N profiles (1 total)" (e.g. a search that matches
    // inside a single profile while several profiles exist).
    // Kept behind SHOW_ANCHOR_BAR (default off) instead of deleted, so the
    // multi-profile jump strip can be re-enabled without digging through git.
    if (SHOW_ANCHOR_BAR && shown.length > 1) anchorBar.appendChild(anchorStrip(shown.map(x => x.p)));
    for (const {p, plist} of shown){
      box.appendChild(profileGroup(p, plist, showOcc));
    }
  }
  // Copy-link / open-folder icon buttons share one wiring helper with the
  // review and cleaner views, so this list never drifts from them.
  wireIconButtons(box);
  document.querySelectorAll(".pjump").forEach(a => a.onclick = ev => {
    ev.preventDefault();
    const t = document.getElementById(a.dataset.target);
    if (t) t.scrollIntoView({behavior: "smooth", block: "start"});
  });
  box.querySelectorAll(".rowck").forEach(c => {
    // The checkbox sits inside <summary>; stop a click from also toggling the
    // row open/closed (defensive — some WebViews still bubble it).
    c.onclick = ev => ev.stopPropagation();
    c.onchange = () => {
      const det = c.closest("details");
      det.querySelectorAll(".one").forEach(x => {
        if (x.checked !== c.checked){ x.checked = c.checked; flip(x.dataset.cid, c.checked); paintChip(x); }
      });
      syncRow(det); updBar(); updCounts();
    };
  });
  box.querySelectorAll(".one").forEach(c => c.onchange = () => {
    flip(c.dataset.cid, c.checked);
    paintChip(c);
    syncRow(c.closest("details")); updBar(); updCounts();
  });
  // A manual expand/collapse (clicking <summary>) only flips d.open — it never
  // touches OPEN_STATE by itself. Record it here, or any re-render (filter,
  // sort, scope) would snap the row back and undo what the user just did.
  box.querySelectorAll("details.qrow").forEach(d => {
    syncRow(d);
    d.ontoggle = () => {
      OPEN_STATE.set(d.dataset.profile + "|" + d.dataset.eid, d.open);
      updateFoldLabel();
    };
  });
  updBar(); updCounts(); refreshEmpty();
  loadExtIcons();
  // Recompute the fold button here: a freshly created <details> is already
  // closed, so rendering rows in the default collapsed state produces NO
  // toggle event and the label would keep whatever the previous (possibly
  // empty) list implied.
  updateFoldLabel();
}

/* Lazily fetch one icon per extension from the backend and drop it into the
   matching <img>. Results are cached by profile+id+version so re-renders
   (filter/sort/language switch) never re-fetch. Empty result hides the img. */
const ICON_CACHE = new Map();
const INFLIGHT = new Set(); // icon keys with a backend request already in flight
function loadExtIcons(){
  for (const img of document.querySelectorAll("#queue img.ext-ico, #rvList img.ext-ico")){
    const key = `${img.dataset.profile}|${img.dataset.eid}|${img.dataset.dir}`;
    if (ICON_CACHE.has(key)){
      if (ICON_CACHE.get(key)){ img.src = ICON_CACHE.get(key); img.style.display = ""; }
      continue;
    }
    if (INFLIGHT.has(key)) continue;
    INFLIGHT.add(key);
    invoke("extension_icon", {
      profile: img.dataset.profile, extId: img.dataset.eid, versionDir: img.dataset.dir
    }).then(uri => {
      ICON_CACHE.set(key, uri);
      if (uri){ img.src = uri; img.style.display = ""; }
    }).catch(() => ICON_CACHE.set(key, ""))
    .finally(() => INFLIGHT.delete(key));
  }
}

function flip(cid, on){
  if (on){ SELECTED.add(cid); } else { SELECTED.delete(cid); }
}

function syncRow(det){
  const ones = [...det.querySelectorAll(".one")];
  const box = det.querySelector(".rowck");
  if (!box || !ones.length) return;
  const all = ones.every(c => c.checked);
  const some = ones.some(c => c.checked);
  box.checked = all;
  box.indeterminate = some && !all;
}

function paintChip(one){
  const chip = one.closest(".vline").querySelector("[data-role]");
  if (!chip) return;
  chip.textContent = one.checked ? T("staleWord") : T("oldWord");
}

/* Visible-selected cids: user intent (SELECTED) intersected with the current
   render (VISIBLE). The red button and openClean() both use this, so hidden
   ticks are kept but never silently deleted. */
function visSelected(){
  return [...SELECTED].filter(cid => VISIBLE.has(cid));
}

/* Bottom red button text; disabled with nothing visible-selected. */
function updBar(){
  const vis = visSelected();
  let size = 0;
  vis.forEach(cid => size += CIDINFO.get(cid) || 0);
  const b = $("btnGoClean");
  $("cleanLabel").textContent = fmt(T("cleanBtnCount"), {n: vis.length, size: fmtSize(size)});
  b.disabled = !vis.length;
}

/* Select-all button: label + icon flip with the collective checked state.
   Pure function of the rendered queue, so it is safe to call from render
   (via updCounts), reset, and the end of applyLang on a language switch.
   No data-i18n on #selAllLabel — the text is computed at runtime. */
function updateSelAllLabel(){
  const btn = $("btnSelAll"); if (!btn) return;
  const all = [...document.querySelectorAll("#queue details.qrow .one")];
  const allSel = all.length > 0 && all.every(c => c.checked);
  $("selAllLabel").textContent = allSel ? T("selNone") : T("selAll");
  // The title was a fixed English "select all / deselect all" that neither
  // localized nor flipped with the button's state.
  btn.title = allSel ? T("selNone") : T("selAll");
  const box = btn.querySelector(".icbox");
  if (box) box.innerHTML = SVG.sel;
}
/* Queue header counts. `old` is the TOTAL number of deletable versions on
   screen, `sel` how many of them are ticked. The previous copy fed the
   SELECTED count into the "N old versions" slot, so with 10 candidates and
   3 ticked the header claimed "3 old versions" and jumped as boxes were
   ticked — it read as a total but tracked the selection. */
function updCounts(){
  const dets = [...document.querySelectorAll("#queue details.qrow")];
  const all = [...document.querySelectorAll("#queue details.qrow .one")];
  let sel = 0, waste = 0;
  for (const c of all){
    if (!c.checked) continue;
    sel++; waste += CIDINFO.get(c.dataset.cid) || 0;
  }
  /* qCount is kept behind SHOW_QCOUNT (see #qCount in index.html), not commented
     out — re-enabling is a one-line const flip, no git dig. `waste` is still
     accumulated so the line repaints correctly when shown. */
  const qc = $("qCount");
  if (SHOW_QCOUNT){
    qc.textContent = fmt(T("qCount"), {ext: dets.length, sel, old: all.length, size: fmtSize(waste)});
    qc.hidden = false;
  } else {
    qc.hidden = true;
  }
  updateSelAllLabel();
  updateGroupSelLabels();
}

/* Conditional qhead right group (select-all/sort/fold): shown iff rows are
   visible. The left filter group stays always — with nothing cleanable the
   filters still drive the full-list view (size sort etc.). `hidden` only
   (never DOM removal), so wiring survives. */
function updateQheadGroups(visRows){
  const show = visRows > 0;
  $("btnSelAll").hidden = !show;
  const sortWrap = $("fSortBtn").closest(".sel-wrap");
  if (sortWrap) sortWrap.hidden = !show;
  $("btnFold").hidden = !show;
}
/* Empty states: genuinely clean vs filter-no-match. */
function refreshEmpty(){
  const anyCandTotal = DATA.some(e => e.versions.some(v => v.deletable));
  const visRows = document.querySelectorAll("#queue details.qrow").length;
  const filtering = $("fKw").value.trim() !== "" || STATUS_IDX !== 0 || $("fProfile").value !== "";
  const box = $("empty");
  const showHero = visRows === 0;
  updateQheadGroups(visRows);
  document.querySelector("#queue").style.display = showHero ? "none" : "";
  // The filter bar (.qhead) stays put in the empty state. Hiding it removed the
  // status/profile/scope/search controls, stranding the user with only the
  // empty-state button — that is what made the way back feel broken.
  if (!showHero){ box.classList.remove("show"); return; }
  if (!filtering && !anyCandTotal){
    setEmptyIcon(true);
    $("emptyTitle").textContent = T("emptyCleanTitle");
    $("emptySub").textContent = fmt(T("emptyCleanSub"), {n: DATA.length});
    $("emptyBtns").innerHTML =
      `<button id="btnGoFull" class="empty-btn">${T("emptyFull")}</button>`;
    $("btnGoFull").onclick = () => { SCOPE_FULL = true; updateScopeLabel(); render(); };
  } else {
    setEmptyIcon(false);
    $("emptyTitle").textContent = T("emptyNoMatchTitle");
    $("emptySub").textContent = T("emptyNoMatchSub");
    $("emptyBtns").innerHTML =
      `<button id="btnClearF" class="empty-btn">${T("emptyClear")}</button>`;
    $("btnClearF").onclick = () => {
      $("fKw").value = ""; $("fProfile").value = "";
      STATUS_IDX = 0; updateCycleLabels();
      // Scope is deliberately NOT reset here. "Clear filters" means clearing the
      // filters — folding scope back to "cleanable only" here made the Full-list
      // user land on a second empty state and cost a second click to get back.
      render();
    };
  }
  box.classList.add("show");
}
function setEmptyIcon(ok){
  const el = $("emptyIcon");
  // Reuse the shared SVGs (identical glyphs) instead of duplicating them here.
  el.innerHTML = ok ? SVG.done : SVG.find;
  el.classList.toggle("ok", ok);
  el.classList.toggle("find", !ok);
}

/* ---------- cleaner-style flow: review → cleaning → done ---------- */
function showView(which){
  $("view-list").hidden = which !== "list";
  $("view-clean").hidden = which !== "clean";
  $("view-nopreq").hidden = which !== "nopreq";
  // One shared bottom action bar spans all three views; its buttons swap by
  // view + clean-flow step (see syncActionbar). It never hides within the app.
  syncActionbar();
}
function setStep(s){
  const order = ["review", "cleaning", "done"];
  document.querySelectorAll("#cleanSteps .step").forEach(el => {
    const own = order.indexOf(el.dataset.s), cur = order.indexOf(s);
    el.classList.toggle("cur", own === cur);
    el.classList.toggle("done", own < cur);
  });
  order.forEach(k => $("stage-" + k).hidden = (k !== s));
  // reviewBar holds the review-step total (info only); the action buttons live
  // in the shared bottom bar. doneBar was removed — its Back button moved there.
  $("reviewBar").style.display = s === "review" ? "" : "none";
  syncActionbar();
}

/* The bottom action bar is one persistent bar shared by all three views. Its
   buttons swap by view + clean-flow step:
     list     → [Rescan] ............ [Clean]
     review   → [Back]   ............ [Start cleaning]
     cleaning → (hidden — the progress ring is the focus)
     done     → ..................... [Back to list]
   It hides only when neither list nor clean is on screen (e.g. nopreq). */
function syncActionbar(){
  const list = !$("view-list").hidden;
  const clean = !$("view-clean").hidden;
  let step = "review";
  if (clean){
    if (!$("stage-review").hidden) step = "review";
    else if (!$("stage-cleaning").hidden) step = "cleaning";
    else if (!$("stage-done").hidden) step = "done";
  }
  const on = {
    scan:  list,
    clean: list,
    back:  clean && step === "review",
    start: clean && step === "review",
    done:  clean && step === "done",
  };
  $("btnScan").hidden       = !on.scan;
  $("btnGoClean").hidden    = !on.clean;
  $("btnBackList").hidden   = !on.back;
  $("btnStartClean").hidden = !on.start;
  $("btnDoneBack").hidden   = !on.done;
  $("actionbar").hidden = !(on.scan || on.clean || on.back || on.start || on.done);
}

/* ---------- views rebuilt on demand ----------
   These build strings at runtime and carry no data-i18n, so applyLang()
   re-runs them all. Each reads cached state only (last scan payload, cleanup
   plan, cleanup result) and is a no-op before that state exists. */

/* Fold button label = the NEXT action: anything collapsed -> "expand all".
   Depends on the rendered rows, so it must run after render(). */
function updateFoldLabel(){
  const rows = [...document.querySelectorAll("#queue details.qrow")];
  const b = $("btnFold"); if (!b) return;
  const box = b.querySelector(".icbox");
  if (box) box.innerHTML = SVG.fold;
  const lbl = b.querySelector("#foldLabel");
  if (lbl) lbl.textContent = rows.some(r => !r.open) ? T("unfoldAll") : T("foldAll");
}

/* Summary card sub-lines and the card-1 hover tip. Card VALUES and the
   conditional card frames are state, not wording: they stay in finishScan()
   and startClean() so a language switch cannot roll them back. */
function renderCards(){
  const j = window._scan;
  if (!j || !j.chrome_detected) return;
  const EPS = 0.05 * 1048576;
  const net = j.baseline_total_bytes - j.total_bytes; // signed bytes
  /* Prefixed with "Since launch" so the arrow is self-explanatory
     without hovering: the delta's meaning used to live only in the title tip
     (tooltips are undiscoverable and absent on touch). cNoChange already
     carries the same wording, so it gets no prefix. */
  $("dTotal").textContent =
    net >= EPS ? `${T("cSinceLaunch")} ↓ ${fmtSize(net)}`
      : net <= -EPS ? `${T("cSinceLaunch")} ↑ ${fmtSize(-net)}`
      : T("cNoChange");
  const candExts = DATA.filter(e => e.versions.some(v => v.deletable)).length;
  const candVers = DATA.reduce((s, e) => s + e.versions.filter(v => v.deletable).length, 0);
  $("dWaste").textContent = `${candExts} ${T("cExtensions")} · ${candVers} ${T("cOldVers")}`;
  const cleaned = fmtSize(j.session_freed_bytes);
  const grew = j.session_freed_bytes - net; // bytes Chrome added since launch
  $("dTotal").title = fmt(T("cTipCleaned"), {size: cleaned, n: j.session_cleaned_count}) +
    (grew > EPS ? fmt(T("cTipGrew"), {size: fmtSize(grew)}) : T("cTipGrewNone"));
}

/* Footer status line, Chrome-running banner and the profile filter. */
function renderEnv(){
  const j = window._scan;
  if (!j) return;
  if (!j.chrome_detected){
    /* 2026-09-06: ftMeta hidden (see index.html + SHOW_FTMETA). The early
       return stays: with no Chrome there is no profile list to build and no
       banner to show. Treat chrome as stopped so the warn is hidden. */
    chromeStatus = "stopped"; syncWarn();
    return;
  }
  const sel = $("fProfile"), cur = sel.value;
  const counts = {};
  j.extensions.forEach(e => counts[e.profile] = (counts[e.profile] || 0) + 1);
  sel.innerHTML = `<option value="">${T("allProfiles")}</option>` +
    j.profiles.map(p => `<option value="${esc(p)}">${esc(profLabel(p))} (${counts[p] || 0})</option>`).join("");
  sel.value = cur;
  /* 2026-09-06: removed — the OS/Chrome version line was diagnostic noise and
     its "Chrome running" dot duplicated #chromeWarn, which is now the centred
     notification slot above the actionbar. */
  // const chromeVer = j.chrome_version ? ` · Chrome ${esc(j.chrome_version)}` : "";
  // Last live probe when we have one, otherwise the value the scan reported.
  const st = window._chromeSt || j.chrome_status || "unknown";
  // ftMeta was HTML-commented out before; now it is a gate (SHOW_FTMETA) so it
  // can be re-shown without digging through git.
  const ft = $("ftMeta");
  if (SHOW_FTMETA){
    const chromeVer = j.chrome_version ? ` · Chrome ${esc(j.chrome_version)}` : "";
    ft.innerHTML =
      `${esc(j.os || j.platform)}${chromeVer} · <span class="dot${st === "running" ? " on" : ""}"></span> ${
        st === "running" ? T("stRunning") : st === "stopped" ? T("stQuit") : T("stUnknown")}`;
    ft.hidden = false;
  } else {
    ft.hidden = true;
  }
  const el = $("chromeWarn");
  chromeStatus = st;
  el.querySelector("span").textContent = st === "running" ? T("warn") : T("unknownWarn");
  syncWarn();
}

/* Done stage of the clean flow: freed total plus the skipped/error list. */
function renderDone(){
  const r = window._result;
  if (!r) return;
  document.querySelector("#stage-done .done-hero").innerHTML = SVG.done;
  const dm = fmtSizeParts(r.freed);
  $("doneMb").textContent = dm.num; $("doneUnit").textContent = dm.unit;
  $("doneSub").textContent = fmt(T("deletedSub"), {n: r.ok});
  $("doneList").innerHTML = r.errors.length
    ? `<div class="cl-item"><span>${SVG.warn}</span><b>${T("skipped")}</b></div>` +
      r.errors.map(x => `<div class="cl-item"><span>·</span><span class="ver">${esc(x)}</span></div>`).join("")
    : "";
}

/* Review list. Its ctx line is built with fmt() and carries no data-i18n, so
   it needs an explicit re-render on language switch (see applyLang). */
function renderReview(){
  const plan = window._plan;
  if (!plan) return;
  $("rvList").innerHTML = plan.items.map(it =>
    `<div class="rv-item"><img class="ext-ico" alt="" style="display:none" data-profile="${esc(it.profile)}" data-eid="${esc(it.ext_id)}" data-dir="${esc(it.dir)}"><span><span class="nm">${esc(dispName(it))}</span><span class="del-ver">${esc(it.version)}</span>${iconButtons({id: it.ext_id, profile: it.profile})}<div class="ctx">${esc(fmt(T("rvCtx"), {profile: profLabel(it.profile), active: it.active_version || "-"}))}</div></span><span class="sz"><b>${fmtSize(it.size)}</b></span></div>`).join("");
  wireIconButtons($("rvList"));
}

async function openClean(){
  const cids = visSelected();
  if (!cids.length){ alert(T("cleanNone")); return; }
  // First-cleanup consent gates prepare_cleanup: refuse → back to list,
  // no backend plan is built.
  if (!(await ensureConsent())) return;
  let plan;
  try {
    plan = await invoke("prepare_cleanup", {candidateIds: cids});
  } catch(e){ showFatal(e); return; }
  window._plan = plan;
  showView("clean"); setStep("review");
  renderReview();
  loadExtIcons();
  const rv = fmtSizeParts(plan.total_bytes);
  $("rvTotal").textContent = rv.num; $("rvUnit").textContent = rv.unit;
  $("rvCount").textContent = plan.items.length;
}

function wireIconButtons(root){
  root.querySelectorAll("[data-copy]").forEach(b => b.onclick = ev => {
    ev.preventDefault(); ev.stopPropagation();
    if (navigator.clipboard) navigator.clipboard.writeText(b.dataset.copy);
    toast(fmt(T("toastCopy"), {url: esc(b.dataset.copy)}));
  });
  root.querySelectorAll("[data-reveal]").forEach(b => b.onclick = async ev => {
    ev.preventDefault(); ev.stopPropagation();
    const [eid, profile] = b.dataset.reveal.split(":");
    try {
      const dir = await invoke("reveal_candidate_dir", {extId: eid, profile});
      toast(fmt(T("toastDir"), {dir: esc(dir)}));
    } catch(err){ toast(String(err), true); }
  });
}

const RING_C = 326.7;
const STEP_DELAY_MS = 650; // Pacing: local deletes finish in ms; without this the progress UI flashes by.
const sleep = ms => new Promise(r => setTimeout(r, ms));
function setRing(pct){
  $("ringFg").style.strokeDashoffset = RING_C * (1 - pct / 100);
  $("ringNum").textContent = Math.round(pct) + "%";
}

async function startClean(){
  const plan = window._plan;
  if (!plan || !plan.items.length){ alert(T("cleanNone")); return; }
  const items = plan.items;
  setStep("cleaning"); setRing(0);
  $("clList").innerHTML = items.map((it, i) =>
    `<div class="cl-item" id="cl-${i}"><span class="spin"></span><span class="ver">${esc(dispName(it))} ${esc(it.version)}</span>
     <span class="st muted">…</span></div>`).join("");
  let freed = 0, ok = 0; const errors = [];
  for (let i = 0; i < items.length; i++){
    const it = items[i];
    $("clCur").textContent = fmt(T("cur"), {i: i + 1, n: items.length, name: dispName(it) + " " + it.version});
    const row = $("cl-" + i);
    const mark = row.querySelector("span");
    try{
      const j = await invoke("delete_candidate", {planId: plan.plan_id, cid: it.cid});
      if (j.ok){
        freed += j.freed_bytes || 0; ok++;
        SELECTED.delete(it.cid); // gone from disk: drop it so a no-rescan return can't reselect it
        if (typeof j.session_freed_bytes === "number"){
          const ts2 = fmtSizeParts(j.session_freed_bytes);
          $("tSession").textContent = ts2.num; $("uSession").textContent = ts2.unit;
          $("cardSession").classList.toggle("good", j.session_freed_bytes > 0);
        }
        if (typeof j.session_cleaned_count === "number")
          $("tSessionCount").textContent = j.session_cleaned_count;
        row.querySelector(".st").textContent = fmtSize(j.freed_bytes || 0);
        row.querySelector(".st").classList.remove("muted");
        mark.className = "cl-ok"; mark.innerHTML = SVG.check;
      } else {
        errors.push(`${dispName(it)} ${it.version}: ${j.error || "unknown"}`);
        row.querySelector(".st").textContent = "!";
        mark.className = "cl-err"; mark.innerHTML = SVG.warn;
      }
    }catch(err){
      errors.push(`${dispName(it)} ${it.version}: ${err}`);
      row.querySelector(".st").textContent = "!";
      mark.className = "cl-err"; mark.innerHTML = SVG.warn;
    }
    setRing((i + 1) / items.length * 100);
    await sleep(STEP_DELAY_MS);
  }
  setStep("done");
  window._result = {freed, ok, errors};
  renderDone();
  let cm = fmt(T("toastCleared"), {n: ok, size: fmtSize(freed)});
  if (errors.length) cm += fmt(T("toastClearedFail"), {err: errors.length});
  toast(cm);
}

/* ---------- wiring ---------- */
$("btnScan").onclick = scan;

/* Filter disclosure: ⚙ toggles .filtering on .qhead (CSS grid 0fr→1fr grow). */
$("btnFilter").onclick = () => {
  const qh = document.querySelector(".qhead");
  const on = qh.classList.toggle("filtering");
  $("btnFilter").setAttribute("aria-expanded", String(on));
};

/* TEST ONLY: extension-name locale switcher, kept behind SHOW_NAME_LOCALE_TEST
   (false) instead of an HTML comment. Flip the flag to surface the dropdown. */
const nameLocaleSel = $("fNameLocale");
if (SHOW_NAME_LOCALE_TEST && nameLocaleSel){
  nameLocaleSel.value = NAME_LOCALE;
  nameLocaleSel.onchange = () => { NAME_LOCALE = nameLocaleSel.value; scan(); };
  $("nameLocaleTest").hidden = false;
}

/* Back-to-top floating button: the scroll container is <main>, not the window,
   so listen there and scroll it back. Hidden until the queue is scrolled past
   a small threshold, then re-evaluated on every scroll. */
const mainEl = document.querySelector("main");
const toTopBtn = $("toTop");
const TOP_THRESHOLD = 200;
/* Scroll-driven chrome (single listener): back-to-top visibility + stuck-state
   dividers. Sticky bars dock at main's top edge below the fixed header, so the
   stuck threshold compares against mainEl's rect top, NOT the viewport top
   (comparing to viewport top is always false here). */
function syncScrollChrome(){
  toTopBtn.classList.toggle("show", mainEl.scrollTop > TOP_THRESHOLD);
  const mainTop = mainEl.getBoundingClientRect().top;
  document.querySelectorAll(".stickybar,.topbar").forEach(b =>
    b.classList.toggle("is-stuck", b.getBoundingClientRect().top <= mainTop + 1));
}
mainEl.addEventListener("scroll", syncScrollChrome, {passive:true});
toTopBtn.onclick = () => {
  mainEl.scrollTo({top:0, behavior:"smooth"});
  toTopBtn.classList.remove("show");
};
function updateCycleLabels(){
  // Status filter (All/Enabled/Disabled) uses a check mark — distinct from the
  // scope filter's funnel, so the two adjacent cycle buttons no longer share one
  // glyph (F11a).
  $("fStatusBtn").innerHTML = SVG.check + `<span>${T("stCycle")[STATUS_IDX]}</span>`;
  updateScopeLabel();
}
function updateScopeLabel(){
  const b = $("btnScope");
  b.innerHTML = SVG.funnel + `<span>${T("scopeCycle")[SCOPE_FULL ? 1 : 0]}</span>`;
  b.classList.toggle("ghost-on", !SCOPE_FULL);
}
function updateSortLabel(){
  const sel = $("fSortBtn");
  const labels = T("sortCycle");   // array: [Reclaim size ↓, Name ↑, Size ↓]
  // Rebuild only when the option count changes (first run / structural change);
  // on later calls (e.g. language switch) just refresh the text so the open
  // <select>'s focus/value is never disturbed.
  if (sel.options.length !== labels.length){
    sel.innerHTML = "";
    labels.forEach((label, i) => {
      const o = document.createElement("option");
      o.value = String(i); o.textContent = label;
      sel.appendChild(o);
    });
  } else {
    labels.forEach((label, i) => { sel.options[i].textContent = label; });
  }
  sel.value = String(SORT_IDX);
}
$("fProfile").onchange = render;
$("fSortBtn").onchange = () => {
  SORT_IDX = Number($("fSortBtn").value);
  render();
};
$("fStatusBtn").onclick = () => {
  STATUS_IDX = (STATUS_IDX + 1) % STATUS_VALS.length;
  updateCycleLabels(); render();
};
$("btnScope").onclick = () => {
  SCOPE_FULL = !SCOPE_FULL;
  updateScopeLabel(); render();
};
$("btnFold").onclick = () => {
  const visible = [...document.querySelectorAll("#queue details.qrow")];
  // Target: if any visible row is collapsed → the action is "expand all";
  // otherwise "collapse all".
  const expand = visible.length ? visible.some(r => !r.open) : true;
  // Apply the SAME fold intent to EVERY extension, including ones hidden by an
  // active filter. Otherwise clearing the filter later would re-show rows in
  // their old (inconsistent) open/closed state. render() restores OPEN_STATE,
  // so the intent survives both re-renders and filter changes.
  for (const e of DATA) OPEN_STATE.set(e.profile + "|" + e.id, expand);
  visible.forEach(r => { r.open = expand; });
  updateFoldLabel();
};
let kwTimer = 0;
$("fKw").oninput = () => {
  clearTimeout(kwTimer);
  kwTimer = setTimeout(render, 150);
};
$("btnSelAll").onclick = () => {
  const all = [...document.querySelectorAll("#queue details.qrow .one")];
  const next = !(all.length > 0 && all.every(c => c.checked));
  all.forEach(x => {
    if (x.checked !== next){ x.checked = next; flip(x.dataset.cid, next); paintChip(x); }
  });
  // Flip in place: selecting/deselecting never changes which rows are visible,
  // so we must NOT call render() (that would wipe manual fold state + reload).
  document.querySelectorAll("#queue details.qrow").forEach(d => syncRow(d));
  updBar(); updCounts();
};
$("btnTheme").onclick = () => {
  const cur = document.documentElement.dataset.theme;
  localStorage.setItem("cec-theme", cur === "dark" ? "light" : "dark");
  applyTheme();
};
$("btnLang").onclick = (e) => {
  e.stopPropagation();
  const pop = $("langPop");
  const open = pop.hidden;
  pop.hidden = !open;
  $("btnLang").setAttribute("aria-expanded", String(open));
};
function buildLangPop(){
  const pop = $("langPop");
  if (!pop) return;
  pop.innerHTML = "";
  for (const l of LANGS){
    const b = document.createElement("button");
    b.type = "button";
    b.className = "lang-opt" + (l.code === LANG ? " on" : "");
    b.dataset.code = l.code;
    b.setAttribute("role", "menuitem");
    b.innerHTML = `<span class="lang-opt-name">${esc(l.label)}</span>`;
    b.onclick = (ev) => {
      ev.stopPropagation();
      setLang(l.code);
      pop.hidden = true;
      $("btnLang").setAttribute("aria-expanded", "false");
    };
    pop.appendChild(b);
  }
}
function setLang(code){
  LANG = code;
  localStorage.setItem("cec-lang", code);
  const sh = $("langShort"); if (sh) sh.textContent = langShort(code);
  buildLangPop();
  applyLang();
  syncNativeMenuLang();
}
/* Native app menu tracks the in-app language (macOS only; elsewhere no-op).
   Mirrors RetroPlay's useMenubarI18n: labels resolved via web i18n and pushed
   to Rust, which rebuilds the whole menu from the received strings.
   Fire-and-forget: menu sync must never break a language switch. */
function syncNativeMenuLang(){
  if (document.documentElement.dataset.os !== "macos") return;
  try {
    const r = invoke("set_native_menu_labels", {
      about: T("menuAbout"), quit: T("menuQuit"), hide: T("menuHide"),
    });
    if (r && r.catch) r.catch(() => {});
  } catch (_) {}
}
/* Close the language popover on any outside click. */
document.addEventListener("click", (e) => {
  const sw = $("langSw");
  if (sw && !sw.contains(e.target)){
    const pop = $("langPop");
    if (pop && !pop.hidden){ pop.hidden = true; $("btnLang").setAttribute("aria-expanded", "false"); }
  }
});
$("btnGoClean").onclick = openClean;
$("btnBackList").onclick = () => { showView("list"); };
$("btnStartClean").onclick = startClean;
$("btnDoneBack").onclick = () => { showView("list"); scan(); };
$("warnX").onclick = () => { warnDismissed = true; syncWarn(); };
$("btnRecheck").onclick = () => { scan(); };

/* Top-level safety net: any uncaught error or rejected promise that escapes
   the local try/catch blocks lands here, as a readable overlay with a
   GitHub link — instead of a blank screen or a silent console entry. */
window.addEventListener("error", ev => {
  const d = ev.error && ev.error.stack ? ev.error.stack : (ev.message || "Unknown error");
  showFatal(d);
});
window.addEventListener("unhandledrejection", ev => {
  const r = ev.reason;
  showFatal(r && r.stack ? r.stack : (r ? String(r) : "Unhandled promise rejection"));
});
$("btnFatalCopy").onclick = async () => {
  const text = $("fatalMsg").textContent;
  try { await navigator.clipboard.writeText(text); toast(T("toastReportCopied")); }
  catch (_) { toast(T("toastClipFail"), true); }
};
$("btnFatalGitHub").onclick = openGitHub;
$("btnFatalClose").onclick = () => {
  // In Tauri, actually close the window. Outside Tauri (e.g. opening
  // index.html directly in a browser) there is no window to close, so just
  // hide the overlay so the page behind it is usable.
  const w = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow();
  if (w) { w.close(); return; }
  $("fatal").style.display = "none";
  $("fatal").classList.remove("show");
};
$("toastX").onclick = hideToast;  // manual dismiss for the auto-hiding toast

/* First-cleanup consent: blocking modal before the first prepare_cleanup.
   Back = refuse (back to list, no plan built); Go = record ack, proceed.
   Text refreshes with language; profile path recomputed on every show. */
const OS_USER_DATA = {
  macos: "~/Library/Application Support/Google/Chrome",
  windows: "%LOCALAPPDATA%\\Google\\Chrome\\User Data",
  linux: "~/.config/google-chrome",
};
function firstConsentProfile(){
  const ck = document.querySelector('#queue details.qrow .one:checked');
  const det = ck && ck.closest("details");
  return (det && det.dataset.profile) || "Default";
}
function consentProfileDir(){
  const base = OS_USER_DATA[document.documentElement.dataset.os] || OS_USER_DATA.macos;
  return base + "/" + firstConsentProfile() + "/Extensions";
}
function syncConsentTexts(){
  const set = (id, k, vars) => { const el = $(id); if (!el) return;
    el.innerHTML = fmt(T(k), vars || {}); };
  set("consentP1", "consentP1");
  set("consentP2", "consentP2");
  set("consentP3", "consentP3");
  set("consentP4", "consentP4");
  const pathEl = $("consentPath");
  if (pathEl) pathEl.textContent = consentProfileDir();
  const openBtn = $("btnConsentOpen");
  if (openBtn) { openBtn.title = T("consentOpen"); openBtn.setAttribute("aria-label", T("consentOpen")); }
}
function showConsent(){
  const c = $("consent"); if (!c) return;
  syncConsentTexts();
  // #consent carries an inline display:none (never flash before CSS loads);
  // inline style outranks the .show class rule, so set display explicitly.
  c.classList.add("show"); c.style.display = "flex";
}
function hideConsent(){
  const c = $("consent"); if (!c) return;
  c.classList.remove("show"); c.style.display = "none";
}
function ensureConsent(){
  if (localStorage.getItem("cec-consent-ack")) return Promise.resolve(true);
  return new Promise((resolve) => {
    showConsent();
    $("btnConsentBack").onclick = () => { hideConsent(); showView("list"); resolve(false); };
    $("btnConsentGo").onclick = () => {
      try { localStorage.setItem("cec-consent-ack", "1"); } catch (_) {}
      hideConsent(); resolve(true);
    };
  });
}
function wireConsentOpen(){
  // TEMP preview key: shown while gated on, never touches ack state.
  const eye = $("btnConsentPrev");
  if (eye) {
    eye.hidden = !SHOW_CONSENT_PREVIEW;
    eye.onclick = () => showConsent();
  }
  // "Open folder" jumps to the Extensions dir of the profile owning the
  // first checked row (reuses reveal_candidate_dir: locate only, no writes).
  const openBtn = $("btnConsentOpen"); if (!openBtn) return;
  // Default behavior (the preview key only calls showConsent, so it uses
  // this set; the real flow in ensureConsent() overrides per invocation):
  // Back closes and returns to list, Go only closes.
  $("btnConsentBack").onclick = () => { hideConsent(); showView("list"); };
  $("btnConsentGo").onclick = hideConsent;
  openBtn.onclick = async () => {
    try {
      const ck = document.querySelector('#queue details.qrow .one:checked');
      if (!ck) { toast(T("cleanNone")); return; }
      const det = ck.closest("details");
      const dir = await invoke("reveal_candidate_dir", {profile: det.dataset.profile, extId: det.dataset.eid});
      toast(fmt(T("toastDir"), {dir: esc(dir)}));
    } catch (e) { toast(String(e), true); }
  };
}
/* TEMP PREVIEW ONLY (remove before Phase W ships): OS switcher that fakes
   `data-os` so Win/Linux caption styles can be previewed on macOS. Visual
   only — caption buttons are wired once at boot from the REAL os, so an
   overridden value can never arm a real close/minimize. */
function wireTempOs(){
  const box = $("osPreview"); if (!box) return;
  if (!SHOW_OS_PREVIEW) { box.hidden = true; return; } // gated off for release
  box.hidden = false;
  const mark = (os) => box.querySelectorAll("button").forEach(
    (b) => b.classList.toggle("on", b.dataset.os === os));
  box.querySelectorAll("button").forEach((b) => {
    b.onclick = () => {
      document.documentElement.dataset.os = b.dataset.os;
      mark(b.dataset.os);
    };
  });
  mark(document.documentElement.dataset.os || "macos");
}

/* Self-drawn caption (Phase W1, Win/Linux only): max and restore share one
   button driven by toggleMaximize; state sync listens to window events
   (covered by core:default, no extra permission). Lucide paths, copied. */
const CAP_SVG_MAX = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/></svg>';
const CAP_SVG_RESTORE = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>';
let capMaximized = false; // mirrors the native maximized state for the toggle
function syncCaptionLabels(){
  const set = (id, k) => { const el = $(id); if (!el) return;
    const v = T(k); el.setAttribute("aria-label", v); el.title = v; };
  set("btnCapMin", "caption.minimize");
  set("btnCapMax", capMaximized ? "caption.restore" : "caption.maximize");
  set("btnCapClose", "caption.close");
}
function syncMaxIcon(){
  const b = $("btnCapMax"); if (!b) return;
  b.innerHTML = capMaximized ? CAP_SVG_RESTORE : CAP_SVG_MAX;
  syncCaptionLabels();
}
function wireCaption(){
  syncCaptionLabels();
  // macOS keeps native lights and plain-browser preview has no window to
  // drive: stay static. (The TEMP os switcher only fakes data-os for CSS;
  // wiring runs once here from the real os, so it can never arm preview keys.)
  const os = OS;
  const TAU = window.__TAURI__;
  const w = TAU && TAU.window && TAU.window.getCurrentWindow
    ? TAU.window.getCurrentWindow() : null;
  if (os === "macos" || !w) return;
  try {
    $("btnCapMin").onclick = () => w.minimize();
    $("btnCapClose").onclick = () => w.close();
    $("btnCapMax").onclick = () => w.toggleMaximize();
    w.listen("tauri://maximize", () => { capMaximized = true; syncMaxIcon(); });
    w.listen("tauri://unmaximize", () => { capMaximized = false; syncMaxIcon(); });
    w.isMaximized().then((m) => { capMaximized = !!m; syncMaxIcon(); }).catch(() => {});
    w.listen("tauri://blur", () => document.body.classList.add("win-blur"));
    w.listen("tauri://focus", () => document.body.classList.remove("win-blur"));
  } catch (_) { /* stay static */ }
}

/* Fill the version chip next to the product name from Tauri's app version
   (single source of truth = tauri.conf.json "version"). Falls back to the
   literal below only when run outside Tauri (e.g. opening index.html in a
   plain browser), so bump it together with the version field.
   2026-09-08: no build number. It only ever appeared in the footer chip that
   was deleted with the footer; the header badge stays a plain "v1.0.0". */
const APP_VERSION_FALLBACK = "1.0.0";
async function applyVersion(){
  const badge = document.getElementById("verBadge");
  let v = APP_VERSION_FALLBACK;
  try {
    const app = window.__TAURI__ && window.__TAURI__.app;
    if (app && app.getVersion) v = (await app.getVersion()) || v;
  } catch (_) { /* keep fallback */ }
  if (badge) badge.textContent = "v" + v;
}

/* Strings must be in place before anything renders. A failure here is fatal
   for the UI, so surface it instead of booting with empty labels. */
(async () => {
  try {
    await loadI18n();
  } catch (e) {
    showFatal("Failed to load UI strings (i18n.json): " + e);
    return;
  }
  applyTheme(); applyLang(); buildLangPop(); applyVersion(); wireCaption(); wireTempOs(); wireConsentOpen(); syncNativeMenuLang(); scan();
})();
