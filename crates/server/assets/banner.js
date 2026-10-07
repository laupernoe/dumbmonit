/* DumbMonit status banner. Usage:
   <script async src="https://HOST/api/public/status/SLUG/banner.js"
     data-position="top|bottom" data-show="issues|always" data-lang="en|fr"></script> */
(function () {
  var s = document.currentScript;
  if (!s || !s.src) return;
  var api = s.src.split("?")[0].replace(/\/banner\.js$/, "");
  var d = s.dataset;
  var bottom = d.position === "bottom";
  var always = d.show === "always";
  var fr = (d.lang || document.documentElement.lang || "en").slice(0, 2) === "fr";
  var T = fr
    ? { ok: "Tous les systèmes sont opérationnels", maint: "Maintenance en cours",
        one: "1 service est en panne", many: " services sont en panne",
        deg: "Incident en cours", close: "Fermer", more: "Détails" }
    : { ok: "All systems operational", maint: "Maintenance in progress",
        one: "1 service is down", many: " services are down",
        deg: "Incident in progress", close: "Dismiss", more: "Details" };
  var key = "dmt-banner-" + api;
  try { if (sessionStorage.getItem(key) === "1") return; } catch (e) {}

  fetch(api, { credentials: "omit", headers: { Accept: "application/json" } })
    .then(function (r) { return r.ok ? r.json() : Promise.reject(); })
    .then(function (j) {
      var n = 0;
      (j.groups || []).forEach(function (g) {
        (g.items || []).forEach(function (i) {
          if (i.state === "down" || i.state === "degraded") n++;
        });
      });
      var tone = "ok", text = T.ok;
      if (j.overall === "maintenance") { tone = "info"; text = T.maint; }
      else if (n > 0) { tone = j.overall === "major" ? "bad" : "warn"; text = n === 1 ? T.one : n + T.many; }
      else if (j.overall !== "operational") { tone = "warn"; text = T.deg; }
      if (tone === "ok" && !always) return;

      var host = document.createElement("div");
      var root = host.attachShadow({ mode: "closed" });
      var css = document.createElement("style");
      css.textContent =
        ":host{all:initial}" +
        ".b{position:fixed;left:0;right:0;" + (bottom ? "bottom" : "top") + ":0;z-index:2147483000;" +
        "display:flex;gap:12px;align-items:center;justify-content:center;padding:10px 16px;" +
        "font:14px/1.4 system-ui,sans-serif;background:var(--bg);color:var(--fg)}" +
        ".ok{--bg:#e6f4ea;--fg:#14532d}.warn{--bg:#fef3c7;--fg:#78350f}" +
        ".bad{--bg:#fee2e2;--fg:#7f1d1d}.info{--bg:#dbeafe;--fg:#1e3a8a}" +
        "@media(prefers-color-scheme:dark){.ok{--bg:#14532d;--fg:#dcfce7}.warn{--bg:#78350f;--fg:#fef3c7}" +
        ".bad{--bg:#7f1d1d;--fg:#fee2e2}.info{--bg:#1e3a8a;--fg:#dbeafe}}" +
        "a{color:inherit;text-decoration:underline}" +
        "button{all:unset;cursor:pointer;padding:0 6px;font-size:18px;line-height:1}" +
        "button:focus-visible{outline:2px solid currentColor}";
      var bar = document.createElement("div");
      bar.className = "b " + tone;
      bar.setAttribute("role", "status");
      var span = document.createElement("span");
      span.textContent = text;
      var a = document.createElement("a");
      a.textContent = T.more;
      a.href = api.replace("/api/public/status/", "/s/");
      a.target = "_blank";
      a.rel = "noopener";
      var x = document.createElement("button");
      x.textContent = "×";
      x.setAttribute("aria-label", T.close);
      x.onclick = function () {
        host.remove();
        try { sessionStorage.setItem(key, "1"); } catch (e) {}
      };
      bar.append(span, a, x);
      root.append(css, bar);
      document.body.appendChild(host);
    })
    .catch(function () {});
})();
