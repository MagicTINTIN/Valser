const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);
const send = (type, args = {}) => invoke("send", { cmd: { type, ...args } });
const mobile = /android/i.test(navigator.userAgent);
document.body.classList.toggle("mobile", mobile);

let status = {};
let seeking = false;
let restored = false;

function el(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function formatTime(secs) {
  const total = Math.floor(secs || 0);
  const pad = (n) => String(n).padStart(2, "0");
  return `${pad(Math.floor(total / 60))}:${pad(total % 60)}`;
}

function sendFilter() {
  send("filter", { text: $("filter").value, scope: $("scope").value });
}

function renderPlaylist(view) {
  $("tracks").replaceChildren(
    ...view.rows.map((row) => {
      const li = el("li");
      li.dataset.index = row.index;
      const name_span = el("span", undefined, "name")
      name_span.append(el("span", `${row.index + 1}.`, "track_num"))
      name_span.append(el("span", `${row.name}`, "track_name"))
      li.append(name_span);
      if (row.duration) li.append(el("span", formatTime(row.duration)));
      li.append(el("button", "x", "remove"));
      return li;
    }),
  );
  $("empty").hidden = view.total > 0;

  $("genre-list").replaceChildren(
    ...view.genres.map((genre) => {
      const li = el("li", `${genre.name} (${genre.count})`, genre.mode);
      li.dataset.name = genre.name;
      return li;
    }),
  );
  $("clear-genres").hidden = !view.genres.some((g) => g.mode);

  if (!restored) {
    restored = true;
    $("filter").value = view.filter;
    $("scope").value = view.scope;
  }
  markCurrent();
}

function markCurrent() {
  for (const li of $("tracks").children) {
    const current = Number(li.dataset.index) === status.current;
    li.classList.toggle("current", current);
    li.classList.toggle("playing", current && status.state === "Playing");
  }
}

function renderStatus(next) {
  const changed = next.current !== status.current || next.state !== status.state;
  status = next;

  $("play-icon").src = status.state === "Playing" ? "assets/pause.svg" : "assets/play.svg";
  $("stop").disabled = status.state === "Stopped";
  $("prev").disabled = !status.has_prev;
  $("next").disabled = !status.has_next;
  $("now").textContent = status.title ?? "";

  const duration = status.duration ?? 0;
  $("seek").max = duration;
  $("seek").disabled = duration === 0;
  if (!seeking) {
    $("seek").value = status.position;
    $("time").textContent = `${formatTime(status.position)} / ${formatTime(duration)}`;
  }
  if (document.activeElement !== $("volume")) $("volume").value = status.volume;

  $("loading").hidden = status.loaded >= status.queued;
  $("loading-text").textContent = `Loading ${status.loaded}/${status.queued}`;
  $("loading-bar").max = status.queued || 1;
  $("loading-bar").value = status.loaded;

  if (changed) markCurrent();
}

$("add-dir").onclick = () => invoke("pick", { folder: true });
$("add-files").onclick = () => invoke("pick", { folder: false });
$("shuffle").onclick = () => send("shuffle");
$("clear").onclick = () => send("clear");

$("filter").oninput = sendFilter;
$("scope").onchange = sendFilter;
$("genres-toggle").onclick = () => {
  $("genres").hidden = !$("genres").hidden;
  $("genres-toggle").classList.toggle("active", !$("genres").hidden);
};

$("genre-list").onclick = (e) => {
  const name = e.target.dataset.name;
  if (name) send("genre", { name, blacklist: e.shiftKey });
};
$("genre-list").oncontextmenu = (e) => {
  e.preventDefault();
  const name = e.target.dataset.name;
  if (name) send("genre", { name, blacklist: true });
};
$("clear-genres").onclick = () => send("clear_genres");

$("tracks").addEventListener(mobile ? "click" : "dblclick", (e) => {
  const li = e.target.closest("li");
  if (li && !e.target.matches(".remove")) send("play", { index: Number(li.dataset.index) });
});
$("tracks").addEventListener("click", (e) => {
  if (e.target.matches(".remove")) send("remove", { index: Number(e.target.parentNode.dataset.index) });
});

$("seek").oninput = () => {
  seeking = true;
  $("time").textContent = `${formatTime($("seek").value)} / ${formatTime(status.duration)}`;
};
$("seek").onchange = () => {
  seeking = false;
  send("seek", { secs: Number($("seek").value) });
};
$("volume").oninput = () => send("volume", { value: Number($("volume").value) });

$("prev").onclick = () => send("prev");
$("play").onclick = () => send("toggle");
$("stop").onclick = () => send("stop");
$("next").onclick = () => send("next");

document.addEventListener("keydown", (e) => {
  if (e.target.matches("input, select") || $("browser").open) return;
  const ctrl = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();

  if (key === " " || e.key === "MediaPlayPause") send("toggle");
  else if (e.key === "ArrowRight") send("seek_by", { secs: 0.5 });
  else if (e.key === "ArrowLeft") send("seek_by", { secs: -0.5 });
  else if (key === "j" && !ctrl) send("seek_by", { secs: 10 });
  else if (key === "l" && !ctrl) send("seek_by", { secs: -10 });
  else if (e.key === "MediaTrackNext") send("next");
  else if (e.key === "MediaTrackPrevious") send("prev");
  else if (ctrl && key === "o" && !mobile) invoke("pick", { folder: e.shiftKey });
  else if (ctrl && key === "s") send("shuffle");
  else return;
  e.preventDefault();
});

const trail = [];

async function openDir(path) {
  const entries = await invoke("list_dir", { path });
  $("cwd").textContent = path ?? "Storage";
  $("up").disabled = path === null;
  $("entries").replaceChildren(
    ...entries.map((entry) => {
      const li = el("li");
      const open = el("button", entry.name, entry.dir ? "open" : "open file");
      open.onclick = () => {
        if (!entry.dir) return send("add", { paths: [entry.path] });
        trail.push(entry.path);
        openDir(entry.path);
      };
      li.append(open);
      if (entry.dir) {
        const add = el("button", "Add all");
        add.onclick = () => {
          send("add", { paths: [entry.path] });
          $("browser").close();
        };
        li.append(add);
      }
      return li;
    }),
  );
}

$("browse").onclick = () => {
  openDir(trail.at(-1) ?? null);
  $("browser").showModal();
};
$("up").onclick = () => {
  trail.pop();
  openDir(trail.at(-1) ?? null);
};
$("close-browser").onclick = () => $("browser").close();

Promise.all([
  listen("playlist", (e) => renderPlaylist(e.payload)),
  listen("status", (e) => renderStatus(e.payload)),
]).then(() => send("ready"));
