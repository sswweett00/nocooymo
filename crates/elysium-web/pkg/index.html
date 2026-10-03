<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Elysium Engine — Real-Time Game Engine</title>
<style>
  :root {
    --bg: #05060c;
    --bg-elev: #0d0f1a;
    --glass: rgba(13,15,26,0.86);
    --border: rgba(255,255,255,0.09);
    --text: #e8eaf0;
    --muted: #8a90a6;
    --accent: #4f8efc;
    --accent-2: #2bd4a1;
    --danger: #f0586e;
    --radius: 16px;
    --shadow: 0 24px 64px rgba(0,0,0,.45);
    --font: 'Inter','Segoe UI',system-ui,-apple-system,sans-serif;
  }
  * { margin:0; padding:0; box-sizing:border-box; }
  html,body { height:100%; }
  body {
    background: radial-gradient(1200px 700px at 70% -10%, #11162b 0%, var(--bg) 55%);
    color: var(--text);
    font-family: var(--font);
    overflow: hidden;
    -webkit-font-smoothing: antialiased;
  }
  a { color: inherit; text-decoration: none; }

  /* Canvas viewport */
  #viewport {
    position: fixed; inset:0; z-index:1;
    background:#02030a;
  }
  #canvas { display:block; width:100%; height:100%; cursor:crosshair; }
  #canvas:focus { outline:2px solid var(--accent); }

  /* Overlay panel for tool info in viewport */
  #info-overlay {
    position:fixed; bottom 18px right 18px; z-index:5;
    background: var(--glass); border:1px solid var(--border);
    border-radius:12px; padding:12px 16px; backdrop-filter:blur(10px);
    font-size:12px; color:var(--muted); max-width:260px; pointer-events:none;
  }
  #info-overlay b { color:var(--text); }

  /* Navigation top */
  nav {
    position:fixed; top:0; left:0; right:0; z-index:20;
    display:flex; align-items:center; justify-content:space-between;
    padding:14px 24px;
  }
  .logo {
    font-weight:900; letter-spacing:3px; font-size:20px;
    background:linear-gradient(135deg,var(--accent),var(--accent-2));
    background-size:200% 200%; -webkit-background-clip:text; -webkit-text-fill-color:transparent;
    animation:shimmer 4s ease-in-out infinite;
  }
  .logo small { font-weight:600; -webkit-text-fill-color:var(--muted); font-size:12px; letter-spacing:1px; }
  @keyframes shimmer { 0%,100%{background-position:0% 50%} 50%{background-position:100% 50%} }
  .nav-links { display:flex; gap:18px; list-style:none; }
  .nav-links a { color:var(--muted); font-size:13px; font-weight:600; transition:color .2s; }
  .nav-links a:hover { color:var(--text); }
  .cta-btn {
    background:linear-gradient(135deg,var(--accent),#7aa9ff); color:#0a0c18;
    padding:10px 18px; border-radius:999px; font-weight:700; font-size:13px;
    border:none; cursor:pointer; transition:transform .15s, box-shadow .15s;
  }
  .cta-btn:hover { transform:translateY(-1px); box-shadow:0 8px 24px rgba(79,142,252,.35); }
  .cta-btn.secondary { background:rgba(255,255,255,.07); border:1px solid var(--border); color:var(--text); }
  .cta-btn.secondary:hover { background:rgba(255,255,255,.12); border-color:var(--accent); }

  /* Hero */
  .hero {
    position:fixed; top:50%; left:50%; transform:translate(-50%,-50%);
    z-index:5; width:min(960px, 100%); padding:64px 72px;
    display:grid; grid-template-columns: 1.15fr 0.85fr; gap:48px; align-items:center;
  }
  .hero h1 { font-size:clamp(34px, 5vw, 56px); line-height:1.08; font-weight:900; letter-spacing:-.5px; }
  .hero h1 .gradient { background:linear-gradient(135deg,var(--accent),var(--accent-2),#b08cff); -webkit-background-clip:text; -webkit-text-fill-color:transparent; background-clip:text; }
  .hero p.lead { color:var(--muted); font-size:17px; line-height:1.7; margin-top:18px; max-width:560px; }
  .hero .actions { display:flex; gap:14px; margin-top:32px; flex-wrap:wrap; }
  .hero .actions .btn { display:inline-flex; align-items:center; gap:10px; padding:14px 24px; border-radius:12px; font-weight:700; font-size:14px; cursor:pointer; transition:.18s; }
  .hero .actions .btn.primary { background:linear-gradient(135deg,var(--accent),#6f96f5); color:#0a0c18; box-shadow:0 10px 30px rgba(79,142,252,.3); }
  .hero .actions .btn.primary:hover { transform:translateY(-2px); box-shadow:0 14px 36px rgba(79,142,252,.4); }
  .hero .actions .btn.ghost { background:rgba(255,255,255,.06); border:1px solid var(--border); color:var(--text); }
  .hero .actions .btn.ghost:hover { background:rgba(255,255,255,.12); border-color:var(--accent); }
  .hero .badge-row { display:flex; gap:12px; margin-top:28px; flex-wrap:wrap; }
  .hero .badge-row span { background:rgba(255,255,255,.06); border:1px solid var(--border); padding:8px 14px; border-radius:10px; font-size:12px; color:var(--muted); font-weight:600; }

  .hero-visual { display:flex; align-items:center; justify-content:center; }
  .grid-kv { display:grid; grid-template-columns:repeat(2,1fr); gap:14px; }
  .kv { background:rgba(255,255,255,.05); border:1px solid var(--border); border-radius:12px; padding:16px; }
  .kv .k { font-size:10px; text-transform:uppercase; letter-spacing:1.5px; color:var(--muted); font-weight:700; }
  .kv .v { font-size:15px; font-weight:700; color:var(--text); margin-top:6px; }
  .kv.accent .v { color:var(--accent); }
  .kv.accent-2 .v { color:var(--accent-2); }
  .kv .sub { font-size:11px; color:var(--muted); margin-top:4px; }

  /* Features */
  .features { padding:72px 24px; max-width:1100px; margin:0 auto; }
  .features-header { text-align:center; margin-bottom:44px; }
  .features-header h2 { font-size:34px; font-weight:900; letter-spacing:-.5px; }
  .features-header p { color:var(--muted); font-size:16px; margin-top:10px; }
  .grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(260px,1fr)); gap:20px; }
  .feat { padding:26px; border:1px solid var(--border); border-radius:14px; background:rgba(255,255,255,.03); transition:.2s; }
  .feat:hover { border-color:var(--accent); background:rgba(79,142,252,.06); transform:translateY(-3px); }
  .feat .icon { font-size:24px; margin-bottom:14px; }
  .feat h3 { font-size:18px; font-weight:800; margin-bottom:8px; }
  .feat p { color:var(--muted); font-size:14px; line-height:1.6; }

  /* Showcase */
  .showcase { padding:64px 24px; background:rgba(255,255,255,.015); }
  .showcase-inner { max-width:1180px; margin:0 auto; }
  .showcase-top { display:flex; justify-content:space-between; align-items:center; gap:24px; flex-wrap:wrap; margin-bottom:32px; }
  .showcase-top h2 { font-size:28px; font-weight:900; }
  .showcase-top p { color:var(--muted); font-size:15px; }
  .showcase-cards { display:grid; grid-template-columns:repeat(auto-fit,minmax(220px,1fr)); gap:18px; }
  .card { background:linear-gradient(180deg,rgba(255,255,255,.04),rgba(255,255,255,.01)); border:1px solid var(--border); border-radius:14px; padding:22px; }
  .card .t { font-weight:800; font-size:16px; margin-bottom:8px; }
  .card .d { font-size:13px; color:var(--muted); line-height:1.6; }

  /* CTA / Auth wall */
  .auth {
    position:fixed; inset:0; z-index:15; display:flex; align-items:center; justify-content:center;
    padding:24px; background:var(--bg); overflow:hidden;
  }
  .auth-card {
    width:min(460px,100%); background:linear-gradient(180deg,rgba(16,18,30,.96),rgba(9,11,22,.94));
    border:1px solid var(--border); border-radius:20px; padding:40px 36px;
    box-shadow:var(--shadow); backdrop-filter:blur(18px);
  }
  .auth-card h2 { font-size:26px; font-weight:900; margin-bottom:6px; }
  .auth-card .sub { color:var(--muted); font-size:15px; margin-bottom:24px; }
  .input-wrap { position:relative; margin-bottom:16px; }
  .input-wrap label { font-size:12px; color:var(--muted); font-weight:700; text-transform:uppercase; letter-spacing:1px; display:block; margin-bottom:7px; }
  .input-wrap input, .auth-card button { width:100%; padding:13px 14px; border-radius:10px; border:1px solid var(--border); background:rgba(255,255,255,.05); color:var(--text); font-family:var(--font); font-size:15px; outline:none; transition:.2s; }
  .input-wrap input:focus, .auth-card button:focus { border-color:var(--accent); box-shadow:0 0 0 3px rgba(79,142,252,.2); }
  .auth-card button { background:linear-gradient(135deg,var(--accent),#6f96f5); color:#0a0c18; font-weight:700; font-size:15px; cursor:pointer; border:none; transition:.2s; }
  .auth-card button:hover { transform:translateY(-1px); box-shadow:0 8px 24px rgba(79,142,252,.35); }
  .auth-card .link { margin-top:16px; text-align:center; font-size:14px; color:var(--muted); }
  .auth-card .link a { color:var(--accent); font-weight:700; }
  .auth-card .spinner { display:inline-block; width:16px; height:16px; border:2px solid rgba(255,255,255,.25); border-top-color:var(--accent); border-radius:50%; animation:spin .8s linear infinite; margin-right:8px; vertical-align:middle; }
  @keyframes spin { to{transform:rotate(360deg)} }
  .auth-footer { margin-top:20px; text-align:center; font-size:12px; color:var(--muted); }

  /* Footer */
  footer { position:fixed; bottom:0; left:0; right:0; z-index:20; padding:18px 24px; display:flex; justify-content:space-between; font-size:12px; color:var(--muted); }

  @media (max-width:900px) {
    .hero { grid-template-columns:1fr; padding:40px 24px; }
    .hero-visual { order:-1; }
    .grid-kv { grid-template-columns:repeat(2,1fr); }
    .nav-links { display:none; }
    .features, .showcase { padding:48px 18px; }
  }
  @media (max-width:520px) {
    .grid-kv { grid-template-columns:1fr; }
  }
</style>
</head>
<body>
  <div id="viewport">
    <canvas id="canvas" tabindex="0" aria-label="Elysium Engine real-time viewport"></canvas>
    <div id="info-overlay"><b>Elysium Engine</b> &middot; Real-time 3D viewport</div>
  </div>

  <nav>
    <div class="logo">Elysium<small>Real-time Engine</small></div>
    <ul class="nav-links">
      <li><a href="#features">Features</a></li>
      <li><a href="#showcase">Gallery</a></li>
      <li><a href="#auth">Getstarted</a></li>
    </ul>
    <a class="cta-btn" onclick="document.getElementById('auth').scrollIntoView({behavior:'smooth'})">Sign in to dashboard</a>
  </nav>

  <section class="hero">
    <div>
      <h1>Build worlds in <span class="gradient">real time</span>.</h1>
      <p class="lead">Elysium is a complete game engine: ECS, real-time rendering, physics, skeletal animation, terrain, and multiplayer — all in one workspace.</p>
      <div class="actions">
        <button class="btn primary" onclick="document.getElementById('auth').scrollIntoView({behavior:'smooth'})">Open the editor</button>
        <button class="btn ghost" onclick="document.getElementById('features').scrollIntoView({behavior:'smooth'})">Explore features</button>
      </div>
      <div class="badge-row">
        <span>ECS + scene graph</span>
        <span>Software rasterizer</span>
        <span>Photon physics</span>
        <span>WebGL2 build</span>
      </div>
    </div>
    <div class="hero-visual">
      <div class="grid-kv">
        <div class="kv"><div class="k">Entities</div><div class="v">120</div><div class="sub">spawned &amp; animating</div></div>
        <div class="kv accent"><div class="k">FPS</div><div class="v">60</div><div class="sub">target gameplay loop</div></div>
        <div class="kv"><div class="k">Terrain</div><div class="v">auto</div><div class="sub">procedural generation</div></div>
        <div class="kv accent-2"><div class="k">Render</div><div class="v">PBR</div><div class="sub">shadows &amp; fog</div></div>
      </div>
    </div>
  </section>

  <section class="features" id="features">
    <div class="features-header">
      <h2>Engine under the hood</h2>
      <p>Every subsystem is wired together and ready to ship.</p>
    </div>
    <div class="grid">
      <div class="feat"><div class="icon">🔷</div><h3>ECS core</h3><p>Archetype-based entities, command buffers, fixed timestep scheduling, and thread-safe component storage.</p></div>
      <div class="feat"><div class="icon">🖥️</div><h3>Software renderer</h3><p>Full PBR rasterizer with skybox, fog, post-processing, FXAA, bloom, and 2D UI overlay in one framebuffer.</p></div>
      <div class="feat"><div class="icon">⚡</div><h3>Real-time loop</h3><p>Delta-time stepping, performance monitoring, pause/play, and camera tracking with orbit controls.</p></div>
      <div class="feat"><div class="icon">🌍</div><h3>Terrain &amp; physics</h3><p>Procedural terrain, XPBD-style solver, collision, joints, fluids, vehicles, and raycasting.</p></div>
      <div class="feat"><div class="icon">🦴</div><h3>Animation</h3><p>Skeleton rigging, blend state machines, keyframe timelines, skinned meshes, and IK.</p></div>
      <div class="feat"><div class="icon">🔌</div><h3>Multiplayer</h3><p>Netcode, networked transforms, state replication, and WebSocket transport.</p></div>
    </div>
  </section>

  <section class="showcase" id="showcase">
    <div class="showcase-inner">
      <div class="showcase-top">
        <h2>From demo to production</h2>
        <p>A complete engine, not a starter kit.</p>
      </div>
      <div class="showcase-cards">
        <div class="card"><div class="t">Scene editor</div><div class="d">Hierarchy, inspector, gizmos, box selection, undo/redo, and a full console.</div></div>
        <div class="card"><div class="t">Asset pipeline</div><div class="d">OBJ/GLB import, texture baking, cookers, and runtime streaming.</div></div>
        <div class="card"><div class="t">Audio &amp; 3D</div><div class="d">Spatial mixing, reverb zones, music engine, and voice chat.</div></div>
        <div class="card"><div class="t">Visual scripting</div><div class="d">Node graphs, animation blending, cinematics, and scripting plugins.</div></div>
      </div>
    </div>
  </section>

  <div class="auth" id="auth">
    <div class="auth-card">
      <h2>Welcome back</h2>
      <p class="sub">Open the engine and start building.</p>
      <form id="login-form">
        <div class="input-wrap">
          <label>Email</label>
          <input type="email" id="email" placeholder="you@example.com" autocomplete="email">
        </div>
        <div class="input-wrap">
          <label>Password</label>
          <input type="password" id="password" placeholder="••••••••" autocomplete="current-password">
        </div>
        <button type="submit">Sign in</button>
        <div class="link">New to Elysium? <a href="#" id="open-signup">Create account</a></div>
        <div class="spinner" id="spinner" style="display:none;"></div>
      </form>
      <div class="auth-footer">Connected workspace · managed GitHub App creds in use</div>
    </div>
  </div>

  <footer>
    <span>Elysium Engine — v2.0 Engine kit</span>
    <span>© 2026 Elysium Contributors</span>
  </footer>

  <script type="module">
    const form = document.getElementById('login-form');
    const spinner = document.getElementById('spinner');
    const openSignup = document.getElementById('open-signup');
    form.addEventListener('submit', async (e) => {
      e.preventDefault();
      const email = document.getElementById('email').value.trim();
      const password = document.getElementById('password').value;
      if (!email || !password) return;
      spinner.style.display = 'inline-block';
      form.querySelector('button').disabled = true;
      try {
        const res = await fetch('/api/auth/login', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, password })
        });
        if (!res.ok) throw new Error('Auth failed');
        const data = await res.json();
        window.location.href = data.returnTo || '/dashboard';
      } catch (err) {
        alert(err.message);
        spinner.style.display = 'none';
        form.querySelector('button').disabled = false;
      }
    });
    openSignup.addEventListener('click', (e) => {
      e.preventDefault();
      const card = document.querySelector('.auth-card');
      const h3 = document.createElement('h3');
      h3.textContent = 'Create account';
      const email2 = document.createElement('div');
      email2.innerHTML = '<div class="input-wrap"><label>Email</label><input type="email" id="email2" placeholder="you@example.com" autocomplete="email"></div>';
      const pass2 = document.createElement('div');
      pass2.innerHTML = '<div class="input-wrap"><label>Password</label><input type="password" id="password2" placeholder="Min 8 characters" autocomplete="new-password"></div>';
      const btn2 = document.createElement('button');
      btn2.type = 'button';
      btn2.textContent = 'Sign up';
      btn2.style.background = 'linear-gradient(135deg,#2bd4a1,#55d89a)';
      btn2.addEventListener('click', async () => {
        const email = document.getElementById('email2').value.trim();
        const password = document.getElementById('password2').value;
        if (!email || !password) return;
        btn2.disabled = true; btn2.textContent = 'Creating…';
        try {
          const res = await fetch('/api/auth/signup', { method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({ email, password }) });
          const data = await res.json();
          if (!res.ok) throw new Error(data.error || 'Signup failed');
          window.location.href = data.returnTo || '/dashboard';
        } catch (err) { alert(err.message); btn2.disabled = false; btn2.textContent = 'Sign up'; }
      });
      card.replaceChild(email2, card.querySelector('.input-wrap:first-child'));
      card.replaceChild(pass2, card.querySelector('.input-wrap:nth-child(2)'));
      card.insertBefore(h3, card.firstChild);
      card.querySelector('form').replaceChild(btn2, card.querySelector('button'));
      card.querySelector('.link').remove();
      card.querySelector('.auth-footer').remove();
    });
  </script>
</body>
</html>
